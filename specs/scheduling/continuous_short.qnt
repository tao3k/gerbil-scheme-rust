// Refinement of short-concurrency ownership: a bounded open producer's window
// spans root-free input, native work and unread owned results. Two window slots
// are a finite symmetry reduction; the API uses an explicit <= queue bound.
module continuous_short {
  import bounded_short(SHORT_LIMIT=3) as base from "./short_concurrency"
  val nativeHeld=base::nativeHeld
  val readyIds=base::readyIds
  val projectedIds=base::projected
  val publishedIds=base::published
  var window: Set[int]
  var sealed: bool
  var parked: bool
  var wake: bool
  val rootFree = base::held.exclude(base::nativeHeld)
  action keepWake = all { parked'=parked, wake'=wake }
  action keep = all { window'=window, sealed'=sealed, keepWake }
  action init = all { base::init, window'=Set(), sealed'=false, parked'=false, wake'=false }
  action idle = all { base::idle, keep }
  action feed(r:int):bool = all {
    not(sealed), window.size()<2, not(window.contains(r)), base::reserveClass(r),
    window'=window.union(Set(r)), sealed'=sealed, parked'=parked, wake'=true
  }
  action enter(r:int):bool = all {
    not(parked), window.contains(r), base::reserveNative(r), keep
  }
  action admit(r:int):bool = all { base::admit(r), keep }
  action submit(r:int):bool = all { base::submit(r), keep }
  action start(r:int):bool = all { base::start(r), keep }
  action finish(r:int,failed:bool):bool = all { base::finish(r,failed), keep }
  action project(r:int):bool = all { base::project(r), keep }
  action retire(r:int):bool = all { base::retire(r), keep }
  action release(r:int):bool = all { base::releaseOutput(r), keep }
  action publish(r:int):bool = all { base::publishResult(r), keep }
  action consume(r:int):bool = all {
    window.contains(r), base::consume(r), window'=window.exclude(Set(r)), sealed'=sealed, keepWake
  }
  action reject(r:int):bool = all {
    window.contains(r), base::cancelUnaccepted(r), window'=window.exclude(Set(r)), sealed'=sealed, keepWake
  }
  // Abandoning an accepted ticket frees only its host window. Native ownership
  // stays in base until projection/retirement, regardless of client consumption.
  action abandon(r:int):bool = all {
    window.contains(r), base::dropClient(r), window'=window.exclude(Set(r)), sealed'=sealed, keepWake
  }
  action seal = all { base::idle, window'=window, sealed'=true, parked'=parked, wake'=true }
  action close = all { base::close, window'=window, sealed'=true, parked'=parked, wake'=true }
  val safety = and { (parked and rootFree.size()>0) implies wake, base::safety, window.size()<=2, window.subseteq(base::shorts),
    base::held.exclude(base::abandoned).subseteq(window),
    base::projected.exclude(base::observed).exclude(base::abandoned).subseteq(window) }
  action park = all {
    rootFree==Set(), not(wake), base::idle, window'=window, sealed'=sealed, parked'=true, wake'=wake
  }
  action takeWake = all {
    wake, base::idle, window'=window, sealed'=sealed, parked'=false, wake'=false
  }
  action unsafeLoseWake = all {
    base::idle, window'=window, sealed'=sealed, parked'=parked, wake'=false
  }
  run emptyOpenPublicationWakeTest = init.then(park).then(feed(2))
    .then(all { assert(parked),assert(wake),assert(safety),idle })
    .then(takeWake).then(enter(2)).then(all { assert(safety),idle })
  run lostWakeMutationTest = init.then(park).then(feed(2)).then(unsafeLoseWake)
    .then(all { assert(not(safety)),idle })
  action step = {
    nondet r=base::shorts.oneOf()
    nondet failed=Bool.oneOf()
    any { feed(r),enter(r),admit(r),submit(r),start(r),finish(r,failed),
      project(r),retire(r),release(r),publish(r),consume(r),reject(r),abandon(r),seal,close,park,takeWake,idle }
  }
  action unsafeReclaimAtProjection(r:int):bool = all {
    base::project(r), window'=window.exclude(Set(r)), sealed'=sealed, keepWake
  }
  run unreadWindowEarlyReleaseMutationTest = init.then(feed(2)).then(enter(2)).then(admit(2))
    .then(submit(2)).then(start(2)).then(finish(2,false)).then(unsafeReclaimAtProjection(2))
    .then(all { assert(not(safety)),idle })
  run preEntryCancelTest = init.then(feed(2)).then(reject(2))
    .then(all { assert(window==Set()),assert(base::nativeHeld==Set()),assert(safety),idle })
  run supplyBeforeConsumptionTest = init.then(feed(2)).then(enter(2)).then(admit(2))
    .then(submit(2)).then(start(2)).then(finish(2,false)).then(project(2))
    .then(retire(2)).then(release(2)).then(publish(2))
    .then(feed(3)).then(enter(3)).then(admit(3)).then(submit(3)).then(start(3))
    .then(finish(3,false)).then(project(3)).then(retire(3)).then(release(3)).then(publish(3))
    .then(all { assert(window==Set(2,3)), assert(base::observed==Set()), assert(safety),idle })
    .then(consume(2)).then(feed(4)).then(all { assert(window==Set(3,4)),assert(safety),idle })
  run sealDrainsTest = init.then(feed(2)).then(seal).then(enter(2)).then(admit(2))
    .then(submit(2)).then(start(2)).then(finish(2,true)).then(project(2))
    .then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(all { assert(window==Set()),assert(safety),idle })
  run cancellationRetainsNativeTest = init.then(feed(2)).then(enter(2)).then(admit(2))
    .then(submit(2)).then(start(2)).then(abandon(2)).then(close)
    .then(all { assert(window==Set()),assert(base::nativeHeld==Set(2)),assert(safety),idle })
    .then(finish(2,false)).then(project(2)).then(retire(2)).then(release(2))
    .then(all { assert(base::nativeHeld==Set()),assert(safety),idle })
}
