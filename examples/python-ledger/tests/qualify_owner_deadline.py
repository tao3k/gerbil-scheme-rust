"""Fresh-process native deadline qualification; an observation timeout is not cancellation."""
import argparse, json, os, time
import gerbil_ledger
parser=argparse.ArgumentParser()
parser.add_argument("--dispatch",default="direct",choices=["direct","tokio","rayon","rayon-blocking","tokio-pipeline"])
args=parser.parse_args()
library=gerbil_ledger.open_library(owners=1)
try:
    library.set_owner_profiling(True)
    started=time.monotonic()
    try:
        roots=library.call_many([("actors",tuple((0,(),True) for _ in range(256)),1)]*128,
            dispatch=args.dispatch,admission_window=16)
    except RuntimeError as error:
        assert "Timeout" in str(error),str(error)
    else:
        library.release_many(roots)
        raise AssertionError("smaller turns restarted the original deadline")
    observed=time.monotonic()-started
    for owner in range(1):
        root=library.call_on(owner,("echo",42))
        assert root.to_python()==42;root.close()
    trace=json.loads(library.owner_profile())
    calls=sum(row["kind"]=="batch-call" for row in trace["spans"])
    assert 0<calls<128 and trace["dropped"]==0
    assert all(not row["failed"] for row in trace["spans"])
    assert all(row[3]==0 for row in library.collect()),"abandoned deadline results retained roots"
    library.set_owner_profiling(False)
    path=library.runtime_directory;library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0]
    assert not os.path.exists(path)
    print(json.dumps(dict(status="PASS",stage="original-deadline",observation_seconds=observed,
        accepted_completed=calls,requested=128,roots=library.shutdown_roots,files_removed=True)))
finally:
    library.close()
