"""Matched full native Actor calls: per-value versus owner-batched projection/retirement."""
import argparse, collections, json, math, os, statistics, threading, time
import gerbil_ledger
from benchmark_host import host_snapshot
from benchmark_metrics import assigned_owner_fairness, pending_completion_gaps


def percentile(values, p=.99):
    return sorted(values)[max(0, math.ceil(len(values)*p)-1)] if values else None


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--owners',type=int,default=4)
    parser.add_argument('--workers',type=int,default=2)
    parser.add_argument('--rounds',type=int,default=16)
    parser.add_argument('--requests',type=int,default=128)
    parser.add_argument('--cpu',action='store_true')
    parser.add_argument('--probes',action='store_true')
    parser.add_argument('--probe-interval-ms',type=float,default=20)
    args=parser.parse_args()
    if min(args.owners,args.workers,args.requests,args.rounds)<1 or args.rounds%4 or args.probe_interval_ms<=0:
        parser.error('positive counts, rounds divisible by four and positive probe interval required')
    def emit(**record):print(json.dumps(record),flush=True)
    library=gerbil_ledger.open_library(owners=args.owners,dispatch_workers=args.workers)
    states=library.vm_states
    stop=threading.Event();lock=threading.Lock();active={};probe_records=[];failures=[]
    sessions=[library.session(owner) for owner in range(args.owners)]
    def probe(owner):
        serial=0
        while not stop.is_set():
            with lock:tag=dict(active)
            started=time.perf_counter_ns();root=None
            try:
                root=sessions[owner].call(('echo',serial))
                assert root.to_python()==serial
                root.close();root=None
                ended=time.perf_counter_ns()
                with lock:probe_records.append(dict(owner=owner,start_ns=started,end_ns=ended,ns=ended-started,**tag))
            except BaseException as error:
                with lock:failures.append(dict(owner=owner,error=repr(error),**tag))
                if root is not None:root.close()
                stop.set();return
            serial+=1
            stop.wait(args.probe_interval_ms/1000)
    threads=[threading.Thread(target=probe,args=(owner,),daemon=True) for owner in range(args.owners)] if args.probes else []
    records=[]
    emit(kind='configuration',**vars(args),states=states,host_activity=host_snapshot(),
         timing_contract='same live VMs and direct stateless admission in both arms; bulk API start through projected/released response; scalar emits per-result completion, owner-batch completes all at bulk return',
         probes_contract='one outstanding echo per owner, 20ms think interval by default; closed-loop fairness diagnostic, not open-loop request p99')
    try:
        for thread in threads:thread.start()
        items=tuple((i,tuple((j,3) for j in range(48)),not args.cpu) for i in range(32))
        requests=[('actors',items,1)]*args.requests
        for round_index in range(args.rounds+2):
            warmup=round_index<2
            arms=['scalar','owner-batch'] if round_index%4 in (0,3) else ['owner-batch','scalar']
            for arm in arms:
                busy_before=host_snapshot()
                with lock:active.clear();active.update(arm=arm,round=round_index,warmup=warmup,stage='execute')
                started=time.perf_counter_ns()
                roots,ready=library.call_many_profiled(requests,dispatch='direct')
                returned=time.perf_counter_ns()
                owners=[root.owner for root in roots]
                metadata_done=time.perf_counter_ns()
                with lock:active['stage']='project'
                rows=[];responses=[]
                if arm=='scalar':
                    for root in roots:
                        rows.append(root.to_python());root.close()
                        responses.append(time.perf_counter_ns()-started)
                else:
                    rows=library.project_many(roots,release=True)
                    responses=[time.perf_counter_ns()-started]*len(roots)
                ended=time.perf_counter_ns()
                with lock:active.clear()
                expected=[(owners[0]+i)%args.owners for i in range(args.requests)]
                assert owners==expected
                actor_gaps=[0]*4;worker_counts=[0]*4
                for row in rows:
                    assert [item[0] for item in row]==list(range(32))
                    assert {item[1] for item in row}=={3384}
                    assert sorted(item[2] for item in row)==list(range(1,33))
                    for item in row:worker_counts[item[3]]+=1
                    gap=pending_completion_gaps([item[3] for item in sorted(row,key=lambda item:item[2])],4)
                    actor_gaps=[max(old,new) for old,new in zip(actor_gaps,gap['max_other_completions_while_pending'])]
                assert worker_counts==[args.requests*8]*4
                record=dict(kind='sample',arm=arm,round=round_index,warmup=warmup,start_ns=started,end_ns=ended,
                    dispatch_api_ns=returned-started,owner_metadata_ns=metadata_done-returned,
                    projection_release_ns=ended-metadata_done,response_ns=responses,
                    native_ready_ns=ready,owners=owners,
                    actors_per_second=args.requests*32*1e9/(ended-started),
                    native_ready_p99_ns=percentile(ready),response_p99_ns=percentile(responses),
                    owner_fairness=assigned_owner_fairness(ready,args.owners,first_owner=owners[0]),
                    actor_max_other_completions_while_pending=actor_gaps,worker_counts=worker_counts,
                    host_activity_before=busy_before,host_activity_after=host_snapshot())
                records.append(record);emit(**record)
                if failures:raise RuntimeError(f'probe failure: {failures!r}')
        stop.set()
        for thread in threads:thread.join(10)
        assert not any(thread.is_alive() for thread in threads)
        assert not failures,failures
        for record in probe_records:emit(kind='probe',**record)
        summary={}
        for arm in ('scalar','owner-batch'):
            arm_rows=[r for r in records if not r['warmup'] and r['arm']==arm]
            probes=[r for r in probe_records if r.get('arm')==arm and not r.get('warmup',True)]
            projection_probes=[r for r in probes if r.get('stage')=='project']
            summary[arm]=dict(samples=len(arm_rows),actors_per_second_median=statistics.median(r['actors_per_second'] for r in arm_rows),
                response_p99_ns=percentile([n for r in arm_rows for n in r['response_ns']]),
                native_ready_p99_ns=percentile([n for r in arm_rows for n in r['native_ready_ns']]),
                projection_release_ns_median=statistics.median(r['projection_release_ns'] for r in arm_rows),
                probe_count=len(probes),probe_p99_ns=percentile([r['ns'] for r in probes]),
                projection_probe_count=len(projection_probes),projection_probe_p99_ns=percentile([r['ns'] for r in projection_probes]),
                probe_counts_by_owner=[sum(r['owner']==owner for r in probes) for owner in range(args.owners)],
                probe_p99_ns_by_owner=[percentile([r['ns'] for r in probes if r['owner']==owner]) for owner in range(args.owners)],
                owner_max_pending_gap_ns=[max(r['owner_fairness']['assigned_owner_max_pending_gap_ns'][owner] for r in arm_rows) for owner in range(args.owners)],
                owner_max_other_completions_while_pending=[max(r['owner_fairness']['assigned_owner_max_other_completions_while_pending'][owner] for r in arm_rows) for owner in range(args.owners)],
                actor_max_other_completions_while_pending=[max(r['actor_max_other_completions_while_pending'][worker] for r in arm_rows) for worker in range(4)])
        assert library.vm_states==states
        emit(kind='gc',usage=library.collect())
        path=library.runtime_directory;library.close()
        assert library.shutdown_clean and library.shutdown_roots==[0]*args.owners
        assert not os.path.exists(path)
        emit(kind='summary',summary=summary,roots=library.shutdown_roots,states=states,probe_failures=failures)
    finally:
        stop.set()
        for thread in threads:
            if thread.is_alive():thread.join(10)
        library.close()

if __name__=='__main__':main()
