import pathlib,json,statistics,random,math
p=pathlib.Path(__file__).parent
rng=random.Random(20261020)
result={}
def interval(values):
 estimates=sorted(statistics.median(rng.choices(values,k=len(values))) for _ in range(4000))
 return [estimates[99],estimates[3899]]
for f in sorted(p.glob('dispatch-*.jsonl')):
 rows=[json.loads(x) for x in f.read_text().splitlines()]
 if not any(r.get('kind')=='summary' for r in rows):continue
 groups={}
 for r in rows:
  if 'index'in r and not r['warmup']:groups.setdefault((r['index']-4)//4,{})[r['mode']]=r
 assert len(groups)==16 and all(len(g)==4 for g in groups.values())
 controls={}
 for mode in ('tokio','rayon','rayon-blocking'):
  controls[mode]={}
  for metric in ('actors_per_second','projected_actors_per_second','batch_projected_response_p99_ns'):
   ratios=[g[mode][metric]/g['direct'][metric] for g in groups.values()]
   controls[mode][metric]={'rounds':len(ratios),'median_ratio':statistics.median(ratios),'bootstrap_round_median_ratio_95pct':interval(ratios),'rounds_above_direct':sum(x>1 for x in ratios),'ratios':ratios}
 result[f.stem]=controls
(p/'paired-rounds.json').write_text(json.dumps(result,indent=2))
print('Paired round comparisons:',len(result))
