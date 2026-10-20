"""Same-VM read-only Scheme Actor load: fresh graphs versus owner-local native handles."""
import argparse,concurrent.futures,hashlib,json,math,os,platform,statistics,sys,threading,time
from pathlib import Path
import gerbil_ledger
from benchmark_host import host_snapshot

def percentile(v):return sorted(v)[max(0,math.ceil(len(v)*.99)-1)]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--rounds",type=int,default=16,help="Balanced measured rounds; positive even number")
parser.add_argument("--owner-trace",action="store_true",help="Diagnostic run only; profiling perturbs performance")
args=parser.parse_args()
if args.rounds<2 or args.rounds%2:parser.error("--rounds must be an even number >= 2")
def identity(path):
    path=Path(path)
    return dict(path=str(path),sha256=hashlib.sha256(path.read_bytes()).hexdigest())
started=time.perf_counter_ns();library=gerbil_ledger.open_library();initialization=time.perf_counter_ns()-started
sessions=[library.session(i) for i in range(4)];states=library.vm_states
items=tuple((i,tuple((j,3) for j in range(48)),False) for i in range(32))
started=time.perf_counter_ns();payloads=[session.value(items) for session in sessions];preparation=time.perf_counter_ns()-started
main_pool=concurrent.futures.ThreadPoolExecutor(max_workers=4)
probe_pool=concurrent.futures.ThreadPoolExecutor(max_workers=32)
records=[];probes=[]
library.set_owner_profiling(args.owner_trace)
def emit(**row):print(json.dumps(row),flush=True)
def job(owner,arm,wake):
    wake.wait()
    request=("actors",payloads[owner] if arm=="native" else items,1)
    roots=sessions[owner].call_many([request]*32)
    return roots,time.perf_counter_ns()
def probe(owner,index,wake,clock):
    wake.wait();due=clock[0]+index*20000000
    delay=(due-time.perf_counter_ns())/1e9
    if delay>0:time.sleep(delay)
    start=time.perf_counter_ns();root=sessions[owner].call(("echo",index))
    assert root.to_python()==index;root.close();end=time.perf_counter_ns()
    return dict(owner=owner,index=index,ns=end-due,service_ns=end-start,lateness_ns=start-due,completed_at_ns=end-clock[0])
try:
    emit(kind="configuration",schema="owner-native-payload-v2",measured_rounds=args.rounds,warmup_rounds=2,
        tracing=args.owner_trace,python=dict(executable=sys.executable,version=sys.version),platform=platform.platform(),
        artifacts=dict(harness=identity(__file__),extension=identity(gerbil_ledger._core.__file__)),states=states,initialization_ns=initialization,native_payload_prepare_ns=preparation,
        host=host_snapshot(),contract="same four VMs; four pinned API clients; 128 requests, 32 Actor items, 48 pairs, quantum 1, no sleep; input graph is read-only; native arm reuses one rooted graph per owner, never shares Scheme objects between owners; both arms use session.call_many and bulk projection/release; preparation excluded from warm reuse timing")
    for round_index in range(args.rounds+2):
        warmup=round_index<2
        for arm in (["fresh","native"] if round_index%4 in (0,3) else ["native","fresh"]):
            wake=threading.Event();clock=[0]
            main=[main_pool.submit(job,owner,arm,wake) for owner in range(4)]
            short=[probe_pool.submit(probe,owner,index,wake,clock) for owner in range(4) for index in range(8)]
            begin=time.perf_counter_ns();clock[0]=begin;wake.set()
            outputs=[future.result(timeout=20) for future in main]
            roots=[root for values,_ in outputs for root in values]
            assert [root.owner for root in roots]==[owner for owner in range(4) for _ in range(32)]
            returned=time.perf_counter_ns()
            rows=library.project_many(roots,release=True);end=time.perf_counter_ns()
            cohort=[future.result(timeout=20) for future in short]
            assert len(cohort)==32
            assert len(rows)==128
            assert [sum(r['owner']==o for r in cohort) for o in range(4)]==[8]*4
            if args.owner_trace:
                trace=json.loads(library.owner_profile())
                assert trace['dropped']==0 and all(not span['failed'] for span in trace['spans'])
                emit(kind="owner-trace",arm=arm,round=round_index,warmup=warmup,**trace)
            for row in rows:
                assert [item[0] for item in row]==list(range(32))
                assert {item[1] for item in row}=={3384}
                assert sorted(item[2] for item in row)==list(range(1,33))
                assert [sum(item[3]==worker for item in row) for worker in range(4)]==[8]*4
            row=dict(kind="sample",arm=arm,round=round_index,warmup=warmup,
                ns=end-begin,client_return_ns=returned-begin,projection_release_ns=end-returned,
                probes_completed_before_main_end=sum(r['completed_at_ns']<=end-begin for r in cohort),
                actors_per_second=4096e9/(end-begin),owner_client_response_ns=[at-begin for _,at in outputs],
                probe_p99_ns=percentile([r['ns'] for r in cohort]),probe_count=32,host=host_snapshot())
            records.append(row);emit(**row)
            for result in cohort:probes.append(dict(kind="probe",arm=arm,round=round_index,warmup=warmup,**result))
    for row in probes:emit(**row)
    summary={}
    for arm in ("fresh","native"):
        rr=[r for r in records if r['arm']==arm and not r['warmup']]
        pp=[r for r in probes if r['arm']==arm and not r['warmup']]
        summary[arm]=dict(client_return_median_ns=statistics.median(r['client_return_ns'] for r in rr),
            projection_release_median_ns=statistics.median(r['projection_release_ns'] for r in rr),
            probes_completed_before_main_end=sum(r['probes_completed_before_main_end'] for r in rr),
            actors_per_second_median=statistics.median(r['actors_per_second'] for r in rr),main_response_p99_ns=percentile([r['ns'] for r in rr]),probe_p99_ns=percentile([r['ns'] for r in pp]),probe_counts_by_owner=[sum(r['owner']==o for r in pp) for o in range(4)],probe_p99_ns_by_owner=[percentile([r['ns'] for r in pp if r['owner']==o]) for o in range(4)])
    usage=library.collect()
    for owner in range(4):
        root=sessions[owner].call(("actors",payloads[owner],1));assert {r[1] for r in root.to_python()}=={3384};root.close()
    for value in payloads:value.close()
    assert library.vm_states==states
    path=library.runtime_directory;library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0]*4 and not os.path.exists(path)
    emit(kind="summary",throughput_ratio=summary['native']['actors_per_second_median']/summary['fresh']['actors_per_second_median'],
        units="Actor work items/s; 32 items/request; response times in ns",tracing=args.owner_trace,summary=summary,usage=usage,roots=library.shutdown_roots,files_removed=True)
finally:
    main_pool.shutdown(wait=True);probe_pool.shutdown(wait=True)
    for value in payloads:value.close()
    library.close()
