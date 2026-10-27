// Refine the original ticket window with joint staged-job/client custody.
// A dropped client cannot make a still-queued input disappear from capacity.
module continuous_lease {
  import continuous_short as old from "./continuous_short"
  var custody:Set[int]
  var dropped:Set[int]
  val window=custody
  val sealed=old::sealed
  val projectedIds=old::projectedIds
  val readyIds=old::readyIds
  action keep=all {custody'=custody,dropped'=dropped}
  action init=all {old::init,custody'=Set(),dropped'=Set()}
  action idle=all {old::idle,keep}
  action feed(id:int):bool=all {
    custody.size()<2,not(custody.contains(id)),old::feed(id),custody'=custody.union(Set(id)),dropped'=dropped.exclude(Set(id))
  }
  action enter(id:int):bool=all {not(dropped.contains(id)),old::enter(id),keep}
  action admit(id:int):bool=all {old::admit(id),keep}
  action submit(id:int):bool=all {old::submit(id),keep}
  action start(id:int):bool=all {old::start(id),keep}
  action finish(id:int,failed:bool):bool=all {old::finish(id,failed),keep}
  action project(id:int):bool=all {
    old::project(id),custody'=if (dropped.contains(id)) custody.exclude(Set(id)) else custody,dropped'=dropped
  }
  action retire(id:int):bool=all {old::retire(id),keep}
  action release(id:int):bool=all {old::release(id),keep}
  action publish(id:int):bool=all {old::publish(id),keep}
  action consume(id:int):bool=all {old::consume(id),custody'=custody.exclude(Set(id)),dropped'=dropped}
  action cancelQueued(id:int):bool=all {
    old::rootFree.contains(id),old::idle,custody'=custody,dropped'=dropped.union(Set(id))
  }
  action reject(id:int):bool=all {old::reject(id),custody'=custody.exclude(Set(id)),dropped'=dropped}
  action abandon(id:int):bool=all {
    old::abandon(id),custody'=if (old::projectedIds.contains(id)) custody.exclude(Set(id)) else custody,
    dropped'=dropped.union(Set(id))
  }
  action close=all {old::close,keep}
  val safety=and {old::safety,custody.size()<=2,old::window.subseteq(custody)}
  action unsafeFreeQueuedLease(id:int):bool=all {old::idle,custody'=custody.exclude(Set(id)),dropped'=dropped.union(Set(id))}
  run canceledQueueRetainsCreditTest=init.then(feed(2)).then(feed(3)).then(cancelQueued(2))
    .then(all {assert(custody==Set(2,3)),assert(safety),idle})
    .then(reject(2)).then(feed(4)).then(all {assert(custody==Set(3,4)),assert(safety),idle})
  run prematureReturnMutationTest=init.then(feed(2)).then(unsafeFreeQueuedLease(2))
    .then(all {assert(not(safety)),idle})
  action step={
    nondet id=Set(2,3,4).oneOf()
    nondet failed=Bool.oneOf()
    any {feed(id),enter(id),admit(id),submit(id),start(id),finish(id,failed),project(id),retire(id),
      release(id),publish(id),consume(id),cancelQueued(id),reject(id),abandon(id),close,idle}
  }
}
