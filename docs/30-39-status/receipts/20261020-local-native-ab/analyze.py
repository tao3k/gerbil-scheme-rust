import json,pathlib,math,statistics,hashlib
p=pathlib.Path(__file__).parent
pct=lambda xs: sorted(xs)[math.ceil(len(xs)*.99)-1]
result={'macro':{},'dispatch':{}}
for f in sorted(p.glob('macro-*.jsonl')):
 rows=[json.loads(x) for x in f.read_text().splitlines()];summ=[r for r in rows if r.get('kind')=='summary']
 if summ:
  result['macro'][f.stem]={'summary':summ[-1]['summary'],'quiet_snapshots':summ[-1]['quiet_snapshots'],'roots':summ[-1]['roots']}
for f in sorted(p.glob('dispatch-*.jsonl')):
 rows=[json.loads(x) for x in f.read_text().splitlines()];summ=[r for r in rows if r.get('kind')=='summary'];cfg=next((r for r in rows if r.get('kind')=='configuration'),{})
 if summ:
  result['dispatch'][f.stem]={'configuration':cfg,'summary':summ[-1]['summary'],'quiet_snapshots':summ[-1]['quiet_snapshots'],'os_threads':sorted(set(r['os_threads'] for r in rows if 'os_threads'in r and not r['warmup'])), 'os_threads_by_mode':{mode:sorted(set(r['os_threads'] for r in rows if r.get('mode')==mode and not r['warmup'])) for mode in summ[-1]['summary']}}
macro={}
for mode in ('macro-expand','macro-eval'):
 macro[mode]={}
 for variant in ('O0','O2'):
  samples=[]
  for f in p.glob('macro-'+variant+'-*.jsonl'):
   samples.extend(r for r in map(json.loads,f.read_text().splitlines()) if r.get('kind')=='sample' and not r['warmup'] and r['mode']==mode)
  if samples:macro[mode][variant]={'samples':len(samples),'ops_per_second':sum(r['iterations'] for r in samples)*1e9/sum(r['ns'] for r in samples),'p99_ms':pct([r['ns'] for r in samples])/1e6,'response_p99_ms':pct([r['response_ns'] for r in samples])/1e6}
result['macro_pooled']=macro
(p/'summary.json').write_text(json.dumps(result,indent=2))
print(json.dumps({'macro_pooled':macro,'macro_processes':len(result['macro']),'dispatch_cells':len(result['dispatch'])},indent=2))
