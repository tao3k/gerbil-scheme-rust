// Pure Gerbil Actor epoch lifecycle. No Rust scheduler or timing prediction.
module actor_population {
  const PEERS:Set[int]
  const JOBS:Set[int]
  const BUDGET:int
  import actor_batch_core(ACTORS=PEERS,PROCESSORS=Set(1)) as cpu from "./actor_batch_core"
  // actorEpoch is the worker birth epoch. jobEpoch identifies the source invocation.
  type Population = {
    phase:str, epoch:int, resident:Set[int], cohort:Set[int], baselineSize:int, born:Set[int], stopping:Set[int],
    actorEpoch:int->int, queue:int->List[int], work:int->int, waiting:Set[int],
    retained:Set[int], arrived:Set[int], published:Set[int], completed:Set[int],
    consumed:Set[int], failed:Set[int], owner:int->int, jobEpoch:int->int, count:int->int,
    remaining:int->int, timedOut:Set[int], available:bool, profile:str, tokens:int
  }
  var p:Population
  val runningActors = cpu::running
  val inflight = p.published.exclude(p.consumed)
  val executing = p.published.exclude(p.completed)
  val ready = p.completed.exclude(p.consumed)
  val busy = p.resident.filter(a=>p.work.get(a)!=0)
  val queued = PEERS.fold(Set(), (s,a)=>s.union(p.queue.get(a).foldl(Set(),(v,m)=>v.union(Set(m)))))
  val busyJobs = busy.map(a=>p.work.get(a))
  val runnable = p.resident.filter(a=>not(p.waiting.contains(a)) and
    (p.work.get(a)!=0 or p.queue.get(a).length()>0) and not(cpu::terminal.contains(a)))
  val memory = p.resident.size()*2+inflight.size()+p.retained.size()
  val canGrow = p.phase=="configure" and p.available and
    p.profile=="green-wait" and memory+2<=BUDGET
  val drained = p.arrived==p.consumed and busy.size()==0 and queued.size()==0 and
    p.waiting.size()==0 and p.retained.size()==0 and cpu::running.size()==0
  def canResume(a:int, epoch:int, m:int):bool =
    JOBS.contains(m) and p.resident.contains(a) and p.epoch==epoch and p.jobEpoch.get(m)==epoch and
    p.owner.get(m)==a and p.work.get(a)==m and not(p.completed.contains(m)) and p.waiting.contains(a)
  action init = all {
    cpu::init,
    p'={phase:"configure",epoch:1,resident:Set(),cohort:Set(),baselineSize:0,born:Set(),stopping:Set(),
      actorEpoch:PEERS.mapBy(_=>0),queue:PEERS.mapBy(_=>List()),work:PEERS.mapBy(_=>0),
      waiting:Set(),retained:Set(),arrived:Set(),published:Set(),completed:Set(),consumed:Set(),failed:Set(),
      owner:JOBS.mapBy(_=>0),jobEpoch:JOBS.mapBy(_=>0),count:JOBS.mapBy(_=>0),
      remaining:JOBS.mapBy(_=>2),timedOut:Set(),available:true,profile:"cpu",tokens:1}
  }
  action stay = all { cpu::stutter, p'=p }
  action spawn(a:int):bool = all {
    p.phase=="configure",p.available,not(p.resident.contains(a)),p.actorEpoch.get(a)<p.epoch,
    memory+2<=BUDGET,p.epoch==1 or p.resident.size()<p.baselineSize or canGrow,
    if (cpu::joined.contains(a)) cpu::reopenActor(a) else cpu::stutter,
    p'={...p,resident:p.resident.union(Set(a)),born:p.born.union(Set(a)),actorEpoch:p.actorEpoch.set(a,p.epoch)}
  }
  action setTokens(k:int):bool = all {
    p.phase=="configure",p.available,1<=k,k<=JOBS.size(),cpu::stutter,p'={...p,tokens:k}
  }
  action open = all {
    p.phase=="configure",p.available,p.resident.size()>0,
    p.resident.intersect(cpu::terminal).size()==0,cpu::running.size()==0,
    cpu::stutter,p'={...p,phase:"run",cohort:p.resident}
  }
  // Planned arrivals are environment events: owner-wide suspension cannot stop time.
  action arrive(m:int):bool = all {
    p.phase=="run",not(p.arrived.contains(m)),cpu::stutter,p'={...p,arrived:p.arrived.union(Set(m))}
  }
  action publish(m:int,a:int):bool = all {
    p.phase=="run",p.available,p.arrived.contains(m),not(p.published.contains(m)),
    p.resident.contains(a),not(cpu::terminal.contains(a)),inflight.size()<p.tokens,memory+1<=BUDGET,
    cpu::stutter,p'={...p,published:p.published.union(Set(m)),owner:p.owner.set(m,a),
      jobEpoch:p.jobEpoch.set(m,p.epoch),queue:p.queue.set(a,p.queue.get(a).append(m))}
  }
  action start(a:int):bool = all {
    Set("run","draining").contains(p.phase),p.available,p.resident.contains(a),
    p.work.get(a)==0,p.queue.get(a).length()>0,not(p.waiting.contains(a)),cpu::start(a,1),
    p'={...p,work:p.work.set(a,p.queue.get(a).head()),queue:p.queue.set(a,p.queue.get(a).tail())}
  }
  action continue(a:int):bool = all {
    Set("run","draining").contains(p.phase),p.available,p.work.get(a)!=0,
    not(p.waiting.contains(a)),cpu::start(a,1),p'=p
  }
  action unit(a:int):bool = all {
    p.available,cpu::running.contains(a),p.work.get(a)!=0,p.remaining.get(p.work.get(a))>1,cpu::yieldActor(a),
    p'={...p,remaining:p.remaining.set(p.work.get(a),p.remaining.get(p.work.get(a))-1)}
  }
  action finish(a:int, failed:bool):bool = all {
    p.available,cpu::running.contains(a),p.work.get(a)!=0,
    failed or p.remaining.get(p.work.get(a))==1,cpu::yieldActor(a),
    p'={...p,completed:p.completed.union(Set(p.work.get(a))),
      failed:if(failed) p.failed.union(Set(p.work.get(a))) else p.failed,
      count:p.count.set(p.work.get(a),p.count.get(p.work.get(a))+1),
      remaining:p.remaining.set(p.work.get(a),0),work:p.work.set(a,0)}
  }
  // Result ownership remains charged until the source actually consumes it.
  action consume(m:int):bool = all {
    Set("run","draining").contains(p.phase),p.available,p.completed.contains(m),
    not(p.consumed.contains(m)),cpu::stutter,p'={...p,consumed:p.consumed.union(Set(m))}
  }
  action suspend(a:int):bool = all {
    p.available,cpu::running.contains(a),p.work.get(a)!=0,cpu::yieldActor(a),
    p'={...p,waiting:p.waiting.union(Set(a))}
  }
  action resume(a:int, epoch:int, m:int):bool = all {
    p.available,canResume(a,epoch,m),cpu::start(a,1),p'={...p,waiting:p.waiting.exclude(Set(a))}
  }
  action capture(a:int):bool = all {
    p.available,cpu::running.contains(a),p.work.get(a)!=0,
    not(p.retained.contains(p.work.get(a))),memory+1<=BUDGET,
    cpu::stutter,p'={...p,retained:p.retained.union(Set(p.work.get(a)))}
  }
  action release(m:int):bool = all {
    p.available,p.retained.contains(m),cpu::stutter,p'={...p,retained:p.retained.exclude(Set(m))}
  }
  action cancel(m:int):bool = all {
    p.available,p.published.contains(m),not(p.completed.contains(m)),not(p.timedOut.contains(m)),
    cpu::stutter,p'={...p,timedOut:p.timedOut.union(Set(m))}
  }
  action pauseOwner = all { p.available,cpu::stutter,p'={...p,available:false} }
  action restoreOwner = all { not(p.available),cpu::stutter,p'={...p,available:true} }
  action drain = all {
    p.phase=="run",p.available,p.arrived==p.published,cpu::stutter,p'={...p,phase:"draining"}
  }
  action quiesce = all { p.phase=="draining",p.available,drained,cpu::stutter,p'={...p,phase:"quiescent"} }
  action configure(profile:str):bool = all {
    p.phase=="quiescent",p.available,p.epoch<2,
    Set("cpu","green-wait","owner-stall").contains(profile),drained,
    cpu::stutter,p'={...p,phase:"configure",epoch:p.epoch+1,profile:profile,baselineSize:p.resident.size(),
      arrived:Set(),published:Set(),completed:Set(),consumed:Set(),failed:Set(),timedOut:Set(),
      owner:JOBS.mapBy(_=>0),jobEpoch:JOBS.mapBy(_=>0),count:JOBS.mapBy(_=>0),remaining:JOBS.mapBy(_=>2)}
  }
  action closing = all { p.phase=="quiescent",p.available,drained,cpu::stutter,p'={...p,phase:"closing"} }
  action stop(a:int):bool = all {
    Set("configure","closing").contains(p.phase),p.available,p.resident.contains(a),drained,
    not(cpu::terminal.contains(a)),cpu::start(a,1),p'={...p,stopping:p.stopping.union(Set(a))}
  }
  action stopped(a:int):bool = all {
    p.available,p.stopping.contains(a),cpu::finish(a,false),p'={...p,stopping:p.stopping.exclude(Set(a))}
  }
  action join(a:int):bool = all {
    p.available,p.resident.contains(a),cpu::terminal.contains(a),cpu::joinActor(a),
    p'={...p,resident:p.resident.exclude(Set(a))}
  }
  action close = all {
    p.phase=="closing",p.available,p.resident.size()==0,drained,
    cpu::stutter,p'={...p,phase:"closed"}
  }
  val safety = and {
    cpu::safety,cpu::running.subseteq(p.resident),p.waiting.subseteq(busy),
    p.waiting.intersect(cpu::running).size()==0,p.resident.intersect(cpu::joined).size()==0,
    p.consumed.subseteq(p.completed),p.timedOut.subseteq(p.published),p.completed.subseteq(p.published),p.published.subseteq(p.arrived),p.failed.subseteq(p.completed),
    queued.intersect(busyJobs).size()==0,queued.union(busyJobs)==executing,
    executing.intersect(ready).size()==0,executing.union(ready)==inflight,
    p.published.forall(m=>p.jobEpoch.get(m)==p.epoch),
    JOBS.forall(m=>p.count.get(m)==(if(p.completed.contains(m)) 1 else 0)),
    JOBS.forall(m=>(p.remaining.get(m)==0)==p.completed.contains(m)),
    PEERS.forall(a=>p.queue.get(a).length()==p.queue.get(a).foldl(Set(),(s,m)=>s.union(Set(m))).size()),
    PEERS.forall(a=>p.queue.get(a).foldl(true,(ok,m)=>ok and p.owner.get(m)==a and p.jobEpoch.get(m)==p.epoch)),
    busy.forall(a=>p.owner.get(p.work.get(a))==a and p.jobEpoch.get(p.work.get(a))==p.epoch),
    busy.map(a=>p.work.get(a)).size()==busy.size(),
    inflight.size()<=p.tokens,memory<=BUDGET,
    implies(Set("run","draining").contains(p.phase),p.resident==p.cohort),
    implies(Set("configure","quiescent","closing","closed").contains(p.phase),inflight.size()==0 and p.retained.size()==0),
    implies(p.phase=="closed",p.resident.size()==0 and p.born.subseteq(cpu::joined) and drained)
  }
  action step = {
    nondet a=PEERS.oneOf()
    nondet m=JOBS.oneOf()
    any { spawn(a),setTokens(1),setTokens(JOBS.size()),open,arrive(m),publish(m,a),start(a),continue(a),
      unit(a),finish(a,false),finish(a,true),consume(m),suspend(a),resume(a,p.epoch,p.work.get(a)),
      capture(a),release(m),cancel(m),pauseOwner,restoreOwner,drain,quiesce,configure("cpu"),configure("green-wait"),
      configure("owner-stall"),closing,stop(a),stopped(a),join(a),close,stay }
  }
  // Mutants are excluded from step: each retained scenario must expose a violation.
  action unsafeRetireEmptyMailbox(a:int):bool = all {
    p.resident.contains(a),p.queue.get(a).length()==0,p.work.get(a)==0,
    cpu::stutter,p'={...p,resident:p.resident.exclude(Set(a))}
  }
  action unsafeReturnBeforeConsume(m:int):bool = all {
    p.published.contains(m),not(p.completed.contains(m)),cpu::stutter,
    p'={...p,consumed:p.consumed.union(Set(m))}
  }
  action unsafeDuplicateCompletion(m:int):bool = all {
    p.completed.contains(m),cpu::stutter,p'={...p,count:p.count.set(m,p.count.get(m)+1)}
  }
  action unsafeClose = all { cpu::stutter,p'={...p,phase:"closed"} }
  action unsafeGrow(a:int):bool = all {
    not(p.resident.contains(a)),cpu::stutter,p'={...p,resident:p.resident.union(Set(a))}
  }
}
// Canonical checker entries contain no run/then initializers. Quint's TLA+
// conversion must not treat scenario initialization as a next-state action.
module actor_population_check {
  import actor_population(PEERS=Set(1,2),JOBS=Set(1,2),BUDGET=8).* from "./actor_population"
}
module actor_population_tight_check {
  import actor_population(PEERS=Set(1,2),JOBS=Set(1,2),BUDGET=3).* from "./actor_population"
}
module actor_population_shrink_mutant {
  import actor_population(PEERS=Set(1,2),JOBS=Set(1,2),BUDGET=8).* from "./actor_population"
  action mutantStep = any { step,
    all { p.phase=="run",inflight.size()>0,unsafeRetireEmptyMailbox(1) },
    all { p.phase=="run",inflight.size()>0,unsafeRetireEmptyMailbox(2) } }
}
