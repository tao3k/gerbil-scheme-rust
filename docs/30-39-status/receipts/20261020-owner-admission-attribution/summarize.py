import collections,json,math,statistics,random
from pathlib import Path
p=Path('/private/tmp/gerbil-owner-attribution-20261020')
def quantile(v,q=.99):return sorted(v)[max(0,math.ceil(len(v)*q)-1)] if v else None
def paired(a,b,key):
    av={r['round']:r[key] for r in a};bv={r['round']:r[key] for r in b}
    v=[bv[i]/av[i] for i in sorted(av.keys() & bv.keys())];rng=random.Random(20261020)
    boot=[statistics.median(rng.choices(v,k=len(v))) for _ in range(4000)]
    return dict(median_ratio=statistics.median(v),descriptive_interval=[quantile(boot,.025),quantile(boot,.975)],pairs=len(v))
result={}
for case in ['mixed-primary','cpu-primary','mixed-trace','mixed-diagnostic']:
    f=p/(case+'.jsonl')
    if not f.exists():continue
    rows=[json.loads(l) for l in f.read_text().splitlines()]
    if not any(r['kind']=='summary' for r in rows):continue
    summary=next(r['summary'] for r in rows if r['kind']=='summary')
    case_result=dict(summary=summary,configuration=rows[0])
    for arm in summary:
        ss=[s for r in rows if r['kind']=='owner-trace' and not r['warmup'] and r['arm']==arm for s in r['spans']]
        assert all(r['dropped']==0 for r in rows if r['kind']=='owner-trace')
        if ss:
            phases={}
            for kind in sorted({s['kind'] for s in ss}):
                spans=[s for s in ss if s['kind']==kind];ds=[s['diagnostics_delta'] for s in spans if s['diagnostics_delta'] is not None]
                phases[kind]=dict(count=len(spans),queue_ns_median=quantile([s['start_ns']-s['queued_ns'] for s in spans],.5),queue_ns_p99=quantile([s['start_ns']-s['queued_ns'] for s in spans]),execution_ns_median=quantile([s['end_ns']-s['start_ns'] for s in spans],.5),execution_ns_p99=quantile([s['end_ns']-s['start_ns'] for s in spans]),execution_ns_max=max(s['end_ns']-s['start_ns'] for s in spans),encoding_ns_total=sum(s['stages_ns'][0] for s in spans),scheme_ns_total=sum(max(0,s['stages_ns'][1]-s['stages_ns'][0]) for s in spans),execution_ns_total=sum(s['end_ns']-s['start_ns'] for s in spans),gc_real_seconds=sum(d[2] for d in ds),thread_cpu_seconds=sum(d[3] for d in ds),collections=sum(d[5] for d in ds))
            summary[arm]['owner_phases']=phases
            exposures={}
            for short_kind in ['call','inspect','release']:
                selected=[s for s in ss if s['kind']==short_kind] if any(s['kind']=='batch-call' for s in ss) else []
                exposure=collections.defaultdict(list)
                for short in selected:
                    busy=collections.Counter()
                    for other in ss:
                        if other['owner']==short['owner'] and other is not short:
                            overlap=max(0,min(short['start_ns'],other['end_ns'])-max(short['queued_ns'],other['start_ns']))
                            if overlap:busy[other['kind']]+=overlap
                    for kind in ['batch-call','projection-turn','release-turn']:
                        exposure[kind].append(busy[kind])
                    exposure['unattributed'].append(max(0,short['start_ns']-short['queued_ns']-sum(busy.values())))
                exposures[short_kind]={kind:quantile(v) for kind,v in exposure.items()}
            summary[arm]['short_owner_exposure_ns_p99_by_operation']=exposures
        probes=[r for r in rows if r['kind']=='probe' and r['arm']==arm and not r['warmup']]
        summary[arm]['client_lateness_ns_median']=quantile([r['admission_lateness_ns'] for r in probes],.5)
        summary[arm]['client_lateness_ns_p99']=quantile([r['admission_lateness_ns'] for r in probes])
        summary[arm]['short_service_ns_p99']=quantile([r['service_ns'] for r in probes])
    if 'window-16' in summary:
        samples=[r for r in rows if r['kind']=='sample' and not r['warmup']]
        a=[r for r in samples if r['arm']=='window-128'];b=[r for r in samples if r['arm']=='window-16']
        assert len(a)==len(b)
        case_result['paired']={key:paired(a,b,key) for key in ['actors_per_second','response_p99_ns','native_ready_p99_ns','projection_release_ns']}
        for sample in samples:
            assert len(sample['response_ns'])==128
            assert sample['worker_counts']==[1024]*4
            assert sample['owner_fairness']['assigned_owner_counts']==[32]*4
        for arm in summary:
            assert summary[arm]['probe_counts_by_owner']==[len(a)*8]*4
    result[case]=case_result
(p/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
for case,r in result.items():
    print(case)
    for arm,s in r['summary'].items():
        print(arm,'rate',round(s['actors_per_second_median'],1),'response_p99_ms',round(s['response_p99_ns']/1e6,2),'probe_p99_ms',round(s['probe_p99_ns']/1e6,2),'owner_probe_p99_ms',[round(v/1e6,1) for v in s['probe_p99_ns_by_owner']],'lateness_p99_ms',round(s['client_lateness_ns_p99']/1e6,1))
    print('paired',r.get('paired'))
