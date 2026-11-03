// Joint admission/delivery/harvest refinement. No duplicate Actor lifecycle.
module mailbox_joint_cycle {
  import actor_service_contract as gate from "./actor_service_contract"
  import mailbox_packet_protocol(ACTORS=Set(1,2,3,4),REQUESTS=1.to(6),LIMIT=16) as mp from "./mailbox_packets"
  type Control={phase:str,offered:Set[int],reserved:List[int],affinity:int->int,certified:Set[int],
    selected:int,rounds:int,routes:int,harvested:int,emptyProbes:int,closed:bool,opportunity:str,opportunityChecks:int}
  var c:Control
  val reserved=mp::members(c.reserved)
  val charged=reserved.size()+mp::outstanding.size()
  val pendingResults=mp::ready
  val publishedRequests=mp::actorState.published
  val consumedRequests=mp::actorState.consumed
  val unsafeActors=mp::actorState.published.exclude(mp::actorState.completed)
    .filter(m=>not(c.certified.contains(m))).map(m=>mp::actorState.owner.get(m))
  val reservedPins=reserved.filter(m=>c.affinity.get(m)!=0).map(m=>c.affinity.get(m))
  val safe=mp::safe and Set("selected","waiting","runnable").contains(c.opportunity) and
    (c.opportunity=="waiting" implies mp::p.parked) and
    (mp::p.parked implies c.phase=="await") and
    c.reserved.length()==reserved.size() and charged<=mp::actorState.tokens and
    reserved.intersect(mp::actorState.published)==Set() and
    reserved.union(mp::actorState.published).subseteq(c.offered) and
    mp::actorState.published.forall(m=>c.affinity.get(m)==0 or mp::actorState.owner.get(m)==c.affinity.get(m)) and
    (mp::p.parked implies c.reserved==List() and mp::p.staged==List()) and
    (c.closed implies mp::p.closed and charged==0 and c.offered==mp::actorState.consumed)
  action boot(cpuOnly:bool):bool=all {mp::initPackets,c'={phase:"collect",offered:Set(),reserved:List(),
      affinity:1.to(6).mapBy(m=>if(not(cpuOnly) and Set(3,6).contains(m)) 2 else 0),
      certified:if(cpuOnly) 1.to(6) else Set(1,2,4,5),selected:0,rounds:0,routes:0,
      harvested:0,emptyProbes:0,closed:false,opportunity:"selected",opportunityChecks:0}}
    .then(all {mp::create(1),c'=c}).then(all {mp::create(2),c'=c})
    .then(all {mp::create(3),c'=c}).then(all {mp::create(4),c'=c})
    .then(all {mp::credits(3),c'=c}).then(all {mp::open,c'=c})
  action bootCpu=boot(true)
  action bootMixed=boot(false)
  action bootUnknown=bootMixed.then(all {mp::stay,c'={...c,certified:Set()}})
  action stay=all {mp::stay,c'=c}
  action arrive(m:int):bool=all {not(c.closed),not(c.offered.contains(m)),mp::stay,c'={...c,offered:c.offered.union(Set(m))}}
  def canReserve(m:int):bool=c.opportunity=="selected" and c.phase=="collect" and mp::ready==Set() and
    c.offered.contains(m) and not(reserved.union(mp::actorState.published).contains(m)) and charged<mp::actorState.tokens
  action reserve(m:int):bool=all {canReserve(m),mp::stay,c'={...c,reserved:c.reserved.append(m)}}
  // Original eager policy: already-ready siblings do not suppress replenishment
  // after one real consumption. Reuse the same custody and publication actions.
  def canReserveEager(m:int):bool=c.opportunity=="selected" and c.phase=="collect" and
    c.offered.contains(m) and not(reserved.union(mp::actorState.published).contains(m)) and charged<mp::actorState.tokens
  action reserveEager(m:int):bool=all {canReserveEager(m),mp::stay,c'={...c,reserved:c.reserved.append(m)}}
  action endHarvestEager=all {c.opportunity=="selected",c.phase=="harvest",c.harvested>0,mp::stay,
    c'={...c,phase:"collect",selected:0}}

  // A pre-entry reservation holds no Scheme payload and may be withdrawn; its
  // offered request is not silently reported as completed or removed from debt.
  action withdraw(m:int):bool=all {c.opportunity=="selected",c.phase=="collect",reserved.contains(m),mp::stay,
    c'={...c,reserved:mp::keep(c.reserved,reserved.exclude(Set(m)))}}
  action seal=all {c.opportunity=="selected",c.phase=="collect",c.reserved.length()>0,mp::stay,
    c'={...c,phase:"publish",selected:0,rounds:c.rounds+1}}
  action chooseOwner(a:int):bool=all {c.opportunity=="selected",c.phase=="publish",c.selected==0,not(unsafeActors.contains(a)),
    reserved.exists(m=>c.affinity.get(m)==0 and c.certified.contains(m)),mp::stay,
    c'={...c,selected:a,routes:c.routes+1}}
  def idle(a:int):bool=mp::actorState.work.get(a)==0 and mp::actorState.queue.get(a)==List() and
    not(mp::actorState.waiting.contains(a))
  def canOwner(m:int,a:int):bool=if(c.affinity.get(m)!=0) c.affinity.get(m)==a
    else if(c.certified.contains(m)) c.selected==a and not(unsafeActors.contains(a))
    else idle(a) and not(reservedPins.contains(a))
  def canPublish(m:int,a:int):bool=c.opportunity=="selected" and c.phase=="publish" and c.reserved.length()>0 and
    c.reserved.head()==m and not(mp::actorState.arrived.contains(m)) and canOwner(m,a)
  action publish(m:int,a:int):bool=if(canPublish(m,a))
    all {mp::offer(m),c'=c}.then(all {mp::stage(m,a),
      c'={...c,reserved:c.reserved.tail(),routes:c.routes+(if(c.affinity.get(m)==0 and not(c.certified.contains(m))) 1 else 0)}})
    else all {false,mp::stay,c'=c}
  def packetFor(a:int):List[int]=mp::keep(mp::p.staged,
    mp::staged.filter(m=>mp::actorState.owner.get(m)==a))
  action flush(a:int):bool=all {c.opportunity=="selected",c.phase=="publish",c.reserved==List(),mp::send(packetFor(a),1),c'=c}
  action await=all {c.opportunity=="selected",c.phase=="publish",c.reserved==List(),mp::p.staged==List(),
    mp::ready==Set(),mp::outstanding.size()>0,mp::park,c'={...c,phase:"await",opportunity:"waiting"}}
  action harvestDirect=all {c.opportunity=="selected",c.phase=="publish",c.reserved==List(),mp::p.staged==List(),
    mp::ready.size()>0,mp::stay,c'={...c,phase:"harvest",harvested:0}}
  action harvestWoken=all {c.phase=="await",mp::wake,c'={...c,phase:"harvest",harvested:0,opportunity:"selected"}}
  action consume(m:int):bool=all {c.opportunity=="selected",c.phase=="harvest",mp::consume(m),c'={...c,harvested:c.harvested+1}}
  val canEndHarvest=c.phase=="harvest" and mp::ready==Set()
  action endHarvest=all {c.opportunity=="selected",canEndHarvest,mp::stay,c'={...c,phase:"collect",selected:0,
    emptyProbes:c.emptyProbes+(if(mp::outstanding.size()>0 and c.harvested>0) 1 else 0)}}
  action continueOutstanding=all {c.opportunity=="selected",c.phase=="collect",c.reserved==List(),mp::outstanding.size()>0,
    mp::stay,c'={...c,phase:"publish"}}
  action idleReturn=all {c.opportunity=="selected",c.phase=="publish",c.reserved==List(),mp::p.staged==List(),
    mp::outstanding==Set(),mp::stay,c'={...c,phase:"collect"}}
  action decode(a:int):bool=all {mp::decode(a),c'=c}
  action unit(a:int):bool=all {mp::unit(a),c'=c}
  action proceed(a:int):bool=all {mp::proceed(a),c'=c}
  action finish(a:int,m:int,failed:bool):bool=all {mp::finish(a,m,1,failed),c'={...c,opportunity:if(c.opportunity=="waiting") "runnable" else c.opportunity}}
  action success(a:int,m:int):bool=decode(a).then(unit(a)).then(proceed(a)).then(finish(a,m,false))
  action waitFor(a:int,m:int):bool=all {not(c.certified.contains(mp::actorState.work.get(a))),mp::waitFor(a,m),c'=c}
  action resume(a:int,m:int,epoch:int):bool=all {mp::resume(a,m,epoch),c'=c}
  action cancel(m:int):bool=all {mp::cancel(m),c'=c}
  action close=all {c.opportunity=="selected",c.phase=="collect",c.reserved==List(),c.offered==mp::actorState.consumed,mp::startDrain,c'=c}
    .then(all {mp::finishDrain,c'=c}).then(all {mp::startClose,c'=c})
    .then(all {mp::stop(1),c'=c}).then(all {mp::stopped(1),c'=c}).then(all {mp::join(1),c'=c})
    .then(all {mp::stop(2),c'=c}).then(all {mp::stopped(2),c'=c}).then(all {mp::join(2),c'=c})
    .then(all {mp::stop(3),c'=c}).then(all {mp::stopped(3),c'=c}).then(all {mp::join(3),c'=c})
    .then(all {mp::stop(4),c'=c}).then(all {mp::stopped(4),c'=c}).then(all {mp::join(4),c'=c})
    .then(all {mp::finishClose,c'={...c,closed:true,phase:"closed"}})
  val descriptors=mp::p.groups.foldl(0,(n,g)=>n+(if(g.length()>1) 1 else 0))
  val packetMembers=mp::p.groups.foldl(0,(n,g)=>n+(if(g.length()>1) g.length() else 0))
  // Extra costs include singleton dispatch checks, route/round setup and failed
  // nonblocking receives. Weights are conditional inputs, not native CPU ns.
  pure def extra(n:int,k:int,desc:int,members:int,probes:int,routes:int,u:int,
    pack:int,decode:int,probe:int,route:int,check:int,setup:int):int=
    desc*pack+members*decode+probes*probe+routes*route+n*check+setup-(n-k)*u
  action admit3=arrive(1).then(arrive(2)).then(arrive(3)).then(reserve(1)).then(reserve(2)).then(reserve(3)).then(seal)
  action cpu3=bootCpu.then(admit3).then(chooseOwner(1)).then(publish(1,1)).then(publish(2,1)).then(publish(3,1)).then(flush(1))
  action cpuReady3=cpu3.then(success(1,1)).then(success(1,2)).then(success(1,3)).then(harvestDirect)
  run lateBindingAndNaturalHarvestExposeTwoBoundedPacketsTest=cpuReady3
    .then(consume(1)).then(all {assert(safe),assert(charged==2),assert(not(canEndHarvest)),assert(not(canReserve(4))),stay})
    .then(consume(2)).then(consume(3)).then(endHarvest)
    .then(arrive(4)).then(arrive(5)).then(arrive(6)).then(reserve(4)).then(reserve(5)).then(reserve(6))
    .then(seal).then(chooseOwner(2)).then(publish(4,2)).then(publish(5,2)).then(publish(6,2)).then(flush(2))
    .then(success(2,4)).then(success(2,5)).then(success(2,6)).then(harvestDirect)
    .then(consume(4)).then(consume(5)).then(consume(6)).then(endHarvest).then(close)
    .then(all {assert(safe),assert(mp::p.groups.length()==2),assert(mp::p.replies==6),
      assert(descriptors==2),assert(packetMembers==6),assert(c.emptyProbes==0),assert(charged==0),stay})
  run partialReadyDoesNotWaitForThePacketOrReleaseSiblingsTest=cpu3.then(success(1,1)).then(harvestDirect)
    .then(consume(1)).then(endHarvest).then(all {assert(safe),assert(charged==2),
      assert(mp::outstanding==Set(2,3)),assert(c.emptyProbes==1),assert(c.phase=="collect"),stay})
  run pinnedSuspensionCanResumeFromANewStatelessPeerTest=bootMixed.then(admit3).then(chooseOwner(1))
    .then(publish(1,1)).then(publish(2,1)).then(all {assert(not(canOwner(3,1))),stay})
    .then(publish(3,2)).then(flush(1)).then(flush(2)).then(decode(2)).then(waitFor(2,4))
    .then(success(1,1)).then(harvestDirect).then(consume(1)).then(endHarvest)
    .then(arrive(4)).then(reserve(4)).then(seal).then(all {assert(not(canOwner(4,2))),stay})
    .then(chooseOwner(3)).then(publish(4,3)).then(flush(3)).then(success(3,4)).then(harvestDirect)
    .then(consume(4)).then(endHarvest).then(resume(2,3,1)).then(unit(2)).then(proceed(2)).then(finish(2,3,false))
    .then(success(1,2)).then(continueOutstanding).then(harvestDirect).then(consume(3)).then(consume(2)).then(endHarvest).then(close)
    .then(all {assert(safe),assert(mp::actorState.owner.get(3)==2),assert(mp::p.waits==1),assert(mp::p.resumes==1),stay})
  run offeredReservationCanBeWithdrawnWithoutCreatingOrLosingPayloadTest=bootCpu.then(arrive(1)).then(reserve(1))
    .then(withdraw(1)).then(all {assert(safe),assert(charged==0),assert(c.offered==Set(1)),
      assert(mp::workLedger.constructed.get(1)==0),stay})
    .then(reserve(1)).then(seal).then(chooseOwner(1)).then(publish(1,1)).then(flush(1))
    .then(success(1,1)).then(harvestDirect).then(consume(1)).then(endHarvest).then(close).then(all {assert(safe),stay})
  run acceptedCancellationAndFailureRetainIndividualCapacityTest=cpu3.then(cancel(2)).then(decode(1)).then(finish(1,1,true))
    .then(harvestDirect).then(consume(1)).then(endHarvest).then(all {
      assert(safe),assert(mp::outstanding==Set(2,3)),assert(charged==2),assert(mp::actorState.failed==Set(1)),stay})
  run unknownStatelessHandlersKeepSeparateActorsAndFullSuspensionTest=bootUnknown.then(admit3)
    .then(publish(1,1)).then(all {assert(not(canOwner(2,1))),assert(not(canOwner(2,2))),stay})
    .then(publish(2,3)).then(publish(3,2)).then(flush(1)).then(flush(3)).then(flush(2))
    .then(decode(1)).then(waitFor(1,2)).then(success(3,2)).then(harvestDirect).then(consume(2)).then(endHarvest)
    .then(resume(1,1,1)).then(unit(1)).then(proceed(1)).then(finish(1,1,false)).then(success(2,3))
    .then(continueOutstanding).then(harvestDirect).then(consume(1)).then(consume(3)).then(endHarvest).then(close)
    .then(all {assert(safe),assert(mp::p.groups.length()==3),assert(descriptors==0),stay})
  run publishedStatelessRequestCannotBeReboundOrReturnedTwiceTest=cpu3.then(success(1,1)).then(harvestDirect)
    .then(consume(1)).then(endHarvest).then(all {assert(safe),assert(not(canPublish(1,2))),
      assert(not(canReserve(1))),assert(not(mp::canFinish(1,1,1))),assert(not(mp::canFinish(1,1,0))),
      assert(mp::actorState.owner.get(1)==1),stay})
  // Conditional source scheduling boundary, not an OS trace or a CPU timer.
  // These transitions describe a pending source continuation: runnable is not
  // yet selected to execute its next publication/consumption transaction.
  action sourceBoundary(choosePeer:bool):bool=all {c.opportunity=="selected",not(mp::p.parked),mp::ready==Set(),mp::stay,
    c'={...c,opportunity:if(choosePeer) "runnable" else "selected",opportunityChecks:c.opportunityChecks+1}}
  action sourceReselected=all {c.opportunity=="runnable",mp::stay,c'={...c,opportunity:"selected"}}

  action step={
    nondet a=1.to(4).oneOf()
    nondet m=1.to(6).oneOf()
    any {arrive(m),reserve(m),reserveEager(m),endHarvestEager,withdraw(m),seal,chooseOwner(a),publish(m,a),flush(a),await,
      harvestDirect,harvestWoken,consume(m),endHarvest,continueOutstanding,idleReturn,
      decode(a),unit(a),proceed(a),finish(a,m,false),finish(a,m,true),waitFor(a,m),waitFor(a,0),resume(a,m,1),cancel(m),sourceBoundary(true),sourceBoundary(false),sourceReselected,stay}
  }

  run routeAndNaturalDrainDoNotGuaranteeCpuBenefitTest=assert(
    extra(6,2,2,6,0,2,10,2,1,1,1,1,2)<0 and
    extra(6,2,2,6,0,2,10,20,4,1,4,3,10)>0 and
    extra(6,6,0,0,5,6,10,2,1,1,1,1,2)>0)
  // Retained after the full joint-profile rejection: lower owner CPU did not
  // imply higher throughput. These hypothetical witnesses are NOT fitted native
  // costs and do not attribute the wall gap to the empty probe.
  run cpuSavingCannotApproveLostOrUnqualifiedExecutionOpportunityTest={
    val ref={source:1,construct:1,send:1,execute:1,verify:1,consume:1,schedule:1,observe:0}
    val cand={...ref,source:0}
    val goal={periodNs:20,burst:1,maxDebt:2,latencyNs:20}
    val good={numerator:1,denominator:1,latencyNs:0}
    val bad={numerator:1,denominator:2,latencyNs:40}
    val fair={sourceNs:0,workerNs:0,externalNs:0,readyNs:0,pauseTotalNs:0}
    val incompleteEvidence={cost:"qualified-upper",availability:"unqualified-probe-opportunity",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:true,privateInbox:true,rewoundCursor:true}
    assert(gate::effectiveCycle(cand,good)<gate::effectiveCycle(ref,good) and
      gate::conditionsFit(goal,ref,good,fair) and not(gate::conditionsFit(goal,cand,bad,fair)) and
      not(gate::claimSupported(goal,cand,good,fair,incompleteEvidence)))
  }
}
