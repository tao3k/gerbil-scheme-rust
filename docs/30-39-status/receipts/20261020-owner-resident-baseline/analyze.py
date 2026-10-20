import collections,hashlib,json,statistics
from pathlib import Path
base=Path(__file__).parent
rows=[json.loads(s) for s in (base/'trace.jsonl').read_text().splitlines()]
summary={}
for arm in ['fresh','native']:
    groups=[r for r in rows if r['kind']=='owner-trace' and r['arm']==arm and not r['warmup']]
    spans=[s for r in groups for s in r['spans'] if s['owner'] is not None]
    out={}
    for kind in sorted({s['kind'] for s in spans}):
        ss=[s for s in spans if s['kind']==kind]
        out[kind]=dict(count=len(ss),wall_median_ms=statistics.median(s['end_ns']-s['start_ns'] for s in ss)/1e6,wait_median_ms=statistics.median(s['start_ns']-s['queued_ns'] for s in ss)/1e6)
    turns=[]
    for group in groups:
        ps=[s for s in group['spans'] if s['kind']=='projection-turn']
        rs=[s for s in group['spans'] if s['kind']=='release-turn']
        assert len(ps)==len(rs)==4
        turns.append(dict(projection_envelope_ms=(max(s['end_ns'] for s in ps)-min(s['queued_ns'] for s in ps))/1e6,release_envelope_ms=(max(s['end_ns'] for s in rs)-min(s['queued_ns'] for s in rs))/1e6,between_envelopes_ms=(min(s['queued_ns'] for s in rs)-max(s['end_ns'] for s in ps))/1e6))
    summary[arm]=dict(spans=out,phase_medians_ms={k:statistics.median(t[k] for t in turns) for k in turns[0]})
(base/'trace-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
