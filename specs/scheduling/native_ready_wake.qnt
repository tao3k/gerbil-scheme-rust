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
  // Slot order can differ from completion publication order.
  // In this trace, the original reply mailbox starts with the occupant of slot 1.
  run rotatingReadyScanCanOvertakeFirstPublishedReplyTest=init
    .then(offer(2)).then(feed(2)).then(offer(3)).then(feed(3))
    .then(enter(p::frames.get(1))).then(admit(p::frames.get(1))).then(submit(p::frames.get(1))).then(start(p::frames.get(1))).then(finish(p::frames.get(1)))
    .then(project(p::frames.get(1))).then(verify(p::frames.get(1))).then(retire(p::frames.get(1))).then(release(p::frames.get(1))).then(publish(p::frames.get(1))).then(signal(p::frames.get(1)))
    .then(enter(p::frames.get(0))).then(admit(p::frames.get(0))).then(submit(p::frames.get(0))).then(start(p::frames.get(0))).then(finish(p::frames.get(0)))
    .then(project(p::frames.get(0))).then(verify(p::frames.get(0))).then(retire(p::frames.get(0))).then(release(p::frames.get(0))).then(publish(p::frames.get(0))).then(signal(p::frames.get(0)))
    .then(all {assert(safety),assert(issued==1),assert(selected==p::frames.get(0)),idle})

  // A lone completion arriving after park has to notify once. Taking the
  // frame and returning its credit cannot substitute for waking the source.
  run singletonAfterParkStillNeedsOneNotificationTest=init.then(offer(2)).then(feed(2))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(park)
    .then(publish(2)).then(all {assert(receiverParked),assert(not(queued)),assert(safety),idle})
    .then(signal(2)).then(takeWake).then(consume(2))
    .then(all {assert(issued==1),assert(taken==1),assert(safety),idle})

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

// Refine the same frame protocol with publication order and an atomic wait
// registration. The queue holds only frame identities, never extra payloads.
module native_ready_fifo {
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"

  type Edge = { fifo:List[int], wait:str, issued:int, taken:int }
  var edge:Edge
  val safe = and {
    p::safety,
    edge.fifo.length() <= 2,
    edge.fifo.indices().map(i => edge.fifo[i]) == p::ready,
    edge.fifo.length() == p::ready.size(),
    edge.issued - edge.taken == (if (edge.wait == "claimed" or edge.wait == "queued") 1 else 0),
    implies(edge.wait == "armed", edge.fifo == List())
  }

