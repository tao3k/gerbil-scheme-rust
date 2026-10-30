"""Native dispatcher overload, concurrent callers and in-flight close contracts."""
import argparse
import concurrent.futures
import json
import os
import subprocess
import threading
import time

import gerbil_ledger

MODES=("direct","tokio","rayon","rayon-blocking","tokio-pipeline")
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--dispatch",choices=MODES,required=True)
parser.add_argument("--case",choices=("concurrent","overload","close"),required=True)
args=parser.parse_args()
def os_threads():
    return max(0,len(subprocess.check_output(["/bin/ps","-M","-p",str(os.getpid())],text=True).splitlines())-1)

initial_threads=os_threads()
library=gerbil_ledger.open_library(owners=1 if args.case=="overload" else 4,dispatch_workers=2)
states=library.vm_states
path=library.runtime_directory

def recover():
    for owner in range(library.vm_count):
        root=library.call_on(owner,("echo",owner));assert root.to_python()==owner;root.close()
    assert library.vm_states==states

def work(caller,barrier):
    barrier.wait(timeout=5)
    if args.case=="concurrent":
        for turn in range(8):
            values=[(caller,turn,i,2**170,b"\x00\xff") for i in range(33)]
            roots=library.call_many([("echo",v) for v in values],dispatch=args.dispatch,admission_window=4)
            assert library.project_many(roots,release=True)==[list(v) for v in values]
        return "complete"
    items=tuple((0,(),True) for _ in range(8 if args.case=="overload" else 16))
    try:
        roots=library.call_many([("actors",items,1)]*128,dispatch=args.dispatch,admission_window=128 if args.case=="overload" else 16)
    except RuntimeError as error:
        text=str(error)
        if args.case=="overload":assert "QueueFull" in text or "Timeout" in text,text
        else:assert "closed" in text.lower(),text
        return "rejected"
    if args.case=="overload":
        rows=library.project_many(roots,release=True)
        assert len(rows)==128 and all(len(row)==8 for row in rows)
    else:
        # Explicit close invalidates every returned handle, including a reply
        # that completed before shutdown but was observed afterward.
        for root in roots:root.close()
    return "complete"

try:
    library.set_owner_profiling(True)
    callers=8 if args.case=="concurrent" else 4
    barrier=threading.Barrier(callers+1)
    with concurrent.futures.ThreadPoolExecutor(max_workers=callers) as pool:
        pending=[pool.submit(work,i,barrier) for i in range(callers)]
        barrier.wait(timeout=5)
        if args.case=="close":
            # Observe a real completed native request before closing with more
            # work pending; do not substitute an arbitrary sleep for admission.
            deadline=time.monotonic()+5
            while True:
                trace=json.loads(library.owner_profile())
                assert trace["dropped"]==0
                if any(span["kind"]=="batch-call" for span in trace["spans"]):break
                assert time.monotonic()<deadline,"no observed native progress"
                time.sleep(.001)
            library.close()
        outcomes=[future.result(timeout=20) for future in pending]
    if args.case=="overload" and args.dispatch!="rayon-blocking":assert "rejected" in outcomes,outcomes
    if args.case!="close":
        recover()
        assert all(row[3]==0 for row in library.collect()),"orphan native roots after observed replies"
    library.close()
    deadline=time.monotonic()+5
    while os_threads()>initial_threads and time.monotonic()<deadline:time.sleep(.01)
    final_threads=os_threads()
    assert final_threads<=initial_threads,(initial_threads,final_threads)
    library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0]*len(states)
    assert not os.path.exists(path)
    for operation in (lambda:library.call(("echo",42)),lambda:library.call_many([("echo",42)],dispatch=args.dispatch)):
        try:operation()
        except RuntimeError:pass
        else:raise AssertionError("closed library admitted work")
    print(json.dumps(dict(status="PASS",case=args.case,dispatch=args.dispatch,callers=callers,outcomes=outcomes,initial_os_threads=initial_threads,final_os_threads=final_threads,roots=library.shutdown_roots,files_removed=True)),flush=True)
finally:
    library.close()
