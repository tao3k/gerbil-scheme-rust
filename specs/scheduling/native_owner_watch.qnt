// Reuse the native Actor/root/capacity model. The observer borrows only the
// existing atomic progress word and a retained kernel thread port. It never
// advances completion or calls Scheme from its sampling thread.
module native_owner_watch {
  import native_mailbox_backpressure as p from "./native_mailbox_backpressure"
  var phase:str
  var borrowed:bool
  var port:bool
  var thread:bool
  var returned:bool
  var reported:bool
  action keep=all {phase'=phase,borrowed'=borrowed,port'=port,thread'=thread,returned'=returned,reported'=reported}
  action init=all {p::init,phase'="idle",borrowed'=false,port'=false,thread'=false,returned'=false,reported'=false}
  action idle=all {p::idle,keep}
  action start=all {phase=="idle",not(returned),p::idle,phase'="sampling",borrowed'=true,
    port'=true,thread'=true,returned'=returned,reported'=reported}
  action sample=all {phase=="sampling" or phase=="stopping",borrowed,port,thread,not(returned),p::idle,keep}
  action stop=all {phase=="sampling",p::idle,phase'="stopping",borrowed'=borrowed,port'=port,
    thread'=thread,returned'=returned,reported'=reported}
  action join=all {phase=="stopping",p::idle,phase'="joined",borrowed'=borrowed,port'=port,
    thread'=false,returned'=returned,reported'=reported}
  action release=all {phase=="joined",p::idle,phase'="released",borrowed'=false,port'=false,
    thread'=false,returned'=returned,reported'=reported}
  action snapshot=all {phase=="released",p::idle,phase'=phase,borrowed'=borrowed,port'=port,
    thread'=thread,returned'=returned,reported'=true}
  action returnCall=all {not(borrowed),not(thread),not(port),p::idle,phase'=phase,
    borrowed'=borrowed,port'=port,thread'=thread,returned'=true,reported'=reported}
  action nextCall=all {returned,not(borrowed),not(thread),not(port),p::live==Set(),p::idle,
    phase'="idle",borrowed'=false,port'=false,thread'=false,returned'=false,reported'=false}
  action protocol=all {not(returned),p::step,keep}
  val safety=and {p::safety,thread implies (borrowed and port and not(returned)),
    returned implies (not(thread) and not(borrowed) and not(port)),
    reported implies (phase=="released" and not(thread))}
  action unsafeEarlyReturn=all {p::idle,phase'=phase,borrowed'=false,port'=port,
    thread'=thread,returned'=true,reported'=reported}
  action unsafeLiveSnapshot=all {p::idle,phase'=phase,borrowed'=borrowed,port'=port,
    thread'=thread,returned'=returned,reported'=true}
  run borrowedProgressOutlivesReaderTest=init.then(start).then(sample).then(stop).then(sample)
    .then(join).then(release).then(snapshot).then(returnCall).then(all {assert(safety),idle})
  run earlyReturnMutationTest=init.then(start).then(unsafeEarlyReturn).then(all {assert(not(safety)),idle})
  run liveReportMutationTest=init.then(start).then(unsafeLiveSnapshot).then(all {assert(not(safety)),idle})
  action step=any {start,sample,stop,join,release,snapshot,returnCall,nextCall,protocol,idle}
}

// OS samples and owner self CPU/GC must refer to the same completion interval.
// Remote cumulative CPU alone is deliberately not a classification premise.
module native_owner_evidence {
  import native_owner_watch as p from "./native_owner_watch"
  var kernel:int
  var self:int
  var bracketed:bool
  var claimed:bool
  action keep=all {kernel'=kernel,self'=self,bracketed'=bracketed,claimed'=claimed}
  action init=all {p::init,kernel'=0,self'=0,bracketed'=false,claimed'=false}
  action idle=all {p::idle,keep}
  action kernelSample(key:int):bool=all {p::idle,kernel'=key,self'=self,bracketed'=bracketed,claimed'=false}
  action selfSample(key:int,valid:bool):bool=all {p::idle,kernel'=kernel,self'=key,bracketed'=valid,claimed'=false}
  action classify=all {kernel>0,kernel==self,bracketed,p::idle,
    kernel'=kernel,self'=self,bracketed'=bracketed,claimed'=true}
  action unsafeClassify=all {p::idle,kernel'=kernel,self'=self,bracketed'=bracketed,claimed'=true}
  val safety=and {p::safety,claimed implies (kernel>0 and kernel==self and bracketed)}
  run matchedWindowTest=init.then(kernelSample(2)).then(selfSample(2,true)).then(classify)
    .then(all {assert(safety),idle})
  run differentGapMutationTest=init.then(kernelSample(2)).then(selfSample(3,true)).then(unsafeClassify)
    .then(all {assert(not(safety)),idle})
  run remoteOnlyMutationTest=init.then(kernelSample(2)).then(unsafeClassify)
    .then(all {assert(not(safety)),idle})
  action step={nondet key=Set(2,3).oneOf() nondet valid=Bool.oneOf()
    any {kernelSample(key),selfSample(key,valid),classify,idle}}
}

// An explicit experimental priority scope is restored before returning to the
// caller, including a failure before the observer thread was even started.
module native_owner_priority {
  import native_owner_watch as p from "./native_owner_watch"
  var elevated:bool
  action keep=elevated'=elevated
  action init=all {p::init,elevated'=false}
  action idle=all {p::idle,keep}
  action enter=all {not(p::returned),p::phase=="idle",p::idle,elevated'=true}
  action restore=all {p::phase=="idle" or p::phase=="released",p::idle,elevated'=false}
  action start=all {p::start,keep}
  action sample=all {p::sample,keep}
  action stop=all {p::stop,keep}
  action join=all {p::join,keep}
  action release=all {p::release,keep}
  action snapshot=all {p::snapshot,keep}
  action returnCall=all {not(elevated),p::returnCall,keep}
  action unsafeReturn=all {p::returnCall,keep}
  val safety=and {p::safety,p::returned implies not(elevated)}
  run priorityLifetimeTest=init.then(enter).then(start).then(sample).then(stop).then(join)
    .then(release).then(snapshot).then(restore).then(returnCall).then(all {assert(safety),idle})
  run failedAdmissionRestoresPriorityTest=init.then(enter).then(restore).then(returnCall)
    .then(all {assert(safety),idle})
  run leakedPriorityMutationTest=init.then(enter).then(unsafeReturn).then(all {assert(not(safety)),idle})
  action step=any {enter,restore,start,sample,stop,join,release,snapshot,returnCall,idle}
}

// The fixed owner installs the optional progress callback only after successful
// reader startup, and detaches it before stop/join releases borrowed resources.
// Completion and the original watchdog store do not depend on this callback.
module native_progress_binding {
  import native_owner_watch as p from "./native_owner_watch"
  var bound:bool
  action keep=bound'=bound
  action init=all {p::init,bound'=false}
  action idle=all {p::idle,keep}
  action start=all {p::start,bound'=true}
  action stop=all {p::stop,bound'=false}
  action sample=all {p::sample,keep}
  action join=all {p::join,keep}
  action release=all {p::release,keep}
  action snapshot=all {p::snapshot,keep}
  action protocol=all {p::protocol,keep}
  action returnCall=all {not(bound),p::returnCall,keep}
  action nextCall=all {not(bound),p::nextCall,keep}
  action failedStart=all {p::phase=="idle",not(bound),p::idle,keep}
  action unsafeRetainedBinding=all {p::stop,keep}
  val safety=and {p::safety,bound implies (p::phase=="sampling" and p::thread and p::borrowed)}
  run observerStartStopDetachesCallbackTest=init.then(start).then(sample).then(stop)
    .then(join).then(release).then(snapshot).then(returnCall).then(all {assert(safety),idle})
  run failedObserverStartKeepsCompletionUninstrumentedTest=init.then(failedStart)
    .then(all {assert(not(bound)),assert(safety),idle}).then(returnCall)
  run ordinaryProgressDoesNotNeedObserverTest=init.then(protocol)
    .then(all {assert(not(bound)),assert(safety),idle})
  run stoppingWithLiveCallbackMutationTest=init.then(start).then(unsafeRetainedBinding)
    .then(all {assert(not(safety)),idle})
  run reusedOwnerRebindsAfterCompleteRetirementTest=init.then(start).then(stop)
    .then(join).then(release).then(returnCall).then(nextCall)
    .then(all {assert(not(bound)),assert(safety),idle})
    .then(start).then(sample).then(stop).then(join).then(release).then(snapshot)
    .then(returnCall).then(all {assert(not(bound)),assert(safety),idle})
  action step=any {start,stop,sample,join,release,snapshot,protocol,returnCall,nextCall,failedStart,idle}
}
