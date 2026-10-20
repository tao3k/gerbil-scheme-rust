"""Matched direct/Tokio/Rayon dispatch with actual owner-resident Scheme inputs."""
import argparse
import concurrent.futures
import hashlib
import json
import math
import os
import statistics
import subprocess
import threading
import time
from pathlib import Path

import gerbil_ledger
from benchmark_host import host_snapshot

MODES=("direct","tokio","rayon","rayon-blocking")
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--owners",type=int,default=4)
parser.add_argument("--workers",type=int,default=2)
parser.add_argument("--rounds",type=int,default=8)
parser.add_argument("--lines",type=int,default=48)
parser.add_argument("--admission-window",type=int,default=16)
parser.add_argument("--mixed",action="store_true")
a=parser.parse_args()
if a.owners not in (1,2,4,8) or a.workers not in (1,2,4,8) or a.rounds<4 or a.rounds%4 or a.lines<1:parser.error("owners/workers in 1,2,4,8; rounds a positive multiple of four; lines positive")
if not 1<=a.admission_window<=128:parser.error("window in 1..128")
def emit(**row):print(json.dumps(row),flush=True)
def p99(v):return sorted(v)[math.ceil(.99*len(v))-1]
opened=time.perf_counter_ns();library=gerbil_ledger.open_library(owners=a.owners,dispatch_workers=a.workers);initialization=time.perf_counter_ns()-opened
sessions=[library.session(o) for o in range(a.owners)];states=library.vm_states
items=tuple((i,tuple((j,3) for j in range(a.lines)),a.mixed) for i in range(32))
started=time.perf_counter_ns();payloads=[s.value(items) for s in sessions];preparation=time.perf_counter_ns()-started
records=[];probes=[]

def bulk(owner,mode,wake):
    wake.wait()
    roots=sessions[owner].call_many([("actors",payloads[owner],1)]*(128//a.owners),dispatch=mode,admission_window=a.admission_window)
    return roots,time.perf_counter_ns()
def short(owner,index,wake,clock):
    wake.wait();due=clock[0]+index*20000000
    delay=(due-time.perf_counter_ns())/1e9
    if delay>0:time.sleep(delay)
    root=sessions[owner].call(("echo",index));assert root.to_python()==index;root.close()
    return dict(owner=owner,ns=time.perf_counter_ns()-due)

try:
    emit(kind="configuration",owners=a.owners,workers=a.workers,lines=a.lines,mixed=a.mixed,rounds=a.rounds,admission_window=a.admission_window,initialization_ns=initialization,preparation_ns=preparation,harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),host=host_snapshot(),order_contract="four-treatment Williams rotation; every mode once per position and every directed adjacent pair once per cycle",contract="same live VMs, native read-only inputs, fixed owner routes, 128 requests x 32 items; scheduled short calls include projection and release; batch response includes bulk projection/release; late short cohort drains before next arm; no fixed performance threshold")
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.owners) as main_pool,concurrent.futures.ThreadPoolExecutor(max_workers=a.owners*4) as short_pool:
        orders=[tuple(MODES[i] for i in order) for order in ((0,1,3,2),(1,2,0,3),(2,3,1,0),(3,0,2,1))]
        assert all(sorted(order[position] for order in orders)==sorted(MODES) for position in range(4))
        assert len({(order[i],order[i+1]) for order in orders for i in range(3)})==12
        for round_index in range(a.rounds+4):
            warmup=round_index<4
            order=(MODES[round_index:round_index+1] if warmup else orders[(round_index-4)%4])
            for mode in order:
                wake=threading.Event();clock=[0]
                main=[main_pool.submit(bulk,o,mode,wake) for o in range(a.owners)]
                short_jobs=[short_pool.submit(short,o,i,wake,clock) for o in range(a.owners) for i in range(4)]
                started=time.perf_counter_ns();clock[0]=started;wake.set()
                outputs=[f.result(timeout=20) for f in main]
                roots=[root for values,_ in outputs for root in values]
                returned=time.perf_counter_ns();assert len(roots)==128
                assert [root.owner for root in roots]==[o for o in range(a.owners) for _ in range(128//a.owners)]
                rows=library.project_many(roots,release=True);ended=time.perf_counter_ns()
                cohort=[f.result(timeout=20) for f in short_jobs]
                for row in rows:
                    assert [v[0] for v in row]==list(range(32))
                    assert {v[1] for v in row}=={3*a.lines*(a.lines-1)//2}
                    assert sorted(v[2] for v in row)==list(range(1,33))
                    assert [sum(v[3]==w for v in row) for w in range(4)]==[8]*4
                record=dict(kind="sample",mode=mode,round=round_index,warmup=warmup,ns=ended-started,owner_client_response_ns=[at-started for _,at in outputs],client_return_ns=returned-started,projection_release_ns=ended-returned,actors_per_second=4096e9/(ended-started),darwin_os_threads=max(0,len(subprocess.check_output(["/bin/ps","-M","-p",str(os.getpid())],text=True).splitlines())-1),host=host_snapshot())
                records.append(record);emit(**record)
                probes.extend(dict(mode=mode,warmup=warmup,**p) for p in cohort)
    summary={}
    for mode in MODES:
        rr=[r for r in records if r['mode']==mode and not r['warmup']];pp=[p for p in probes if p['mode']==mode and not p['warmup']]
        counts=[sum(p['owner']==o for p in pp) for o in range(a.owners)]
        assert counts==[a.rounds*4]*a.owners
        summary[mode]=dict(actors_per_second_median=statistics.median(r['actors_per_second'] for r in rr),main_p99_ns=p99([r['ns'] for r in rr]),short_p99_ns=p99([p['ns'] for p in pp]),short_counts=counts,owner_client_response_p99_ns=[p99([r["owner_client_response_ns"][o] for r in rr]) for o in range(a.owners)],os_threads_max=max(r["darwin_os_threads"] for r in rr),owner_short_p99_ns=[p99([p['ns'] for p in pp if p['owner']==o]) for o in range(a.owners)])
    for p in probes:emit(kind="probe",**p)
    library.collect()
    for o,s in enumerate(sessions):
        root=s.call(("actors",payloads[o],1));assert len(root.to_python())==32;root.close()
    for p in payloads:p.close()
    assert library.vm_states==states
    path=library.runtime_directory;library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0]*a.owners and not os.path.exists(path)
    emit(kind="summary",summary=summary,roots=library.shutdown_roots,files_removed=True)
finally:
    for p in payloads:p.close()
    library.close()
