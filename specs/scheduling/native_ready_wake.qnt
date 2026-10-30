// Gerbil Actor completion edges reuse the existing fixed frame/root protocol.
// Only one immutable wake epoch can be queued. Ready results remain in frames.
module native_ready_wake {
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"
  var scanNext:int
  var issued:int
  var taken:int
  var receiverParked:bool
  // Ghost publication obligations: a worker may be switched out between marking
  // its frame ready and publishing a wake. This is not an implementation queue.
  var publishing:Set[int]
  val ready=p::ready
  // Includes the short CAS-claimed/before-send window; a claim is not execution.
  val queued=issued>taken
  val selected=if (ready.contains(p::frames.get(scanNext))) p::frames.get(scanNext)
    else if (ready.contains(p::frames.get(1-scanNext))) p::frames.get(1-scanNext) else -1
  action keep=all {scanNext'=scanNext,issued'=issued,taken'=taken,receiverParked'=receiverParked,publishing'=publishing}
  action init=all {p::init,scanNext'=0,issued'=0,taken'=0,receiverParked'=false,publishing'=Set()}
  action idle=all {p::idle,keep}
  action offer(id:int):bool=all {p::offer(id),keep}
  action feed(id:int):bool=all {p::feed(id),keep}
  action enter(id:int):bool=all {p::enter(id),keep}
  action admit(id:int):bool=all {p::admit(id),keep}
  action submit(id:int):bool=all {p::submit(id),keep}
  action start(id:int):bool=all {p::start(id),keep}
  action finish(id:int):bool=all {p::finish(id),keep}
  action project(id:int):bool=all {p::project(id),keep}
  action verify(id:int):bool=all {p::verify(id),keep}
  action retire(id:int):bool=all {p::retire(id),keep}
  action release(id:int):bool=all {p::release(id),keep}
  action publish(id:int):bool=all {p::publish(id),scanNext'=scanNext,issued'=issued,taken'=taken,
    receiverParked'=receiverParked,publishing'=publishing.union(Set(id))}
  action signal(id:int):bool=all {publishing.contains(id),p::idle,scanNext'=scanNext,
    issued'=if (queued) issued else issued+1,taken'=taken,receiverParked'=receiverParked,
    publishing'=publishing.exclude(Set(id))}
  action takeWake=all {queued,p::idle,scanNext'=scanNext,issued'=issued,taken'=taken+1,
    receiverParked'=false,publishing'=publishing}
  action consume(id:int):bool=all {not(receiverParked),id==selected,ready.contains(id),p::consume(id),
    scanNext'=if (p::frames.get(0)==id) 1 else 0,issued'=issued,taken'=taken,
    receiverParked'=receiverParked,publishing'=publishing}
  action park=all {ready==Set(),not(queued),p::idle,scanNext'=scanNext,issued'=issued,taken'=taken,
    receiverParked'=true,publishing'=publishing}
  action close=all {p::close,keep}
  val safety=and {p::safety,0<=taken,taken<=issued,issued-taken<=1,
    (receiverParked and ready!=Set()) implies (queued or publishing!=Set())}
  action unsafeAckAfterScan=all {p::idle,scanNext'=scanNext,issued'=issued,taken'=issued,
    receiverParked'=true,publishing'=publishing}
  action unsafeSplitClaim(id:int):bool=all {publishing.contains(id),p::idle,
    scanNext'=scanNext,issued'=issued+1,taken'=taken,receiverParked'=false,
    publishing'=publishing.exclude(Set(id))}
  run publicationBetweenEmptyScanAndWaitTest=init.then(offer(2)).then(feed(2))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(park)
    .then(publish(2)).then(all {assert(safety),assert(receiverParked),idle})
    .then(signal(2)).then(takeWake).then(consume(2)).then(all {assert(safety),idle})
  run staleWakeCannotConsumeOrReleaseAFrameTest=init.then(offer(2)).then(feed(2))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2))
    .then(publish(2)).then(signal(2)).then(consume(2)).then(offer(3)).then(feed(3))
    .then(takeWake).then(all {assert(p::live==Set(3)),assert(ready==Set()),
      assert(p::consumed==Set(2)),assert(safety),idle})
  run delayedSignalAfterFastConsumptionTest=init.then(offer(2)).then(feed(2))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2))
    .then(publish(2)).then(consume(2)).then(park).then(signal(2)).then(takeWake)
    .then(all {assert(ready==Set()),assert(p::live==Set()),assert(safety),idle})
  run ackAfterScanLosesReadyEdgeMutationTest=init.then(offer(2)).then(feed(2))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2))
    .then(publish(2)).then(signal(2)).then(unsafeAckAfterScan)
    .then(all {assert(not(safety)),idle})
  run oneWakeCoalescesPeersAndRefillCannotOvertakeTest=init
    .then(offer(2)).then(feed(2)).then(offer(3)).then(feed(3))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(publish(2)).then(signal(2))
    .then(enter(3)).then(admit(3)).then(submit(3)).then(start(3)).then(finish(3))
    .then(project(3)).then(verify(3)).then(retire(3)).then(release(3)).then(publish(3)).then(signal(3))
    .then(all {assert(issued==1),assert(ready==Set(2,3)),assert(safety),idle})
    .then(consume(selected)).then(offer(4)).then(feed(4))
    .then(enter(4)).then(admit(4)).then(submit(4)).then(start(4)).then(finish(4))
    .then(project(4)).then(verify(4)).then(retire(4)).then(release(4)).then(publish(4)).then(signal(4))
    .then(all {assert(selected!=4),assert(issued==1),assert(safety),idle})
    .then(consume(selected)).then(consume(4)).then(takeWake).then(close)
    .then(all {assert(p::live==Set()),assert(p::consumed==Set(2,3,4)),assert(safety),idle})
  run nonAtomicNotificationClaimMutationTest=init
    .then(offer(2)).then(feed(2)).then(offer(3)).then(feed(3))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(publish(2))
    .then(enter(3)).then(admit(3)).then(submit(3)).then(start(3)).then(finish(3))
    .then(project(3)).then(verify(3)).then(retire(3)).then(release(3)).then(publish(3))
    .then(signal(2)).then(unsafeSplitClaim(3))
    .then(all {assert(issued-taken==2),assert(not(safety)),idle})
  action step={nondet id=Set(2,3,4).oneOf()
    any {offer(id),feed(id),enter(id),admit(id),submit(id),start(id),finish(id),
      project(id),verify(id),retire(id),release(id),publish(id),signal(id),takeWake,
      consume(id),park,close,idle}}
}
