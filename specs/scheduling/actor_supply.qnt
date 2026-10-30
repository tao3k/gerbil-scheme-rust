// A host supply opportunity is not Actor execution. Reuse the existing bounded
// continuous-source and owner/Actor lifecycle models; do not model Tokio as VM CPU.
module actor_supply {
  const WIDTH:int
  import continuous_lease as c from "./continuous_lease"
  val readyIds=c::readyIds
  val projectedIds=c::projectedIds
  var left:int
  var sent:Set[int]
  var opportunitySet:Set[int]
  var advances:int
  var interrupted:bool
  action keep = all {left'=left,sent'=sent,opportunitySet'=opportunitySet,advances'=advances,interrupted'=interrupted}
  action init = all {c::init,left'=WIDTH,sent'=Set(),opportunitySet'=Set(),advances'=0,interrupted'=false}
  action idle = all {c::idle,keep}
  action feed(r:int):bool=all {c::feed(r),keep}
  action reserve(r:int):bool=all {c::enter(r),keep}
  action admit(r:int):bool=all {c::admit(r),keep}
  action supply(r:int):bool=all {
    left>0,not(interrupted),c::submit(r),left'=left-1,sent'=sent.union(Set(r)),
    opportunitySet'=opportunitySet,advances'=advances,interrupted'=interrupted
  }
  action advance = all {
    c::idle,left'=WIDTH,sent'=Set(),opportunitySet'=opportunitySet.union(sent),advances'=advances+1,interrupted'=false
  }
  action controlArrives = all {c::idle,left'=left,sent'=sent,opportunitySet'=opportunitySet,advances'=advances,interrupted'=true}
  action start(r:int):bool=all {c::start(r),keep}
  action finish(r:int):bool=all {c::finish(r,false),keep}
  action project(r:int):bool=all {c::project(r),keep}
  action retire(r:int):bool=all {c::retire(r),keep}
  action release(r:int):bool=all {c::release(r),keep}
  action publish(r:int):bool=all {c::publish(r),keep}
  action consume(r:int):bool=all {c::consume(r),keep}
  action abandon(r:int):bool=all {c::abandon(r),keep}
  action reject(r:int):bool=all {c::reject(r),keep}
  action close = all {c::close,left'=left,sent'=sent,opportunitySet'=opportunitySet,advances'=advances,interrupted'=true}
  val safety=and {c::safety,0<=left,left<=WIDTH,sent.size()<=WIDTH}
  action step = {
    nondet r=Set(2,3).oneOf()
    any {feed(r),reserve(r),admit(r),supply(r),advance,controlArrives,start(r),finish(r),
      project(r),retire(r),release(r),publish(r),consume(r),abandon(r),close,idle}
  }
}
module serial_actor_supply {
  import actor_supply(WIDTH=1).*
  run serialEnvelopeTest=init.then(feed(2)).then(reserve(2)).then(admit(2)).then(supply(2))
    .then(all {assert(left==0),assert(advances==0),assert(safety),idle})
    .then(advance).then(start(2)).then(finish(2)).then(project(2)).then(retire(2)).then(release(2))
    .then(publish(2)).then(consume(2)).then(all {assert(safety),idle})
}
module pipelined_actor_supply {
  import actor_supply(WIDTH=2).*
  run twoEnvelopesOneOpportunityTest=init.then(feed(2)).then(feed(3))
    .then(reserve(2)).then(admit(2)).then(supply(2)).then(reserve(3)).then(admit(3)).then(supply(3))
    .then(all {assert(sent==Set(2,3)),assert(advances==0),assert(safety),idle})
    .then(advance).then(start(2)).then(finish(2)).then(start(3)).then(finish(3))
    .then(all {assert(advances==1),assert(safety),idle})
  run controlInterruptsSupplyTest=init.then(feed(2)).then(reserve(2)).then(admit(2)).then(supply(2))
    .then(controlArrives).then(all {assert(interrupted),assert(left==1),assert(safety),idle})
}
