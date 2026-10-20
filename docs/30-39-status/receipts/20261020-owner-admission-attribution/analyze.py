import json,sys,statistics,collections,math
p=lambda a,q=.99: sorted(a)[max(0,math.ceil(len(a)*q)-1)] if a else 0
rows=[json.loads(l) for l in open(sys.argv[1])]
for arm in sorted({r['arm'] for r in rows if r['kind']=='sample'}):
    samples=[r for r in rows if r['kind']=='sample' and r['arm']==arm and not r['warmup']]
    spans=[s for r in rows if r['kind']=='owner-trace' and r['arm']==arm and not r['warmup'] for s in r['spans']]
    probes=[r for r in rows if r['kind']=='probe' and r['arm']==arm and not r['warmup']]
    print(arm,'samples',len(samples),'rate',round(statistics.median([r['actors_per_second'] for r in samples]),1) if samples else 0,'response_p99_ms',round(p([v for r in samples for v in r['response_ns']])/1e6,2),'probe_p99_ms',round(p([r['ns'] for r in probes])/1e6,2))
    for kind in sorted(set(s['kind'] for s in spans)):
        ss=[s for s in spans if s['kind']==kind];d=[s['diagnostics_delta'] for s in ss if s['diagnostics_delta'] is not None]
        print(kind,len(ss),'queue_med/p99_ms',*[round(p([s['start_ns']-s['queued_ns'] for s in ss],q)/1e6,3) for q in [.5,.99]],'run_med/p99_ms',*[round(p([s['end_ns']-s['start_ns'] for s in ss],q)/1e6,3) for q in [.5,.99]],'encode_total_ms',round(sum(s['stages_ns'][0] for s in ss)/1e6,1),'scheme_total_ms',round(sum(max(0,s['stages_ns'][1]-s['stages_ns'][0]) for s in ss)/1e6,1),'run_total_ms',round(sum(s['end_ns']-s['start_ns'] for s in ss)/1e6,1),'gc_real_ms',round(sum(x[2] for x in d)*1000,2),'thread_cpu_ms',round(sum(x[3] for x in d)*1000,2),'gcs',sum(x[5] for x in d))
