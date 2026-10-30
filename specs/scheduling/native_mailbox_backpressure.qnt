// Reuse native Actor execution/verification ownership. Source debt is distinct
// from the fixed materialized window; a completed frame is not reusable until
// verification, retirement and consumption have all finished.
module native_mailbox_backpressure {
  import native_actor_reference as n from "./native_actor_reference"
  var offered: Set[int]
  var pending: Set[int]
  var delivered: Set[int]
  var consumed: Set[int]
  var frames: int -> int
  val ready=n::published.exclude(consumed)
  val live = Set(0,1).map(slot => frames.get(slot)).exclude(Set(-1))
  action keep = all {offered'=offered,pending'=pending,delivered'=delivered,consumed'=consumed,frames'=frames}
  action init = all {n::init,offered'=Set(),pending'=Set(),delivered'=Set(),consumed'=Set(),frames'=Map(0-> -1,1-> -1)}
  action idle = all {n::idle,keep}
  // Diagnostic phase transitions are observational stutters. Reading clocks
  // never verifies a result, returns a frame or advances service progress.
  action observe = idle
  action offer(id:int):bool = all {
    not(offered.contains(id)),n::idle,offered'=offered.union(Set(id)),
    pending'=pending.union(Set(id)),delivered'=delivered,consumed'=consumed,frames'=frames
  }
  action feed(id:int):bool = {
    nondet slot=Set(0,1).oneOf()
    all {pending.contains(id),frames.get(slot)== -1,n::feed(id),
      offered'=offered,pending'=pending.exclude(Set(id)),delivered'=delivered.union(Set(id)),
      consumed'=consumed,frames'=frames.put(slot,id)}
  }
  action enter(id:int):bool = all {live.contains(id),n::enter(id),keep}
  action admit(id:int):bool = all {n::admit(id),keep}
  action submit(id:int):bool = all {n::submit(id),keep}
  action start(id:int):bool = all {n::start(id),keep}
  action finish(id:int):bool = all {n::finish(id),keep}
  action project(id:int):bool = all {n::project(id),keep}
  action verify(id:int):bool = all {n::verify(id),keep}
  action retire(id:int):bool = all {n::retire(id),keep}
  action release(id:int):bool = all {n::release(id),keep}
  action publish(id:int):bool = all {n::publish(id),keep}
  action consume(id:int):bool = all {
    not(n::nativeHeld.contains(id)),n::consume(id),offered'=offered,pending'=pending,delivered'=delivered,
    consumed'=consumed.union(Set(id)),
    frames'=Set(0,1).mapBy(slot => if (frames.get(slot)==id) -1 else frames.get(slot))
  }
  action close = all {n::close,keep}
  val safety = and {n::safety,offered==pending.union(delivered),pending.intersect(delivered)==Set(),
    consumed.subseteq(delivered),live==delivered.exclude(consumed),n::nativeHeld.subseteq(live),
    frames.get(0)== -1 or frames.get(1)== -1 or frames.get(0)!=frames.get(1)}
  // Resource safety and zero drops do not imply bounded source debt. Two is
  // the inherited finite window reduction, not a new implementation limit.
  val sourceBounded = pending.size()+live.size()<=2
  action unsafeDrop(id:int):bool = all {n::idle,offered'=offered,pending'=pending.exclude(Set(id)),
    delivered'=delivered,consumed'=consumed,frames'=frames}
  action unsafeReuse(id:int):bool = all {n::idle,offered'=offered,pending'=pending,delivered'=delivered,
    consumed'=consumed,frames'=Set(0,1).mapBy(slot => if (frames.get(slot)==id) -1 else frames.get(slot))}
  action unsafeConsumeBeforeRetirement(id:int):bool = all {
    n::consume(id),offered'=offered,pending'=pending,delivered'=delivered,
    consumed'=consumed.union(Set(id)),
    frames'=Set(0,1).mapBy(slot => if (frames.get(slot)==id) -1 else frames.get(slot))
  }
  run boundedSourceDebtTest = init.then(offer(2)).then(offer(3)).then(offer(4)).then(feed(2)).then(feed(3))
    .then(all {assert(pending==Set(4)),assert(live==Set(2,3)),assert(safety),idle})
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2)).then(project(2))
    .then(verify(2)).then(retire(2)).then(release(2)).then(publish(2))
    .then(all {assert(live.contains(2)),assert(safety),idle}).then(consume(2)).then(feed(4))
    .then(all {assert(pending==Set()),assert(live==Set(3,4)),assert(safety),idle})
  run sourceDropMutationTest = init.then(offer(2)).then(unsafeDrop(2)).then(all {assert(not(safety)),idle})
  run zeroDropIsNotBoundedDebtTest = init.then(offer(2)).then(offer(3)).then(offer(4))
    .then(feed(2)).then(feed(3))
    .then(all {assert(safety),assert(not(sourceBounded)),assert(pending==Set(4)),idle})
  run unreadReuseMutationTest = init.then(offer(2)).then(feed(2)).then(enter(2)).then(admit(2))
    .then(submit(2)).then(start(2)).then(finish(2)).then(project(2)).then(verify(2)).then(retire(2))
    .then(release(2)).then(publish(2)).then(unsafeReuse(2)).then(all {assert(not(safety)),idle})
  run consumptionBeforeNativeRetirementMutationTest = init.then(offer(2)).then(feed(2)).then(enter(2))
    .then(admit(2)).then(submit(2)).then(start(2)).then(finish(2)).then(project(2)).then(verify(2))
    .then(publish(2)).then(unsafeConsumeBeforeRetirement(2)).then(all {assert(not(safety)),idle})
  run observationCannotCompleteOrReturnFrameTest = init.then(offer(2)).then(feed(2)).then(enter(2))
    .then(admit(2)).then(submit(2)).then(start(2)).then(observe)
    .then(all {assert(n::verified==Set()),assert(live==Set(2)),assert(safety),idle})
    .then(finish(2)).then(project(2)).then(observe)
    .then(all {assert(n::verified==Set()),assert(n::nativeHeld.contains(2)),assert(safety),idle})
    .then(verify(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(all {assert(live==Set()),assert(safety),idle})
  action step = {nondet id=Set(2,3,4).oneOf()
    any {offer(id),feed(id),enter(id),admit(id),submit(id),start(id),finish(id),project(id),verify(id),
      retire(id),release(id),publish(id),consume(id),close,observe,idle}}
}
