// Physical request packets refine canonical logical work, not a new scheduler.
module mailbox_packet_protocol {
  const ACTORS:Set[int]
  const REQUESTS:Set[int]
  const LIMIT:int
  import actor_mailbox_work(ACTORS=ACTORS,REQUESTS=REQUESTS,LIMIT=LIMIT) as w from "./actor_mailbox_work"
  type PacketState={staged:List[int],groups:List[List[int]],decoded:Set[int],replies:int,
    waits:int,resumes:int,awaiting:int->int,parked:bool,closed:bool}
  var p:PacketState
  pure def members(xs:List[int]):Set[int]=xs.foldl(Set(),(s,m)=>s.union(Set(m)))
  pure def keep(xs:List[int],ids:Set[int]):List[int]=xs.foldl(List(),(res,m)=>if(ids.contains(m)) res.append(m) else res)
  val staged=members(p.staged)
  val sent=p.groups.foldl(Set(),(s,g)=>s.union(members(g)))
  val encoded=p.groups.foldl(0,(n,g)=>n+g.length())
  def sameOwner(xs:List[int]):bool=if(xs.length()==0) false else
    members(xs).forall(m=>w::state.owner.get(m)==w::state.owner.get(xs.head()))
  val safe=w::cycleSafe and p.staged.length()==staged.size() and
    staged.intersect(sent)==Set() and staged.union(sent)==w::state.published and
    encoded==sent.size() and p.groups.foldl(true,(ok,g)=>ok and g.length()>0 and sameOwner(g)) and
    p.decoded.subseteq(sent) and w::state.completed.subseteq(p.decoded) and
    p.replies==w::state.completed.size() and p.waits-p.resumes==w::state.waiting.size() and
    (p.parked implies p.staged==List()) and (p.closed implies w::state.phase=="closed")
  action initPackets=all {w::beginCycle,p'={staged:List(),groups:List(),decoded:Set(),replies:0,
    waits:0,resumes:0,awaiting:ACTORS.mapBy(_=>0),parked:false,closed:false}}
  action create(a:int):bool=all {w::createResident(a),p'=p}
  action credits(n:int):bool=all {w::configureCredits(n),p'=p}
  action open=all {w::openCycle,p'=p}
  val actorState=w::state
  val outstanding=w::nativeOutstanding
  val ready=w::completedReady
  val canonicalSafe=w::cycleSafe
  val workLedger=w::work
  val lifecycleMemory=w::liveMemory
  val runningActors=w::runningActors
  val runnableActors=w::runnableActors
  val readyToRunActors=w::readyToRunActors
  action stay=all {w::unchanged,p'=p}
  action offer(m:int):bool=all {not(p.parked),w::offer(m),p'=p}
  action stage(m:int,a:int):bool=all {not(p.parked),w::supply(m,a),p'={...p,staged:p.staged.append(m)}}
  def canSend(xs:List[int],epoch:int):bool=not(p.closed) and not(p.parked) and
    Set("run","draining").contains(w::state.phase) and epoch==w::state.epoch and
    xs.length()>0 and xs.length()==members(xs).size() and members(xs).subseteq(staged) and
    sameOwner(xs) and xs==keep(p.staged,members(xs))
  action send(xs:List[int],epoch:int):bool=all {canSend(xs,epoch),w::unchanged,
    p'={...p,staged:keep(p.staged,staged.exclude(members(xs))),groups:p.groups.append(xs)}}
  def canDecode(a:int):bool=if(w::state.queue.get(a).length()==0) false else
    w::state.work.get(a)==0 and not(w::state.waiting.contains(a)) and
    sent.contains(w::state.queue.get(a).head()) and not(p.decoded.contains(w::state.queue.get(a).head()))
  action decode(a:int):bool=all {canDecode(a),w::enter(a),
    p'={...p,decoded:p.decoded.union(Set(w::state.queue.get(a).head()))}}
  action unit(a:int):bool=all {w::executeUnit(a),p'=p}
  action proceed(a:int):bool=all {w::continueUnit(a),p'=p}
  def canFinish(a:int,m:int,epoch:int):bool=epoch==w::state.epoch and
    w::state.work.get(a)==m and p.decoded.contains(m) and not(w::state.completed.contains(m))
  action finish(a:int,m:int,epoch:int,failed:bool):bool=all {canFinish(a,m,epoch),w::returnResult(a,failed),p'={...p,replies:p.replies+1}}
  action success(a:int,m:int):bool=decode(a).then(unit(a)).then(proceed(a)).then(finish(a,m,1,false))
  action consume(m:int):bool=all {not(p.parked),w::takeVerified(m),p'=p}
  action waitFor(a:int,m:int):bool=all {w::waitGreen(a),p'={...p,waits:p.waits+1,awaiting:p.awaiting.set(a,m)}}
  def canResume(a:int):bool=w::state.waiting.contains(a) and
    (p.awaiting.get(a)==0 or w::state.completed.contains(p.awaiting.get(a)))
  def canWakeActor(a:int,m:int,epoch:int):bool=canResume(a) and w::canWakeGreen(a,epoch,m)
  // resumes counts leaving registered wait, not acquiring a processor.
  action wakeActor(a:int,m:int,epoch:int):bool=all {canWakeActor(a,m,epoch),w::wakeGreen(a,epoch,m),
    p'={...p,resumes:p.resumes+1,awaiting:p.awaiting.set(a,0)}}
  action resume(a:int,m:int,epoch:int):bool=all {canResume(a),w::resumeGreen(a,epoch,m),
    p'={...p,resumes:p.resumes+1,awaiting:p.awaiting.set(a,0)}}
  action cancel(m:int):bool=all {w::cancelCaller(m),p'=p}
  action park=all {p.staged==List(),w::unchanged,p'={...p,parked:true}}
  action wake=all {p.parked,w::completedReady.size()>0,w::unchanged,p'={...p,parked:false}}
  action unsafeParkWithHeldInput=all {p.staged.length()>0,w::unchanged,p'={...p,parked:true}}
  val dispatchable=ACTORS.filter(a=>canDecode(a))
  val resumable=ACTORS.filter(a=>canResume(a))
  // Checker steps remain single guarded transitions; compound close is only a
  // deterministic scenario helper, not a branch that may be disabled in `then`.
  action startDrain=all {p.staged==List(),not(p.parked),w::beginDrain,p'=p}
  action finishDrain=all {w::endDrain,p'=p}
  action startClose=all {w::beginClose,p'=p}
  action stop(a:int):bool=all {w::requestStop(a),p'=p}
  action stopped(a:int):bool=all {w::confirmStop(a),p'=p}
  action join(a:int):bool=all {w::joinResident(a),p'=p}
  action finishClose=all {w::finishClose,p'={...p,closed:true}}

  action step={
    nondet a=ACTORS.oneOf()
    nondet m=REQUESTS.oneOf()
    any {offer(m),stage(m,a),send(List(m),1),send(List(1,2),1),send(List(2,3),1),send(List(1,2,3),1),
      decode(a),unit(a),proceed(a),finish(a,m,1,false),finish(a,m,1,true),consume(m),
      waitFor(a,0),waitFor(a,m),wakeActor(a,m,1),resume(a,m,1),cancel(m),park,wake,
      startDrain,finishDrain,startClose,stop(a),stopped(a),join(a),finishClose,stay}
  }

  // u = one mailbox send+receive pair; work = unchanged complete business cycle.
  // pack/decode/resume weights are hypothetical inputs, never fitted native ns.
  pure def original(n:int,u:int,work:int):int=n*(2*u+work)
  pure def packed(n:int,k:int,u:int,pack:int,decode:int,work:int,extraResume:int):int=
    (k+n)*u+k*pack+n*decode+n*work+extraResume
  pure def destinations(first:int,n:int):Set[int]=0.to(n-1).map(i=>(first+i)%4)
}
module mailbox_packets {
  import mailbox_packet_protocol(ACTORS=Set(1,2),REQUESTS=Set(1,2,3),LIMIT=16).*
  action boot=initPackets.then(create(1)).then(create(2)).then(credits(3)).then(open)
  action close=startDrain.then(finishDrain).then(startClose)
    .then(stop(1)).then(stopped(1)).then(join(1)).then(stop(2)).then(stopped(2)).then(join(2)).then(finishClose)
  action pair=boot.then(offer(1)).then(stage(1,1)).then(offer(2)).then(stage(2,1)).then(send(List(1,2),1))
  run onePacketStillPublishesAndRetiresResultsIndividuallyTest=pair.then(success(1,1)).then(consume(1))
    .then(all {assert(safe),assert(p.groups.length()==1),assert(p.replies==1),
      assert(actorState.consumed==Set(1)),assert(outstanding==Set(2)),stay})
    .then(success(1,2)).then(consume(2)).then(close).then(all {
      assert(safe),assert(p.replies==2),assert(workLedger.transfers==4),assert(lifecycleMemory==0),stay})
  run suspendedPacketDoesNotHoldOtherActorsOrReturnItsCreditsTest=pair.then(decode(1)).then(waitFor(1,3))
    .then(offer(3)).then(stage(3,2)).then(send(List(3),1)).then(success(2,3)).then(consume(3)).then(cancel(1))
    .then(all {assert(safe),assert(outstanding==Set(1,2)),assert(not(canDecode(1))),stay})
    .then(resume(1,1,1)).then(unit(1)).then(proceed(1)).then(finish(1,1,1,false)).then(consume(1))
    .then(success(1,2)).then(consume(2)).then(close).then(all {
      assert(safe),assert(p.waits==1),assert(p.resumes==1),stay})
  run heldInputBeforeSourceParkCanCreateInternalWaitCycleTest=boot.then(offer(1)).then(stage(1,1))
    .then(send(List(1),1)).then(decode(1)).then(waitFor(1,2)).then(offer(2)).then(stage(2,2))
    .then(unsafeParkWithHeldInput).then(all {
      assert(canonicalSafe),assert(not(safe)),assert(dispatchable==Set()),assert(resumable==Set()),
      assert(ready==Set()),assert(outstanding==Set(1,2)),stay})
  run flushBeforeBlockingBreaksTheWaitDependencyTest=boot.then(offer(1)).then(stage(1,1)).then(send(List(1),1))
    .then(decode(1)).then(waitFor(1,2)).then(offer(2)).then(stage(2,2)).then(send(List(2),1))
    .then(success(2,2)).then(consume(2)).then(resume(1,1,1)).then(unit(1)).then(proceed(1))
    .then(finish(1,1,1,false)).then(consume(1)).then(close).then(all {assert(safe),stay})
  run failureDoesNotReleaseUnreadSiblingTest=pair.then(decode(1)).then(finish(1,1,1,true)).then(consume(1))
    .then(all {assert(safe),assert(actorState.failed==Set(1)),assert(outstanding==Set(2)),stay})
    .then(success(1,2)).then(consume(2)).then(close).then(all {assert(safe),stay})
  run duplicateOldEpochAndReenteredCompletionAreRejectedTest=pair.then(success(1,1)).then(consume(1))
    .then(all {assert(not(canSend(List(1,2),1))),assert(not(canSend(List(1,2),0))),
      assert(not(canFinish(1,1,1))),stay})
    .then(success(1,2)).then(consume(2)).then(close).then(all {assert(safe),assert(not(canSend(List(1),1))),stay})
  // Resident count, processor ownership and unread-result capacity are
  // independent. This is a finite protocol witness, not a thread-count tuning.
  action twoBusy=boot.then(offer(1)).then(stage(1,1)).then(send(List(1),1))
    .then(decode(1)).then(waitFor(1,0)).then(offer(2)).then(stage(2,2))
    .then(send(List(2),1)).then(decode(2))
  run wakeDoesNotPreemptPeerOrReturnUnreadCapacityTest=twoBusy
    .then(wakeActor(1,1,1)).then(all {
      assert(safe),assert(actorState.resident==Set(1,2)),assert(actorState.tokens==3),
      assert(runningActors==Set(2)),assert(readyToRunActors==Set(1)),
      assert(outstanding==Set(1,2)),assert(actorState.consumed==Set()),stay})
    .then(unit(2)).then(proceed(2)).then(finish(2,2,1,false)).then(all {
      assert(outstanding==Set(1,2)),assert(ready==Set(2)),assert(readyToRunActors==Set(1)),stay})
    .then(consume(2)).then(all {assert(outstanding==Set(1)),assert(safe),stay})
    .then(proceed(1)).then(unit(1)).then(proceed(1)).then(finish(1,1,1,false))
    .then(consume(1)).then(close).then(all {assert(safe),assert(lifecycleMemory==0),
      assert(not(canWakeActor(1,1,1))),stay})
  run wakeRetainsEpochRequestAndSingleDeliveryGuardsTest=twoBusy.then(cancel(1)).then(all {
      assert(not(canWakeActor(1,1,0))),assert(not(canWakeActor(1,2,1))),stay})
    .then(wakeActor(1,1,1)).then(all {
      assert(not(canWakeActor(1,1,1))),assert(p.waits==p.resumes),
      assert(runningActors==Set(2)),assert(outstanding==Set(1,2)),
      assert(actorState.timedOut==Set(1)),assert(safe),stay})
  run originalThreeSlotFourDestinationSupplyCannotCombineRequestsTest=assert(
    0.to(3).forall(first=>1.to(3).forall(n=>destinations(first,n).size()==n)))
  run unchangedDestinationsMakePacketOverheadStrictlyAdditionalTest=assert(
    1.to(3).forall(n=>packed(n,n,10,1,1,20,0)>original(n,10,20)))
  run groupingWinIsConditionalOnAllNewCostsIncludingResumeTest=assert(
    packed(3,1,10,1,1,20,0)<original(3,10,20) and
    packed(3,1,10,1,1,20,30)>original(3,10,20))
}