  action keep = edge' = edge
  action init = all { p::init, edge' = {fifo:List(),wait:"running",issued:0,taken:0} }
  action check = all { p::idle, keep }
  action prepare(id:int):bool = all {p::offer(id),keep}
    .then(all {p::feed(id),keep}).then(all {p::enter(id),keep})
    .then(all {p::admit(id),keep}).then(all {p::submit(id),keep})
    .then(all {p::start(id),keep}).then(all {p::finish(id),keep})
    .then(all {p::project(id),keep}).then(all {p::verify(id),keep})
    .then(all {p::retire(id),keep}).then(all {p::release(id),keep})

  action arm = all {
    edge.wait == "running", edge.fifo == List(), p::idle,
    edge' = {...edge, wait:"armed"}
  }
  action publish(id:int):bool = all {
    p::publish(id),
    edge' = if (edge.wait == "armed")
      {...edge, fifo:edge.fifo.append(id), wait:"claimed", issued:edge.issued+1}
      else {...edge, fifo:edge.fifo.append(id)}
  }
  action send = all {
    edge.wait == "claimed", p::idle, edge' = {...edge, wait:"queued"}
  }
  // Source may win the CPU after arming but before sleeping. A claimed wake
  // cannot be revoked by the source; consume it before arming another wait.
  action disarm = all {
    edge.wait == "armed", p::idle, edge' = {...edge, wait:"running"}
  }
  action acknowledge = all {
    edge.wait == "queued", p::idle,
    edge' = {...edge, wait:"running", taken:edge.taken+1}
  }
  action consume(id:int):bool = all {
    edge.wait != "armed", edge.fifo != List(), edge.fifo[0] == id, p::consume(id),
    edge' = {...edge, fifo:edge.fifo.slice(1,edge.fifo.length())}
  }
  // Once wait is committed, the source receives before taking a frame.
  // The wake acknowledgement and head removal share one transaction.
  action takeWoken(id:int):bool = all {
    edge.wait == "queued", edge.fifo != List(), edge.fifo.head() == id, p::consume(id),
    edge' = {...edge, fifo:edge.fifo.tail(), wait:"running", taken:edge.taken+1}
  }
  action close = all {
    edge.wait == "running", edge.fifo == List(), p::close, keep
  }
  action unsafeArmWithoutRecheck = all {
    edge.wait == "running", p::idle, edge' = {...edge, wait:"armed"}
  }

  run publicationBeforeArmPreservesOpportunityTest = init.then(prepare(2)).then(publish(2))
    .then(all {assert(safe),assert(edge.issued==0),check})
    .then(consume(2)).then(close)
  run armBeforePublicationWakesExactlyOnceTest = init.then(prepare(2)).then(prepare(3))
    .then(arm).then(publish(3)).then(publish(2))
    .then(all {assert(safe),assert(edge.wait=="claimed"),assert(edge.issued==1),check})
    .then(send).then(acknowledge).then(consume(3)).then(consume(2)).then(close)
  run committedReceiveFusesAcknowledgementAndHeadRemovalTest = init
    .then(prepare(2)).then(prepare(3)).then(arm).then(publish(3)).then(publish(2))
    .then(all {assert(safe),assert(edge.wait=="claimed"),assert(p::consumed==Set()),check})
    .then(send).then(takeWoken(3))
    .then(all {assert(safe),assert(edge.fifo==List(2)),assert(edge.issued==edge.taken),
      assert(p::consumed==Set(3)),check})
    .then(prepare(4)).then(publish(4))
    .then(all {assert(safe),assert(edge.fifo==List(2,4)),assert(edge.issued==1),check})
    .then(consume(2)).then(consume(4)).then(close)
  run consumedResultLeavesOldWakeForDrainTest = init.then(prepare(2)).then(arm).then(publish(2))
    .then(consume(2)).then(send).then(acknowledge)
    .then(all {assert(safe),assert(edge.fifo==List()),assert(p::consumed==Set(2)),check})
    .then(close)
  run fifoReturnsOneFrameAndRefillsWithoutWaitingForPeerTest = init.then(prepare(2)).then(prepare(3))
    .then(publish(3)).then(publish(2)).then(consume(3)).then(prepare(4)).then(publish(4))
    .then(all {assert(safe),assert(edge.fifo==List(2,4)),assert(edge.issued==0),check})
    .then(consume(2)).then(consume(4)).then(close)
  // Reusing the Mailbox's wait cannot make an out-of-band frame visible
  // to its inbox check. A separate publication needs a queue entry or its
  // own synchronized wait predicate.
  action unsafePublishOutsideWaitQueue(id:int):bool = all {p::publish(id),keep}
  run outOfBandPublicationCannotReuseMailboxWaitTest = init.then(prepare(2)).then(arm)
    .then(unsafePublishOutsideWaitQueue(2))
    .then(all {assert(edge.fifo==List()),assert(edge.wait=="armed"),
      assert(p::ready==Set(2)),assert(not(safe)),check})
  run staleEmptyCheckCanParkOverPublishedResultTest = init.then(prepare(2)).then(publish(2))
    .then(unsafeArmWithoutRecheck).then(all {assert(not(safe)),check})
  run disarmBeforePublishNeedsNoWakeTest = init.then(prepare(2)).then(arm).then(disarm).then(publish(2))
    .then(all {assert(safe),assert(edge.issued==0),check}).then(consume(2)).then(close)
  action step = {
    nondet id = Set(2,3,4).oneOf()
    any {
      all {p::offer(id),keep}, all {p::feed(id),keep}, all {p::enter(id),keep},
      all {p::admit(id),keep}, all {p::submit(id),keep}, all {p::start(id),keep},
      all {p::finish(id),keep}, all {p::project(id),keep}, all {p::verify(id),keep},
      all {p::retire(id),keep}, all {p::release(id),keep},
      publish(id), arm, disarm, send, acknowledge, consume(id), close, check
    }
  }

}
