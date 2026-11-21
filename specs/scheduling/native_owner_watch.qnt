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

// Coverage refinement only: the one existing owner/frame model remains the
// authority. A wrapper envelope is not a kernel state trace or a second timer.
module owner_wait_coverage {
  import native_owner_availability as o from "./native_source_wait"
  action init=o::init
  action idle=o::idle
  val safety=o::safety
  type Span = { scope:int, begin:int, end:int }
  type CpuCut = { scope:int, begin:int, end:int, cpu:int }
  pure def valid(s:Span):bool=s.scope>0 and 0<=s.begin and s.begin<=s.end
  pure def overlap(g:Span,w:Span):int=
    if (not(valid(g) and valid(w) and g.scope==w.scope)) -1
    else {
      val left=if (g.begin>w.begin) g.begin else w.begin
      val right=if (g.end<w.end) g.end else w.end
      if (right>left) right-left else 0
    }
  pure def enclosed(g:Span,b:CpuCut,a:CpuCut):bool=and {
    valid(g),g.begin<g.end,g.scope==b.scope,g.scope==a.scope,
    0<=b.begin,b.begin<=b.end,b.end<=g.begin,
    g.end<=a.begin,a.begin<=a.end,0<=b.cpu,b.cpu<=a.cpu}
  pure def nonCpuLower(g:Span,b:CpuCut,a:CpuCut):int=
    if (not(enclosed(g,b,a))) -1
    else if (g.end-g.begin>a.cpu-b.cpu) g.end-g.begin-(a.cpu-b.cpu) else 0
  pure def selectWrapperRecords(backend:str,recording:bool):bool=recording and backend=="select"
  // Even all select calls are a subset of possible native waits. Closed scope,
  // zero overflow and a verified call site are necessary, not all-wait coverage.
  pure def canExcludeSelect(g:Span,closed:bool,recording:bool,boundToWrapper:bool,
      missingEvents:int,matchingCalls:int):bool=and {
    valid(g),closed,recording,boundToWrapper,missingEvents==0,matchingCalls==0}
  pure val gap:Span={scope:1,begin:3,end:103}
  pure val before:CpuCut={scope:1,begin:1,end:2,cpu:10}
  pure val after:CpuCut={scope:1,begin:104,end:105,cpu:12}
  run pendingWorkDoesNotProveNativeBlockingTest=o::init.then(o::arrival(2))
    .then(all {assert(o::canServe),assert(not(o::nativeBlocked)),assert(o::safety),o::idle})
  run blockingOutsideSelectedBackendHasNoSelectRecordTest=o::init.then(o::block)
    .then(all {assert(o::nativeBlocked),assert(not(o::canServe)),
      assert(not(selectWrapperRecords("nanosleep",true))),
      assert(not(selectWrapperRecords("poll",true))),assert(o::safety),o::idle})
  run disabledObserverCannotEstablishNoSelectTest=o::init.then(all {
    assert(not(selectWrapperRecords("select",false))),
    assert(not(canExcludeSelect(gap,true,false,true,0,0))),assert(o::safety),o::idle})
  run overflowOrOpenScopePreventsNegativeCoverageClaimTest=o::init.then(all {
    assert(not(canExcludeSelect(gap,true,true,true,1,0))),
    assert(not(canExcludeSelect(gap,false,true,true,0,0))),
    assert(not(canExcludeSelect(gap,true,true,false,0,0))),assert(o::safety),o::idle})
  run callSpanMustMatchScopeAndBeClippedTest=o::init.then(all {
    assert(overlap(gap,{scope:1,begin:0,end:200})==100),
    assert(overlap(gap,{scope:1,begin:20,end:40})==20),
    assert(overlap(gap,{scope:1,begin:104,end:105})==0),
    assert(overlap(gap,{scope:2,begin:20,end:40})== -1),assert(o::safety),o::idle})
  run cpuBracketsCannotBeBorrowedFromAnotherGapTest=o::init.then(all {
    assert(enclosed(gap,before,after)),assert(nonCpuLower(gap,before,after)==98),
    assert(not(enclosed(gap,before.with("end",4),after))),
    assert(nonCpuLower(gap,before,after.with("scope",2))== -1),assert(o::safety),o::idle})
  run enclosingCpuMayExceedGapWallTest=o::init.then(all {
    assert(enclosed(gap,before,after.with("cpu",210))),
    assert(nonCpuLower(gap,before,after.with("cpu",210))==0),assert(o::safety),o::idle})
  run callEnvelopeDoesNotIdentifyKernelBlockedTimeTest=o::init.then(all {
    // Both decompositions produce the same 100-unit call envelope. Either
    // kernel blocking or runnable OS wait can dominate its non-CPU remainder.
    assert(2+0+98==2+98+0),assert(not(0==98)),
    assert(overlap(gap,{scope:1,begin:3,end:103})==100),assert(o::safety),o::idle})
  run completeSelectCoverageStillDoesNotCoverOtherWaitsTest=o::init.then(all {
    assert(canExcludeSelect(gap,true,true,true,0,0)),
    assert(not(selectWrapperRecords("other-native-wait",true))),assert(o::safety),o::idle})
  run wakeAndOwnerExecutionRemainDifferentTransitionsTest=o::init.then(o::preempt)
    .then(o::arrival(2)).then(all {assert(not(o::canServe)),assert(not(o::nativeBlocked)),
      assert(o::safety),o::idle}).then(o::scheduled)
    .then(all {assert(o::canServe),assert(o::safety),o::idle})
}

