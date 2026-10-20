import json,hashlib,zipfile,subprocess
from pathlib import Path
r=Path('/Users/guangtao/ghq/github.com/tao3k/gerbil-scheme-rust');d=Path('/private/tmp/gerbil-client-admission-20261020')
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
w=next((d/'wheels').glob('*.whl'));prev=Path('/private/tmp/gerbil-owner-attribution-20261020/wheels-qualified/gerbil_ledger-0.1.0-cp310-abi3-macosx_11_0_arm64.whl');core=next((d/'venv/lib/python3.14/site-packages/gerbil_ledger').glob('_core*.so'))
with zipfile.ZipFile(w) as a,zipfile.ZipFile(prev) as b:
    core_name='gerbil_ledger/'+core.name
    assert hashlib.sha256(a.read(core_name)).hexdigest()==h(core)
    same={n:a.read(n)==b.read(n) for n in b.namelist() if n.startswith('gerbil_ledger/_resources/') and not n.endswith('/')}
    assert all(same.values()),[n for n,v in same.items() if not v]
sources=[r/'crates/gerbil-scheme-python/src/prepared.rs',r/'crates/gerbil-scheme-python/src/lib.rs',r/'examples/python-ledger/src/api.rs',*r.glob('crates/gerbil-scheme-python/src/instance*.rs'),r/'crates/gerbil-scheme-python/src/runtime.rs',r/'crates/gerbil-scheme-python/src/session.rs',r/'examples/python-ledger/tests/bench_owner_projection.py',r/'examples/python-ledger/tests/test_owner_routing.py',r/'examples/python-ledger/tests/benchmark_metrics.py',r/'examples/python-ledger/tests/qualify_owner_deadline.py',r/'Justfile']
record=dict(head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip(),branch=subprocess.check_output(['git','branch','--show-current'],cwd=r,text=True).strip(),dirty=True,wheel=str(w),wheel_sha256=h(w),installed_core=str(core),installed_core_sha256=h(core),installed_matches_wheel=True,native_resources_identical_to_prior_O2=same,source_sha256={str(p.relative_to(r)):h(p) for p in sources},boundary='Same four live VMs and native Actor workload in both arms; direct dispatch; raw versus explicit prepared immutable host inputs at fixed window, then prepared inputs with 128 versus 16 admission turns; unchanged 128-request/five-second groups; same fresh native Scheme computation and bulk projection; finite fixed scheduled clients; primary timings disable tracing; preparation cost reported separately.')
(d/'identity.json').write_text(json.dumps(record,indent=2)+'\n');print('identity verified',len(same),'unchanged packaged resources',h(w))
