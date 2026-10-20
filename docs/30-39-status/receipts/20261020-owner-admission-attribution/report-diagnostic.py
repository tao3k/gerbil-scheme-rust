import json,collections,statistics
from pathlib import Path
p=Path('/private/tmp/gerbil-owner-attribution-20261020')
rows=[json.loads(l) for l in (p/'mixed-diagnostic.jsonl').read_text().splitlines()]
for arm in ['scalar','owner-batch']:
    probes=[r for r in rows if r['kind']=='probe' and r['arm']==arm and not r['warmup']]
    print(arm,'probe_counts',collections.Counter(r['owner'] for r in probes),'lateness_median_ms',statistics.median(r['admission_lateness_ns'] for r in probes)/1e6)
    traces=[r for r in rows if r['kind']=='owner-trace' and r['arm']==arm and not r['warmup']]
    assert all(r['dropped']==0 and not any(s['failed'] for s in r['spans']) for r in traces)
    for r in traces:
        counts=collections.Counter(s['kind'] for s in r['spans'])
        assert counts['call']==160,counts
        assert sum(s['kind']=='call' for s in r['spans'])==128+32
    print('trace_valid',len(traces))
