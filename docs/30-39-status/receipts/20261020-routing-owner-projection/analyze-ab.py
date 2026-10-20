import pathlib,json,statistics,random,math
p=pathlib.Path(__file__).parent
rng=random.Random(20261020)
summary={}
def interval(values):
 xs=sorted(statistics.median(rng.choices(values,k=len(values))) for _ in range(4000))
 return [xs[99],xs[3899]]
for name in ('cpu-alone','mixed-alone','cpu-probes','mixed-probes','cpu-scheduled','mixed-scheduled'):
 f=p/(name+'.jsonl')
 if not f.exists():continue
 rows=[json.loads(x) for x in f.read_text().splitlines()]
 totals=[r for r in rows if r.get('kind')=='summary']
 if not totals:summary[name]={'status':'INCOMPLETE_OR_FAILED','last_kind':rows[-1].get('kind') if rows else None};continue
 measured=[r for r in rows if r.get('kind')=='sample' and not r['warmup']]
 pairs={}
 for row in measured:pairs.setdefault(row['round'],{})[row['arm']]=row
 assert len(pairs)==16 and all(len(pair)==2 for pair in pairs.values())
 ratios={}
 for metric in ('actors_per_second','projection_release_ns','response_p99_ns','native_ready_p99_ns'):
  values=[pair['owner-batch'][metric]/pair['scalar'][metric] for pair in pairs.values()]
  ratios[metric]={'median_ratio':statistics.median(values),'round_bootstrap_95pct':interval(values),'rounds_above_one':sum(v>1 for v in values),'ratios':values}
 summary[name]={'status':'COMPLETED','configuration':rows[0],'summary':totals[-1]['summary'],'paired':ratios,'vm_states':totals[-1]['states'],'roots':totals[-1]['roots']}
(p/'ab-summary.json').write_text(json.dumps(summary,indent=2))
for name,r in summary.items():
 print(name,r['status'])
 if r['status']!='COMPLETED':continue
 for arm,m in r['summary'].items():print(arm,'items/s',round(m['actors_per_second_median'],1),'response-p99-ms',round(m['response_p99_ns']/1e6,2),'projection-ms',round(m['projection_release_ns_median']/1e6,2),'probe-p99-ms',None if m['probe_p99_ns'] is None else round(m['probe_p99_ns']/1e6,2))
 print('paired',[(key,round(v['median_ratio'],3),[round(x,3) for x in v['round_bootstrap_95pct']]) for key,v in r['paired'].items()])
