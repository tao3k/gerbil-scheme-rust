// Refine existing owner/root lifecycle. File preparation precedes the coupled
// dyld/SDK transaction; arbitrary application callbacks never own exclusion.
module native_lifecycle {
  import owner_lifecycle(JOBS=Set(1,2,3),ASSIGNED=Map(1->1,2->2,3->1)) as host from "../owner_lifecycle"
  val owners=Set(1,2)
  var phase:int->str
  var sdk:int
  action keep=all {phase'=phase,sdk'=sdk}
  action init=all {host::init,phase'=owners.mapBy(_=>"cold"),sdk'=0}
  action idle=all {host::stutter,keep}
  action prepare(o:int):bool=all {
    phase.get(o)=="cold",host::stutter,phase'=phase.set(o,"prepared"),sdk'=sdk
  }
  action beginSetup(o:int):bool=all {
    phase.get(o)=="prepared",sdk==0,host::stutter,phase'=phase.set(o,"setup"),sdk'=o
  }
  action setupOk(o:int):bool=all {
    sdk==o,phase.get(o)=="setup",host::stutter,phase'=phase.set(o,"ready"),sdk'=0
  }
  action rollback(o:int):bool=all {
    sdk==o,phase.get(o)=="setup",host::stutter,phase'=phase.set(o,"failed"),sdk'=0
  }
  action admit(r:int):bool=all {phase.get(host::ownerOf(r))=="ready",host::admit(r),keep}
  action complete(r:int):bool=all {host::complete(r),keep}
  action release(r:int):bool=all {host::release(r,host::ownerOf(r)),keep}
  action close=all {host::close,keep}
  def canFinalize(o:int):bool=and {
    host::s.phase=="closing",phase.get(o)=="ready",
    host::s.accepted.filter(r=>host::ownerOf(r)==o).subseteq(host::s.completed),
    host::s.roots.filter(r=>host::ownerOf(r)==o)==Set(),
    host::s.serviceRoots.filter(r=>host::ownerOf(r)==o)==Set()
  }
  // An arbitrary finalizer can remain blocked while another owner's preflight
  // and SDK setup make progress. It never owns sdk, including exception paths.
  action beginFinalizer(o:int):bool=all {
    canFinalize(o),host::stutter,phase'=phase.set(o,"finalizer"),sdk'=sdk
  }
  action finalized(o:int):bool=all {
    phase.get(o)=="finalizer",host::stutter,phase'=phase.set(o,"quiesced"),sdk'=sdk
  }
  action beginCleanup(o:int):bool=all {
    phase.get(o)=="quiesced",sdk==0,host::stutter,phase'=phase.set(o,"cleanup"),sdk'=o
  }
  action cleaned(o:int):bool=all {
    sdk==o,phase.get(o)=="cleanup",host::stutter,phase'=phase.set(o,"clean"),sdk'=0
  }
  action unsafeFinalizerLock(o:int):bool=all {
    phase.get(o)=="finalizer",sdk==0,host::stutter,phase'=phase,sdk'=o
  }
  val safety=and {host::safety,owners.union(Set(0)).contains(sdk),
    sdk==0 or Set("setup","cleanup").contains(phase.get(sdk)),
    owners.filter(o=>Set("setup","cleanup").contains(phase.get(o))).size()<=1}
  action step={
    nondet o=owners.oneOf()
    nondet r=Set(1,2).oneOf()
    any {prepare(o),beginSetup(o),setupOk(o),rollback(o),admit(r),complete(r),release(r),
      close,beginFinalizer(o),finalized(o),beginCleanup(o),cleaned(o),idle}
  }
  run preflightWhilePeerSetupTest=init.then(prepare(1)).then(beginSetup(1)).then(prepare(2))
    .then(all {assert(sdk==1),assert(phase.get(2)=="prepared"),assert(safety),idle})
    .then(setupOk(1)).then(beginSetup(2)).then(rollback(2)).then(all {assert(safety),idle})
  run finalizerDoesNotBlockPeerSetupTest=init.then(prepare(1)).then(beginSetup(1)).then(setupOk(1))
    .then(close).then(beginFinalizer(1)).then(prepare(2)).then(beginSetup(2)).then(setupOk(2))
    .then(all {assert(phase.get(1)=="finalizer"),assert(phase.get(2)=="ready"),assert(safety),idle})
    .then(finalized(1)).then(beginCleanup(1)).then(cleaned(1)).then(all {assert(safety),idle})
  run callbackLockMutationTest=init.then(prepare(1)).then(beginSetup(1)).then(setupOk(1))
    .then(close).then(beginFinalizer(1)).then(unsafeFinalizerLock(1))
    .then(all {assert(not(safety)),idle})
}

