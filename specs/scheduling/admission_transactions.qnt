// Refine existing owner capacity/publication semantics with captured identity.
// Epochs stand for Arc identity, not a second runtime generation counter.
module admission_transactions {
  val jobs = Set(1,2,3)
  import owner_lifecycle(JOBS=Set(1,2,3), ASSIGNED=Set(1,2,3).mapBy(r => if (r==2) 2 else 1))
    as host from "../owner_lifecycle"
  var epoch: int
  var captured: int -> int
  var committed: int -> int
  var staleHeld: Set[int]
  action keep = all {
    epoch'=epoch, captured'=captured, committed'=committed, staleHeld'=staleHeld
  }
  action init = all {
    host::init, epoch'=0, captured'=jobs.mapBy(_ => -1),
    committed'=jobs.mapBy(_ => -1), staleHeld'=Set()
  }
  action idle = all { host::stutter, keep }
  action capture(r: int): bool = all {
    host::s.phase=="open", captured.get(r)==-1,
    host::stutter, epoch'=epoch, captured'=captured.set(r,epoch),
    committed'=committed, staleHeld'=staleHeld
  }
  val canReset = host::s.phase=="open" and host::s.capacityHeld==Set()
    and host::s.workHeld==Set() and host::s.producers==Set()
  action reset = all {
    epoch<3, host::configureCapacity(3), epoch'=epoch+1,
    captured'=captured, committed'=committed, staleHeld'=staleHeld
  }
  // An old snapshot may acquire before old semaphores are closed outside the
  // table lock. Such a permit cannot charge or enter the replacement capacity.
  action acquire(r: int): bool = all {
    captured.get(r)>=0, not(host::s.accepted.contains(r)),
    not(staleHeld.contains(r)), not(host::s.reserved.contains(r)),
    if (captured.get(r)==epoch)
      all { host::reserve(r), keep }
    else all { host::stutter, epoch'=epoch, captured'=captured,
      committed'=committed, staleHeld'=staleHeld.union(Set(r)) }
  }
  val canPublish = host::s.phase=="open"
  action publish(r: int): bool = all {
    captured.get(r)==epoch, host::admitReserved(r), epoch'=epoch,
    captured'=captured, committed'=committed.set(r,captured.get(r)), staleHeld'=staleHeld
  }
  action cancel(r: int): bool = all {
    host::s.reserved.contains(r) or staleHeld.contains(r),
    if (host::s.reserved.contains(r)) host::releaseReservation(r) else host::stutter,
    epoch'=epoch, captured'=captured, committed'=committed, staleHeld'=staleHeld.exclude(Set(r))
  }
  action close = all { host::close, keep }
  action complete(r: int): bool = all { host::complete(r), keep }
  action timeout(r: int): bool = all { host::observeTimeout(r), keep }
  // Read-only projections for owner-local refinements; no copied lifecycle state.
  val phase = host::s.phase
  val capacityHeld = host::s.capacityHeld
  val accepted = host::s.accepted
  def ownerOf(r: int): int = host::ownerOf(r)
  val live = host::s.accepted.exclude(host::s.completed)
  val safety = and {
    host::safety,
    live.forall(r => committed.get(r)==epoch),
    staleHeld.intersect(host::s.capacityHeld)==Set()
  }
  // Negative control: charging replacement credits cannot legitimize stale input.
  action unsafeStalePublish(r: int): bool = all {
    staleHeld.contains(r), host::admit(r), epoch'=epoch,
    captured'=captured, committed'=committed.set(r,captured.get(r)),
    staleHeld'=staleHeld.exclude(Set(r))
  }
  action step = {
    nondet r=jobs.oneOf()
    any { capture(r), reset, acquire(r), publish(r), cancel(r), close,
      complete(r), timeout(r), idle }
  }
  run capturedPermitPublicationTest = init.then(capture(1)).then(acquire(1)).then(publish(1))
    .then(all { assert(live==Set(1)), assert(safety), idle })
    .then(complete(1)).then(reset)
    .then(all { assert(host::s.capacityHeld==Set()), assert(safety), idle })
  run resetBeforeAcquireTest = init.then(capture(1)).then(reset).then(acquire(1))
    .then(all { assert(staleHeld==Set(1)), assert(host::s.capacityHeld==Set()), assert(safety), idle })
    .then(cancel(1)).then(all { assert(staleHeld==Set()), assert(safety), idle })
  run reservedCloseCancelTest = init.then(capture(1)).then(acquire(1)).then(close)
    .then(all { assert(not(canPublish)), assert(host::s.capacityHeld==Set(1)), assert(safety), idle })
    .then(cancel(1)).then(all { assert(host::s.capacityHeld==Set()), assert(safety), idle })
  run acceptedTimeoutDrainsTest = init.then(capture(1)).then(acquire(1)).then(publish(1))
    .then(timeout(1)).then(close)
    .then(all { assert(host::s.capacityHeld==Set(1)), assert(safety), idle })
    .then(complete(1)).then(all { assert(host::s.capacityHeld==Set()), assert(safety), idle })
  run partialWorkBlocksResetTest = init
    .then(all { host::reserveWork(1), keep })
    .then(all { assert(host::s.workHeld==Set(1)), assert(not(canReset)), assert(safety), idle })
    .then(all { host::cancelWork(1), keep }).then(reset)
    .then(all { assert(epoch==1), assert(safety), idle })
  run staleGenerationMutationTest = init.then(capture(1)).then(reset).then(acquire(1))
    .then(unsafeStalePublish(1)).then(all { assert(not(safety)), idle })
}
