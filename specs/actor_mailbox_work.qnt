// Work-accounting refinement of the canonical Actor lifecycle, not a scheduler.
// Counts are logical protocol events; no time, bytes, or performance prediction.
module actor_mailbox_work {
  const ACTORS:Set[int]
  const REQUESTS:Set[int]
  const LIMIT:int
  import actor_population(PEERS=ACTORS,JOBS=REQUESTS,BUDGET=LIMIT) as life from "./actor_population"
  type Work = { constructed:int->int, verified:int->int, transfers:int, extra:int, created:int }
  val state = life::p
  val lifetimeSafe = life::safety
  val completedReady = life::ready
  val nativeOutstanding = life::inflight
  val lifecycleDrained = life::drained
  val runningActors = life::runningActors
  val runnableActors = life::runnable
  val liveMemory = life::memory
  var work:Work
  val sourceDebt = life::p.arrived.exclude(life::p.published)
  val debtPartition = sourceDebt.union(life::executing).union(life::ready)==life::p.arrived.exclude(life::p.consumed) and
    sourceDebt.intersect(life::executing).size()==0 and sourceDebt.intersect(life::ready).size()==0 and
    life::executing.intersect(life::ready).size()==0
  val workPreserved = and {
    REQUESTS.forall(m=>work.constructed.get(m)==(if(life::p.published.contains(m)) 1 else 0)),
    REQUESTS.forall(m=>work.verified.get(m)==(if(life::p.completed.contains(m) and not(life::p.failed.contains(m))) 1 else 0)),
    work.transfers==life::p.published.size()+life::p.completed.size(),work.extra==0
  }
  val cycleSafe = life::safety and debtPartition and workPreserved
  action beginCycle = all { life::init,
    work'={constructed:REQUESTS.mapBy(_=>0),verified:REQUESTS.mapBy(_=>0),transfers:0,extra:0,created:0} }
  action createResident(a:int):bool = all { life::spawn(a),work'={...work,created:work.created+1} }
  action configureCredits(k:int):bool = all { life::setTokens(k),work'=work }
  action pauseOwner = all { life::pauseOwner,work'=work }
  action restoreOwner = all { life::restoreOwner,work'=work }
  action openCycle = all { life::open,work'=work }
  action offer(m:int):bool = all { life::arrive(m),work'=work }
  // Construction/publication is one transaction after obtaining canonical credit.
  // No payload construction is charged to an unpublished, waiting arrival.
  action supply(m:int,a:int):bool = all { life::publish(m,a),
    work'={...work,constructed:work.constructed.set(m,work.constructed.get(m)+1),transfers:work.transfers+1} }
  action enter(a:int):bool = all { life::start(a),work'=work }
  action executeUnit(a:int):bool = all { life::unit(a),work'=work }
  action continueUnit(a:int):bool = all { life::continue(a),work'=work }
  action returnResult(a:int,failed:bool):bool = all { life::finish(a,failed),
    work'={...work,transfers:work.transfers+1,
      verified:if(failed) work.verified else work.verified.set(life::p.work.get(a),work.verified.get(life::p.work.get(a))+1)} }
  action takeVerified(m:int):bool = all { life::consume(m),work'=work }
  action waitGreen(a:int):bool = all { life::suspend(a),work'=work }
  def canResumeGreen(a:int,epoch:int,m:int):bool = life::canResume(a,epoch,m)
  action resumeGreen(a:int,epoch:int,m:int):bool = all { life::resume(a,epoch,m),work'=work }
  action captureCaller(a:int):bool = all { life::capture(a),work'=work }
  action releaseCaller(m:int):bool = all { life::release(m),work'=work }
  action cancelCaller(m:int):bool = all { life::cancel(m),work'=work }
  action beginDrain = all { life::drain,work'=work }
  action endDrain = all { life::quiesce,work'=work }
  action configureNextCycle = all { life::configure("cpu"),
    work'={...work,constructed:REQUESTS.mapBy(_=>0),verified:REQUESTS.mapBy(_=>0),transfers:0} }
  action beginClose = all { life::closing,work'=work }
  action requestStop(a:int):bool = all { life::stop(a),work'=work }
  action confirmStop(a:int):bool = all { life::stopped(a),work'=work }
  action joinResident(a:int):bool = all { life::join(a),work'=work }
  action finishClose = all { life::close,work'=work }
  action unchanged = all { life::stay,work'=work }
  action step = {
    nondet a = ACTORS.oneOf()
    nondet m = REQUESTS.oneOf()
    any { createResident(a),openCycle,offer(m),supply(m,a),enter(a),executeUnit(a),
      continueUnit(a),returnResult(a,false),returnResult(a,true),takeVerified(m),
      waitGreen(a),resumeGreen(a,life::p.epoch,m),cancelCaller(m),
      beginDrain,endDrain,configureNextCycle,beginClose,requestStop(a),confirmStop(a),joinResident(a),finishClose,unchanged }
  }
  // Retained rejected mechanisms change only the event ledger, not lifecycle.
  action unsafeConstructBeforeCredit(m:int):bool = all { life::stay,
    work'={...work,constructed:work.constructed.set(m,work.constructed.get(m)+1)} }
  action unsafeDuplicateVerification(m:int):bool = all { life::stay,
    work'={...work,verified:work.verified.set(m,work.verified.get(m)+1)} }
  action unsafeExtraCoordination = all { life::stay,work'={...work,extra:work.extra+1} }
  action unsafeReleaseFrame(m:int):bool = all { life::unsafeReturnBeforeConsume(m),work'=work }
}

// Public API/type-proof refinement. No new Actor lifecycle or runtime checks.
module mailbox_send_scope {
  import actor_mailbox_work(ACTORS=Set(1,2),REQUESTS=Set(1,2),LIMIT=8).*
  run ResidentActorReuseRejectsPreviousEpochForTheSameOrdinalTest=beginCycle
    .then(createResident(1)).then(openCycle).then(offer(1)).then(supply(1,1))
    .then(enter(1)).then(waitGreen(1)).then(resumeGreen(1,1,1))
    .then(executeUnit(1)).then(continueUnit(1)).then(returnResult(1,false))
    .then(takeVerified(1)).then(beginDrain).then(endDrain).then(configureNextCycle)
    .then(openCycle).then(offer(1)).then(supply(1,1)).then(enter(1)).then(waitGreen(1))
    .then(all {assert(not(canResumeGreen(1,1,1))),assert(cycleSafe),unchanged})
    .then(resumeGreen(1,2,1)).then(executeUnit(1)).then(continueUnit(1))
    .then(returnResult(1,false)).then(takeVerified(1)).then(beginDrain).then(endDrain)
    .then(beginClose).then(requestStop(1)).then(confirmStop(1)).then(joinResident(1))
    .then(finishClose).then(all {assert(work.created==1),assert(cycleSafe),unchanged})

  type SourceScope={privateTarget:bool,initializedBySpawn:bool,checkedLookup:bool,
    generatedDestination:str,newCoordination:int}
  pure def eliminatedWrapper(s:SourceScope):bool=s.generatedDestination=="##thread-send"
  pure def scopeQualified(s:SourceScope):bool=s.privateTarget and s.initializedBySpawn and
    s.checkedLookup and eliminatedWrapper(s) and s.newCoordination==0
  pure val typedScope={privateTarget:true,initializedBySpawn:true,checkedLookup:true,
    generatedDestination:"##thread-send",newCoordination:0}
  run notSafeGlobalJumpDoesNotProveWrapperEliminationTest=assert(
    not(scopeQualified({...typedScope,generatedDestination:"thread-send"})))
  run broadUnsafeScopeCannotWaiveCheckedTargetLookupTest=assert(
    scopeQualified(typedScope) and not(scopeQualified({...typedScope,checkedLookup:false})))
  run annotationCannotManufactureTargetInitializationTest=assert(
    not(scopeQualified({...typedScope,initializedBySpawn:false})) and
      not(scopeQualified({...typedScope,privateTarget:false})))
  action boot=beginCycle.then(createResident(1)).then(createResident(2))
    .then(configureCredits(2)).then(openCycle)
  run typedSendStillRequiresIndividualConsumptionAndReturnTest=boot.then(offer(1))
    .then(supply(1,1)).then(enter(1)).then(executeUnit(1)).then(continueUnit(1))
    .then(returnResult(1,false)).then(all {
      assert(cycleSafe),assert(nativeOutstanding==Set(1)),assert(work.transfers==2),unchanged})
    .then(takeVerified(1)).then(all {
      assert(cycleSafe),assert(nativeOutstanding==Set()),assert(state.consumed==Set(1)),unchanged})
  run registeredActorPrefixDrainsBeforeTheSourceOpensTest=beginCycle
    .then(createResident(1)).then(requestStop(1)).then(confirmStop(1)).then(joinResident(1))
    .then(all {assert(cycleSafe),assert(lifecycleDrained),assert(liveMemory==0),
      assert(state.published==Set()),assert(state.consumed==Set()),unchanged})
  // Native registered-prefix coverage cannot certify an unknown failure
  // after SDK enqueue but before returning the worker handle to the caller.
  type CreationUnwind={startedKnown:bool,started:int,registered:int,joined:int,published:int}
  pure def fullCreationUnwindCovered(c:CreationUnwind):bool =
    c.startedKnown and c.started==c.registered and c.registered==c.joined and c.published==0
  run RegisteredPrefixDoesNotCoverAnUnknownStartedWorkerTest={
    val normal={startedKnown:true,started:2,registered:2,joined:2,published:0}
    assert(fullCreationUnwindCovered(normal) and
      not(fullCreationUnwindCovered({...normal,startedKnown:false})) and
      not(fullCreationUnwindCovered({...normal,started:3})) and
      not(fullCreationUnwindCovered({...normal,published:1})))
  }
  // A join exception can prove terminal execution but never retire a frame
  // after a timeout or an unclassified return. Attempt every registered join.
  type StopOutcome={registered:Set[int],attempted:Set[int],terminal:Set[int],
    bodyFailed:bool,cleanupFailed:bool}
  pure def frameReleaseSafe(s:StopOutcome):bool=
    s.attempted==s.registered and s.terminal==s.registered
  pure def reportedStopFailure(s:StopOutcome):str=
    if(s.bodyFailed) "original" else if(s.cleanupFailed) "cleanup" else "none"
  run WorkerFailureDoesNotExcuseSkippingAnotherJoinTest={
    val legacy={registered:Set(1,2),attempted:Set(1),terminal:Set(1),
      bodyFailed:true,cleanupFailed:true}
    val drained={...legacy,attempted:Set(1,2),terminal:Set(1,2)}
    assert(not(frameReleaseSafe(legacy)) and frameReleaseSafe(drained) and
      reportedStopFailure(drained)=="original" and
      reportedStopFailure({...drained,bodyFailed:false})=="cleanup")
  }
  run JoinTimeoutCannotAuthorizeFrameReleaseOrOwnerReuseTest={
    val timeout={registered:Set(1,2),attempted:Set(1,2),terminal:Set(2),
      bodyFailed:false,cleanupFailed:true}
    assert(not(frameReleaseSafe(timeout)) and reportedStopFailure(timeout)=="cleanup" and
      reportedStopFailure({...timeout,bodyFailed:true})=="original")
  }

  // An incomplete close must fence the current VM before the Rust job
  // returns. Matching the original read-only receipt does not authorize work.
  type CloseFence={terminalProved:bool,quiescenceFault:bool,admissionClosed:bool,
    receiptAvailable:bool,matchingCommand:bool,serviceCloseStatus:int}
  pure def canReuseMailboxOwner(f:CloseFence):bool=
    f.terminalProved and not(f.quiescenceFault) and not(f.admissionClosed)
  pure def canReadMailboxFailure(f:CloseFence):bool=
    f.receiptAvailable and f.matchingCommand
  pure def canEnterSdkCleanup(f:CloseFence,rootCount:int):bool=
    f.terminalProved and f.serviceCloseStatus==0 and rootCount==0
  pure def fenceInstalledBeforeForeignReturn(f:CloseFence):bool=
    f.terminalProved or (f.quiescenceFault and f.admissionClosed)
  run FailedCloseMustFenceAdmissionBeforeTheOwnerReturnsTest={
    val pending={terminalProved:false,quiescenceFault:true,admissionClosed:true,
      receiptAvailable:true,matchingCommand:true,serviceCloseStatus:-73}
    assert(fenceInstalledBeforeForeignReturn(pending) and
      not(fenceInstalledBeforeForeignReturn({...pending,admissionClosed:false})) and
      canReadMailboxFailure(pending) and
      not(canReadMailboxFailure({...pending,matchingCommand:false})) and
      not(canReuseMailboxOwner(pending)) and not(canEnterSdkCleanup(pending,0)))
  }
  run ZeroExternalRootsCannotOverrideFailedServiceCloseTest={
    val failed={terminalProved:false,quiescenceFault:true,admissionClosed:true,
      receiptAvailable:false,matchingCommand:false,serviceCloseStatus:-73}
    val healthy={...failed,terminalProved:true,quiescenceFault:false,
      admissionClosed:false,serviceCloseStatus:0}
    assert(not(canEnterSdkCleanup(failed,0)) and not(canReuseMailboxOwner(failed)) and
      canReuseMailboxOwner(healthy) and canEnterSdkCleanup(healthy,0) and
      not(canEnterSdkCleanup(healthy,1)) and
      not(canEnterSdkCleanup({...healthy,serviceCloseStatus:-73},0)))
  }

  // A C-export wrapper is only one way to enter. A previously acquired
  // Scheme closure checks its own endpoint state and needs the same fence.
  pure def mailboxEntryAllowed(viaWrapper:bool,admissionClosed:bool,
      endpointPreparing:bool):bool =
    if(viaWrapper) not(admissionClosed) else not(endpointPreparing)
  run WrapperFenceDoesNotRevokeAnAlreadyAcquiredEntryClosureTest=assert(
    not(mailboxEntryAllowed(true,true,false)) and
    mailboxEntryAllowed(false,true,false) and
    not(mailboxEntryAllowed(false,true,true)))
  run ArchivedNativeJoinTimeoutQuarantinesEvenWithZeroExternalRootsTest={
    val stop={registered:Set(1,2,3,4),attempted:Set(1,2,3,4),terminal:Set(2,3,4),
      bodyFailed:true,cleanupFailed:true}
    val fence={terminalProved:false,quiescenceFault:true,admissionClosed:true,
      receiptAvailable:true,matchingCommand:true,serviceCloseStatus:-73}
    assert(not(frameReleaseSafe(stop)) and reportedStopFailure(stop)=="original" and
      canReadMailboxFailure(fence) and not(canReuseMailboxOwner(fence)) and
      not(canEnterSdkCleanup(fence,0)))
  }

  type TerminalReceipt={primarySnapshot:bool,cleanupError:bool}
  pure def terminalReceiptKind(r:TerminalReceipt):str=
    if(r.primarySnapshot) "snapshot" else if(r.cleanupError) "cleanup-only" else "absent"
  pure def takeMailboxFailure(f:CloseFence,r:TerminalReceipt):TerminalReceipt=
    if(canReadMailboxFailure(f)) {...r,primarySnapshot:false,cleanupError:false} else r
  run CleanupOnlyReceiptHasNoCountsAndCannotBeConsumedByAnotherCommandTest={
    val receipt={primarySnapshot:false,cleanupError:true}
    val owner={terminalProved:false,quiescenceFault:true,admissionClosed:true,
      receiptAvailable:true,matchingCommand:true,serviceCloseStatus:-73}
    val foreign={...owner,matchingCommand:false}
    assert(terminalReceiptKind(receipt)=="cleanup-only" and
      terminalReceiptKind({...receipt,primarySnapshot:true})=="snapshot" and
      takeMailboxFailure(foreign,receipt)==receipt and
      terminalReceiptKind(takeMailboxFailure(owner,receipt))=="absent" and
      not(canReuseMailboxOwner(owner)))
  }

  run WorkerFailureSnapshotAndSuccessfulUnwindHaveSeparateAuthorityTest={
    val receipt={primarySnapshot:true,cleanupError:false}
    val drained={terminalProved:true,quiescenceFault:false,admissionClosed:false,
      receiptAvailable:true,matchingCommand:true,serviceCloseStatus:0}
    val pending={...drained,terminalProved:false,quiescenceFault:true,admissionClosed:true}
    val foreign={...drained,matchingCommand:false}
    val taken={...drained,receiptAvailable:false}
    assert(takeMailboxFailure(foreign,receipt)==receipt and
      terminalReceiptKind(takeMailboxFailure(drained,receipt))=="absent" and
      not(canReadMailboxFailure(taken)) and canReuseMailboxOwner(taken) and
      canReadMailboxFailure(pending) and not(canReuseMailboxOwner(pending)))
  }

  // A constructor throw has no successful supply action. It must not
  // advance the original source cursor or release an earlier charged frame.
  run UnpublishedConstructionFailureKeepsSiblingCustodyAndNoReceiptTest=boot
    .then(offer(1)).then(offer(2)).then(supply(1,1)).then(all {
      assert(cycleSafe),assert(state.published==Set(1)),assert(state.consumed==Set()),
      assert(sourceDebt==Set(2)),assert(nativeOutstanding==Set(1)),
      assert(work.constructed.get(2)==0),assert(work.transfers==1),
      assert(terminalReceiptKind({primarySnapshot:false,cleanupError:false})=="absent"),
      unchanged})
  run MissingConstructorReceiptCannotBeReadAsAZeroCountTest={
    val absent={primarySnapshot:false,cleanupError:false}
    val drained={terminalProved:true,quiescenceFault:false,admissionClosed:false,
      receiptAvailable:false,matchingCommand:true,serviceCloseStatus:0}
    assert(terminalReceiptKind(absent)=="absent" and
      not(canReadMailboxFailure(drained)) and canReuseMailboxOwner(drained))
  }

  // The layout snapshot chooses a dispatch branch. The service-unit
  // lease decision belongs to the fresh context read in that branch.
  type UnplannedLayout={snapshotHasWork:bool,liveContext:str}
  pure def unitNeedsWind(layout:UnplannedLayout):bool =
    layout.liveContext=="different-owner"
  pure def legacyUnitWrapper(layout:UnplannedLayout):int =
    if(layout.snapshotHasWork) 0 else 1
  pure def candidateUnitWrapper(layout:UnplannedLayout):int =
    if(not(layout.snapshotHasWork) and unitNeedsWind(layout)) 1 else 0
  run LayoutDispatchCannotUseAnEarlierEmptySnapshotAsLeaseAuthorityTest={
    val hot={snapshotHasWork:false,liveContext:"empty"}
    val owned={...hot,liveContext:"same-owner"}
    val contended={...hot,liveContext:"different-owner"}
    assert(not(unitNeedsWind(hot)) and not(unitNeedsWind(owned)) and
      unitNeedsWind(contended) and legacyUnitWrapper(hot)==1 and
      candidateUnitWrapper(hot)==0 and candidateUnitWrapper(owned)==0 and
      candidateUnitWrapper(contended)==legacyUnitWrapper(contended))
  }

  // A retained historical receipt is not a currently qualified comparison arm.
  type BaselineEvidence={historicalReceipt:bool,matchedArtifacts:bool,matchedWorkload:bool,
    timingAudited:bool,completeReproductions:int,repeatabilityAccepted:bool}
  pure def comparisonBaselineQualified(b:BaselineEvidence):bool=b.historicalReceipt and
    b.matchedArtifacts and b.matchedWorkload and b.timingAudited and
    b.completeReproductions>=3 and b.repeatabilityAccepted
  pure val historicalSevenStatus={historicalReceipt:true,matchedArtifacts:true,
    matchedWorkload:true,timingAudited:false,completeReproductions:0,repeatabilityAccepted:false}

  // Native VM behavior, repeatable controls and performance are separate gates.
  type Coverage={sdkFixture:bool,ordinaryActorBuild:bool,fullSourceImage:bool,fullVmGc:bool,fullVmException:bool,
    fullVmContinuation:bool,fullVmClose:bool,generatedCost:bool,changedPath:bool,
    initFailureCleanup:bool,matchedNativeOutcome:bool,passesCurrentLimits:bool}
  pure def mayEnable(c:Coverage,b:BaselineEvidence):bool=c.sdkFixture and c.ordinaryActorBuild and c.fullSourceImage and
    c.fullVmGc and c.fullVmException and c.fullVmContinuation and c.fullVmClose and
    c.generatedCost and c.changedPath and c.initFailureCleanup and c.matchedNativeOutcome and
    c.passesCurrentLimits and comparisonBaselineQualified(b)
  // Hypothetical positive premises for testing the gate, not observed results.
  pure val fullyQualifiedExample={sdkFixture:true,ordinaryActorBuild:true,fullSourceImage:true,fullVmGc:true,
    fullVmException:true,fullVmContinuation:true,fullVmClose:true,generatedCost:true,
    changedPath:true,initFailureCleanup:true,matchedNativeOutcome:true,passesCurrentLimits:true}
  pure val repeatableExample={...historicalSevenStatus,timingAudited:true,
    completeReproductions:3,repeatabilityAccepted:true}
  run localFixtureDoesNotReplaceFullVmAndCostQualificationTest=assert(
    not(mayEnable({...fullyQualifiedExample,fullVmGc:false},repeatableExample)) and
    not(mayEnable({...fullyQualifiedExample,generatedCost:false},repeatableExample)))
  type RelativeControl={planned:int,completed:int,rejected:int,errors:int,timeouts:int,
    clean:bool,p99Ns:int,p999Ns:int,peak:int}
  // Reference values belong to one cohort; this is not a product-limit override.
  pure def historicalControlFits(reference:RelativeControl,current:RelativeControl,sameInputs:bool):bool=
    sameInputs and current.planned==reference.planned and current.completed==current.planned and
      current.rejected==0 and current.errors==0 and current.timeouts==0 and current.clean and
      current.p99Ns<=reference.p99Ns and current.p999Ns<=reference.p999Ns and current.peak<=reference.peak
  pure val recordedReference={planned:1000000,completed:1000000,rejected:0,errors:0,timeouts:0,
    clean:true,p99Ns:6393000,p999Ns:6740000,peak:6746}
  pure val workingControlEnvelope={...recordedReference,p99Ns:15000000,p999Ns:15000000,peak:12969}
  // For complete equal-size runs, the rate floor is exactly an
  // upper bound on the native inner elapsed time.
  pure val workingControlElapsedNs=1012725666
  pure def workingControlFits(current:RelativeControl,elapsedNs:int):bool=
    historicalControlFits(workingControlEnvelope,current,true) and
      0<elapsedNs and elapsedNs<=workingControlElapsedNs
  run WorkingLineRetains3477a02AndStopsAboveFifteenMsTest={
    val replay={...workingControlEnvelope,p99Ns:12869000,p999Ns:12948000}
    val local={...replay,p99Ns:19561000,p999Ns:19698000,peak:19717}
    assert(workingControlFits(replay,workingControlElapsedNs) and
      workingControlFits({...replay,p99Ns:15000000,p999Ns:15000000},workingControlElapsedNs) and
      not(workingControlFits({...replay,p99Ns:15001000,p999Ns:15001000},workingControlElapsedNs)) and
      not(workingControlFits(local,1019221625)) and
      not(workingControlFits(replay,workingControlElapsedNs+1)))
  }
  run AuthorizedOriginalReplayStopsOnFailedMainTest={
    val main={...workingControlEnvelope,p99Ns:69350000,p999Ns:69505000,peak:69369}
    assert(main.completed==1000000 and main.rejected==0 and main.timeouts==0 and main.clean and
      not(workingControlFits(main,1070022416)) and
      main.p99Ns>workingControlEnvelope.p99Ns and
      main.p999Ns>workingControlEnvelope.p999Ns and
      main.peak>workingControlEnvelope.peak and 1070022416>workingControlElapsedNs)
  }
  pure val recordedCurrentControl={...recordedReference,p99Ns:51139000,p999Ns:51291000,peak:51316}
  run sameFrozenInputsAndZeroRejectsDoNotQualifySlowControlTest=assert(
    not(historicalControlFits(recordedReference,recordedCurrentControl,true)) and
      historicalControlFits(recordedReference,recordedReference,true) and
      not(historicalControlFits(recordedReference,recordedReference,false)))
  pure val publishedCandidateReference={...recordedReference,p99Ns:4513000,p999Ns:4819000,peak:4853}
  pure val publishedCandidateElapsedNs=1000055125
  // Receipt authenticity, full call elapsed and the ordered control
  // sequence are checked separately by the launcher.
  pure def candidateReplayFits(current:RelativeControl,elapsedNs:int):bool=
    historicalControlFits(publishedCandidateReference,current,true) and
    0<elapsedNs and elapsedNs<=publishedCandidateElapsedNs and current.p99Ns<=418000 and
    current.p99Ns<publishedCandidateReference.p99Ns and
    current.peak+1<=128 and current.peak+1<publishedCandidateReference.peak
  run AbsoluteLatencyAndBacklogDoNotWaiveRelativeTailOrThroughputTest={
    val candidate={...recordedReference,p99Ns:400000,p999Ns:4819000,peak:127}
    assert(candidateReplayFits(candidate,publishedCandidateElapsedNs) and
      not(candidateReplayFits({...candidate,p999Ns:4820000},publishedCandidateElapsedNs)) and
      not(candidateReplayFits({...candidate,peak:128},publishedCandidateElapsedNs)) and
      not(candidateReplayFits(candidate,publishedCandidateElapsedNs+1)))
  }
  pure def improvesMatchedControl(control:RelativeControl,candidate:RelativeControl,
      controlRate:int,candidateRate:int):bool=
    historicalControlFits(control,candidate,true) and candidateRate>=controlRate and
    candidate.p99Ns<control.p99Ns and candidate.peak+1<control.peak
  run HistoricGainCannotCoverRegressionAgainstTheMatchedBuildTest={
    val rebuilt={...recordedReference,p99Ns:200000,p999Ns:410000,peak:100}
    val improved={...rebuilt,p99Ns:199000,p999Ns:405000,peak:80}
    val regression=Set(200000,201000,300000,400000,401000)
    assert(candidateReplayFits({...improved,p99Ns:401000},publishedCandidateElapsedNs) and
      regression.forall(p99=>not(improvesMatchedControl(
        rebuilt,{...improved,p99Ns:p99},100000000,110000000))) and
      not(improvesMatchedControl(rebuilt,{...improved,peak:99},100000000,110000000)) and
      not(improvesMatchedControl(rebuilt,improved,100000000,99999999)) and
      improvesMatchedControl(rebuilt,improved,100000000,100000000))
  }
  run PrivateOrderImageLifecyclePassCannotAdmitFailedFrozenControlTest={
    val frozen={...recordedReference,p99Ns:120686000,p999Ns:120844000,peak:116723}
    val privateImage={...fullyQualifiedExample,ordinaryActorBuild:false,
      generatedCost:false,matchedNativeOutcome:false,passesCurrentLimits:false}
    val actual={...historicalSevenStatus,timingAudited:true,completeReproductions:0}
    assert(not(historicalControlFits(recordedReference,frozen,true)) and
      not(mayEnable(privateImage,actual)))
  }
  run NativeMatchedReplayStillStopsAtTheFirstFrozenControlTest={
    val frozen={...recordedReference,p99Ns:118138000,p999Ns:118718000,peak:115280}
    assert(not(historicalControlFits(recordedReference,frozen,true)) and
      not(candidateReplayFits(frozen,publishedCandidateElapsedNs)))
  }
  run BothOracleTimeoutPrefixesFailAdmissionIndependentlyTest={
    val apparentFast={...recordedReference,p99Ns:400000,p999Ns:410000,peak:100}
    val original={...apparentFast,completed:936340,timeouts:1,clean:false}
    val changed={...apparentFast,completed:791216,timeouts:1,clean:false}
    // Even optimistic prefix quantiles and peak cannot qualify either arm.
    assert(not(workingControlFits(original,workingControlElapsedNs)) and
      not(candidateReplayFits(changed,publishedCandidateElapsedNs)))
  }
  run CleanShutdownDoesNotAdmitSlowFullMainOrTruncatedPrefixTest={
    val slowMain={...recordedReference,p99Ns:181533000,p999Ns:181639000,peak:154547}
    // The 493396333 ns observation is a diagnostic cut, not request latency.
    val prefix={...recordedReference,completed:461217,p99Ns:34013000,
      p999Ns:34120000,peak:32180}
    val fastPrefix={...prefix,p99Ns:400000,p999Ns:410000,peak:100}
    assert(not(historicalControlFits(recordedReference,slowMain,true)) and
      not(historicalControlFits(recordedReference,prefix,false)) and
      not(historicalControlFits(recordedReference,fastPrefix,true)))
  }
  type FrozenMainTiming={offered:int,lastStamp:int,elapsed:int,whole:int,drain:int}
  pure def frozenMainTimingFits(t:FrozenMainTiming):bool=
    t.offered==999999000 and t.offered<=t.lastStamp and t.lastStamp<=t.elapsed and
    t.elapsed<=t.whole and t.drain==t.elapsed-t.offered and t.whole<=3000000000
  run RelativeReferenceCannotWaiveNativeTimingOrderOrWholeBudgetTest={
    val replay={offered:999999000,lastStamp:1000122417,elapsed:1000124292,
      whole:1010852000,drain:125292}
    assert(historicalControlFits(recordedReference,recordedReference,true) and
      frozenMainTimingFits(replay) and
      not(frozenMainTimingFits({...replay,lastStamp:replay.elapsed+1})) and
      not(frozenMainTimingFits({...replay,whole:replay.elapsed-1})) and
      not(frozenMainTimingFits({...replay,whole:3000000001})) and
      not(frozenMainTimingFits({...replay,drain:replay.drain+1})))
  }
  run SameWarmupP99CannotPredictFrozenMainAdmissionTest={
    val faster={warmupP99Ns:18000,main:{...recordedReference,
      p99Ns:5951000,p999Ns:6242000,peak:6269}}
    val slower={warmupP99Ns:18000,main:{...recordedReference,
      p99Ns:7817000,p999Ns:8004000,peak:8058}}
    assert(faster.warmupP99Ns==slower.warmupP99Ns and
      historicalControlFits(recordedReference,faster.main,true) and
      not(historicalControlFits(recordedReference,slower.main,true)))
  }
  run fullVmCorrectnessCannotEnableAnUnmeasuredCandidateTest=assert(
    not(mayEnable({...fullyQualifiedExample,matchedNativeOutcome:false},repeatableExample)))
  run successfulStartupDoesNotQualifyInitializationFailureCleanupTest=assert(
    not(mayEnable({...fullyQualifiedExample,initFailureCleanup:false},repeatableExample)))
  run isolatedImageAndHistoricalControlCannotEnableCandidateTest=assert(
    mayEnable(fullyQualifiedExample,repeatableExample) and
    not(mayEnable({...fullyQualifiedExample,fullSourceImage:false},repeatableExample)) and
    not(mayEnable({...fullyQualifiedExample,ordinaryActorBuild:false},repeatableExample)) and
    not(mayEnable(fullyQualifiedExample,historicalSevenStatus)) and
    not(mayEnable({...fullyQualifiedExample,passesCurrentLimits:false},repeatableExample)))

  // Build provenance is an independent gate, never a latency explanation.
  type BuildEvidence={compilerUnchanged:bool,toolSelectionRecorded:bool,
    producedModule:bool,loadedExpectedValue:bool,sentinelContractMatched:bool}
  pure def buildQualified(b:BuildEvidence):bool=b.compilerUnchanged and
    b.toolSelectionRecorded and b.producedModule and b.loadedExpectedValue and b.sentinelContractMatched
  pure val checkedBuild={compilerUnchanged:true,toolSelectionRecorded:true,
    producedModule:true,loadedExpectedValue:true,sentinelContractMatched:true}
  run compilerExitZeroWithoutAnArtifactIsNotBuildSuccessTest=assert(
    not(buildQualified({...checkedBuild,producedModule:false,loadedExpectedValue:false})))
  run sameCompilerDoesNotProveTheSameSubordinateToolsTest=assert(
    not(buildQualified({...checkedBuild,toolSelectionRecorded:false})) and buildQualified(checkedBuild))
  run aMissingTokenSentinelCannotBeTestedAsBooleanFalseTest=assert(
    not(buildQualified({...checkedBuild,sentinelContractMatched:false})))

  run recorded6393DoesNotCertifyCurrentReproducibilityTest=assert(
    not(comparisonBaselineQualified(historicalSevenStatus)))
  run oneGoodHistoricalRunCannotReplaceAQualifiedCohortTest=assert(
    not(comparisonBaselineQualified({...historicalSevenStatus,timingAudited:true,
      completeReproductions:1,repeatabilityAccepted:true})))
  run repeatedButUnstableOrMismatchedRunsDoNotQualifyTest=assert(
    not(comparisonBaselineQualified({...historicalSevenStatus,timingAudited:true,
      completeReproductions:3})) and
    not(comparisonBaselineQualified({...historicalSevenStatus,timingAudited:true,
      completeReproductions:3,repeatabilityAccepted:true,matchedWorkload:false})))

  // Sampled maxima can occur at different iterations of the source loop.
  run peakDifferencesDoNotIdentifyNativeOccupancyTest = {
    val sourcePeakSample = {source: 2, native: 0}
    val totalPeakSample = {source: 1, native: 2}
    val peakSource = sourcePeakSample.source
    val peakTotal = totalPeakSample.source + totalPeakSample.native
    assert(sourcePeakSample.source + sourcePeakSample.native < peakTotal and
      totalPeakSample.source < peakSource and
      peakTotal - peakSource != totalPeakSample.native and
      totalPeakSample.source >= peakTotal - 2)
  }

  // On normal op62 return, two source samples surround at most one
  // verified consumption. The final unsampled suffix has at most one left.
  pure def outstanding(due:int,consumed:int):int = due-consumed
  run IntermediatePeakIsBoundedByNextSamplePlusOneTest = assert(
    0.to(8).forall(due=>0.to(due).forall(consumed=>
      due.to(8).forall(nextDue=>consumed.to(consumed+1).forall(nextConsumed=>
        nextConsumed>nextDue or
        outstanding(due,consumed)<=outstanding(nextDue,nextConsumed)+1)))))
  run SampleAtLimitCanMissOneDueBeforeFrameReturnTest = {
    val limit=128
    val between=outstanding(129,0)
    val nextSample=outstanding(129,1)
    assert(nextSample==limit and between==limit+1)
  }
  run OneSampledRequestGainCanLeaveTruePeaksTiedTest = {
    val candidateSample = 126
    val matchedSample = 127
    val candidateTrue = candidateSample + 1
    val matchedTrue = matchedSample
    assert(candidateSample < matchedSample and candidateTrue == matchedTrue and
      candidateSample + 1 < matchedSample + 1)
  }
  run FinalSuccessfulIterationHasAtMostOneUnconsumedTest = assert(
    1.to(8).forall(total=>
      (total-1).to(total).forall(due=>outstanding(due,total-1)<=1)))
  run TerminalFailureWithoutNextSampleHasNoPlusOneBoundTest = {
    val sampled=outstanding(128,0)
    val atFailure=outstanding(136,0)
    assert(atFailure>sampled+1)
  }

  // The source samples latency before publishing verified progress. A terminal
  // snapshot can contain the sample for a frame whose credit is still charged.
  pure def missingRankAtFailure(planned:int,due:int,completed:int,percentile:int,scale:int):bool =
    due==planned and completed<(planned*percentile+scale-1)/scale
  run timedButUncommittedFrameCannotHideCorpusRankTest = {
    val total=100
    val consumed=98
    val timed=99
    assert(missingRankAtFailure(total,total,consumed,99,100) and
      not(missingRankAtFailure(total,total,timed,99,100)))
  }
  run terminalRankBoundsRequireFullOfferingAndTheRequestedRankTest = assert(
    not(missingRankAtFailure(100,99,98,99,100)) and
    not(missingRankAtFailure(100,100,99,99,100)) and
    missingRankAtFailure(1000,1000,998,999,1000))

  run FourDistinctReadBoundariesRemainAtFiveMillionTest={
    // A complete raw scheduled retirement owns one earlier publication,
    // one due sample, one latency sample, and one fresh progress read.
    val required=5000000-128
    assert(4*required==19999488 and 3*required==14999616 and
      4*(1000000-128)==3999488)
  }
  run EarlierLatencySampleCannotStandInForTheFreshProgressReadTest={
    val deadline=10000000000
    val prior=1000
    val sampled=prior+deadline-1
    val progress=sampled+1
    assert(sampled-prior<deadline and progress-prior>=deadline and
      ledgerSampleCount(1,true)==2 and missingRankAtFailure(2,2,1,99,100))
  }

  // The existing sample is written before progress publication. A dense
  // ledger uses the committed count as its write index, with a terminal
  // receipt carrying whether that last index was actually written.
  pure def ledgerSampleCount(committed:int,wroteUncommitted:bool):int=
    committed + (if(wroteUncommitted) 1 else 0)
  pure def ledgerBytes(planned:int):int=4*planned
  pure val denseBucketBytes=8*10000000

  run TimedPrefixDoesNotReadUnwrittenSlotsOrReturnCreditTest={
    val committed=98
    val planned=100
    assert(ledgerSampleCount(committed,false)==98 and
      ledgerSampleCount(committed,true)==99 and
      ledgerSampleCount(planned,false)==planned and
      missingRankAtFailure(planned,planned,committed,99,100))
  }

  run SampleLedgerSizeNeedsAnInitializationBoundTest=assert(
    ledgerBytes(1000000)==4000000 and ledgerBytes(5000000)==20000000 and
      ledgerBytes(19999999)<denseBucketBytes and
      ledgerBytes(20000000)==denseBucketBytes and
      ledgerBytes(50000000)>denseBucketBytes)

  run SourceRateAndDurationFitNarrowHistogramCountersTest=assert(
    5000000*10<4294967295 and
      (10000000000-1)/1000==9999999 and 9999999<4294967295)

  // Transferring the existing raw histogram is permitted only when the
  // source exits and no alias or captured writer can resume before take.
  type TerminalCustody={sourceOwned:bool,willAbort:bool,writerEscaped:bool,writerCanResume:bool}
  pure def mayTransferTerminalHistogram(c:TerminalCustody):bool=
    c.sourceOwned and c.willAbort and not(c.writerEscaped) and not(c.writerCanResume)
  run TerminalTransferRequiresExclusiveAbortingSourceTest={
    val privateSource={sourceOwned:true,willAbort:true,writerEscaped:false,writerCanResume:false}
    assert(mayTransferTerminalHistogram(privateSource) and
      not(mayTransferTerminalHistogram({...privateSource,sourceOwned:false})) and
      not(mayTransferTerminalHistogram({...privateSource,willAbort:false})) and
      not(mayTransferTerminalHistogram({...privateSource,writerEscaped:true})) and
      not(mayTransferTerminalHistogram({...privateSource,writerCanResume:true})))
  }

  // Refine the existing native rollback receipt; no per-message state is added.
  type RollbackEvidence={phase:int,attempted:bool,roots:int,status:int,mappingCleanPublished:bool,
    joined:bool,leases:int,modulesDrained:bool,unloadAuthorized:bool}
  pure def rollbackClean(r:RollbackEvidence):bool=r.phase==2 and r.attempted and r.roots==0 and r.status==0
  pure def rollbackCanRelease(r:RollbackEvidence):bool=rollbackClean(r) and r.mappingCleanPublished and
    r.joined and r.leases==0 and r.modulesDrained and r.unloadAuthorized
  pure val recordedRollback={phase:2,attempted:true,roots:0,status:0,mappingCleanPublished:false,
    joined:true,leases:0,modulesDrained:false,unloadAuthorized:false}
  run zeroRootsAndSuccessfulCleanupDoNotPublishMappingStateTest=assert(
    rollbackClean(recordedRollback) and not(rollbackCanRelease(recordedRollback)))
  run rollbackPublicationStillRequiresJoinLeaseAndModuleDrainTest={
    val published={...recordedRollback,mappingCleanPublished:true,modulesDrained:true,unloadAuthorized:true}
    assert(rollbackCanRelease(published) and
      not(rollbackCanRelease({...published,joined:false})) and
      not(rollbackCanRelease({...published,leases:1})) and
      not(rollbackCanRelease({...published,modulesDrained:false})))
  }
  run incompleteOrUnbalancedRollbackCannotBecomeCleanTest=assert(
    not(rollbackClean({...recordedRollback,phase:1})) and
    not(rollbackClean({...recordedRollback,status:1})) and
    not(rollbackClean({...recordedRollback,roots:1})) and
    not(rollbackClean({...recordedRollback,attempted:false})))

}

// Declared typed-vector bytes and a separate continuation reserve. No scheduler
// state is added and no bound on the entire Scheme heap is inferred.
module mailbox_payload_budget {
  import actor_mailbox_work(ACTORS=Set(1,2,3),REQUESTS=1.to(5),LIMIT=16) as w
  pure def orderBytes(rows:int):int = 8*(2+3*rows)+8*21
  pure val declaredBytes=1.to(5).mapBy(m=>if(m==5) 0 else orderBytes(if(m==4) 48 else 4))
  pure val continuationReserve=64
  pure val fixtureBudget=1800
  val frameBytes=w::nativeOutstanding.fold(0,(bytes,m)=>bytes+declaredBytes.get(m))
  val retainedBytes=w::state.retained.size()*continuationReserve
  val chargedBytes=frameBytes+retainedBytes
  val safe=w::cycleSafe and chargedBytes<=fixtureBudget
  def canAdmit(m:int):bool=
    w::state.arrived.contains(m) and not(w::state.published.contains(m)) and
    declaredBytes.get(m)>0 and w::nativeOutstanding.size()<w::state.tokens and
    chargedBytes+declaredBytes.get(m)<=fixtureBudget
  action admit(m:int,a:int):bool = all {canAdmit(m),w::supply(m,a)}
  action retain(a:int):bool = all {chargedBytes+continuationReserve<=fixtureBudget,w::captureCaller(a)}
  action boot=w::beginCycle.then(w::createResident(1)).then(w::createResident(2))
    .then(w::configureCredits(3)).then(w::openCycle)

  action step={
    nondet a=Set(1,2).oneOf()
    nondet m=1.to(5).oneOf()
    any {w::offer(m),admit(m,a),w::enter(a),w::executeUnit(a),w::continueUnit(a),
      w::returnResult(a,false),w::returnResult(a,true),w::takeVerified(m),
      retain(a),w::releaseCaller(m),w::waitGreen(a),w::resumeGreen(a,1,m),
      w::cancelCaller(m),w::pauseOwner,w::restoreOwner,w::unchanged}
  }

  // Keep the same two frames and three resident Actors in all three arms.
  // A wait-capable request can occupy a frame and a mailbox target separately.
  action mixedProfile=w::beginCycle.then(w::createResident(1)).then(w::createResident(2))
    .then(w::createResident(3)).then(w::configureCredits(2)).then(w::openCycle)
    .then(w::offer(1)).then(admit(1,1)).then(w::enter(1)).then(w::waitGreen(1))

  run IdleActorsCannotSupplyACreditHeldBySuspendedWorkTest=mixedProfile
    .then(w::offer(2)).then(admit(2,2)).then(w::enter(2)).then(w::waitGreen(2))
    .then(w::offer(3)).then(all {
      assert(safe),assert(w::nativeOutstanding==Set(1,2)),assert(w::sourceDebt==Set(3)),
      assert(w::state.resident.exclude(w::state.waiting)==Set(3)),
      assert(w::runnableActors==Set()),assert(not(canAdmit(3))),w::unchanged})

  run AnAvailableCreditCannotMakeABlockedMailboxRunnableTest=mixedProfile
    .then(w::offer(3)).then(admit(3,1)).then(all {
      assert(safe),assert(w::state.queue.get(1)==List(3)),assert(w::runnableActors==Set()),
      assert(w::state.owner.get(1)==1),assert(w::state.resident.exclude(w::state.waiting)==Set(2,3)),
      w::unchanged})

  run ASeparateRunnableTargetUsesTheFreeCreditWithoutMovingTheSuspendedJobTest=mixedProfile
    .then(w::offer(3)).then(admit(3,3)).then(all {
      assert(safe),assert(w::runnableActors==Set(3)),w::unchanged})
    .then(w::enter(3)).then(w::executeUnit(3)).then(w::continueUnit(3))
    .then(w::returnResult(3,false)).then(w::takeVerified(3)).then(all {
      assert(safe),assert(w::state.consumed==Set(3)),assert(w::nativeOutstanding==Set(1)),
      assert(w::state.owner.get(1)==1),assert(w::state.waiting==Set(1)),
      assert(w::work.transfers==3),w::unchanged})

  run SameThreeCreditsCanAdmitShortsButNotTheMixedPayloadTest=boot
    .then(w::offer(1)).then(admit(1,1)).then(w::offer(2)).then(admit(2,2))
    .then(w::offer(3)).then(admit(3,1)).then(all {
      assert(safe),assert(w::nativeOutstanding.size()==3),assert(frameBytes==840),w::unchanged})
  run BulkAndShortExhaustTheByteBudgetBeforeTheThirdSlotTest=boot
    .then(w::offer(4)).then(admit(4,1)).then(w::offer(1)).then(admit(1,2))
    .then(w::offer(2)).then(w::offer(5)).then(all {
      assert(safe),assert(w::nativeOutstanding.size()==2),assert(frameBytes==1616),
      assert(not(canAdmit(2))),assert(not(canAdmit(5))),w::unchanged})
  run CancellationAndReadyResultKeepBytesUntilTheirRealOwnersReleaseTest=boot
    .then(w::offer(1)).then(admit(1,1)).then(w::enter(1)).then(retain(1))
    .then(w::waitGreen(1)).then(w::cancelCaller(1)).then(all {
      assert(safe),assert(frameBytes==280),assert(retainedBytes==64),w::unchanged})
    .then(w::resumeGreen(1,1,1)).then(w::executeUnit(1)).then(w::continueUnit(1))
    .then(w::returnResult(1,false)).then(all {
      assert(safe),assert(chargedBytes==344),assert(w::completedReady==Set(1)),w::unchanged})
    .then(w::takeVerified(1)).then(all {
      assert(safe),assert(frameBytes==0),assert(retainedBytes==64),w::unchanged})
    .then(w::releaseCaller(1)).then(all {assert(safe),assert(chargedBytes==0),w::unchanged})
}
