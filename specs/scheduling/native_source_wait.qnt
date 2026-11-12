// Native arrival eligibility refines the existing source/frame protocol.
// Sleep is allowed only with no due debt and no live Actor/result custody.
module native_source_wait {
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"
  var asleep:bool
  var timerWake:bool
  var spun:bool
  action keep=all {asleep'=asleep,timerWake'=timerWake,spun'=spun}
  action init=all {p::init,asleep'=false,timerWake'=false,spun'=false}
  action idle=all {p::idle,keep}
  action offer(id:int):bool=all {p::offer(id),asleep'=asleep,timerWake'=(timerWake or asleep),spun'=false}
  action feed(id:int):bool=all {not(asleep),p::feed(id),keep}
  action enter(id:int):bool=all {p::enter(id),keep}
  action admit(id:int):bool=all {p::admit(id),keep}
  action submit(id:int):bool=all {p::submit(id),keep}
  action start(id:int):bool=all {p::start(id),keep}
  action finish(id:int):bool=all {p::finish(id),keep}
  action project(id:int):bool=all {p::project(id),keep}
  action verify(id:int):bool=all {p::verify(id),keep}
  action retire(id:int):bool=all {p::retire(id),keep}
  action release(id:int):bool=all {p::release(id),keep}
  action publish(id:int):bool=all {p::publish(id),keep}
  action consume(id:int):bool=all {p::consume(id),keep}
  action close=all {p::close,asleep'=asleep,timerWake'=true,spun'=spun}
  action park=all {p::pending==Set(),p::live==Set(),p::idle,asleep'=true,timerWake'=false,spun'=false}
  action resume=all {asleep,timerWake,p::idle,asleep'=false,timerWake'=false,spun'=false}
  val safety=and {p::safety,asleep implies p::live==Set(),
    (asleep and p::pending!=Set()) implies timerWake}
  val noIdleSpin=not(spun)
  val admission=and {safety,noIdleSpin}
  // This direct port is only for an empty source. A byte is a wake
  // hint; native acknowledgment and the original plan decide its meaning.
  type DirectPortWake={closed:bool,parkedGeneration:int,
    currentGeneration:int,acknowledged:int,now:int,eligibleAt:int}
  pure def directPortDisposition(w:DirectPortWake):str =
    if(w.closed) "closed"
    else if(w.parkedGeneration!=w.currentGeneration or w.acknowledged==0)
      "invalidated"
    else if(w.acknowledged!=w.parkedGeneration or w.now<w.eligibleAt)
      "recheck"
    else "supply"

  run directPortCancelAndCloseCannotSupplyOrReturnFrameTest=init.then(park)
    .then(all {assert({
      val due={closed:false,parkedGeneration:2,currentGeneration:2,
        acknowledged:2,now:100,eligibleAt:100}
      directPortDisposition(due)=="supply" and
      directPortDisposition({...due,acknowledged:0})=="invalidated" and
      directPortDisposition({...due,closed:true})=="closed" and
      directPortDisposition({...due,currentGeneration:3})=="invalidated" and
      directPortDisposition({...due,now:99})=="recheck" and
      p::live==Set()
    }),idle
    })

  pure def directPortCanRead(open:bool,captured:int,current:int):bool =
    open and captured==current

  // EOF unblocks the reader. The separately retained writer status
  // decides whether this is an ordinary close or the original failure.
  pure def directPortReadOutcome(eof:bool,closed:bool,writerError:int):str =
    if(writerError!=0) "writer-error"
    else if(closed) "closed"
    else if(eof) "unexpected-eof"
    else "check-generation"

  run directPortWriterErrorSurvivesCloseAndFencesReplayTest=init.then(park)
    .then(all {
      assert(directPortCanRead(true,1,1)),
      assert(not(directPortCanRead(false,1,2))),
      assert(not(directPortCanRead(true,1,2))),
      assert(directPortReadOutcome(true,true,9)=="writer-error"),
      assert(directPortReadOutcome(true,true,0)=="closed"),
      assert(directPortReadOutcome(true,false,0)=="unexpected-eof"),
      assert(p::live==Set()),idle
    })

  // The source owns arm and acknowledgment. It cannot arm again
  // while the existing request or its unread notification is live.
  pure def directPortCanArm(entered:bool,closed:bool,armed:bool,pending:bool):bool =
    entered and not(closed or armed or pending)
  pure def directPortEligibilityTime(planned:int,carry:int):int =
    planned + (if(carry==0) 0 else 1)

  run directPortUsesEligibilityCeilingAndSingleOutstandingArmTest=init.then(park)
    .then(all {
      assert(directPortEligibilityTime(100,0)==100),
      assert(directPortEligibilityTime(100,1)==101),
      assert(directPortCanArm(true,false,false,false)),
      assert(not(directPortCanArm(true,false,true,false))),
      assert(not(directPortCanArm(true,false,false,true))),
      assert(not(directPortCanArm(true,true,false,false))),
      assert(p::live==Set()),idle
    })

  action unsafeBusyYield=all {p::pending==Set(),p::live==Set(),p::idle,asleep'=false,timerWake'=timerWake,spun'=true}
  action unsafeParkActive=all {p::idle,asleep'=true,timerWake'=false,spun'=spun}
  action unsafeLoseWake=all {p::idle,asleep'=asleep,timerWake'=false,spun'=spun}
  run timedSourceResumesBeforeSupplyTest=init.then(park).then(offer(2))
    .then(all {assert(asleep),assert(timerWake),assert(admission),idle}).then(resume)
    .then(feed(2)).then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(park).then(all {assert(admission),idle})
  run resourceSafetyDoesNotExcludeBusyYieldTest=init.then(unsafeBusyYield)
    .then(all {assert(safety),assert(not(noIdleSpin)),idle})
  run sleepingWithActorWorkMutationTest=init.then(offer(2)).then(feed(2)).then(unsafeParkActive)
    .then(all {assert(not(safety)),idle})
  run lostArrivalTimerMutationTest=init.then(park).then(offer(2)).then(unsafeLoseWake)
    .then(all {assert(not(safety)),idle})
  action step={nondet id=Set(2,3,4).oneOf()
    any {offer(id),feed(id),enter(id),admit(id),submit(id),start(id),finish(id),project(id),verify(id),
      retire(id),release(id),publish(id),consume(id),close,park,resume,idle}}
}

// External arrivals continue while the OS withholds the one owner CPU. This
// reuses frame/root custody; a runnable Actor is not evidence of owner service.
module native_owner_availability {
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"
  var cpu:bool
  var nativeBlocked:bool
  action keep=all {cpu'=cpu,nativeBlocked'=nativeBlocked}
  action init=all {p::init,cpu'=true,nativeBlocked'=false}
  action idle=all {p::idle,keep}
  action arrival(id:int):bool=all {p::offer(id),keep}
  action preempt=all {p::idle,cpu'=false,nativeBlocked'=nativeBlocked}
  action scheduled=all {p::idle,cpu'=true,nativeBlocked'=nativeBlocked}
  action block=all {cpu,p::idle,cpu'=cpu,nativeBlocked'=true}
  action unblock=all {p::idle,cpu'=cpu,nativeBlocked'=false}
  action ownerStep=all {cpu,not(nativeBlocked),p::step,keep}
  val safety=p::safety
  val canServe=cpu and not(nativeBlocked)
  // Every original periodic arrival inside an owner off-CPU interval must wait
  // until resumption, even with ideal zero-cost application execution. Unknown
  // arrival phase can reduce the count only to this floor, never below it.
  pure def minimumLateArrivals(gap:int,limit:int,period:int):int =
    if (gap>limit) (gap-limit)/period else 0
  pure def phaseLateArrivals(gap:int,limit:int,period:int,phase:int):int =
    0.to(gap-1).filter(t => t%period==phase and gap-t>limit).size()
  run preemptedOwnerAccumulatesDebtWithoutNativeBlockTest=init.then(preempt)
    .then(arrival(2)).then(arrival(3)).then(arrival(4))
    .then(all {assert(safety),assert(not(canServe)),assert(not(nativeBlocked)),
      assert(not(p::sourceBounded)),assert(p::pending==Set(2,3,4)),idle})
    .then(scheduled).then(all {assert(canServe),idle})
  run nativeBlockIsSeparateFromCpuAvailabilityTest=init.then(block)
    .then(arrival(2)).then(all {assert(cpu),assert(nativeBlocked),
      assert(not(canServe)),assert(safety),idle})
    .then(unblock).then(all {assert(canServe),idle})
  run moreActorReadinessCannotScheduleOsOwnerTest=init.then(preempt)
    .then(arrival(2)).then(idle).then(idle)
    .then(all {assert(not(canServe)),assert(p::pending==Set(2)),assert(safety),idle})
  run unknownArrivalPhaseCannotDefeatLatencyFloorTest=init.then(all {
    assert(1.to(4).forall(period => 0.to(16).forall(gap =>
      0.to(8).forall(limit => 0.to(period-1).forall(phase =>
        minimumLateArrivals(gap,limit,period)<=phaseLateArrivals(gap,limit,period,phase)))))),idle})
  run zeroCostExecutionStillMissesDeadlineTest=init.then(preempt)
    .then(all {assert(not(nativeBlocked)),assert(not(canServe)),
      assert(minimumLateArrivals(12,4,2)==4),idle})
  run subDeadlineGapDoesNotProveDeadlineFailureTest=init.then(all {
    assert(minimumLateArrivals(3,4,2)==0),idle})
  run ceilCountWouldOverstateUnknownPhaseTest=init.then(all {
    assert(phaseLateArrivals(5,0,2,1)==2),
    assert(minimumLateArrivals(5,0,2)==2),idle})
  action step={nondet id=Set(2,3,4).oneOf()
    any {arrival(id),preempt,scheduled,block,unblock,ownerStep,idle}}
}

// Necessary service headroom after a stall, not a measured capacity claim.
// Source arrivals keep their original schedule; all quantities use exact units.
module native_service_recovery {
  pure def recoveryNs(stallNs:int,arrivalsPerSecond:int,servicePerSecond:int):int =
    if (servicePerSecond<=arrivalsPerSecond) -1
    else (stallNs*arrivalsPerSecond+servicePerSecond-arrivalsPerSecond-1)
      /(servicePerSecond-arrivalsPerSecond)
  var checked:bool
  action init=checked'=false
  action idle=checked'=checked
  run capacityEqualToArrivalCannotRecoverDebtTest=init.then(all {
    assert(recoveryNs(21249084,500000,500000)== -1),idle})
  run illustrativeHeadroomIsNotAMeasuredServiceRateTest=init.then(all {
    assert(recoveryNs(21249084,500000,600000)==106245420),
    assert(recoveryNs(21249084,500000,1000000)==21249084),idle})
  run increasingQueueCannotChangeRecoveryTimeTest=init.then(all {
    assert(recoveryNs(21249084,500000,600000)>21249084),idle})
  // Count actual consumption against the arrivals due within the same window.
  pure def requiredConsumptions(debt:int,newlyDue:int,limit:int):int =
    if (debt+newlyDue<=limit) 0 else debt+newlyDue-limit

  run frozen6DRecoveryDemandTest=init.then(all {
    assert(requiredConsumptions(351,0,128)==223),
    assert(requiredConsumptions(351,500,128)==723),
    assert(requiredConsumptions(351,5000,128)==5223),idle})
  run frozen7FRecoveryDemandTest=init.then(all {
    assert(requiredConsumptions(6746,0,128)==6618),
    assert(requiredConsumptions(6746,1000,128)==7618),
    assert(requiredConsumptions(6746,10000,128)==16618),idle})
  run debtAtOrBelowLimitOnlyRequiresConsumptionOfExcessTest=init.then(all {
    assert(requiredConsumptions(100,28,128)==0),
    assert(requiredConsumptions(100,29,128)==1),idle})

  // Upper bound for a half-open interval on the original arrival schedule.
  pure def arrivalsDuring(durationNs:int,rate:int):int =
    (durationNs*rate+999999999)/1000000000

  // During the offered interval, every phase of a periodic schedule
  // contributes at least floor(duration * rate) due arrivals.
  pure def minimumDueDuring(durationNs:int,rate:int):int =
    durationNs*rate/1000000000

  run noOwnerRetirementCanBreachBothFrozenGatesTest=init.then(all {
    assert(requiredConsumptions(0,minimumDueDuring(258000,500000),128)==1),
    assert(requiredConsumptions(0,minimumDueDuring(129000,1000000),128)==1),
    assert(requiredConsumptions(120,minimumDueDuring(9000,1000000),128)==1),idle})

  run failedMillionRatePeaksNeedMoreThanOneShortPauseTest=init.then(all {
    assert({
      val shortPause=arrivalsDuring(50000000,1000000)
      shortPause==50000 and
    // Outside the pause, consumption must keep pace for this bound to apply.
      128+shortPause<915441 and 128+shortPause<931464 and
    // Otherwise this much debt had to predate a single 50 ms pause.
      915441-shortPause==865441 and 931464-shortPause==881464
    }),idle})

  run subArrivalServiceCannotCatchUpTest=init.then(all {
    assert(recoveryNs(21249084,500000,450000)== -1),idle})
  action step=idle
}
