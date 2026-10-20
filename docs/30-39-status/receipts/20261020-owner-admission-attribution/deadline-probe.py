import gerbil_ledger,time,json
l=gerbil_ledger.open_library()
try:
 for owner in range(4):
  r=l.call_on(owner,('dsl','42'));assert r.to_python()==42;r.close()
 l.set_owner_profiling(True)
 t=time.monotonic()
 try:
  r=l.call_many([('dsl','(begin (##thread-sleep! 0.2) 42)')]*128,dispatch='direct',admission_window=16)
  print('UNEXPECTED-SUCCESS',time.monotonic()-t,len(r),flush=True);l.release_many(r)
 except Exception as e:print('result',str(e),'seconds',time.monotonic()-t,flush=True)
 for i in range(4):
  r=l.call_on(i,('echo',42));assert r.to_python()==42;r.close()
 trace=json.loads(l.owner_profile());print('batch_calls',sum(s['kind']=='batch-call' for s in trace['spans']),flush=True)
finally:
 l.close();print('roots',l.shutdown_roots,flush=True)
