import gerbil_ledger,time
l=gerbil_ledger.open_library()
try:
 for owner in range(4):
  for duration in [.01,.2]:
   t=time.monotonic()
   try:
    r=l.call_on(owner,('dsl',f'(begin (##thread-sleep! {duration}) 42)'));print(owner,duration,r.to_python(),time.monotonic()-t,flush=True);r.close()
   except Exception as e:print(owner,duration,str(e),flush=True)
 for window in [1,16]:
  try:
   rs=l.call_many([('dsl','(begin (##thread-sleep! 0.01) 42)')]*16,dispatch='direct',admission_window=window);print('batch',window,l.project_many(rs,release=True),flush=True)
  except Exception as e:print('batch',window,str(e),flush=True)
finally:l.close()
