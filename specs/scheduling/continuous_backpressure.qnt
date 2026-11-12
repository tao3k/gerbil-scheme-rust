// Waiting is an upstream obligation, not another queue slot or Scheme root.
module continuous_backpressure {
  import continuous_lease as c from "./continuous_lease"
  var waiting:Set[int]
  action keep=all {waiting'=waiting}
  action init=all {c::init,waiting'=Set()}
  action idle=all {c::idle,keep}
  action feed(id:int):bool=all {not(waiting.contains(id)),c::feed(id),keep}
  action wait(id:int):bool=all {
    not(c::sealed),c::window.size()==2,not(c::window.contains(id)),not(waiting.contains(id)),
    c::idle,waiting'=waiting.union(Set(id))
  }
  action acquire(id:int):bool=all {waiting.contains(id),c::feed(id),waiting'=waiting.exclude(Set(id))}
  action cancelWait(id:int):bool=all {waiting.contains(id),c::idle,waiting'=waiting.exclude(Set(id))}
  action enter(id:int):bool=all {c::enter(id),keep}
  action admit(id:int):bool=all {c::admit(id),keep}
  action submit(id:int):bool=all {c::submit(id),keep}
  action start(id:int):bool=all {c::start(id),keep}
  action finish(id:int):bool=all {c::finish(id,false),keep}
  action project(id:int):bool=all {c::project(id),keep}
  action retire(id:int):bool=all {c::retire(id),keep}
  action release(id:int):bool=all {c::release(id),keep}
  action publish(id:int):bool=all {c::publish(id),keep}
  action consume(id:int):bool=all {c::consume(id),keep}
  action cancelQueued(id:int):bool=all {c::cancelQueued(id),keep}
  action removeQueued(id:int):bool=all {c::reject(id),keep}
  action close=all {c::close,waiting'=Set()}
  val safety=and {c::safety,waiting.intersect(c::window)==Set()}
  run consumptionReturnsAdmissionTest=init.then(feed(2)).then(feed(3)).then(wait(4))
    .then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(retire(2)).then(release(2)).then(publish(2))
    .then(all {assert(c::window==Set(2,3)),assert(waiting==Set(4)),assert(safety),idle})
    .then(consume(2)).then(acquire(4)).then(all {assert(c::window==Set(3,4)),assert(safety),idle})
  run timeoutOrDroppedWaiterOwnsNothingTest=init.then(feed(2)).then(feed(3)).then(wait(4)).then(cancelWait(4))
    .then(all {assert(waiting==Set()),assert(c::window==Set(2,3)),assert(safety),idle})
  run closeWakesUnadmittedWaitersTest=init.then(feed(2)).then(feed(3)).then(wait(4)).then(close)
    .then(all {assert(waiting==Set()),assert(c::sealed),assert(safety),idle})
  run canceledQueuedInputWakesWaitingAdmissionTest=init.then(feed(2)).then(feed(3)).then(wait(4)).then(cancelQueued(2))
    .then(all {assert(c::window==Set(2,3)),assert(waiting==Set(4)),assert(safety),idle})
    .then(removeQueued(2)).then(acquire(4)).then(all {assert(c::window==Set(3,4)),assert(safety),idle})
  action step={
    nondet id=Set(2,3,4).oneOf()
    any {feed(id),wait(id),acquire(id),cancelWait(id),enter(id),admit(id),submit(id),start(id),finish(id),
      project(id),retire(id),release(id),publish(id),consume(id),cancelQueued(id),removeQueued(id),close,idle}
  }
}
