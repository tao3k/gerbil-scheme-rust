// Reuse supply, Actor execution, result snapshots and independent root retirement.
// Completion mailbox custody does not release native or unread-output capacity.
module mailbox_handoff {
  import actor_completion(HARVEST=2) as c from "./actor_completion"
  var pending:Set[int]
  action keep=all {pending'=pending}
  action init=all {c::init,pending'=Set()}
  action idle=all {c::idle,keep}
  action feed(id:int):bool=all {c::feed(id),keep}
  action reserve(id:int):bool=all {c::reserve(id),keep}
  action admit(id:int):bool=all {c::admit(id),keep}
  action supply(id:int):bool=all {c::supply(id),keep}
  action advance=all {c::advance,keep}
  action start(id:int):bool=all {c::start(id),keep}
  action finish(id:int):bool=all {c::finish(id),pending'=pending.union(Set(id))}
  action deliver(id:int):bool=all {pending.contains(id),c::idle,pending'=pending.exclude(Set(id))}
  action claim=all {c::claimVisible(c::readyIds.exclude(pending)),keep}
  action project(id:int):bool=all {not(pending.contains(id)),c::project(id),keep}
  action retire(id:int):bool=all {c::retire(id),keep}
  action release(id:int):bool=all {c::release(id),keep}
  action publish(id:int):bool=all {c::publish(id),keep}
  action consume(id:int):bool=all {c::consume(id),keep}
  action abandon(id:int):bool=all {c::abandon(id),keep}
  action close=all {c::close,keep}
  val safety=and {c::safety,pending.subseteq(c::readyIds),pending.intersect(c::snapshot)==Set()}
  action unsafeClaimBeforeDelivery=all {c::claim,keep}
  run completionMailboxRetainsCustodyTest=init.then(feed(2)).then(reserve(2)).then(admit(2)).then(supply(2))
    .then(advance).then(start(2)).then(finish(2)).then(close)
    .then(all {assert(pending==Set(2)),assert(safety),idle})
    .then(deliver(2)).then(claim).then(project(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(all {assert(safety),assert(pending==Set()),idle})
  run deliveryBeforeClaimMutationTest=init.then(feed(2)).then(reserve(2)).then(admit(2)).then(supply(2))
    .then(advance).then(start(2)).then(finish(2)).then(unsafeClaimBeforeDelivery)
    .then(all {assert(not(safety)),idle})
  run readyResultDoesNotWaitForOtherDeliveryTest=init.then(feed(2)).then(feed(3))
    .then(reserve(2)).then(admit(2)).then(supply(2)).then(reserve(3)).then(admit(3)).then(supply(3))
    .then(advance).then(start(2)).then(finish(2)).then(deliver(2)).then(start(3)).then(finish(3)).then(claim)
    .then(all {assert(c::snapshot==Set(2)),assert(pending==Set(3)),assert(safety),idle})
    .then(project(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(abandon(3)).then(close).then(deliver(3)).then(claim).then(project(3)).then(retire(3)).then(release(3))
    .then(all {assert(c::snapshot==Set()),assert(pending==Set()),assert(safety),idle})
  run supplyBeforeAllCompletionsRetireTest=init.then(feed(2)).then(feed(3))
    .then(reserve(2)).then(admit(2)).then(supply(2)).then(reserve(3)).then(admit(3)).then(supply(3))
    .then(advance).then(start(2)).then(finish(2)).then(deliver(2)).then(start(3)).then(finish(3)).then(claim)
    .then(project(2)).then(retire(2)).then(release(2)).then(publish(2)).then(consume(2))
    .then(feed(4)).then(reserve(4)).then(admit(4)).then(supply(4))
    .then(all {assert(pending==Set(3)),assert(safety),idle})
    .then(start(4)).then(finish(4)).then(abandon(3)).then(close).then(deliver(3)).then(deliver(4)).then(claim)
    .then(project(3)).then(retire(3)).then(release(3)).then(project(4)).then(retire(4)).then(release(4)).then(publish(4)).then(consume(4))
    .then(all {assert(c::snapshot==Set()),assert(pending==Set()),assert(safety),idle})
  action step={
    nondet id=Set(2,3).oneOf()
    any {feed(id),reserve(id),admit(id),supply(id),advance,start(id),finish(id),deliver(id),claim,
      project(id),retire(id),release(id),publish(id),consume(id),abandon(id),close,idle}
  }
}
