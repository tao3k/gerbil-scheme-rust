// Diagnostic ownership only; reuse the complete native capacity/frame model.
// Two green owners and one mark each expose the lost-update interleaving.
module native_observation {
  const PRIVATE:bool
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"
  val owners=Set(2,3)
  var started:Set[int]
  var finished:Set[int]
  var reads:int -> int
  var localCounts:int -> int
  var sharedCount:int
  val observed=if (PRIVATE) owners.fold(0,(sum,id)=>sum+localCounts.get(id)) else sharedCount
  action keep=all {started'=started,finished'=finished,reads'=reads,localCounts'=localCounts,sharedCount'=sharedCount}
  action init=all {p::init,started'=Set(),finished'=Set(),reads'=owners.mapBy(_=>0),
    localCounts'=owners.mapBy(_=>0),sharedCount'=0}
  action idle=all {p::idle,keep}
  action begin(id:int):bool=all {not(started.contains(id)),p::idle,started'=started.union(Set(id)),
    finished'=finished,reads'=reads.put(id,if (PRIVATE) localCounts.get(id) else sharedCount),
    localCounts'=localCounts,sharedCount'=sharedCount}
  action commit(id:int):bool=all {started.contains(id),not(finished.contains(id)),p::idle,
    started'=started,finished'=finished.union(Set(id)),reads'=reads,
    localCounts'=if (PRIVATE) localCounts.put(id,reads.get(id)+1) else localCounts,
    sharedCount'=if (PRIVATE) sharedCount else reads.get(id)+1}
  action protocol=all {p::step,keep}
  val safety=and {p::safety,observed==finished.size(),finished.subseteq(started)}
  action step={nondet id=owners.oneOf() any {protocol,begin(id),commit(id),idle}}
}
module native_observation_private {
  import native_observation(PRIVATE=true).*
  run interruptedPrivateMarkTest=init.then(begin(2)).then(begin(3)).then(commit(3)).then(commit(2))
    .then(all {assert(observed==2),assert(safety),idle})
}
module native_observation_shared_mutation {
  import native_observation(PRIVATE=false).*
  run sharedGreenOwnerMutationTest=init.then(begin(2)).then(begin(3)).then(commit(3)).then(commit(2))
    .then(all {assert(observed==1),assert(not(safety)),idle})
}
