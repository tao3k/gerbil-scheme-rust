// Reuse frame/root custody. A source-owned rational cursor assigns immutable
// original planned times, including out-of-order completion and frame reuse.
module native_source_plan {
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"
  val units=10
  val rate=3
  var ordinal:int
  var nextTime:int
  var carry:int
  var stamps:int -> int
  pure def original(n:int):int=n*units/rate
  action keep=all {ordinal'=ordinal,nextTime'=nextTime,carry'=carry,stamps'=stamps}
  action init=all {p::init,ordinal'=0,nextTime'=0,carry'=0,stamps'=Map(2-> -1,3-> -1,4-> -1)}
  action idle=all {p::idle,keep}
  action offer(id:int):bool=all {p::offer(id),keep}
  action feed(id:int):bool=all {id==ordinal+2,p::feed(id),ordinal'=ordinal+1,
    nextTime'=nextTime+units/rate+(if (carry+units%rate>=rate) 1 else 0),
    carry'=(carry+units%rate)%rate,stamps'=stamps.put(id,nextTime)}
  action enter(id:int):bool=all {p::enter(id),keep}
  action admit(id:int):bool=all {p::admit(id),keep}
  action submit(id:int):bool=all {p::submit(id),keep}
  action start(id:int):bool=all {p::start(id),keep}
  action finish(id:int):bool=all {p::finish(id),keep}
  action project(id:int):bool=all {p::project(id),keep}
  action verify(id:int):bool=all {p::verify(id),keep}
  action retire(id:int):bool=all {p::retire(id),keep}
  action release(id:int):bool=all {p::release(id),keep}
  action publish(id:int):bool=all {p::publish(id),keep}
  action consume(id:int):bool=all {stamps.get(id)==original(id-2),p::consume(id),
    ordinal'=ordinal,nextTime'=nextTime,carry'=carry,stamps'=stamps.put(id,-1)}
  action close=all {p::close,keep}
  val safety=and {p::safety,nextTime==original(ordinal),carry==(ordinal*units)%rate,
    Set(2,3,4).forall(id => if (p::live.contains(id)) stamps.get(id)==original(id-2) else stamps.get(id)== -1)}
  action unsafeRetime(id:int):bool=all {p::idle,ordinal'=ordinal,nextTime'=nextTime,
    carry'=carry,stamps'=stamps.put(id,nextTime+1)}
  action unsafeSkipCarry=all {p::idle,ordinal'=ordinal,nextTime'=nextTime+1,
    carry'=carry,stamps'=stamps}
  run sourceStallDoesNotMoveOriginalTimestampTest=init.then(offer(2)).then(feed(2))
    .then(idle).then(idle).then(all {assert(stamps.get(2)==0),assert(safety),idle})
  run lateWorkCannotResetItsClockMutationTest=init.then(offer(2)).then(feed(2))
    .then(unsafeRetime(2)).then(all {assert(not(safety)),idle})
  run missingFractionalCarryMutationTest=init.then(unsafeSkipCarry)
    .then(all {assert(not(safety)),idle})
  run outOfOrderCompletionAndReuseKeepsEachOriginalTimeTest=init
    .then(offer(2)).then(feed(2)).then(offer(3)).then(feed(3))
    .then(enter(3)).then(admit(3)).then(submit(3)).then(start(3)).then(finish(3))
    .then(project(3)).then(verify(3)).then(retire(3)).then(release(3)).then(publish(3))
    .then(consume(3)).then(offer(4)).then(feed(4))
    .then(enter(2)).then(admit(2)).then(submit(2))
    .then(enter(4)).then(admit(4)).then(submit(4)).then(close)
    .then(all {assert(stamps.get(2)==0),assert(stamps.get(4)==6),assert(safety),idle})
    .then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(start(4)).then(finish(4))
    .then(project(4)).then(verify(4)).then(retire(4)).then(release(4)).then(publish(4)).then(consume(4))
    .then(all {assert(stamps==Map(2-> -1,3-> -1,4-> -1)),assert(safety),idle})
  run integerAndFractionalTransitionIdentityTest=init.then(all {
    assert(1.to(9).forall(r => 0.to(20).forall(n =>
      (n*units/r)+(units/r)+(if ((n*units)%r+units%r>=r) 1 else 0)==((n+1)*units)/r))),idle})
  action step={nondet id=Set(2,3,4).oneOf()
    any {offer(id),feed(id),enter(id),admit(id),submit(id),start(id),finish(id),project(id),
      verify(id),retire(id),release(id),publish(id),consume(id),close,idle}}
}