// The test lease owns process-wide fault injection and FD observations. It is
// separate from owner SDK exclusion and never limits concurrency inside a case.
module native_case_lease {
  import native_lifecycle as life
  val cases=Set(101,102)
  var holder:int
  var watching:Set[int]
  action keep=all {holder'=holder,watching'=watching}
  action init=all {life::init,holder'=0,watching'=Set()}
  action idle=all {life::idle,keep}
  action acquire(c:int):bool=all {cases.contains(c),holder==0,life::sdk==0,life::owners.forall(o=>Set("cold","clean").contains(life::phase.get(o))),life::init,holder'=c,watching'=watching}
  action watch(c:int):bool=all {holder==c,life::idle,holder'=holder,watching'=watching.union(Set(c))}
  action prepare(c:int,o:int):bool=all {holder==c,watching.contains(c),life::prepare(o),keep}
  action beginSetup(c:int,o:int):bool=all {holder==c,watching.contains(c),life::beginSetup(o),keep}
  action setupOk(c:int,o:int):bool=all {holder==c,life::setupOk(o),keep}
  action close(c:int):bool=all {holder==c,life::close,keep}
  action finalizer(c:int,o:int):bool=all {holder==c,life::beginFinalizer(o),keep}
  action finalized(c:int,o:int):bool=all {holder==c,life::finalized(o),keep}
  action cleanup(c:int,o:int):bool=all {holder==c,life::beginCleanup(o),keep}
  action cleaned(c:int,o:int):bool=all {holder==c,life::cleaned(o),keep}
  action releaseCase(c:int):bool=all {
    holder==c,life::sdk==0,life::owners.forall(o=>Set("cold","clean").contains(life::phase.get(o))),
    life::idle,holder'=0,watching'=Set()
  }
  action unsafeWatch(c:int):bool=all {life::idle,holder'=holder,watching'=watching.union(Set(c))}
  val safety=and {life::safety,cases.union(Set(0)).contains(holder),
    watching.subseteq(if (holder==0) Set() else Set(holder))}
  action step={
    nondet c=cases.oneOf()
    nondet o=Set(1,2).oneOf()
    any {acquire(c),watch(c),prepare(c,o),beginSetup(c,o),setupOk(c,o),close(c),
      finalizer(c,o),finalized(c,o),cleanup(c,o),cleaned(c,o),releaseCase(c),idle}
  }
  run caseLeaseKeepsInternalOwnersTest=init.then(acquire(101)).then(watch(101))
    .then(prepare(101,1)).then(prepare(101,2)).then(beginSetup(101,1)).then(setupOk(101,1))
    .then(beginSetup(101,2)).then(setupOk(101,2))
    .then(all {assert(holder==101),assert(life::phase.get(1)=="ready"),assert(life::phase.get(2)=="ready"),assert(safety),idle})
  run caseReuseAfterCleanRetirementTest=init.then(acquire(101)).then(watch(101))
    .then(prepare(101,1)).then(beginSetup(101,1)).then(setupOk(101,1)).then(close(101))
    .then(finalizer(101,1)).then(finalized(101,1)).then(cleanup(101,1)).then(cleaned(101,1)).then(releaseCase(101))
    .then(acquire(102)).then(watch(102)).then(prepare(102,1)).then(beginSetup(102,1)).then(setupOk(102,1))
    .then(all {assert(holder==102),assert(safety),idle})
  run watchdogBeforeLeaseMutationTest=init.then(unsafeWatch(102))
    .then(all {assert(not(safety)),idle})
}
