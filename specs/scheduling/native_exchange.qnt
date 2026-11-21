// Typed transfer refines existing Actor supply/completion and joint client lease.
// Rust input borrows end only after the synchronous owner copy. Job identities
// are scalars; a ready snapshot owns slots until copy/rooted projection finishes.
module native_exchange {
  import actor_completion(HARVEST=2) as a from "./actor_completion"
  var borrowed:Set[int]
  var copied:Set[int]
  var invalid:Set[int]
  var scalar:Set[int]
  action keep=all {borrowed'=borrowed,copied'=copied,invalid'=invalid,scalar'=scalar}
  action init=all {a::init,borrowed'=Set(),copied'=Set(),invalid'=Set(),scalar'=Set()}
  action idle=all {a::idle,keep}
  action feed(r:int):bool=all {a::feed(r),keep}
  action reserve(r:int):bool=all {a::reserve(r),borrowed'=borrowed.union(Set(r)),copied'=copied,invalid'=invalid,scalar'=scalar}
  action nativeCopy(r:int):bool=all {
    borrowed.contains(r),not(copied.contains(r)),a::idle,
    borrowed'=borrowed,copied'=copied.union(Set(r)),invalid'=invalid,scalar'=scalar
  }
  action admit(r:int):bool=all {
    copied.contains(r),borrowed.contains(r),a::admit(r),
    borrowed'=borrowed.exclude(Set(r)),copied'=copied,invalid'=invalid,scalar'=scalar.union(Set(r))
  }
  action abort(r:int):bool=all {
    borrowed.contains(r),not(scalar.contains(r)),a::reject(r),
    borrowed'=borrowed.exclude(Set(r)),copied'=copied.exclude(Set(r)),invalid'=invalid,scalar'=scalar
  }
  action supply(r:int):bool=all {scalar.contains(r),a::supply(r),keep}
  action advance=all {a::advance,keep}
  action start(r:int):bool=all {a::start(r),keep}
  action finish(r:int):bool=all {a::finish(r),keep}
  action claim=all {a::claim,keep}
  action invalidCopy(r:int):bool=all {
    a::snapshot.contains(r),not(a::projectedIds.contains(r)),a::idle,
    borrowed'=borrowed,copied'=copied,invalid'=invalid.union(Set(r)),scalar'=scalar
  }
  // A failed bounds/type check retains the slot; rooted discard then retires it.
  action project(r:int):bool=all {a::project(r),keep}
  action retire(r:int):bool=all {a::retire(r),keep}
  action release(r:int):bool=all {a::release(r),keep}
  action publish(r:int):bool=all {a::publish(r),keep}
  action consume(r:int):bool=all {a::consume(r),keep}
  action abandon(r:int):bool=all {a::abandon(r),keep}
  action close=all {borrowed==Set(),a::close,keep}
  val safety=and {a::safety,scalar.subseteq(copied),borrowed.intersect(scalar)==Set()}
  action unsafeAcknowledge(r:int):bool=all {a::idle,borrowed'=Set(),copied'=copied,invalid'=invalid,scalar'=scalar.union(Set(r))}
  action step={
    nondet r=Set(2,3).oneOf()
    any {feed(r),reserve(r),nativeCopy(r),admit(r),abort(r),supply(r),advance,start(r),finish(r),claim,
      invalidCopy(r),project(r),retire(r),release(r),publish(r),consume(r),abandon(r),close,idle}
  }
  run copyBeforeAcknowledgeTest=init.then(feed(2)).then(reserve(2)).then(nativeCopy(2)).then(admit(2))
    .then(supply(2)).then(advance).then(start(2)).then(finish(2)).then(claim)
    .then(invalidCopy(2)).then(all {assert(a::snapshot==Set(2)),assert(safety),idle})
    .then(abandon(2)).then(close).then(project(2)).then(retire(2)).then(release(2))
    .then(all {assert(a::snapshot==Set()),assert(safety),idle})
  run nativeCopyExceptionBeforeAdmissionTest=init.then(feed(2)).then(reserve(2)).then(nativeCopy(2))
    .then(abort(2)).then(close)
    .then(all {assert(borrowed==Set()),assert(copied==Set()),assert(scalar==Set()),assert(safety),idle})
  run earlyAcknowledgeMutationTest=init.then(feed(2)).then(reserve(2)).then(unsafeAcknowledge(2))
    .then(all {assert(not(safety)),idle})
}
