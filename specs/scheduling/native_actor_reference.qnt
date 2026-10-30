// Native Gerbil baseline reuses bounded source and Actor/root ownership.
// A completion watchdog observes verified results, never supply or scheduler turns.
module native_actor_reference {
  import continuous_short as r from "./continuous_short"
  val nativeHeld=r::nativeHeld
  val published=r::publishedIds
  var verified:Set[int]
  action keep=all {verified'=verified}
  action init=all {r::init,verified'=Set()}
  action idle=all {r::idle,keep}
  action feed(id:int):bool=all {r::feed(id),keep}
  action enter(id:int):bool=all {r::enter(id),keep}
  action admit(id:int):bool=all {r::admit(id),keep}
  action submit(id:int):bool=all {r::submit(id),keep}
  action start(id:int):bool=all {r::start(id),keep}
  action finish(id:int):bool=all {r::finish(id,false),keep}
  action project(id:int):bool=all {r::project(id),keep}
  action verify(id:int):bool=all {
    r::projectedIds.contains(id),not(verified.contains(id)),r::idle,
    verified'=verified.union(Set(id))
  }
  action retire(id:int):bool=all {verified.contains(id),r::retire(id),keep}
  action release(id:int):bool=all {r::release(id),keep}
  action publish(id:int):bool=all {verified.contains(id),r::publish(id),keep}
  action consume(id:int):bool=all {r::consume(id),keep}
  action close=all {r::close,keep}
  val safety=and {r::safety,verified.subseteq(r::projectedIds)}
  action unsafeSupplyHeartbeat(id:int):bool=all {r::idle,verified'=verified.union(Set(id))}
  run supplyIsNotCompletedProgressTest=init.then(feed(2)).then(enter(2)).then(admit(2)).then(submit(2))
    .then(all {assert(verified==Set()),assert(safety),idle})
    .then(start(2)).then(finish(2)).then(project(2)).then(verify(2)).then(retire(2)).then(release(2))
    .then(publish(2)).then(consume(2)).then(all {assert(verified==Set(2)),assert(safety),idle})
  run supplyHeartbeatMutationTest=init.then(feed(2)).then(unsafeSupplyHeartbeat(2))
    .then(all {assert(not(safety)),idle})
  run independentNativeConsumptionTest=init.then(feed(2)).then(feed(3))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(enter(3)).then(admit(3)).then(submit(3))
    .then(start(2)).then(finish(2)).then(project(2)).then(verify(2)).then(retire(2)).then(release(2))
    .then(publish(2)).then(consume(2)).then(close)
    .then(start(3)).then(finish(3)).then(project(3)).then(verify(3)).then(retire(3)).then(release(3))
    .then(publish(3)).then(consume(3)).then(all {assert(safety),assert(verified==Set(2,3)),idle})
  action step={
    nondet id=Set(2,3,4).oneOf()
    any {feed(id),enter(id),admit(id),submit(id),start(id),finish(id),project(id),verify(id),retire(id),
      release(id),publish(id),consume(id),close,idle}
  }
}
