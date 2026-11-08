// Reuse frame/root custody. A source-owned rational cursor assigns immutable
// original planned times, including out-of-order completion and frame reuse.
module native_source_plan {
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"
  val units=10
  val rate=3
  var ordinal:int
  var nextTime:int
  var carry:int
  var stamps:int -> int
  pure def original(n:int):int=n*units/rate
  action keep=all {ordinal'=ordinal,nextTime'=nextTime,carry'=carry,stamps'=stamps}
  action init=all {p::init,ordinal'=0,nextTime'=0,carry'=0,stamps'=Map(2-> -1,3-> -1,4-> -1)}
  action idle=all {p::idle,keep}
  action offer(id:int):bool=all {p::offer(id),keep}
  action feed(id:int):bool=all {id==ordinal+2,p::feed(id),ordinal'=ordinal+1,
    nextTime'=nextTime+units/rate+(if (carry+units%rate>=rate) 1 else 0),
    carry'=(carry+units%rate)%rate,stamps'=stamps.put(id,nextTime)}
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
  action consume(id:int):bool=all {stamps.get(id)==original(id-2),p::consume(id),
    ordinal'=ordinal,nextTime'=nextTime,carry'=carry,stamps'=stamps.put(id,-1)}
  action close=all {p::close,keep}
  val safety=and {p::safety,nextTime==original(ordinal),carry==(ordinal*units)%rate,
    Set(2,3,4).forall(id => if (p::live.contains(id)) stamps.get(id)==original(id-2) else stamps.get(id)== -1)}
  action unsafeRetime(id:int):bool=all {p::idle,ordinal'=ordinal,nextTime'=nextTime,
    carry'=carry,stamps'=stamps.put(id,nextTime+1)}
  action unsafeSkipCarry=all {p::idle,ordinal'=ordinal,nextTime'=nextTime+1,
    carry'=carry,stamps'=stamps}
  run sourceStallDoesNotMoveOriginalTimestampTest=init.then(offer(2)).then(feed(2))
    .then(idle).then(idle).then(all {assert(stamps.get(2)==0),assert(safety),idle})
  run lateWorkCannotResetItsClockMutationTest=init.then(offer(2)).then(feed(2))
    .then(unsafeRetime(2)).then(all {assert(not(safety)),idle})
  run missingFractionalCarryMutationTest=init.then(unsafeSkipCarry)
    .then(all {assert(not(safety)),idle})
  run outOfOrderCompletionAndReuseKeepsEachOriginalTimeTest=init
    .then(offer(2)).then(feed(2)).then(offer(3)).then(feed(3))
    .then(enter(3)).then(admit(3)).then(submit(3)).then(start(3)).then(finish(3))
    .then(project(3)).then(verify(3)).then(retire(3)).then(release(3)).then(publish(3))
    .then(consume(3)).then(offer(4)).then(feed(4))
    .then(enter(2)).then(admit(2)).then(submit(2))
    .then(enter(4)).then(admit(4)).then(submit(4)).then(close)
    .then(all {assert(stamps.get(2)==0),assert(stamps.get(4)==6),assert(safety),idle})
    .then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(start(4)).then(finish(4))
    .then(project(4)).then(verify(4)).then(retire(4)).then(release(4)).then(publish(4)).then(consume(4))
    .then(all {assert(stamps==Map(2-> -1,3-> -1,4-> -1)),assert(safety),idle})
  run integerAndFractionalTransitionIdentityTest=init.then(all {
    assert(1.to(9).forall(r => 0.to(20).forall(n =>
      (n*units/r)+(units/r)+(if ((n*units)%r+units%r>=r) 1 else 0)==((n+1)*units)/r))),idle})
  // Op62 has a fixed drive sample; it can use the next timestamp only
  // as an early-negative predicate. A positive check still needs exact debt.
  pure def legacyDue(now:int,period:int,total:int):int =
    if (now/period+1<total) now/period+1 else total
  pure def frozenDueWithEarlyNegative(now:int,issued:int,period:int,total:int):int =
    if (issued==total or now<issued*period) issued else legacyDue(now,period,total)
  run frozenSourceEarlyNegativeRetainsDueAndRefillTest = assert(
    Set(1000,2000).forall(period => 1.to(5).forall(total =>
      0.to(6).forall(tick => Set(0,1,period-1).forall(offset => {
        val now=tick*period+offset
        val due=legacyDue(now,period,total)
        0.to(due).forall(issued => frozenDueWithEarlyNegative(now,issued,period,total)==due)
      })))))
  run sourceEligibilityIsNotOneItemOfDueDebtTest = assert(
    8000>=1*2000 and legacyDue(8000,2000,10)==5)
  // At fractional rates, the existing forward and inverse rounding
  // are not interchangeable at every integer clock boundary.
  run fractionalTimestampCannotReplaceLegacyDueTest = assert(
    original(1)==3 and 3*rate/units+1==1)
  // The actual loop and failure receipt share the first count. The owned
  // plan timestamps use floor division, whose inverse is the second count.
  pure def capped(count:int,total:int):int = if (count<total) count else total
  pure def sampledDue(t:int,r:int,total:int):int = capped(1+t*r/units,total)
  pure def floorScheduleDue(t:int,r:int,total:int):int = capped(((t+1)*r+units-1)/units,total)
  run EligibilityInvertsFlooredPlannedTimeTest = assert(
    1.to(10).forall(r => 0.to(30).forall(t =>
      floorScheduleDue(t,r,12)==0.to(11).filter(i => i*units/r<=t).size())))
  run IntegralCadenceMatchesAndFractionalCanBeOneDueLateTest = assert(
    1.to(10).forall(r => 0.to(30).forall(t => {
      val sampled=sampledDue(t,r,12)
      val exact=floorScheduleDue(t,r,12)
      sampled<=exact and exact<=sampled+1 and (units%r!=0 or sampled==exact)
    })) and sampledDue(3,3,12)==1 and floorScheduleDue(3,3,12)==2)
  run FractionalCountAndRetirementCanBothHideAnOutstandingRequestTest = assert(
    sampledDue(0,3,12)==1 and sampledDue(6,3,12)-1==1 and
      floorScheduleDue(6,3,12)==3)
  // The ordinary op62 supply scans every frame, then waits for a reply
  // when active is positive. Native frames hold every published,
  // unreturned request, with completed = published - active at loop entry.
  pure def activeAfterSupply(due:int,published:int,active:int,slots:int):int =
    active+capped(due-published,slots-active)
  run FreshAllDueSampleCannotReachZeroActiveYieldWhileUnfinishedTest=assert(
    1.to(8).forall(total=>0.to(total).forall(published=>
      0.to(capped(3,published)).forall(active=>
        published-active==total or activeAfterSupply(total,published,active,3)>0))))
  run CrossingLastTickDoesNotRetroactivelyRefreshSourceSampleTest={
    val total=5
    val published=4
    assert(activeAfterSupply(4,published,0,3)==0 and
      activeAfterSupply(total,published,0,3)==1)
  }
  // A complete integral schedule stamps each successful send. At
  // the sampled total peak, at most frames are already published.
  pure def oldestPublicationStampDelayCeiling(complete:bool,totalPeak:int,frames:int,
      period:int,maxLag:int):int={
    val oldestAge=(totalPeak-frames-1)*period
    if(complete and totalPeak>frames and frames>=0 and period>0 and maxLag>=oldestAge)
      maxLag-oldestAge
    else -1
  }
  run SixDPeakRequiresEarlierSupplyRecoveryButNextStampWasPromptTest={
    assert(oldestPublicationStampDelayCeiling(true,351,3,2000,697625)==3625 and
      oldestPublicationStampDelayCeiling(true,6746,3,1000,6743416)==1416 and
      oldestPublicationStampDelayCeiling(true,4000404,3,200,3984036284)==3183956284 and
      oldestPublicationStampDelayCeiling(false,351,3,2000,697625)== -1 and
      oldestPublicationStampDelayCeiling(true,351,3,2000,693999)== -1)
  }
  run FrozenMillionPeakAlreadyHasDebtBeforePromptRefillTest={
    // The raw drive samples before supply. With three frames, each
    // preceding nonidle turn retires one, leaving at most two charged.
    val activeAtSample=0.to(3).map(afterSupply =>
      if(afterSupply==0) 0 else afterSupply-1)
    assert(activeAtSample==Set(0,1,2) and
      12969-12967==2 and 19717-19715==2 and
      oldestPublicationStampDelayCeiling(true,12969,2,1000,12966875)==875 and
      oldestPublicationStampDelayCeiling(true,19717,2,1000,19714625)==625)
  }
  run LastTurnCanExceedWindowBeforeItsProgressStampTest={
    val period=1000
    val lastPublication=1458458
    val progress=1462416
    val sample=1462458
    val published=1336
    val beforeReturn=1333
    val afterReturn=beforeReturn+1
    val exactCrossing=(beforeReturn+128)*period
    val dueAtPublication=1+lastPublication/period
    assert(dueAtPublication>published and published-afterReturn==2 and
      dueAtPublication-beforeReturn==126 and lastPublication<exactCrossing and
      1+(exactCrossing-1)/period-beforeReturn==128 and
      1+exactCrossing/period-beforeReturn==129 and exactCrossing<progress and
      1+sample/period-afterReturn==129 and progress-lastPublication==3958 and
      sample-progress==42)
  }
  // On the original complete three-frame loop, each sampled breach
  // forces a retirement. Before the first sampled breach at most
  // completed - aboveLimit requests can have returned.
  pure def firstExactBreachCeiling(complete:bool,completed:int,aboveLimit:int,
      limit:int,period:int):int =
    if(complete and 0<aboveLimit and aboveLimit<=completed and limit>=0 and period>0)
      (completed-aboveLimit+limit)*period
    else -1
  run FrozenMillionExceedsWindowDuringInitialSupplyTest={
    assert(firstExactBreachCeiling(true,1000000,999828,128,1000)==300000 and
      firstExactBreachCeiling(true,1000000,998916,128,1000)==1212000 and
      firstExactBreachCeiling(true,1000000,998494,128,1000)==1634000 and
      999828-(115280+1)==884547 and
      firstExactBreachCeiling(false,1000000,999828,128,1000)== -1 and
      firstExactBreachCeiling(true,1000000,0,128,1000)== -1)
  }
  // Debt is due minus retired. After the last scheduled arrival it can
  // only fall. Even zero retirement before cutoff cannot account for
  // more than the arrivals already due at cutoff.
  pure def laterOfferDeficitFloor(total:int,period:int,cutoff:int,sampledPeak:int):int={
    if(total<=0 or period<=0 or cutoff<0 or sampledPeak<0 or sampledPeak>total) -1
    else {
      val dueAtCutoff=capped(total,1+cutoff/period)
      if(sampledPeak>dueAtCutoff) sampledPeak-dueAtCutoff else 0
    }
  }
  run StartupOnlyLossCannotExplainTheRetainedMillionPeaksTest={
    val total=1000000
    val period=1000
    assert(laterOfferDeficitFloor(total,period,300000,115280)==114979 and
      laterOfferDeficitFloor(total,period,10000000,115280)==105279 and
      laterOfferDeficitFloor(total,period,10000000,116723)==106722 and
      laterOfferDeficitFloor(total,period,10000000,154547)==144546 and
      laterOfferDeficitFloor(total,period,115278000,115280)==1 and
      laterOfferDeficitFloor(total,period,115279000,115280)==0)
  }
  run LaterOfferDeficitKeepsCohortsAndInvalidInputsSeparateTest=assert(
    laterOfferDeficitFloor(5000000,2000,10000000,351)==0 and
    laterOfferDeficitFloor(5000000,200,10000000,4000404)==3950403 and
    laterOfferDeficitFloor(1000000,1000,1000000000,115280)==0 and
    laterOfferDeficitFloor(1000000,0,10000000,115280)== -1 and
    laterOfferDeficitFloor(1000000,1000,10000000,1000001)== -1)
  // Frame counts from a sealed source receipt do not recover the
  // missing queued/running/ready marker from an output port.
  run FirstSampledBreachKeepsCountsWhenFrameMarkerIsLostTest={
    val elapsed=178916
    val due=1+elapsed/1000
    val published=49
    val consumed=47
    val charged=published-consumed
    val possible=Set({queued:2,running:0,ready:0},
      {queued:1,running:0,ready:1},{queued:0,running:0,ready:2})
    assert(due==179 and due-consumed==132 and due-published==130 and
      charged==2 and 3-charged==1 and
      possible.forall(s=>s.queued+s.running+s.ready==charged) and
      possible.map(s=>s.ready)==Set(0,1,2))
  }
  run ReadyAtFirstBreachDoesNotProveRepliesAreInTheInboxTest={
    val elapsed=172750
    val due=1+elapsed/1000
    val published=43
    val consumed=41
    val ready=2
    // No enqueue marker was observed. The model leaves delivery
    // unresolved rather than asserting a reachable SDK schedule.
    val unexcludedEnqueued=0.to(ready)
    assert(due==173 and due-consumed==132 and due-published==130 and
      published-consumed==ready and 3-ready==1 and
      unexcludedEnqueued.contains(0) and unexcludedEnqueued.contains(2))
  }
  run ReadyAndPostSendMarkerCannotSubstituteForReceiverEvidenceTest={
    // Explicit handshakes demonstrate each state. They do not
    // imply preemption on the uninterrupted op62 send edge.
    val beforeSend={ready:true,enqueued:false,dequeued:false,senderMarked:false}
    val receiverFirst={...beforeSend,enqueued:true,dequeued:true}
    val afterMark={...receiverFirst,senderMarked:true}
    assert(beforeSend.ready and not(beforeSend.enqueued) and
      receiverFirst.dequeued and not(receiverFirst.senderMarked) and
      afterMark.dequeued and afterMark.senderMarked)
  }
  run DiagnosticDequeueDoesNotRetireTheMatchedFrameTest={
    val sample={elapsed:529000,published:403,consumed:401,charged:2}
    val receive={before:4750,after:5375,id:401,slot:1}
    val due=1+sample.elapsed/1000
    assert(due==530 and due-sample.consumed==129 and
      due-sample.published==127 and receive.id<sample.published and
      0<=receive.slot and receive.slot<3 and
      sample.elapsed+receive.before==533750 and sample.elapsed+receive.after==534375 and
      sample.published-sample.consumed==sample.charged and
      sample.published-(sample.consumed+1)!=sample.charged)
  }
  action step={nondet id=Set(2,3,4).oneOf()
    any {offer(id),feed(id),enter(id),admit(id),submit(id),start(id),finish(id),project(id),
      verify(id),retire(id),release(id),publish(id),consume(id),close,idle}}
}