// Owner-local association lifecycle; time values and original frame custody
// remain in the existing coverage/availability models. No new runtime queue.
module owner_wait_scope_lifecycle {
  import owner_wait_coverage as w from "./native_owner_watch"
  var scopePhase:str
  var generation:int
  var sequence:int
  var callPending:bool
  var primed:bool
  var invalid:bool
  var progressEnd:int
  var bestBegin:int
  var bestEnd:int
  var bestCalls:int
  var currentCalls:int
  var bestPresent:bool
  val canOpen=scopePhase!="active" and not(callPending)
  def matches(g:int,q:int):bool=g==generation and q==sequence
  action keep=all {scopePhase'=scopePhase,generation'=generation,sequence'=sequence,
    callPending'=callPending,primed'=primed,invalid'=invalid,progressEnd'=progressEnd,
    bestBegin'=bestBegin,bestEnd'=bestEnd,bestCalls'=bestCalls,
    currentCalls'=currentCalls,bestPresent'=bestPresent}
  action init=all {w::init,scopePhase'="idle",generation'=0,sequence'=0,
    callPending'=false,primed'=false,invalid'=false,progressEnd'=0,
    bestBegin'=0,bestEnd'=0,bestCalls'=0,currentCalls'=0,bestPresent'=false}
  action idle=all {w::idle,keep}
  action open=all {canOpen,w::idle,scopePhase'="active",generation'=generation+1,
    sequence'=0,callPending'=false,primed'=false,invalid'=false,progressEnd'=0,
    bestBegin'=0,bestEnd'=0,bestCalls'=0,currentCalls'=0,bestPresent'=false}
  action prime(now:int):bool=all {scopePhase=="active",not(callPending),not(primed),
    now>=0,w::idle,scopePhase'=scopePhase,generation'=generation,sequence'=sequence,
    callPending'=callPending,primed'=true,invalid'=invalid,progressEnd'=now,
    bestBegin'=bestBegin,bestEnd'=bestEnd,bestCalls'=bestCalls,
    currentCalls'=0,bestPresent'=bestPresent}
  action enter=all {scopePhase=="active",not(callPending),w::idle,
    scopePhase'=scopePhase,generation'=generation,sequence'=sequence+1,callPending'=true,
    primed'=primed,invalid'=invalid,progressEnd'=progressEnd,bestBegin'=bestBegin,
    bestEnd'=bestEnd,bestCalls'=bestCalls,currentCalls'=currentCalls,bestPresent'=bestPresent}
  action leave(g:int,q:int):bool=all {callPending,matches(g,q),scopePhase=="active",w::idle,
    scopePhase'=scopePhase,generation'=generation,sequence'=sequence,callPending'=false,
    primed'=primed,invalid'=invalid,progressEnd'=progressEnd,bestBegin'=bestBegin,
    bestEnd'=bestEnd,bestCalls'=bestCalls,currentCalls'=if (primed) currentCalls+1 else currentCalls,
    bestPresent'=bestPresent}
  action rejectedTicket(g:int,q:int):bool=all {not(callPending and matches(g,q)),w::idle,keep}
  action progress(now:int):bool=all {scopePhase=="active",primed,not(callPending),now>progressEnd,w::idle,
    scopePhase'=scopePhase,generation'=generation,sequence'=sequence,callPending'=callPending,
    primed'=primed,invalid'=invalid,progressEnd'=now,currentCalls'=0,
    bestBegin'=if (not(bestPresent) or now-progressEnd>bestEnd-bestBegin) progressEnd else bestBegin,
    bestEnd'=if (not(bestPresent) or now-progressEnd>bestEnd-bestBegin) now else bestEnd,
    bestCalls'=if (not(bestPresent) or now-progressEnd>bestEnd-bestBegin) currentCalls else bestCalls,
    bestPresent'=true}
  action malformed=all {scopePhase=="active",w::idle,scopePhase'=scopePhase,
    generation'=generation,sequence'=sequence,callPending'=callPending,primed'=primed,
    invalid'=true,progressEnd'=progressEnd,bestBegin'=bestBegin,bestEnd'=bestEnd,
    bestCalls'=bestCalls,currentCalls'=currentCalls,bestPresent'=bestPresent}
  action close(aborting:bool):bool=all {scopePhase=="active",aborting or not(callPending),w::idle,
    scopePhase'=if (aborting) "aborted" else "closed",generation'=generation,sequence'=sequence,
    callPending'=callPending,primed'=primed,invalid'=invalid,progressEnd'=progressEnd,bestBegin'=bestBegin,
    bestEnd'=bestEnd,bestCalls'=bestCalls,currentCalls'=currentCalls,bestPresent'=bestPresent}
  action acknowledgeAborted(g:int,q:int):bool=all {scopePhase=="aborted",callPending,matches(g,q),w::idle,
    scopePhase'=scopePhase,generation'=generation,sequence'=sequence,callPending'=false,
    primed'=primed,invalid'=invalid,progressEnd'=progressEnd,bestBegin'=bestBegin,
    bestEnd'=bestEnd,bestCalls'=bestCalls,currentCalls'=currentCalls,bestPresent'=bestPresent}
  val canSnapshot=Set("closed","aborted").contains(scopePhase) and not(callPending)
  val evidenceValid=scopePhase=="closed" and canSnapshot and bestPresent and not(invalid)
  val safety=and {w::safety,scopePhase=="closed" implies not(callPending),
    evidenceValid implies (primed and bestEnd>bestBegin and bestCalls>=0)}
  run abortMustRetireItsPendingCallBeforeReuseTest=init.then(open).then(prime(10)).then(enter)
    .then(close(true)).then(all {assert(not(canOpen)),assert(not(canSnapshot)),assert(safety),idle})
    .then(acknowledgeAborted(1,1)).then(all {assert(canSnapshot),assert(not(evidenceValid)),assert(safety),idle})
    .then(open).then(all {assert(generation==2),assert(safety),idle})
  run oldExitCannotCompleteNewGenerationCallTest=init.then(open).then(enter).then(close(true))
    .then(acknowledgeAborted(1,1)).then(open).then(prime(10)).then(enter)
    .then(rejectedTicket(1,1)).then(all {assert(callPending),assert(currentCalls==0),assert(safety),idle})
    .then(leave(2,1)).then(progress(20)).then(close(false))
    .then(all {assert(evidenceValid),assert(bestCalls==1),assert(safety),idle})
  run sameGenerationOldSequenceCannotClaimNewCallTest=init.then(open).then(prime(10))
    .then(enter).then(leave(1,1)).then(enter).then(rejectedTicket(1,1))
    .then(all {assert(callPending),assert(currentCalls==1),assert(safety),idle})
  run initializationDoesNotBecomeACompleteProgressGapTest=init.then(open).then(enter)
    .then(leave(1,1)).then(prime(10)).then(progress(20)).then(close(false))
    .then(all {assert(evidenceValid),assert(bestCalls==0),assert(safety),idle})
  run lateInvalidEventCannotPromoteEarlierGoodEvidenceTest=init.then(open).then(prime(10))
    .then(enter).then(leave(1,1)).then(progress(20)).then(malformed).then(close(false))
    .then(all {assert(canSnapshot),assert(not(evidenceValid)),assert(bestCalls==1),assert(safety),idle})
  run unfinishedTailCannotBeAssignedToEarlierGapTest=init.then(open).then(prime(10))
    .then(progress(30)).then(enter).then(leave(1,1)).then(close(false))
    .then(all {assert(bestBegin==10),assert(bestEnd==30),assert(bestCalls==0),
      assert(currentCalls==1),assert(safety),idle})
  run laterShorterGapCannotEraseTheLargestTest=init.then(open).then(prime(10))
    .then(enter).then(leave(1,1)).then(progress(30)).then(progress(35)).then(close(false))
    .then(all {assert(bestBegin==10),assert(bestEnd==30),assert(bestCalls==1),assert(safety),idle})
  run emptyScopeDoesNotFabricateZeroWaitEvidenceTest=init.then(open).then(close(false))
    .then(all {assert(canSnapshot),assert(not(evidenceValid)),assert(not(bestPresent)),assert(safety),idle})
  run tlsAddressReuseRequiresImageWideActivationIdentityTest=init.then(all {
    // A retired thread's TLS address and local sequence may both recur. An
    // image-wide activation epoch is the distinguishing value, not the address.
    assert((7,1,1)==(7,1,1)),assert(not((7,1,1)==(7,2,1))),assert(safety),idle})
  run nativeCancellationAcknowledgesExitWithoutSuccessTest=init.then(open).then(prime(10))
    .then(enter).then(close(true)).then(acknowledgeAborted(1,1))
    .then(all {assert(canSnapshot),assert(not(evidenceValid)),assert(not(callPending)),
      assert(currentCalls==0),assert(not(bestPresent)),assert(safety),idle})
  action step={nondet g=Set(generation,generation-1).oneOf()
    nondet q=Set(sequence,sequence-1).oneOf()
    nondet increment=1.to(3).oneOf()
    nondet aborting=Bool.oneOf()
    any {open,prime(progressEnd+increment),enter,leave(g,q),rejectedTicket(g,q),
      progress(progressEnd+increment),malformed,close(aborting),acknowledgeAborted(g,q),idle}}
}

// The new diagnostic and legacy row recorder share a wrapper, not ownership.
// Mode exclusion applies in both orders and until an aborted call has exited.
module owner_wait_binding_modes {
  import owner_wait_scope_lifecycle as p from "./native_owner_watch"
  var rowRecording:bool
  val canRecord=p::scopePhase!="active" and not(p::callPending)
  action init=all {p::init,rowRecording'=false}
  action idle=all {p::idle,rowRecording'=rowRecording}
  action recordRows=all {canRecord,p::idle,rowRecording'=true}
  action stopRows=all {p::idle,rowRecording'=false}
  action open=all {not(rowRecording),p::open,rowRecording'=rowRecording}
  action prime(now:int):bool=all {p::prime(now),rowRecording'=rowRecording}
  action enter=all {p::enter,rowRecording'=rowRecording}
  action leave(g:int,q:int):bool=all {p::leave(g,q),rowRecording'=rowRecording}
  action progress(now:int):bool=all {p::progress(now),rowRecording'=rowRecording}
  action close(aborting:bool):bool=all {p::close(aborting),rowRecording'=rowRecording}
  action acknowledgeAborted(g:int,q:int):bool=all {p::acknowledgeAborted(g,q),rowRecording'=rowRecording}
  action unsafeRecordRows=all {p::idle,rowRecording'=true}
  val safety=p::safety and not(rowRecording and (p::scopePhase=="active" or p::callPending))
  run oldRecorderBlocksNewAdmissionTest=init.then(recordRows)
    .then(all {assert(rowRecording),assert(safety),idle}).then(stopRows).then(open)
    .then(all {assert(not(rowRecording)),assert(not(canRecord)),assert(safety),idle})
  run activeAssociationBlocksOldRecordingTest=init.then(open)
    .then(all {assert(not(canRecord)),assert(safety),idle}).then(close(false)).then(recordRows)
    .then(all {assert(rowRecording),assert(safety),idle})
  run abortedPendingCallStillOwnsItsRecordingBoundaryTest=init.then(open).then(prime(10))
    .then(enter).then(close(true)).then(all {assert(not(canRecord)),assert(safety),idle})
    .then(acknowledgeAborted(1,1)).then(recordRows).then(all {assert(rowRecording),assert(safety),idle})
  run allowingModeOverlapIsAnExplicitCounterexampleTest=init.then(open).then(unsafeRecordRows)
    .then(all {assert(not(safety)),idle})
  action step={nondet increment=1.to(3).oneOf()
    nondet aborting=Bool.oneOf()
    any {recordRows,stopRows,open,prime(p::progressEnd+increment),enter,
      leave(p::generation,p::sequence),progress(p::progressEnd+increment),close(aborting),
      acknowledgeAborted(p::generation,p::sequence),idle}}
}

// Explicit Scheme callback generation: stale observation is inert while the
// original verified heartbeat publication remains part of the actual callback.
module owner_wait_scheme_callback {
  import owner_wait_scope_lifecycle as p from "./native_owner_watch"
  var heartbeats:int
  action init=all {p::init,heartbeats'=0}
  action idle=all {p::idle,heartbeats'=heartbeats}
  action open=all {p::open,heartbeats'=heartbeats}
  action epoch(now:int):bool=all {p::prime(now),heartbeats'=heartbeats}
  action close(aborting:bool):bool=all {p::close(aborting),heartbeats'=heartbeats}
  action publish(g:int,now:int):bool=all {
    if (g==p::generation and p::scopePhase=="active")
      (if (p::primed) p::progress(now) else p::prime(now)) else p::idle,
    heartbeats'=heartbeats+1}
  run staleContinuationPublishesWithoutTouchingNewObservationTest=init.then(open).then(epoch(10))
    .then(publish(1,20)).then(close(false)).then(open).then(publish(1,30))
    .then(all {assert(heartbeats==2),assert(p::generation==2),assert(not(p::primed)),
      assert(not(p::bestPresent)),assert(p::safety),idle})
  run sourceEpochMakesFirstCompletionAnObservedGapTest=init.then(open).then(epoch(10))
    .then(publish(1,30)).then(close(false))
    .then(all {assert(heartbeats==1),assert(p::bestBegin==10),assert(p::bestEnd==30),
      assert(p::evidenceValid),assert(p::safety),idle})
  run failedScopeRemainsFailedAfterStaleReentryTest=init.then(open).then(epoch(10))
    .then(close(true)).then(publish(1,20))
    .then(all {assert(heartbeats==1),assert(not(p::evidenceValid)),assert(p::safety),idle})
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
