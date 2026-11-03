// Existing native Actor lifecycle plus a ready-aware gateway opportunity.
module native_mailbox_pipeline {
  import native_actor_reference as n from "./native_actor_reference"
  var visible:Set[int]
  var emptyHandoff:bool
  var leases:int -> int
  val leaseIds=Set(0,1).map(slot=>leases.get(slot))
  action keep=all {visible'=visible,emptyHandoff'=false,leases'=leases}
  action init=all {n::init,visible'=Set(),emptyHandoff'=false,leases'=Map(0-> -1,1-> -1)}
  action idle=all {n::idle,keep}
  action feed(id:int):bool=all {n::feed(id),keep}
  action enter(id:int):bool=all {
    nondet slot=Set(0,1).oneOf()
    all {leases.get(slot)== -1,n::enter(id),visible'=visible,emptyHandoff'=false,leases'=leases.put(slot,id)}
  }
  action admit(id:int):bool=all {n::admit(id),keep}
  action submit(id:int):bool=all {n::submit(id),keep}
  action start(id:int):bool=all {n::start(id),keep}
  action finish(id:int):bool=all {n::finish(id),visible'=visible.union(Set(id)),emptyHandoff'=false,leases'=leases}
  action project(id:int):bool=all {visible.contains(id),leaseIds.contains(id),n::project(id),visible'=visible.exclude(Set(id)),emptyHandoff'=false,leases'=leases}
  action verify(id:int):bool=all {n::verify(id),keep}
  action retire(id:int):bool=all {n::retire(id),keep}
  action release(id:int):bool=all {n::release(id),visible'=visible,emptyHandoff'=false,
    leases'=Set(0,1).mapBy(slot=>if (leases.get(slot)==id) -1 else leases.get(slot))}
  action publish(id:int):bool=all {n::publish(id),keep}
  action consume(id:int):bool=all {n::consume(id),keep}
  action handoff=all {visible==Set(),n::idle,visible'=visible,emptyHandoff'=true,leases'=leases}
  action close=all {n::close,keep}
  val conservation=(emptyHandoff implies visible==Set())
  val safety=and {n::safety,conservation,n::nativeHeld.subseteq(leaseIds),
    leaseIds.subseteq(Set(-1,2,3,4)),leases.get(0)== -1 or leases.get(1)== -1 or leases.get(0)!=leases.get(1)}
  action unconditionalHandoff=all {n::idle,visible'=visible,emptyHandoff'=true,leases'=leases}
  run readyMustBeConsumedBeforeHandoffTest=init.then(feed(2)).then(enter(2)).then(admit(2)).then(submit(2))
    .then(start(2)).then(finish(2)).then(project(2)).then(verify(2)).then(retire(2)).then(release(2))
    .then(publish(2)).then(consume(2)).then(handoff).then(all {assert(safety),idle})
  run forcedHandoffMutationTest=init.then(feed(2)).then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(unconditionalHandoff).then(all {assert(not(conservation)),assert(n::safety),idle})
  action unsafeReturnBySlot(slot:int):bool=all {n::idle,visible'=visible,emptyHandoff'=false,leases'=leases.put(slot,-1)}
  run reusedSlotRejectsStaleOwnershipTest=init.then(feed(2)).then(enter(2)).then(admit(2)).then(submit(2)).then(start(2)).then(finish(2))
    .then(project(2)).then(verify(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(feed(3)).then(enter(3)).then(admit(3)).then(submit(3))
    .then(all {assert(not(leaseIds.contains(2))),assert(leaseIds.contains(3)),assert(safety),idle})
  run prematureSlotReturnMutationTest=init.then(feed(2)).then(enter(2))
    .then({nondet slot=Set(0,1).oneOf() all {leases.get(slot)==2,unsafeReturnBySlot(slot)}})
    .then(all {assert(not(safety)),idle})
  action step={nondet id=Set(2,3,4).oneOf()
    any {feed(id),enter(id),admit(id),submit(id),start(id),finish(id),project(id),verify(id),retire(id),release(id),publish(id),consume(id),handoff,close,idle}}
}
