// One bounded transfer of ready Actor identities. Projection and native credit
// retirement remain per message; the snapshot is not an execution/lease budget.
module actor_completion {
  const HARVEST:int
  import actor_supply(WIDTH=2) as p from "./actor_supply"
  val projectedIds=p::projectedIds
  val readyIds=p::readyIds
  var snapshot:Set[int]
  var claims:int
  action keep=all {snapshot'=snapshot,claims'=claims}
  action init=all {p::init,snapshot'=Set(),claims'=0}
  action idle=all {p::idle,keep}
  action feed(r:int):bool=all {p::feed(r),keep}
  action reserve(r:int):bool=all {p::reserve(r),keep}
  action admit(r:int):bool=all {p::admit(r),keep}
  action supply(r:int):bool=all {p::supply(r),keep}
  action advance=all {p::advance,keep}
  action start(r:int):bool=all {p::start(r),keep}
  action finish(r:int):bool=all {p::finish(r),keep}
  action claimVisible(visible:Set[int]):bool=all {
    nondet r=visible.oneOf()
    all {snapshot==Set(),visible.size()>0,visible.subseteq(p::readyIds),p::idle,
      snapshot'=if (HARVEST==1) Set(r) else visible,claims'=claims+1}
  }
  action claim=claimVisible(p::readyIds)
  action project(r:int):bool=all {
    snapshot.contains(r),not(p::projectedIds.contains(r)),p::project(r),keep
  }
  action retire(r:int):bool=all {
    snapshot.contains(r),p::retire(r),snapshot'=snapshot.exclude(Set(r)),claims'=claims
  }
  action release(r:int):bool=all {p::release(r),keep}
  action publish(r:int):bool=all {p::publish(r),keep}
  action consume(r:int):bool=all {p::consume(r),keep}
  action abandon(r:int):bool=all {p::abandon(r),keep}
  action reject(r:int):bool=all {p::reject(r),keep}
  action close=all {p::close,keep}
  val safety=and {p::safety,snapshot.size()<=HARVEST,snapshot.subseteq(p::readyIds)}
  action step={
    nondet r=Set(2,3).oneOf()
    any {feed(r),reserve(r),admit(r),supply(r),advance,start(r),finish(r),claim,
      project(r),retire(r),release(r),publish(r),consume(r),abandon(r),close,idle}
  }
}
module bounded_actor_completion {
  import actor_completion(HARVEST=2).*
  run independentSnapshotRetirementTest=init.then(feed(2)).then(feed(3))
    .then(reserve(2)).then(admit(2)).then(supply(2)).then(reserve(3)).then(admit(3)).then(supply(3))
    .then(advance).then(start(2)).then(finish(2)).then(start(3)).then(finish(3)).then(claim)
    .then(all {assert(snapshot==Set(2,3)),assert(safety),idle})
    .then(project(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(all {assert(snapshot==Set(3)),assert(safety),idle})
    .then(abandon(3)).then(close).then(project(3)).then(retire(3)).then(release(3))
    .then(all {assert(snapshot==Set()),assert(safety),idle})
}
module serial_actor_completion {
  import actor_completion(HARVEST=1).*
  run oneReadyIdentityPerClaimTest=init.then(feed(2)).then(feed(3))
    .then(reserve(2)).then(admit(2)).then(supply(2)).then(reserve(3)).then(admit(3)).then(supply(3))
    .then(advance).then(start(2)).then(finish(2)).then(start(3)).then(finish(3)).then(claim)
    .then(all {assert(snapshot.size()==1),assert(claims==1),assert(safety),idle})
}
