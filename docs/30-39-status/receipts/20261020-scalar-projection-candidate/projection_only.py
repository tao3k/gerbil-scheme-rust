"""Repeated projection of the same rooted Scheme results; no result memoization."""
import argparse,hashlib,json,math,os,statistics,time
from pathlib import Path
import gerbil_ledger
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--rounds',type=int,default=32)
a=parser.parse_args()
if a.rounds<4 or a.rounds%2:parser.error('even --rounds >= 4 required')
def emit(**r):print(json.dumps(r),flush=True)
def p99(v):return sorted(v)[math.ceil(.99*len(v))-1]
library=gerbil_ledger.open_library();states=library.vm_states
sessions=[library.session(i) for i in range(4)];payloads=[];allroots=[]
try:
 emit(kind='configuration',rounds=a.rounds,warmup=2,states=states,harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),contract='same rooted results, same four VMs, both arms project all native types to Python; no execution or release in timed projection; outputs dropped outside timer; read-only roots reused')
 for workload in ['actor-results','wide-integers']:
  if workload=='actor-results':
   data=tuple((i,tuple((j,3) for j in range(48)),False) for i in range(32))
   payloads=[s.value(data) for s in sessions]
   roots=[root for i,s in enumerate(sessions) for root in s.call_many([('actors',payloads[i],1)]*32)]
   expected=[[[i,3384,i+1,None] for i in range(32)] for _ in range(128)]
  else:
   data=[2**(170+i%3)*(-1 if i%2 else 1) for i in range(128)]
   roots=library.call_many([('echo',data)]*128,dispatch='direct')
   expected=[data]*128
  allroots.extend(roots);samples=[]
  # Establish the generic projection as the value oracle, then check against
  # independently known values rather than accepting two matching wrong paths.
  oracle=library.project_many(roots)
  if workload=='actor-results':
   for row in oracle:
    assert [v[0] for v in row]==list(range(32)) and {v[1] for v in row}=={3384}
    assert sorted(v[2] for v in row)==list(range(1,33))
    assert [sum(v[3]==worker for v in row) for worker in range(4)]==[8]*4
  else:assert oracle==expected
  library.collect()
  for index in range(a.rounds+2):
   for fast in ([False,True] if index%4 in (0,3) else [True,False]):
    start=time.perf_counter_ns();rows=library.project_many(roots,fast_scalars=fast);ns=time.perf_counter_ns()-start
    assert rows==oracle
    del rows
    row=dict(kind='sample',workload=workload,fast_scalars=fast,round=index,warmup=index<2,ns=ns)
    samples.append(row);emit(**row)
  arms={str(fast):dict(median_ns=statistics.median(r['ns'] for r in samples if not r['warmup'] and r['fast_scalars']==fast),p99_ns=p99([r['ns'] for r in samples if not r['warmup'] and r['fast_scalars']==fast])) for fast in [False,True]}
  emit(kind='summary',workload=workload,arms=arms,speed_ratio=arms['False']['median_ns']/arms['True']['median_ns'])
  library.release_many(roots)
 for p in payloads:p.close()
 assert library.vm_states==states
 path=library.runtime_directory;library.close()
 assert library.shutdown_clean and library.shutdown_roots==[0]*4 and not os.path.exists(path)
 emit(kind='shutdown',roots=library.shutdown_roots,files_removed=True)
finally:
 library.release_many(allroots)
 for p in payloads:p.close()
 library.close()
