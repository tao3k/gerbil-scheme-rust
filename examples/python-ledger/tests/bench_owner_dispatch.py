"""Matched installed native Actor workload; no Scheme compiler is invoked."""
import argparse,json, math, os,statistics, subprocess,time
import gerbil_ledger
from benchmark_host import host_snapshot
from benchmark_metrics import assigned_owner_fairness, pending_completion_gaps

def percentile(values,p):
    ordered=sorted(values)
    return ordered[max(0,math.ceil(len(ordered)*p)-1)]

def run():
    parser=argparse.ArgumentParser()
    parser.add_argument("--owners",type=int,default=4)
    parser.add_argument("--workers",type=int,default=2)
    parser.add_argument("--requests",type=int,default=128)
    parser.add_argument("--items",type=int,default=32)
    parser.add_argument("--quantum",type=int,default=1)
    parser.add_argument("--cpu",action="store_true")
    parser.add_argument("--require-quiet",action="store_true")
    parser.add_argument("--rounds",type=int,default=4)
    args=parser.parse_args()
    if args.rounds < 4 or args.rounds % 4:
        parser.error("--rounds must be a positive multiple of four")
    if min(args.owners,args.workers,args.requests,args.items) < 1:
        parser.error("counts must be positive")
    busy=host_snapshot() if args.require_quiet else []
    if busy:
        print(json.dumps(dict(kind="blocked",reason="host is busy",host_activity=busy)),flush=True)
        raise SystemExit(75)
    host = host_snapshot
    before=host();opened=time.perf_counter_ns()
    library=gerbil_ledger.open_library(owners=args.owners,dispatch_workers=args.workers)
    print(json.dumps(dict(kind="configuration",owners=args.owners,workers=args.workers,requests=args.requests,items=args.items,quantum=args.quantum,rounds=args.rounds,require_quiet=args.require_quiet,mixed=not args.cpu,timing_contract="native-ready within 128-request windows; whole-batch-start to projected-and-released response",initialization_ns=time.perf_counter_ns()-opened,host_activity=before)),flush=True)
    states=library.vm_states
    items=tuple((i,tuple((j,3) for j in range(48)),not args.cpu) for i in range(args.items))
    requests=[("actors",items,args.quantum) for _ in range(args.requests)]
    records=[]
    contaminated=bool(before)
    modes=["direct","tokio","rayon","rayon-blocking"]
    measured=[modes,list(reversed(modes)),modes[1:]+modes[:1],list(reversed(modes[1:]+modes[:1]))]
    orders=[measured[round_index % 4] for round_index in range(args.rounds)]
    for index,mode in enumerate(modes+[value for order in orders for value in order]):
        active_before=host()
        start=time.perf_counter_ns()
        roots,ready_ns=library.call_many_profiled(requests,dispatch=mode)
        returned=time.perf_counter_ns()
        first_owner=roots[0].owner if roots else 0
        response_ns=[]
        projected_rows=[]
        for root in roots:
            try:
                rows=root.to_python()
            finally:
                root.close()
            response_ns.append(time.perf_counter_ns()-start)
            projected_rows.append(rows)
        projected=time.perf_counter_ns()
        # Check and summarize after response timing, outside the measured path.
        displacement=[];worker_counts=[0]*4
        actor_pending_gaps=[0]*4
        for rows in projected_rows:
            assert [row[0] for row in rows]==list(range(args.items))
            assert {row[1] for row in rows}=={3384}
            assert sorted(row[2] for row in rows)==list(range(1,args.items+1))
            for ident,total,rank,worker in rows:
                displacement.append(abs(rank-1-ident));worker_counts[worker]+=1
            completion=pending_completion_gaps([row[3] for row in sorted(rows,key=lambda row: row[2])],4)
            actor_pending_gaps=[max(old,new) for old,new in zip(actor_pending_gaps,completion["max_other_completions_while_pending"])]
        expected=[args.requests*len(range(worker,args.items,4)) for worker in range(4)]
        assert worker_counts==expected
        completion_coverage=sum(worker_counts)**2/(4*sum(n*n for n in worker_counts))
        record=dict(index=index,mode=mode,warmup=index<4,requests=args.requests,actors=args.requests*args.items,
            dispatch_api_ns=returned-start,projection_release_ns=projected-returned,
            ready_ns=ready_ns,ready_p99_ns=percentile(ready_ns,.99),
            actors_per_second=(args.requests*args.items)/((returned-start)/1e9),
            projected_actors_per_second=(args.requests*args.items)/((projected-start)/1e9),
            rank_displacement_p99=percentile(displacement,.99),worker_counts=worker_counts,jain_worker_completion_coverage=completion_coverage,
            actor_worker_max_other_completions_while_pending=actor_pending_gaps,
            assigned_owner_fairness=assigned_owner_fairness(ready_ns,args.owners,first_owner=first_owner),
            batch_projected_response_ns=response_ns,
            batch_projected_response_p99_ns=percentile(response_ns,.99),
            host_activity_before=active_before,host_activity_after=host(),os_threads=max(0,len(subprocess.check_output(["/bin/ps","-M","-p",str(os.getpid())],text=True).splitlines())-1))
        contaminated |= bool(record["host_activity_before"] or record["host_activity_after"])
        records.append(record)
        print(json.dumps(record),flush=True)
    assert library.vm_states==states
    print(json.dumps(dict(kind="post-gc",usage=library.collect())),flush=True)
    path=library.runtime_directory;library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0]*args.owners
    assert not os.path.exists(path)
    summary={}
    for mode in modes:
        rows=[r for r in records if not r["warmup"] and r["mode"]==mode]
        summary[mode]=dict(actors_per_second_median=statistics.median(r["actors_per_second"] for r in rows),
            projected_actors_per_second_median=statistics.median(r["projected_actors_per_second"] for r in rows),
            pooled_native_ready_p99_ns=percentile([n for r in rows for n in r["ready_ns"]],.99),
            projection_release_ns_median=statistics.median(r["projection_release_ns"] for r in rows),
            pooled_batch_projected_response_p99_ns=percentile([n for r in rows for n in r["batch_projected_response_ns"]],.99),
            assigned_owner_max_pending_gap_ns=[max(r["assigned_owner_fairness"]["assigned_owner_max_pending_gap_ns"][owner] for r in rows) for owner in range(args.owners)],
            assigned_owner_max_other_completions_while_pending=[max(r["assigned_owner_fairness"]["assigned_owner_max_other_completions_while_pending"][owner] for r in rows) for owner in range(args.owners)],
            actor_worker_max_other_completions_while_pending=[max(r["actor_worker_max_other_completions_while_pending"][worker] for r in rows) for worker in range(4)],
            rank_displacement_p99=max(r["rank_displacement_p99"] for r in rows))
    after=host()
    contaminated |= bool(after)
    print(json.dumps(dict(kind="summary",states=states,summary=summary,host_activity_after=after,
        quiet_snapshots=not contaminated,
        performance_admission="REJECTED_HOST_BUSY" if contaminated else "QUIET_SNAPSHOTS")),flush=True)
    return 75 if args.require_quiet and contaminated else 0

if __name__=="__main__":raise SystemExit(run())
