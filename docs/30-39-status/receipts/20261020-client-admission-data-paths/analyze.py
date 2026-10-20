import collections,json,math,statistics,random
from pathlib import Path
p=Path('/private/tmp/gerbil-client-admission-20261020')
def q(v,p=.99):return sorted(v)[max(0,math.ceil(len(v)*p)-1)] if v else None
result={}
for f in sorted(p.glob('*.jsonl')):
 if f.name=='exits.jsonl' or f.stem=='q-native-payload':continue
 rows=[json.loads(l) for l in f.read_text().splitlines()]
 if not any(r.get('kind')=='summary' for r in rows):continue
 c=dict(configuration=rows[0],summary=next(r['summary'] for r in rows if r['kind']=='summary'),baseline=next(r for r in rows if r['kind']=='short-baseline'),preparation=next(r for r in rows if r['kind']=='preparation'))
 for arm,summary in c['summary'].items():
  samples=[r for r in rows if r.get('kind')=='sample' and not r['warmup'] and r['arm']==arm]
  probes=[r for r in rows if r.get('kind')=='probe' and not r['warmup'] and r['arm']==arm]
  assert len(samples) in [4,16] and len(probes)==32*len(samples)
  assert summary['probe_counts_by_owner']==[8*len(samples)]*4
  summary['lateness_ns_p99']=q([r['admission_lateness_ns'] for r in probes]);summary['service_ns_p99']=q([r['service_ns'] for r in probes])
  for sample in samples:
   assert sample['worker_counts']==[1024]*4
   assert sample['owner_fairness']['assigned_owner_counts']==[32]*4
  captures=[r for r in rows if r.get('kind')=='owner-trace' and not r['warmup'] and r['arm']==arm]
  if captures:
   assert all(r['dropped']==0 for r in captures)
   spans=[s for r in captures for s in r['spans']]
   phases={}
   for kind in sorted({s['kind'] for s in spans}):
    ss=[s for s in spans if s['kind']==kind];ds=[s['diagnostics_delta'] for s in ss if s['diagnostics_delta'] is not None]
    phases[kind]=dict(count=len(ss),run_median_ns=q([s['end_ns']-s['start_ns'] for s in ss],.5),run_p99_ns=q([s['end_ns']-s['start_ns'] for s in ss]),run_total_ns=sum(s['end_ns']-s['start_ns'] for s in ss),queue_p99_ns=q([s['start_ns']-s['queued_ns'] for s in ss]),encode_total_ns=sum(s['stages_ns'][0] for s in ss),scheme_total_ns=sum(s['stages_ns'][1]-s['stages_ns'][0] for s in ss),cpu_total_seconds=sum(d[3] for d in ds) if ds else None,gc_seconds=sum(d[2] for d in ds) if ds else None,gc_count=sum(d[5] for d in ds) if ds else None)
   observed_global=0;observed_owner=[0]*4
   for capture in captures:
    jobs=[x for x in capture['spans'] if x['kind']=='batch-call']
    for owner in [None,0,1,2,3]:
     selected=jobs if owner is None else [x for x in jobs if x['owner']==owner]
     active=maximum=0
     for _,delta in sorted([(x['queued_ns'],1) for x in selected]+[(x['end_ns'],-1) for x in selected]):
      active+=delta;maximum=max(maximum,active)
     assert active==0
     if owner is None:observed_global=max(observed_global,maximum)
     else:observed_owner[owner]=max(observed_owner[owner],maximum)
   summary['observed_batch_inflight']=dict(global_max=observed_global,owner_max=observed_owner)
   summary['phases']=phases
   # Barrier exposure is time unavailable to the following bulk turn, not measured CPU idle.
   exposure=[]
   for capture in captures:
    all_spans=capture['spans'];turns=sorted([s for s in all_spans if s['kind']=='rust-admission-turn'],key=lambda s:s['start_ns'])
    for turn in turns[:-1]:
     batch=[s for s in all_spans if s['kind']=='batch-call' and turn['start_ns']<=s['queued_ns']<=s['end_ns']<=turn['end_ns']]
     for owner in range(4):
      owner_jobs=[s for s in batch if s['owner']==owner]
      if owner_jobs:exposure.append(turn['end_ns']-max(s['end_ns'] for s in owner_jobs))
   summary['following_turn_barrier_exposure_ns']=dict(median=q(exposure,.5),p99=q(exposure),total=sum(exposure),observations=len(exposure))
 arms=list(c['summary']);a,b=arms
 av={r['round']:r for r in rows if r.get('kind')=='sample' and not r['warmup'] and r['arm']==a};bv={r['round']:r for r in rows if r.get('kind')=='sample' and not r['warmup'] and r['arm']==b};rng=random.Random(20261020);ratios={}
 for key in ['actors_per_second','response_p99_ns']:
  vv=[bv[i][key]/av[i][key] for i in sorted(av)];boot=[statistics.median(rng.choices(vv,k=len(vv))) for _ in range(4000)];ratios[key]=dict(median=statistics.median(vv),interval=[q(boot,.025),q(boot,.975)])
 c['paired_order']=arms;c['paired_ratios']=ratios;result[f.stem]=c
(p/'summary.json').write_text(json.dumps(result,indent=2)+'\n')
for case,c in result.items():
 print(case,'baseline_p99_ms',round(c['baseline']['p99_ns']/1e6,3),'prepare_ms',round(c['preparation']['ns']/1e6,3))
 for arm,s in c['summary'].items():
  print(arm,'rate',round(s['actors_per_second_median'],1),'main_p99_ms',round(s['response_p99_ns']/1e6,2),'short_p99_ms',round(s['probe_p99_ns']/1e6,2),'lateness_ms',round(s['lateness_ns_p99']/1e6,2),'service_ms',round(s['service_ns_p99']/1e6,2))
  if 'phases' in s:
   for name in ['python-read-many','rust-clone-turn','gil-reacquire','batch-call','projection-turn']:
    if name not in s['phases']:continue
    t=s['phases'][name];print(name,round(t['run_median_ns']/1e6,3),round(t['run_p99_ns']/1e6,3),'queue_p99',round(t['queue_p99_ns']/1e6,3))
   print('barrier',s['following_turn_barrier_exposure_ns'])
