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
  // Owner CPU charged outside an inner span cannot exceed the outer slack.
  pure def innerCpuFloor(outerScope:int,innerScope:int,outerWall:int,
      innerWall:int,enclosingCpuFloor:int):int=
    if(outerScope<=0 or outerScope!=innerScope or innerWall<0 or innerWall>outerWall or
       enclosingCpuFloor<0 or enclosingCpuFloor>outerWall) -1
    else {
      val outside=outerWall-innerWall
      if(enclosingCpuFloor>outside) enclosingCpuFloor-outside else 0
    }
  run AnchoredMillionReceiptRequiresOwnerCpuAfterTheOfferWindowTest={
    val innerFloor=innerCpuFloor(1,1,1658919875,1638521208,1061999250)
    val lastTick=999999000
    assert(innerFloor==1041600583 and innerFloor-lastTick==41601583 and
      innerFloor>1000000000)
  }
  run BracketedMillionPrefixNeedsBothCpuAndAvailabilityEvidenceTest={
    val innerWall=481782417
    val outerWall=512705875
    val cpuFloor=innerCpuFloor(1,1,outerWall,innerWall,392436874)
    val cpuCeil=392438875
    val finished=310523
    val period=1000
    val limit=128
    // Compressing only non-CPU time preserves this receipt's executed
    // work and retirements. It makes no prediction for a changed run.
    val dueWithOnlyCpu=1+cpuFloor/period
    val latestWithAllowedDebt=(finished+limit)*period-1
    assert(cpuFloor==361513416 and innerWall-cpuCeil==89343542 and
      1+innerWall/period-finished==171260 and dueWithOnlyCpu-finished==50991 and
      cpuFloor-latestWithAllowedDebt==50862417 and dueWithOnlyCpu-finished>limit)
  }
  run FirstBreachWindowWasMostlyOwnerCpuTest={
    val elapsed=173625
    val ownerFloor=innerCpuFloor(1,1,177583,elapsed,176417-2000)
    val retired=42
    val period=1000
    assert(ownerFloor==170459 and elapsed-ownerFloor==3166 and
      2*ownerFloor>elapsed and 1+elapsed/period-retired==132 and
      1+ownerFloor/period-retired==129)
  }
  run CompleteMainNeedsBothCpuAndExecutionOpportunityTest={
    val inner=1185837583
    val floor=innerCpuFloor(1,1,1185843166,inner,1055562459)
    val outside=inner-1055564459
    assert(floor==1055556876 and outside==130273124 and
      floor-(20577999-3083000+1000)==1038060877 and
      outside-(23062944-3072261+1000)==110281441)
  }
  run GcJobCallbackCannotLocateTheCollectorPauseTest={
    // Notification follows collection/finalization, not an entry/exit probe.
    // Both histories expose the same cumulative GC wall and callback stamp.
    val early={gcBegin:10,gcEnd:20,callback:40,gcWall:10}
    val late={gcBegin:30,gcEnd:40,callback:40,gcWall:10}
    assert(early.gcEnd<=early.callback and late.gcEnd<=late.callback and
      early.callback==late.callback and early.gcWall==late.gcWall and
      early.gcBegin!=late.gcBegin and early.gcEnd!=late.gcEnd)
  }
  run HistoricalGcJobsCanCoalesceCollectionsTest={
    // Hash-verified historical SDK fixture, not a current business trace.
    val records=List({collections:1,before:0,after:1,again:1},
      {collections:2,before:0,after:1,again:1},
      {collections:4,before:0,after:1,again:1})
    assert(records.foldl(true,(ok,r)=>ok and r.before==0 and r.after==1 and
      r.again==1) and records.nth(2).collections>records.nth(2).after)
  }
  run FinalizerWaitingNeedNotIncreaseCollectorWallCountersTest={
    // Installed SDK probe: a deliberate finalizer sleep, not frozen workload.
    val measured={wallNs:3010988,gcCountDelta:0,gcWallDeltaNs:0,
      jobAfterFinalizer:true}
    assert(measured.wallNs>0 and measured.gcCountDelta==0 and
      measured.gcWallDeltaNs==0 and measured.jobAfterFinalizer)
  }
  run ProcessCpuDuringGcCannotIdentifyExclusiveCollectorCpuTest={
    // Frozen process_times calls getrusage(RUSAGE_SELF), not per-owner CPU.
    // Abstract decompositions of one interval; not measured phase prices.
    val first={collector:5,otherThreads:3}
    val second={collector:7,otherThreads:1}
    val observed=8
    assert(first.collector+first.otherThreads==observed and
      second.collector+second.otherThreads==observed and
      first.collector!=second.collector and observed>first.collector)
  }
  run StableProcessGcTotalsDoNotSignTheOwnerNonGcDeltaTest={
    // Rounded recorded original totals. Even ideal counters do not fix
    // the owner share of process CPU sampled over GC intervals.
    val ownerA=1013156667
    val ownerB=1019203000
    val processGcA=17427000
    val processGcB=17441000
    val first={ownerGcA:1000000,ownerGcB:16000000}
    val second={ownerGcA:16000000,ownerGcB:1000000}
    val firstNonGcDelta=(ownerB-first.ownerGcB)-(ownerA-first.ownerGcA)
    val secondNonGcDelta=(ownerB-second.ownerGcB)-(ownerA-second.ownerGcA)
    assert(Set(first,second).forall(p=>p.ownerGcA<=processGcA and p.ownerGcB<=processGcB) and
      processGcB-processGcA==14000 and firstNonGcDelta<0 and secondNonGcDelta>0)
  }
  run AggregateFixtureNonCpuDoesNotLocateOneLongPauseTest={
    // One C counter-correctness fixture, not a Gerbil business receipt.
    val coreWall=67786375
    val enclosingSelfCpu=57113917
    val aggregate=coreWall-enclosingSelfCpu
    val oneEpisode=aggregate
    val smallEpisode=aggregate/1000
    val remainder=aggregate%1000
    assert(aggregate==10672458 and oneEpisode>128000 and
      smallEpisode<128000 and 1000*smallEpisode+remainder==aggregate)
  }
  run NominalObserverDeadlineDoesNotCertifyActualCoverageTest={
    // First query is no earlier than the scheduling anchor. Its offset
    // makes these lower bounds on lateness, not exact wake latencies.
    val first=128719865196666
    val fourth=128719902124875
    val fifth=128719932985791
    assert(fourth-first-32000000==4928209 and
      fifth-first-64000000==3789125 and fourth-first>32000000)
  }
  run GcJobLogicalCloseDoesNotRemoveTheRegisteredCallbackTest={
    // The installed-SDK fixture observed eight active calls and one more
    // after logical close. It proves neither arbitrary interrupt coverage
    // nor callback/thread equivalence in a populated business VM.
    val before={registered:true,active:true,kept:8,ignored:0}
    val closed={...before,active:false}
    val after={...closed,ignored:closed.ignored+1}
    assert(after.registered and not(after.active) and after.kept==8 and
      after.ignored==1 and after.registered==before.registered)
  }
  // Conditional single-processor collector budget. Never manufacture a
  // historical clock-error bound or count notification jobs as collections.
  pure def collectorPauseDebtFloor(whole:int,offer:int,gc:int,count:int,
      period:int,error:int,returned:int,withinCall:bool,clockBounded:bool,
      stoppedOwner:bool):int=
    if(not(withinCall and clockBounded and stoppedOwner) or whole<offer or
       offer<=0 or gc<0 or count<=0 or period<=0 or error<0 or returned<0) -1
    else {
      val outside=whole-offer
      val inside=if(gc-error>outside) gc-error-outside else 0
      val pause=(inside+count-1)/count
      // Open interval endpoints may coincide with an arrival. Allow one
      // already-in-flight return at the boundary when requested by the caller.
      val ticks=if(pause>0) (pause+period-1)/period-1 else 0
      if(ticks>returned) ticks-returned else 0
    }
  run CollectorOfferOverlapCanConstrainDebtWithoutPerGcTimestampsTest={
    // Published 4.513 ms cohort, with a HYPOTHETICAL proven 1 ms total
    // clock-overstatement allowance. No such historical proof is asserted.
    val debt=collectorPauseDebtFloor(1009152667,999999000,17893791,39,
      1000,1000000,1,true,true,true)
    assert(debt==197 and debt>128)
  }
  run CollectorDebtBoundRequiresEnvelopeAndClockEvidenceTest=assert(
    collectorPauseDebtFloor(1009152667,999999000,17893791,39,1000,1000000,1,false,true,true)== -1 and
    collectorPauseDebtFloor(1009152667,999999000,17893791,39,1000,1000000,1,true,false,true)== -1 and
    collectorPauseDebtFloor(1009152667,999999000,17893791,39,1000,1000000,1,true,true,false)== -1)
  run SplitSnapshotTimeCannotAssumeWithinCallCoverageTest={
    // A hypothetical before-snapshot collection affects time but precedes
    // the count baseline. The archival totals do not prove this contribution.
    val rawGc=17893791
    val beforeGc=4000000
    val unverified=collectorPauseDebtFloor(1009152667,999999000,rawGc,39,
      1000,0,1,false,true,true)
    val assumed=collectorPauseDebtFloor(1009152667,999999000,rawGc,39,
      1000,0,1,true,true,true)
    val bounded=collectorPauseDebtFloor(1009152667,999999000,rawGc-beforeGc,39,
      1000,0,1,true,true,true)
    assert(unverified== -1 and assumed==223 and bounded==120)
  }
  run CollectorDebtInferenceIsSensitiveToClockErrorTest={
    // Hash-verified 3.927 ms raw record. This is sensitivity arithmetic,
    // not an actual timebase-error measurement or attribution of its peak.
    val atBoundary=collectorPauseDebtFloor(1010852000,999999000,16150474,40,
      1000,97474,1,true,true,true)
    val justInside=collectorPauseDebtFloor(1010852000,999999000,16150474,40,
      1000,97473,1,true,true,true)
    assert(atBoundary==128 and justInside==129)
  }
  run EqualCollectorTotalsDoNotFixPublicationTimelineTest={
    // Inference counterexample only. Ten ordered releases, eight units of
    // work each, and one ten-unit pause. This is not an op62 simulator.
    val releases=List(0,10,20,30,40,50,60,70,80,90)
    val initial={now:0,ordinal:0,lastPublication:0,work:0,pause:0}
    val early=releases.foldl(initial,(state,release)=>{
      val begin=if(state.now>release) state.now else release
      val gc=if(state.ordinal==0) 10 else 0
      {now:begin+gc+8,ordinal:state.ordinal+1,lastPublication:begin+gc,
        work:state.work+8,pause:state.pause+gc}
    })
    val late=releases.foldl(initial,(state,release)=>{
      val begin=if(state.now>release) state.now else release
      val gc=if(state.ordinal==9) 10 else 0
      {now:begin+gc+8,ordinal:state.ordinal+1,lastPublication:begin+gc,
        work:state.work+8,pause:state.pause+gc}
    })
    assert(early.work==80 and late.work==80 and early.pause==10 and late.pause==10 and
      early.lastPublication==90 and late.lastPublication==100 and
      early.now==98 and late.now==108 and
      early.now-early.lastPublication==late.now-late.lastPublication)
  }
  run MissingOrForeignWallEnvelopeCannotGiveACompletionCpuFloorTest=assert(
    innerCpuFloor(1,2,1658919875,1638521208,1061999250)== -1 and
    innerCpuFloor(1,1,100,101,80)== -1 and
    innerCpuFloor(1,1,100,99,101)== -1 and
    innerCpuFloor(1,1,100,50,40)==0)
  run FrozenControlHasTimeOutsideBothOwnerCpuAndCollectorEnvelopeTest={
    // CPU and GC may overlap. Subtracting both upper bounds bounds their
    // union conservatively; it does not decompose the gap by cause.
    val inner={scope:7,begin:0,end:1121129458}
    val before={scope:7,begin:0,end:0,cpu:0}
    val after={scope:7,begin:1131643750,end:1131643750,cpu:1070708083}
    val gcWallUpper=17758847
    assert(enclosed(inner,before,after) and
      nonCpuLower(inner,before,after)==50421375 and
      nonCpuLower(inner,before,after)-gcWallUpper==32662528)
  }
  run SevereFrozenReplayCannotBeExplainedByCpuOrCollectorEnvelopeTest={
    // Same-run bounds, rounded up with the receipt audit's 1 us margin.
    val inner={scope:8,begin:0,end:6200527375}
    val before={scope:8,begin:0,end:0,cpu:0}
    val after={scope:8,begin:6200527375,end:6200527375,cpu:1379825084}
    val gcWallUpper=96688794
    val uncharged=nonCpuLower(inner,before,after)-gcWallUpper
    assert(uncharged==4724013497 and uncharged>3000000000 and
      nonCpuLower(inner,before,{...after,scope:7})== -1)
  }
  run OraclePrefixTimeoutsCannotPriceOwnerWorkTest={
    // Archive CPU and GC are upper bounds with 1 us of margin each.
    val controlOutside=10938620250-1262637418-234370994
    val changedOutside=10799639875-1137100125-204175996
    assert(controlOutside==9441611838 and changedOutside==9458363754 and
      2*controlOutside>10938620250 and 2*changedOutside>10799639875)
  }
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
  run ShorterProcessResidualDoesNotMakeTheSameFrozenMainFasterTest={
    // Retained frozen reader/image and identical arguments. Parent timestamps
    // are millisecond wall times. The inner spans use the original VM clock.
    val fast={child:3032000000,warm:1013282958,main:1010586125,inner:1000000166}
    val slow={child:3300000000,warm:1013568458,main:1518095667,inner:1502227917}
    assert(fast.child-fast.warm-fast.main==1008130917 and
      slow.child-slow.warm-slow.main==768335875 and
      slow.inner-fast.inner==502227751 and
      slow.child-slow.warm-slow.main<fast.child-fast.warm-fast.main)
  }
  run completeSelectCoverageStillDoesNotCoverOtherWaitsTest=o::init.then(all {
    assert(canExcludeSelect(gap,true,true,true,0,0)),
    assert(not(selectWrapperRecords("other-native-wait",true))),assert(o::safety),o::idle})
  run frozenClockBoundsDistinguishOfferAndPostOfferWindowsTest = {
    // Relative nanoseconds from the retained frozen 1M/s before anchor.
    val offered = {scope:1, begin:20398667, end:999999000}
    val afterOffer = {scope:1, begin:1020397667, end:1638521208}
    val acrossEnd = {scope:1, begin:976119542, end:1010488583}
    val insideAfterOffer = {scope:1, begin:1023010042, end:1048129875}
    assert(and {
      overlap(acrossEnd,offered)==23879458,
      overlap(acrossEnd,offered)/1000==23879,
      overlap(acrossEnd,afterOffer)==0,
      overlap(insideAfterOffer,offered)==0,
      overlap(insideAfterOffer,afterOffer)==25119833,
      overlap(acrossEnd,offered.with("scope",2))== -1
    })
  }
  run scopedGapDebtCanGrowBeforeLaterSampledPeakTest = {
    // The prior verified frame can be returned after the first hook.
    val newlyDue = 249506625 / 2000 - 244205209 / 2000
    val previousFrameReturns = Set(0,1)
    assert(newlyDue==2651 and
      previousFrameReturns.forall(returned => newlyDue-returned>=2650))
  }
  // Each consecutive verified-progress hook still owns its current frame.
  pure def maxPublicationsBetweenHooks(credits:int,returned:int):int=
    returned+credits-1
  run ScopedGapBoundsUnpublishedDebtWithoutReadingTheInboxTest={
    val firstDue=249506625 / 2000 - 244205209 / 2000
    val secondDue=247861375 / 2000 - 187861458 / 2000
    val returned=Set(0,1)
    assert(firstDue==2651 and secondDue==30000 and
      returned.forall(n=>firstDue-maxPublicationsBetweenHooks(3,n)>=2648 and
        secondDue-maxPublicationsBetweenHooks(3,n)>=29997))
  }
  // Second values are a structurally matched receipt example; the saved
  // stdout bytes do not match the original digest in that receipt.
  run ACollectorWallEnvelopeDoesNotPriceOwnerCpuInTheSameGapTest={
    val interval={scope:2,begin:187861458,end:247861375}
    val start={scope:2,begin:187628917,end:187629125,cpu:691662041}
    val finish={scope:2,begin:247861792,end:247862250,cpu:692876916}
    val gcWall=59944153
    val recordedSelectWall=4250
    assert(enclosed(interval,start,finish) and
      nonCpuLower(interval,start,finish)-recordedSelectWall==58780792 and
      gcWall>finish.cpu-start.cpu)
  }
  // An op62 observer cut is ordered in the progress word's modification
  // order. A plain load pair does not supply that premise.
  // A one-shot terminal write differs from a two-cut plateau read.
  // Its replaced timestamp may come from a store inside the bracket.
  type PrefixExchange={armed:int,target:int,begin:int,end:int,replaced:int,finished:int}
  pure def prefixExchangeValid(c:PrefixExchange):bool=
    c.armed<c.replaced and c.replaced<=c.end and c.armed<c.target and
    c.target<=c.begin and c.begin<=c.end and c.end<=c.finished
  run MidRunExchangeCanReplaceAHeartbeatWrittenInsideItsBracketTest={
    val c={armed:37030731163875,target:37031231163875,begin:37031236183708,
      end:37031236184041,replaced:37031236183791,finished:37031243797083}
    assert(prefixExchangeValid(c) and c.replaced-c.begin==83 and
      not(prefixExchangeValid({...c,replaced:c.end+1})) and
      not(prefixExchangeValid({...c,replaced:c.armed})) and
      not(prefixExchangeValid({...c,finished:c.end-1})))
  }
  run MillionPrefixSealDistinguishesSourceDebtAndUnfinishedWorkloadTest={
    val elapsed=493396333
    val planned=1000000
    val due=493397
    val published=461220
    val verified=461217
    val timed=461218
    assert(due==1+elapsed/1000 and due-published==32177 and
      published-verified==3 and due-verified==32180 and
      planned-due==506603 and timed==verified+1 and due<planned)
  }

  type HeartbeatCut={owner:int,generation:int,begin:int,end:int,value:int,rmw:bool}
  pure def progressPlateau(a:HeartbeatCut,b:HeartbeatCut,armedAfter:int):bool=
    a.rmw and b.rmw and a.owner>0 and a.owner==b.owner and
    a.generation>0 and a.generation==b.generation and
    armedAfter<a.value and a.value<a.begin and a.begin<=a.end and
    a.end<b.begin and b.begin<=b.end and a.value==b.value
  pure val beatA={owner:1,generation:1,begin:3,end:4,value:2,rmw:true}
  pure val beatB={...beatA,begin:9,end:10}
  run HeartbeatPlateauRequiresLiveOwnerAndLinearizedCutsTest=assert(
    progressPlateau(beatA,beatB,1) and
    not(progressPlateau(beatA,beatB,2)) and
    not(progressPlateau(beatA,{...beatB,generation:2},1)) and
    not(progressPlateau(beatA,{...beatB,owner:2},1)) and
    not(progressPlateau({...beatA,rmw:false},beatB,1)))
  // At an arbitrary observer cut the current frame may already be free.
  // One pre-cut or already-timestamped return is still possible.
  run EqualHeartbeatDoesNotProveZeroReturnsBetweenObserverCutsTest={
    val allowed=Set(0,1)
    val maximumLiveGrowth=3
    assert(progressPlateau(beatA,beatB,1) and allowed.contains(1) and
      allowed.forall(n=>n+maximumLiveGrowth<=4))
  }
  run FrozenWatchdogCadenceCanMissTheLatestScaleOfGapTest={
    // Coverage counterexample, not a reconstruction of the 69.350 ms p99.
    val checks=Set(0,100000000)
    val gap={begin:1000000,end:70350000}
    val resumedStamp=99000000
    val watchdogNs=10000000000
    assert(checks.forall(t=>t<gap.begin or t>gap.end) and
      gap.end-gap.begin==69350000 and resumedStamp>gap.end and
      100000000-resumedStamp<watchdogNs and
      not(progressPlateau({...beatA,rmw:false},beatB,1)))
  }
  run SteadyDeficitCanExceedDebtLimitWithoutAnObservablePlateauTest={
    // A request becomes due every microsecond. A work-conserving FIFO server
    // executes the oldest 970 requests during each millisecond, then cannot
    // retire any in the next 30 microseconds.
    val duePerWindow=1000
    val retirementsPerWindow=970
    val windows=1000
    val longestCompletionGapNs=(duePerWindow-retirementsPerWindow+1)*1000
    val triggerAgeNs=2000000
    assert((duePerWindow-retirementsPerWindow)*windows==30000 and
      (duePerWindow-retirementsPerWindow)*windows>128 and
      longestCompletionGapNs<triggerAgeNs)
  }
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
