import json,statistics,random
from pathlib import Path
base=Path(__file__).parent
rows=[json.loads(s) for s in (base/'projection-ab.jsonl').read_text().splitlines()]
summary=[r for r in rows if r['kind']=='summary'][0]
rr={arm:{r['round']:r for r in rows if r['kind']=='sample' and r['arm']==arm and not r['warmup']} for arm in ['fresh','native']}
ratios=[rr['native'][k]['actors_per_second']/rr['fresh'][k]['actors_per_second'] for k in rr['fresh']]
rng=random.Random(20261020)
boot=sorted(statistics.median(rng.choices(ratios,k=len(ratios))) for _ in range(10000))
result=dict(arm_mapping={'fresh':'existing recursive projection; native input','native':'fast scalar projection; native input'},summary=summary,paired_throughput_median_ratio=statistics.median(ratios),descriptive_bootstrap_95=[boot[250],boot[9749]],paired_rounds_faster=sum(v>1 for v in ratios),measured_pairs=len(ratios))
traces=[json.loads(s) for s in (base/'projection-trace.jsonl').read_text().splitlines()]
phases={}
for arm in ['fresh','native']:
    groups=[r for r in traces if r['kind']=='owner-trace' and r['arm']==arm and not r['warmup']]
    out=[]
    for group in groups:
        assert group['dropped']==0 and all(not s['failed'] for s in group['spans'])
        ps=[s for s in group['spans'] if s['kind']=='projection-turn'];rs=[s for s in group['spans'] if s['kind']=='release-turn']
        assert len(ps)==len(rs)==4
        out.append(dict(projection_envelope_ms=(max(s['end_ns'] for s in ps)-min(s['queued_ns'] for s in ps))/1e6,release_envelope_ms=(max(s['end_ns'] for s in rs)-min(s['queued_ns'] for s in rs))/1e6,between_ms=(min(s['queued_ns'] for s in rs)-max(s['end_ns'] for s in ps))/1e6))
    phases[arm]={k:statistics.median(o[k] for o in out) for k in out[0]}
result['traced_phase_medians_ms']=phases
(base/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
