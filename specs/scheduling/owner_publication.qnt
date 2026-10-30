// Owner-local publication authority; reuse existing capacity/lifecycle refinement.
// Locks protect Rust metadata only. No Scheme execution occurs under a gate.
module owner_publication {
  import admission_transactions as base from "admission_transactions"
  val owners=Set(1,2)
  val jobs=Set(1,2,3)
  var control: str
  var held: Set[int]
  var open: Set[int]
  var view: int -> int
  action keep = all { control'=control, held'=held, open'=open, view'=view }
  action init = all {
    base::init, control'="idle", held'=Set(), open'=owners, view'=owners.mapBy(_ => 0)
  }
  action idle = all { base::idle, keep }
  def ownerOf(r: int): int = base::ownerOf(r)
  action capture(r: int): bool = all {
    open.contains(ownerOf(r)), not(held.contains(ownerOf(r))),
    base::capture(r), keep
  }
  action acquire(r: int): bool = all { base::acquire(r), keep }
  action publish(r: int): bool = all {
    open.contains(ownerOf(r)), not(held.contains(ownerOf(r))),
    base::captured.get(r)==view.get(ownerOf(r)), base::publish(r), keep
  }
  action cancel(r: int): bool = all { base::cancel(r), keep }
  action complete(r: int): bool = all { base::complete(r), keep }
  action begin(kind: str): bool = all {
    control=="idle", base::phase=="open", Set("reset","close").contains(kind),
    base::idle, control'=kind, held'=Set(), open'=open, view'=view
  }
  // Ascending owner order: no reverse acquisition or owner-to-registry edge.
  action lockNext = all {
    control!="idle", held!=owners,
    base::idle, control'=control, open'=open, view'=view,
    held'=if (held==Set()) Set(1) else owners
  }
  action reset = all {
    control=="reset", held==owners, base::reset,
    control'="idle", held'=Set(), open'=open,
    view'=owners.mapBy(_ => base::epoch+1)
  }
  action close = all {
    control=="close", held==owners, base::close,
    control'="idle", held'=Set(), open'=Set(), view'=view
  }
  action abort = all {
    control!="idle", base::idle, control'="idle", held'=Set(), open'=open, view'=view
  }
  val safety = and {
    base::safety,
    open.subseteq(owners), held.subseteq(owners),
    control!="idle" or held==Set(),
    base::phase=="open" or open==Set(),
    open.exclude(held).forall(o => view.get(o)==base::epoch)
  }
  // Negative control: replacing capacity after locking only the first owner
  // exposes a new registry generation beside an old, still-publishable port.
  action unsafePartialReset = all {
    control=="reset", held==Set(1), base::reset,
    control'="idle", held'=Set(), open'=open,
    view'=view.set(1,base::epoch+1)
  }
  action unsafePartialClose = all {
    control=="close", held==Set(1), base::close,
    control'="idle", held'=Set(), open'=open.exclude(Set(1)), view'=view
  }
  action step = {
    nondet r=jobs.oneOf()
    nondet kind=Set("reset","close").oneOf()
    any { capture(r), acquire(r), publish(r), cancel(r), complete(r),
      begin(kind), lockNext, reset, close, abort, idle }
  }
  run independentOwnersTest = init.then(capture(1)).then(capture(2))
    .then(acquire(1)).then(acquire(2)).then(publish(1)).then(publish(2))
    .then(complete(1))
    .then(all { assert(base::capacityHeld==Set(2)), assert(safety), idle })
    .then(complete(2)).then(all { assert(safety), idle })
  run oldSnapshotResetTest = init.then(capture(2)).then(begin("reset"))
    .then(lockNext).then(lockNext).then(reset).then(acquire(2)).then(cancel(2))
    .then(all { assert(base::accepted==Set()), assert(base::staleHeld==Set()), assert(safety), idle })
  run closeWithReservedWorkTest = init.then(capture(1)).then(acquire(1))
    .then(begin("close")).then(lockNext).then(lockNext).then(close).then(cancel(1))
    .then(all { assert(open==Set()), assert(base::capacityHeld==Set()), assert(safety), idle })
  run activeReservationResetAbortTest = init.then(capture(1)).then(acquire(1))
    .then(begin("reset")).then(lockNext).then(lockNext)
    .then(all { assert(not(base::canReset)), assert(safety), idle }).then(abort)
    .then(publish(1)).then(complete(1)).then(all { assert(safety), idle })
  run unheldOwnerCanPublishDuringControlCollectionTest = init.then(capture(2)).then(acquire(2))
    .then(begin("close")).then(lockNext).then(publish(2)).then(lockNext).then(close)
    .then(complete(2)).then(all { assert(safety), idle })
  run partialResetMutationTest = init.then(begin("reset")).then(lockNext).then(unsafePartialReset)
    .then(all { assert(not(safety)), idle })
  run partialCloseMutationTest = init.then(begin("close")).then(lockNext).then(unsafePartialClose)
    .then(all { assert(not(safety)), idle })
}
