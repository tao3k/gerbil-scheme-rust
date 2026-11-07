// Work-accounting refinement of the canonical Actor lifecycle, not a scheduler.
// Counts are logical protocol events; no time, bytes, or performance prediction.
module actor_mailbox_work {
  const ACTORS:Set[int]
  const REQUESTS:Set[int]
  const LIMIT:int
  import actor_population(PEERS=ACTORS,JOBS=REQUESTS,BUDGET=LIMIT) as life from "./actor_population"
  type Work = { constructed:int->int, verified:int->int, transfers:int, extra:int, created:int }
  val state = life::p
  val lifetimeSafe = life::safety
  val completedReady = life::ready
  val nativeOutstanding = life::inflight
  val lifecycleDrained = life::drained
  val runningActors = life::runningActors
  val liveMemory = life::memory
  var work:Work
  val sourceDebt = life::p.arrived.exclude(life::p.published)
  val debtPartition = sourceDebt.union(life::executing).union(life::ready)==life::p.arrived.exclude(life::p.consumed) and
    sourceDebt.intersect(life::executing).size()==0 and sourceDebt.intersect(life::ready).size()==0 and
    life::executing.intersect(life::ready).size()==0
  val workPreserved = and {
    REQUESTS.forall(m=>work.constructed.get(m)==(if(life::p.published.contains(m)) 1 else 0)),
    REQUESTS.forall(m=>work.verified.get(m)==(if(life::p.completed.contains(m) and not(life::p.failed.contains(m))) 1 else 0)),
    work.transfers==life::p.published.size()+life::p.completed.size(),work.extra==0
  }
  val cycleSafe = life::safety and debtPartition and workPreserved
  action beginCycle = all { life::init,
    work'={constructed:REQUESTS.mapBy(_=>0),verified:REQUESTS.mapBy(_=>0),transfers:0,extra:0,created:0} }
  action createResident(a:int):bool = all { life::spawn(a),work'={...work,created:work.created+1} }
  action configureCredits(k:int):bool = all { life::setTokens(k),work'=work }
  action pauseOwner = all { life::pauseOwner,work'=work }
  action restoreOwner = all { life::restoreOwner,work'=work }
  action openCycle = all { life::open,work'=work }
  action offer(m:int):bool = all { life::arrive(m),work'=work }
  // Construction/publication is one transaction after obtaining canonical credit.
  // No payload construction is charged to an unpublished, waiting arrival.
  action supply(m:int,a:int):bool = all { life::publish(m,a),
    work'={...work,constructed:work.constructed.set(m,work.constructed.get(m)+1),transfers:work.transfers+1} }
  action enter(a:int):bool = all { life::start(a),work'=work }
  action executeUnit(a:int):bool = all { life::unit(a),work'=work }
  action continueUnit(a:int):bool = all { life::continue(a),work'=work }
  action returnResult(a:int,failed:bool):bool = all { life::finish(a,failed),
    work'={...work,transfers:work.transfers+1,
      verified:if(failed) work.verified else work.verified.set(life::p.work.get(a),work.verified.get(life::p.work.get(a))+1)} }
  action takeVerified(m:int):bool = all { life::consume(m),work'=work }
  action waitGreen(a:int):bool = all { life::suspend(a),work'=work }
  action resumeGreen(a:int,epoch:int,m:int):bool = all { life::resume(a,epoch,m),work'=work }
  action cancelCaller(m:int):bool = all { life::cancel(m),work'=work }
  action beginDrain = all { life::drain,work'=work }
  action endDrain = all { life::quiesce,work'=work }
  action beginClose = all { life::closing,work'=work }
  action requestStop(a:int):bool = all { life::stop(a),work'=work }
  action confirmStop(a:int):bool = all { life::stopped(a),work'=work }
  action joinResident(a:int):bool = all { life::join(a),work'=work }
  action finishClose = all { life::close,work'=work }
  action unchanged = all { life::stay,work'=work }
  action step = {
    nondet a = ACTORS.oneOf()
    nondet m = REQUESTS.oneOf()
    any { createResident(a),openCycle,offer(m),supply(m,a),enter(a),executeUnit(a),
      continueUnit(a),returnResult(a,false),returnResult(a,true),takeVerified(m),
      waitGreen(a),resumeGreen(a,life::p.actorEpoch.get(a),m),cancelCaller(m),
      beginDrain,endDrain,beginClose,requestStop(a),confirmStop(a),joinResident(a),finishClose,unchanged }
  }
  // Retained rejected mechanisms change only the event ledger, not lifecycle.
  action unsafeConstructBeforeCredit(m:int):bool = all { life::stay,
    work'={...work,constructed:work.constructed.set(m,work.constructed.get(m)+1)} }
  action unsafeDuplicateVerification(m:int):bool = all { life::stay,
    work'={...work,verified:work.verified.set(m,work.verified.get(m)+1)} }
  action unsafeExtraCoordination = all { life::stay,work'={...work,extra:work.extra+1} }
  action unsafeReleaseFrame(m:int):bool = all { life::unsafeReturnBeforeConsume(m),work'=work }
}
