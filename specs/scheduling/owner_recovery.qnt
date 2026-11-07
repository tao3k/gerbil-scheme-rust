// Observation refinement, not another Actor scheduler. Reuse the existing
// callback lifecycle and service-headroom equation; never alter source custody.
module owner_recovery {
  import owner_wait_scheme_callback as p from "./native_owner_watch"
  import native_service_recovery as r from "./native_source_wait"
  action init=all {p::init,r::init}
  action idle=all {p::idle,r::idle}
  type Point={due:int,published:int,consumed:int}
  pure def valid(a:Point):bool=0<=a.consumed and a.consumed<=a.published and a.published<=a.due
  pure def outstanding(a:Point):int=a.due-a.consumed
  pure def sourceDebt(a:Point):int=a.due-a.published
  pure def surplus(a:Point,b:Point):int=(b.consumed-a.consumed)-(b.due-a.due)
  pure def conserved(a:Point,b:Point):bool=valid(a) and valid(b) and
    outstanding(a)-outstanding(b)==surplus(a,b)
  run actualSourceDebtDecompositionTest=init.then({
    val peak={due:4644082,published:4642669,consumed:4642667}
    all {assert(valid(peak)),assert(sourceDebt(peak)==1413),assert(outstanding(peak)==1415),
    assert(peak.published-peak.consumed==2),idle}})
  run recoveryMustSettleNewArrivalsAsWellAsOldDebtTest=init.then({
    val start={due:10,published:9,consumed:9}
    val peak={due:30,published:13,consumed:13}
    val end={due:40,published:40,consumed:38}
    all {assert(conserved(start,peak)),assert(surplus(start,peak)== -16),
    assert(conserved(peak,end)),assert(surplus(peak,end)==15),
    assert(outstanding(peak)==17),assert(outstanding(end)==2),idle}})
  // Count real retirements across an observed interval. Spare peer
  // service can only spend independently available, movable work.
  pure def additionalReturnsNeeded(initialDebt:int,arrivals:int,localReturns:int,limit:int):int={
    val excess=initialDebt+arrivals-localReturns-limit
    if(excess>0) excess else 0
  }
  pure def peerCovers(needed:int,movable:int,peerReturns:int):bool=
    movable>=needed and peerReturns>=needed
  run VerifiedGapExceedsTheLimitEvenWithAnEmptyStartingQueueTest={
    val newArrivals=2651
    val maxLocalReturns=1
    assert(additionalReturnsNeeded(0,newArrivals,maxLocalReturns,128)==2522 and
      additionalReturnsNeeded(128,newArrivals,maxLocalReturns,128)==2650)
  }
  run SpareOwnerCannotRecoverWorkAlreadyPinnedOrNotIndependentlySuppliedTest={
    val required=additionalReturnsNeeded(0,2651,1,128)
    assert(not(peerCovers(required,2521,2651)) and
      not(peerCovers(required,2651,2521)) and
      not(peerCovers(required,2651,0)) and
      peerCovers(required,2522,2522))
  }
  // Optimistic bound: one possible carry-in retirement per available
  // owner and at least cycleNs between successive retirements. No source,
  // scheduling, or native-credit delay is charged to the peers here.
  pure def peerReturnCeiling(windowNs:int,cycleNs:int,available:int):int=
    available*((windowNs+cycleNs-1)/cycleNs)
  run AggregateNominalRateDoesNotSurviveOneUnavailableOwnerAtFiveMillionTest={
    val window=129000
    val arrivals=window/200
    val required=additionalReturnsNeeded(0,arrivals,0,128)
    assert(arrivals==645 and required==517 and
      peerReturnCeiling(window,1000,4)==516 and
      not(peerCovers(required,arrivals,peerReturnCeiling(window,1000,4))) and
      peerReturnCeiling(window,1000,5)==arrivals and
      additionalReturnsNeeded(128,arrivals,peerReturnCeiling(window,1000,5),128)==0 and
      128+arrivals-peerReturnCeiling(window,1000,5)==128 and
      peerReturnCeiling(128000,1000,6)-128000/200==128)
  }
  // Count distinct eligible retirements, not multiple observations of the same
  // available Rust offer or work already pinned in a different owner.
  pure def assignedPeerReturns(peers:Set[int],eligible:int -> Set[int],
      assigned:int -> Set[int],slots:int -> int,needed:int):bool=
    peers.forall(owner=>assigned.get(owner).subseteq(eligible.get(owner)) and
      assigned.get(owner).size()<=slots.get(owner) and
      peers.exclude(Set(owner)).forall(other=>
        assigned.get(owner).intersect(assigned.get(other))==Set())) and
    peers.fold(0,(count,owner)=>count+assigned.get(owner).size())>=needed
  run OverlappingOffersCannotBeCountedAsDistinctPeerSupplyTest={
    val peers=Set(1,2)
    val slots=Map(1->1,2->1)
    val shared=Map(1->Set(1),2->Set(1))
    val separate=Map(1->Set(1),2->Set(2))
    assert(not(assignedPeerReturns(peers,shared,shared,slots,2)) and
      not(assignedPeerReturns(peers,shared,separate,slots,2)) and
      assignedPeerReturns(peers,Map(1->Set(1,2),2->Set(1,2)),separate,slots,2))
  }
  run SharedSourcePauseCanExhaustTheBudgetWithEveryOwnerAvailableTest={
    val arrivals=26000/200
    val required=additionalReturnsNeeded(0,arrivals,0,128)
    assert(arrivals==130 and required==2 and
      not(peerCovers(required,0,peerReturnCeiling(26000,1000,6))))
  }
  // The public pipeline future returns its delivery handle only after
  // publication seals and all producer registrations finish. Native work
  // may already have projected and returned credits before that boundary.
  type PipelineDelivery={verified:int,observed:int,sealed:bool,registered:bool}
  pure def handleAvailable(c:PipelineDelivery):bool=c.sealed and c.registered
  pure def validPipelineDelivery(c:PipelineDelivery):bool=
    0<=c.observed and c.observed<=c.verified and (c.observed==0 or handleAvailable(c))
  run NativeCreditReturnDoesNotProveClientDeliveryBeforeSealTest={
    val open={verified:1,observed:0,sealed:false,registered:true}
    val pending={...open,sealed:true,registered:false}
    val observed={...open,sealed:true,observed:1}
    assert(validPipelineDelivery(open) and not(handleAvailable(open)) and
      validPipelineDelivery(pending) and not(handleAvailable(pending)) and
      not(validPipelineDelivery({...open,observed:1})) and
      not(validPipelineDelivery({...pending,observed:1})) and validPipelineDelivery(observed))
  }
  // Proposed split handle. Cancel uses the shared source, not a delivery
  // handle's snapshot of producer signals. Accepted roots stay at their owner.
  type OpenDelivery={open:bool,cancelled:bool,staged:Set[int],accepted:Set[int],
    ready:Set[int],observed:Set[int],registered:Set[int],wakeOwed:Set[int]}
  pure def cancelOpenDelivery(c:OpenDelivery):OpenDelivery=
    {...c,open:false,cancelled:true,staged:Set(),wakeOwed:c.wakeOwed.union(c.registered)}
  pure def canPublishOpenDelivery(c:OpenDelivery):bool=c.open and not(c.cancelled)
  pure def observeOpenDelivery(c:OpenDelivery,m:int):OpenDelivery=
    if(c.ready.contains(m)) {...c,observed:c.observed.union(Set(m))} else c
  run EarlyDeliveryAndCancellationUseSeparateAuthorityTest={
    val open={open:true,cancelled:false,staged:Set(2),accepted:Set(1,3),ready:Set(1),observed:Set(),
      registered:Set(1),wakeOwed:Set()}
    val observed=observeOpenDelivery(open,1)
    val cancelled=cancelOpenDelivery(observed)
    assert(canPublishOpenDelivery(observed) and observed.staged==Set(2) and
      observeOpenDelivery(open,3)==open and
      observed.accepted==Set(1,3) and observed.observed==Set(1) and
      not(canPublishOpenDelivery(cancelled)) and cancelled.staged==Set() and
      cancelled.accepted==Set(1,3) and cancelled.wakeOwed==Set(1))
  }
  run DroppedUnpolledPublisherCannotLeaveAnOpenSourceTest={
    val unpolled={open:true,cancelled:false,staged:Set(),accepted:Set(),ready:Set(),observed:Set(),
      registered:Set(),wakeOwed:Set()}
    val cancelled=cancelOpenDelivery(unpolled)
    assert(not(canPublishOpenDelivery(cancelled)) and cancelled.accepted==Set() and
      cancelled.wakeOwed==Set())
  }
  // A producer wake targets registered owners. A pending registration
  // future also needs its own wake; otherwise a held permit can keep it asleep.
  type PendingPublisher={permitAvailable:bool,sourceCancelled:bool,publisherWake:bool}
  pure def publisherOutcome(c:PendingPublisher):str=
    if(c.sourceCancelled and c.publisherWake) "cancelled"
    else if(c.permitAvailable) "registration-ready" else "pending"
  run SourceRejectionDoesNotWakePublisherWaitingOnAnotherPermitTest={
    val held={permitAvailable:false,sourceCancelled:true,publisherWake:false}
    assert(publisherOutcome(held)=="pending" and
      publisherOutcome({...held,publisherWake:true})=="cancelled" and
      not(held.permitAvailable))
  }
  run NoEarlyClientObservationsAndOpposingPairsDoNotQualifySplitDeliveryTest={
    val firstControl={wall:604255500,tail:173200334,spread:23112208}
    val firstSplit={wall:359045333,tail:51096875,spread:28037833,early:0}
    val lastControl={wall:278227208,tail:47002333,spread:59411417}
    val lastSplit={wall:355597084,tail:58840959,spread:19549375,early:0}
    assert(firstSplit.wall<firstControl.wall and firstSplit.tail<firstControl.tail and
      firstSplit.spread>firstControl.spread and
      lastSplit.wall>lastControl.wall and lastSplit.tail>lastControl.tail and
      lastSplit.spread<lastControl.spread and firstSplit.early+lastSplit.early==0)
  }
  run returnedFramesDoNotProveRecoveryHeadroomTest=init.then(all {
    assert(r::recoveryNs(2826000,500000,500000)== -1),
    assert(r::recoveryNs(2826000,500000,600000)==14130000),idle})
  // A synthetic distribution, not a reconstruction of the native histogram.
  // A favorable p99 can coexist with a much worse maximum and failed debt gate.
  run favorableQuantileDoesNotAdmitMaximumDebtTest=init.then(all {
    assert((5000000*99+99)/100==4950000),
    assert(4999000>=4950000),assert(4999000+1000==5000000),
    assert(298000<317000),assert(1415>128),idle})
  run independentComplementRanksMatchOriginalCeilTest=init.then(all {
    assert(1.to(10000).forall(n => (n*99+99)/100==n-n/100 and
      (n*999+999)/1000==n-n/1000)),idle})
  run laterPeakCannotBorrowEarlierGcWindowTest=init.then(all {
    assert(9288162667>722629542),assert(4642667!=360849),idle})
  // The rejected observer's debt fell only after the finite source stopped.
  // A lower average arrival rate across that drain is not 500k/s headroom.
  run sourceStopDrainCannotAdmitSustainedRecoveryTest=init.then(all {
    assert(9730646583<9999998000),assert(10768433416>9999998000),
    assert(488294>129772),assert(488294<500000),
    assert(r::recoveryNs(744396000,500000,488294)== -1),idle})
  // One rejected cold-report extraction run, not proof of its cause. The
  // native image stayed identical; zero drops and final drain do not admit it.
  run coldReportVariantCannotOverrideOriginalGatesTest=init.then({
    val peak={due:2335581,published:2190967,consumed:2190965}
    all {assert(valid(peak)),assert(sourceDebt(peak)==144614),
      assert(outstanding(peak)==144616),assert(outstanding(peak)>128),
      assert(270485000>418000),idle}})
}
