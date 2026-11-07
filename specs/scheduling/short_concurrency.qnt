// Route-class reservation through projection; native retirement and client
// observation remain separate. Actor CPU is one existing Gerbil executor.
module bounded_short {
  const SHORT_LIMIT: int
  import owner_lifecycle(JOBS=Set(1,2,3,4), ASSIGNED=Set(1,2,3,4).mapBy(_ => 1))
    as host from "../owner_lifecycle"
  import actor_batch(ACTORS=Set(2,3,4), PROCESSORS=Set(1)) as actors from "../actor_batch"
  val shorts=Set(2,3,4)
  val nativeHeld=host::s.capacityHeld
  val readyIds=host::s.serviceReady
  var held: Set[int]
  var projected: Set[int]
  var published: Set[int]
  var observed: Set[int]
  var abandoned: Set[int]
  action keep = all { held'=held, projected'=projected, published'=published, observed'=observed, abandoned'=abandoned }
  action idle = all { host::stutter, actors::stutter, keep }
  action init = all { host::init, actors::init,
    held'=Set(), projected'=Set(), published'=Set(), observed'=Set(), abandoned'=Set() }
  def canClass(r: int): bool = and {
    host::s.phase=="open", shorts.contains(r), not(held.contains(r)),
    not(projected.contains(r)), held.size()<SHORT_LIMIT
  }
  action reserveClass(r: int): bool = all {
    canClass(r), host::stutter, actors::stutter,
    held'=held.union(Set(r)), projected'=projected, published'=published, observed'=observed, abandoned'=abandoned
  }
  action reserveNative(r: int): bool = all { held.contains(r), host::reserve(r), actors::stutter, keep }
  action admit(r: int): bool = all { held.contains(r), host::admitReserved(r), actors::stutter, keep }
  action submit(r: int): bool = all { host::submitService(r), actors::stutter, keep }
  action start(r: int): bool = all {
    host::s.servicePending.contains(r), host::stutter, actors::start(r,1), keep
  }
  action yieldActor(r: int): bool = all { host::stutter, actors::yieldActor(r), keep }
  action finish(r: int, failed: bool): bool = all {
    actors::running.contains(r), host::readyService(r,failed), actors::finish(r,failed), keep
  }
  // Actual RoutedRequest is consumed by project; its class guard drops here.
  // Do not wait for the Tokio client to poll, or release native roots/slots early.
  action project(r: int): bool = all {
    host::s.serviceReady.contains(r), not(projected.contains(r)),
    host::stutter, actors::stutter, held'=held.exclude(Set(r)),
    projected'=projected.union(Set(r)), published'=published, observed'=observed, abandoned'=abandoned
  }
  action retire(r: int): bool = all { projected.contains(r), host::complete(r), actors::stutter, keep }
  // Cleanup the base model's conservative output-root abstraction separately.
  // Owned Rust output may already have been observed before this action.
  action releaseOutput(r: int): bool = all { host::release(r,1), actors::stutter, keep }
  // Owned Rust result publication can precede native permit retirement under
  // the original policy. It requires projection/root-balanced FFI return.
  action publishResult(r: int): bool = all {
    projected.contains(r), not(published.contains(r)), host::stutter, actors::stutter,
    held'=held, projected'=projected, published'=published.union(Set(r)),
    observed'=observed, abandoned'=abandoned
  }
  action consume(r: int): bool = all {
    published.contains(r), not(abandoned.contains(r)),
    host::stutter, actors::stutter, held'=held, projected'=projected,
    published'=published, observed'=observed.union(Set(r)), abandoned'=abandoned
  }
  action dropClient(r: int): bool = all {
    host::observeTimeout(r), actors::stutter, held'=held, projected'=projected,
    published'=published, observed'=observed, abandoned'=abandoned.union(Set(r))
  }
  action cancelUnaccepted(r: int): bool = all {
    held.contains(r), not(host::s.accepted.contains(r)),
    if (host::s.reserved.contains(r)) host::releaseReservation(r) else host::stutter,
    actors::stutter, held'=held.exclude(Set(r)), projected'=projected,
    published'=published, observed'=observed, abandoned'=abandoned
  }
  action reserveBulk = all { host::reserve(1), actors::stutter, keep }
  action close = all { host::close, actors::close, keep }
  val safety = and {
    host::safety, actors::safety, held.subseteq(shorts), held.size()<=SHORT_LIMIT,
    host::s.accepted.intersect(shorts).exclude(projected).subseteq(held),
    projected.intersect(held)==Set(), observed.subseteq(published), published.subseteq(projected),
    host::s.completed.intersect(shorts).subseteq(projected),
    actors::running.size()<=1
  }
  action unsafeEarlyClassRelease(r: int): bool = all {
    held.contains(r), host::s.accepted.contains(r), not(projected.contains(r)),
    host::stutter, actors::stutter, held'=held.exclude(Set(r)), projected'=projected,
    published'=published, observed'=observed, abandoned'=abandoned
  }
action unsafeEarlyNativeRetire(r: int): bool = all {
  not(projected.contains(r)), host::complete(r), actors::stutter, keep
}
run nativeRetirementBeforeProjectionMutationTest = init.then(reserveClass(2)).then(reserveNative(2))
  .then(admit(2)).then(submit(2)).then(start(2)).then(finish(2,false))
  .then(unsafeEarlyNativeRetire(2)).then(all { assert(not(safety)), idle })
  action step = {
    nondet r=shorts.oneOf()
    nondet failed=Bool.oneOf()
    any { reserveClass(r), reserveNative(r), admit(r), submit(r), start(r),
      yieldActor(r), finish(r,failed), project(r), retire(r), releaseOutput(r),
      publishResult(r), consume(r), dropClient(r), cancelUnaccepted(r), reserveBulk, close, idle }
  }
  run projectionBeforeConsumptionTest = init.then(reserveClass(2)).then(reserveNative(2))
    .then(admit(2)).then(submit(2)).then(start(2)).then(finish(2,false)).then(project(2))
    .then(all { assert(held==Set()), assert(observed==Set()), assert(host::s.capacityHeld==Set(2)), assert(safety), idle })
    .then(publishResult(2)).then(consume(2))
    .then(all { assert(host::s.capacityHeld==Set(2)), assert(safety), idle })
    .then(reserveClass(3)).then(retire(2)).then(releaseOutput(2))
    .then(all { assert(held==Set(3)), assert(safety), idle })
run retirementBeforePublicationAlsoSupportedTest = init.then(reserveClass(2)).then(reserveNative(2))
  .then(admit(2)).then(submit(2)).then(start(2)).then(finish(2,false)).then(project(2))
  .then(retire(2)).then(releaseOutput(2)).then(publishResult(2)).then(consume(2))
  .then(all { assert(held==Set()), assert(safety), idle })
  run acceptedCancelCloseDrainTest = init.then(reserveClass(2)).then(reserveNative(2))
    .then(admit(2)).then(submit(2)).then(start(2)).then(dropClient(2)).then(close)
    .then(all { assert(held==Set(2)), assert(host::s.capacityHeld==Set(2)), assert(safety), idle })
    .then(finish(2,false)).then(project(2)).then(retire(2)).then(releaseOutput(2))
    .then(all { assert(held==Set()), assert(host::s.capacityHeld==Set()), assert(observed==Set()), assert(safety), idle })
  run failureRetiresExactlyOnceTest = init.then(reserveClass(2)).then(reserveNative(2))
    .then(admit(2)).then(submit(2)).then(start(2)).then(finish(2,true))
    .then(project(2)).then(retire(2)).then(releaseOutput(2))
    .then(all { assert(held==Set()), assert(host::s.capacityHeld==Set()), assert(safety), idle })
  run earlyReleaseMutationTest = init.then(reserveClass(2)).then(reserveNative(2)).then(admit(2))
    .then(unsafeEarlyClassRelease(2)).then(all { assert(not(safety)), idle })
}
module short_serial {
  import bounded_short(SHORT_LIMIT=1).*
  run singleClassReallyBlocksTest = init.then(reserveClass(2)).then(reserveNative(2)).then(admit(2))
    .then(all { assert(not(canClass(3))), assert(safety), idle })
}
module short_pipeline {
  import bounded_short(SHORT_LIMIT=3).*
  run threeOutstandingOneExecutorAndBulkReserveTest = init
    .then(reserveClass(2)).then(reserveNative(2)).then(admit(2)).then(submit(2))
    .then(reserveClass(3)).then(reserveNative(3)).then(admit(3)).then(submit(3))
    .then(reserveClass(4)).then(reserveNative(4)).then(admit(4)).then(submit(4))
    .then(reserveBulk).then(start(2)).then(yieldActor(2)).then(start(3))
    .then(all { assert(held.size()==3), assert(safety), idle })
}
