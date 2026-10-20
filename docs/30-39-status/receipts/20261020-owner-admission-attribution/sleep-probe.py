import gerbil_ledger,time
lib=gerbil_ledger.open_library()
try:
 for expr in ['(begin (##thread-sleep! 0.01) 42)','(begin (import :gerbil/gambit) (thread-sleep! 0.01) 42)','(begin (import :gerbil/gambit/threads) (thread-sleep! 0.01) 42)']:
  start=time.monotonic()
  try:
   root=lib.call(('dsl',expr));print(expr,root.to_python(),time.monotonic()-start,flush=True);root.close()
  except Exception as e:print(expr,str(e),flush=True)
finally:lib.close()
