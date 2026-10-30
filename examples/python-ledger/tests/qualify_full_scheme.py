"""Full expander, native retained procedures and bounded cleanup on every VM."""
import json,os,time
import gerbil_ledger
library=gerbil_ledger.open_library()
try:
    for owner in range(4):
        start=time.perf_counter_ns()
        value=library.call_on(owner,("dsl","(let ((foldl #f)) (ledger-sum 20 22))"))
        assert value.to_python()==42;value.close()
        print(json.dumps(dict(status="PASS",stage="hygienic-dsl",owner=owner,ns=time.perf_counter_ns()-start)),flush=True)
        value=library.call_on(owner,("dsl","(begin (defrule (twice x) (+ x x)) (twice 21))"))
        assert value.to_python()==42;value.close()
        procedure=library.call_on(owner,("dsl","(let ((base 10)) (lambda (x) (+ base x)))"))
        library.call_on(owner,("gc",)).close()
        value=procedure.apply(32);assert value.to_python()==42;value.close();procedure.close()
        try:library.call_on(owner,("dsl",'(error "injected DSL failure")'))
        except RuntimeError:pass
        else:raise AssertionError("Scheme exception was not raised")
        value=library.call_on(owner,("dsl","(+ 20 22)"));assert value.to_python()==42;value.close()
    path=library.runtime_directory;library.close()
    assert library.shutdown_clean and library.shutdown_roots==[0]*4
    assert not os.path.exists(path)
    print(json.dumps(dict(status="PASS",stage="shutdown",roots=library.shutdown_roots,files_removed=True)),flush=True)
except Exception as error:
    print(json.dumps(dict(status="FAIL",error=str(error))),flush=True)
    try:library.close()
    except Exception as cleanup:print(json.dumps(dict(status="INCOMPLETE",stage="shutdown",error=str(cleanup))),flush=True)
    os._exit(1)
