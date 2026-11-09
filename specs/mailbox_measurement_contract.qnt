// Measurement evidence only. No runtime clocks, queues, probes or CPU predictor.
module mailbox_measurement_contract {
  type Workload={rate:int,seconds:int,total:int,owners:int,workers:int,frames:int,
    unit:str,arrival:str,startup:str,deadlineSeconds:int}
  type RunKey={runId:str,artifact:str,route:str}
  type Outcome={planned:int,completed:int,samples:int,rejected:int,errors:int,p99Ns:int,
    p999Ns:int,peak:int,wholeNs:int,clean:bool,work:Workload,key:RunKey,phase:str,
    peakUncertainty:int,bucketNs:int,quantileMethod:str}
  type Limits={p99Ns:int,peak:int,wholeNs:int}
  pure def validWorkload(w:Workload):bool=w.rate>0 and w.seconds>0 and
    w.total==w.rate*w.seconds and w.owners>0 and w.workers>0 and w.frames>0 and
    w.unit!="" and w.arrival!="" and w.startup!="" and w.deadlineSeconds>0
  pure def validKey(k:RunKey):bool=k.runId!="" and k.artifact!="" and k.route!=""
  pure def validLimits(l:Limits):bool=l.p99Ns>0 and l.peak>=0 and l.wholeNs>0
  pure def limitsWithinApproved(requested:Limits,approved:Limits):bool=
    validLimits(requested) and validLimits(approved) and requested.p99Ns<=approved.p99Ns and
    requested.peak<=approved.peak and requested.wholeNs<=approved.wholeNs
  pure def outcomeMatches(o:Outcome,w:Workload,k:RunKey):bool=
    validWorkload(w) and validKey(k) and o.work==w and o.planned==w.total and o.key==k and o.phase=="main"
  pure def validQuantiles(o:Outcome):bool=o.p99Ns>0 and o.p999Ns>=o.p99Ns and
    ((o.quantileMethod=="exact-order-statistic" and o.bucketNs==0) or
     (o.quantileMethod=="histogram-upper" and o.bucketNs>0)) and
    o.p999Ns<=o.wholeNs+o.bucketNs
  pure def validCounts(o:Outcome):bool=o.planned>0 and 0<=o.completed and o.completed<=o.planned and
    0<=o.samples and o.samples<=o.completed and o.rejected>=0 and o.errors>=0 and
    o.completed+o.rejected+o.errors<=o.planned and 0<=o.peak and o.peak<=o.planned and
    o.peakUncertainty>=0 and o.wholeNs>0
  pure def peakUpper(o:Outcome):int=if(o.peak+o.peakUncertainty<o.planned)
    o.peak+o.peakUncertainty else o.planned
  pure def allArrivalsCanHaveCompleted(o:Outcome):bool=
    validWorkload(o.work) and o.wholeNs>=(o.work.total-1)*1000000000/o.work.rate
  pure def absolutePassed(o:Outcome,l:Limits):bool=validLimits(l) and validCounts(o) and
    validWorkload(o.work) and o.planned==o.work.total and validKey(o.key) and o.phase=="main" and
    o.completed==o.planned and o.samples==o.completed and o.rejected==0 and o.errors==0 and
    allArrivalsCanHaveCompleted(o) and validQuantiles(o) and o.p99Ns<=l.p99Ns and peakUpper(o)<=l.peak and o.wholeNs<=l.wholeNs and o.clean

  pure val sampleWork={rate:1000000,seconds:1,total:1000000,owners:1,workers:4,frames:3,
    unit:"validated-four-line-order",arrival:"open-planned-to-consumed",startup:"fresh-process-per-arm",deadlineSeconds:10}
  pure val sampleKey={runId:"paired-main-1",artifact:"candidate-image",route:"op62"}
  pure val sampleLimits={p99Ns:418000,peak:128,wholeNs:3000000000}
  pure val sampleOutcome={planned:1000000,completed:1000000,samples:1000000,rejected:0,errors:0,
    p99Ns:300000,p999Ns:400000,peak:100,wholeNs:1010000000,clean:true,
    work:sampleWork,key:sampleKey,phase:"main",peakUncertainty:0,bucketNs:1000,quantileMethod:"histogram-upper"}
  pure val countCannotBeSubstitutedByAComponent=
    not(outcomeMatches({...sampleOutcome,planned:20000,completed:20000,samples:20000},sampleWork,sampleKey))
  pure val aDifferentInvocationImageOrRouteIsNotTheSameOutcome=
    not(outcomeMatches({...sampleOutcome,key:{...sampleKey,runId:"earlier-main"}},sampleWork,sampleKey)) and
    not(outcomeMatches({...sampleOutcome,key:{...sampleKey,artifact:"control-image"}},sampleWork,sampleKey)) and
    not(outcomeMatches({...sampleOutcome,key:{...sampleKey,route:"op56"}},sampleWork,sampleKey))
  pure val warmupAndCleanupCannotStandInForMain=
    Set("warmup","cleanup","initialization").forall(p=>
      not(outcomeMatches({...sampleOutcome,phase:p},sampleWork,sampleKey)) and
      not(absolutePassed({...sampleOutcome,phase:p},sampleLimits)))
  pure val negativePeakIsMissingEvidenceNotZeroBacklog=
    not(absolutePassed({...sampleOutcome,peak:-1},sampleLimits))
  pure val unknownOrCrossingPeakUncertaintyCannotPass=
    not(absolutePassed({...sampleOutcome,peakUncertainty:-1},sampleLimits)) and
    not(absolutePassed({...sampleOutcome,peak:127,peakUncertainty:2},sampleLimits)) and
    // Frozen reader accepts sampled 128; research still needs true upper <=128.
    not(absolutePassed({...sampleOutcome,peak:128,peakUncertainty:1},sampleLimits)) and
    absolutePassed({...sampleOutcome,peak:127,peakUncertainty:1},sampleLimits)
  pure val peakCannotExceedThePlannedPopulation=
    not(validCounts({...sampleOutcome,peak:1000001}))
  pure val contradictoryCountsCannotBeNormalized=
    not(validCounts({...sampleOutcome,completed:1000001})) and
    not(validCounts({...sampleOutcome,rejected:1})) and
    not(validCounts({...sampleOutcome,errors:-1}))
  pure val populationAndSampleCompletenessRemainSeparate=
    not(absolutePassed({...sampleOutcome,completed:999999,samples:999999},sampleLimits)) and
    not(absolutePassed({...sampleOutcome,samples:999999},sampleLimits))
  pure val quantilesCannotBeReorderedOrExceedTheirWindow=
    not(absolutePassed({...sampleOutcome,p999Ns:299999},sampleLimits)) and
    not(absolutePassed({...sampleOutcome,p999Ns:1010001001},sampleLimits))
  pure val bucketRoundingIsExplicitNotAChangedClock=
    absolutePassed({...sampleOutcome,planned:1,completed:1,samples:1,peak:1,
      work:{...sampleWork,rate:1,total:1},p99Ns:1000,p999Ns:2000,wholeNs:1001},sampleLimits) and
    not(absolutePassed({...sampleOutcome,bucketNs:-1},sampleLimits)) and
    not(absolutePassed({...sampleOutcome,bucketNs:0},sampleLimits))
  pure val exactAndHistogramQuantilesRequireDifferentEvidence=
    absolutePassed({...sampleOutcome,quantileMethod:"exact-order-statistic",bucketNs:0},sampleLimits) and
    not(absolutePassed({...sampleOutcome,quantileMethod:"unknown"},sampleLimits))
  pure val malformedWorkloadsCannotBeCertified=
    not(validWorkload({...sampleWork,rate:0})) and not(validWorkload({...sampleWork,seconds:-1})) and
    not(validWorkload({...sampleWork,total:5000000})) and not(validWorkload({...sampleWork,owners:0}))
  pure val relaxationCannotBeSmuggledThroughTheLimitsArgument=
    not(limitsWithinApproved({...sampleLimits,p99Ns:5000000000},sampleLimits)) and
    not(limitsWithinApproved({...sampleLimits,peak:1000000},sampleLimits)) and
    not(limitsWithinApproved({...sampleLimits,wholeNs:10000000000},sampleLimits)) and
    limitsWithinApproved({...sampleLimits,p99Ns:300000,peak:100},sampleLimits)
  pure val emptyIdentityAndInvalidLimitsStayUnknown=
    not(validKey({...sampleKey,runId:""})) and not(validKey({...sampleKey,artifact:""})) and
    not(validLimits({...sampleLimits,p99Ns:0})) and not(validLimits({...sampleLimits,peak:-1}))
  pure val completionCannotPrecedeTheLastPlannedArrival=
    not(absolutePassed({...sampleOutcome,wholeNs:1001,p99Ns:1000,p999Ns:1000},sampleLimits))
  // A GC count of zero does not authenticate allocation accounting across
  // continuation/stack segments. Keep the original signed observation.
  pure def usableAllocationSample(delta:int,gcCount:int,counterSemanticsQualified:bool):bool=
    delta>=0 and gcCount>=0 and counterSemanticsQualified
  pure val negativeAllocationWithNoGcCannotBeClampedIntoEvidence=
    not(usableAllocationSample(-17,0,true)) and
    not(usableAllocationSample(0,0,false)) and usableAllocationSample(17,0,true)
  // An enclosing wall window cannot be labeled exclusive GC CPU. The clock
  // scope and CPU accounting need independent qualification for that episode.
  pure def exclusiveGcCpuClaimAllowed(wall:int,cpu:int,sameEpisode:bool,exclusiveAccounting:bool):bool=
    wall>=0 and cpu>=0 and cpu<=wall and sameEpisode and exclusiveAccounting
  pure val gcWallEnvelopeIsNotExclusiveCpuOrAStallCause=
    not(exclusiveGcCpuClaimAllowed(708625,708625,true,false)) and
    not(exclusiveGcCpuClaimAllowed(708625,100000,false,true)) and
    not(exclusiveGcCpuClaimAllowed(708625,900000,true,true)) and
    exclusiveGcCpuClaimAllowed(708625,100000,true,true)
  // Frozen diagnostics reads GC time, calls a Scheme root-count export, then
  // reads GC count. Contributions are abstract nonnegative units, not timings.
  type GcContribution={time:int,count:int}
  pure def splitSnapshotDelta(beforeReentry:GcContribution,body:GcContribution,
      afterReentry:GcContribution):GcContribution={
    time:beforeReentry.time+body.time,count:body.count+afterReentry.count}
  pure val splitSnapshotZeroCountDoesNotExcludeTimeEnvelopeGc={
    val none={time:0,count:0}
    val read=splitSnapshotDelta({time:7,count:1},none,none)
    read=={time:7,count:0}
  }
  pure val splitSnapshotCountMayIncludeGcAbsentFromItsTime={
    val none={time:0,count:0}
    splitSnapshotDelta(none,none,{time:7,count:1})=={time:0,count:1}
  }
  pure val equalSplitSnapshotsDoNotIdentifyTheNativeGcCohort={
    val none={time:0,count:0}
    val firstBody={time:17,count:39}
    val secondBody={time:12,count:38}
    splitSnapshotDelta(none,firstBody,none)==
      splitSnapshotDelta({time:5,count:1},secondBody,{time:2,count:1}) and
      firstBody!=secondBody
  }
  pure val measurementChecks=and {splitSnapshotZeroCountDoesNotExcludeTimeEnvelopeGc,
    splitSnapshotCountMayIncludeGcAbsentFromItsTime,equalSplitSnapshotsDoNotIdentifyTheNativeGcCohort,
    negativeAllocationWithNoGcCannotBeClampedIntoEvidence,
    gcWallEnvelopeIsNotExclusiveCpuOrAStallCause,countCannotBeSubstitutedByAComponent,
    aDifferentInvocationImageOrRouteIsNotTheSameOutcome,warmupAndCleanupCannotStandInForMain,
    negativePeakIsMissingEvidenceNotZeroBacklog,unknownOrCrossingPeakUncertaintyCannotPass,
    peakCannotExceedThePlannedPopulation,contradictoryCountsCannotBeNormalized,
    populationAndSampleCompletenessRemainSeparate,quantilesCannotBeReorderedOrExceedTheirWindow,
    bucketRoundingIsExplicitNotAChangedClock,exactAndHistogramQuantilesRequireDifferentEvidence,
    malformedWorkloadsCannotBeCertified,relaxationCannotBeSmuggledThroughTheLimitsArgument,
    emptyIdentityAndInvalidLimitsStayUnknown,completionCannotPrecedeTheLastPlannedArrival}
}
module mailbox_measurement_tests {
  import mailbox_measurement_contract.*
  run splitSnapshotBeforeReentryTest=assert(splitSnapshotZeroCountDoesNotExcludeTimeEnvelopeGc)
  run splitSnapshotAfterReentryTest=assert(splitSnapshotCountMayIncludeGcAbsentFromItsTime)
  run splitSnapshotCohortAmbiguityTest=assert(equalSplitSnapshotsDoNotIdentifyTheNativeGcCohort)
  run signedAllocationTest=assert(negativeAllocationWithNoGcCannotBeClampedIntoEvidence)
  run gcWallIsNotCpuTest=assert(gcWallEnvelopeIsNotExclusiveCpuOrAStallCause)
  run componentCountTest=assert(countCannotBeSubstitutedByAComponent)
  run PreSourceAdmissionDelayDoesNotDirectlyChangeFixedInnerTraceTest={
    // Conditional unchanged post-anchor execution, not a performance model:
    // shifting cold admission shifts both planned and consumed timestamps.
    val ordinary={submitted:0,sourceAnchor:1000,consumed:12700,nativeEntry:900,nativeReturn:13000}
    val shift=6497000
    val delayed={...ordinary,sourceAnchor:ordinary.sourceAnchor+shift,
      consumed:ordinary.consumed+shift,nativeEntry:ordinary.nativeEntry+shift,
      nativeReturn:ordinary.nativeReturn+shift}
    assert(delayed.consumed-delayed.sourceAnchor==ordinary.consumed-ordinary.sourceAnchor and
      delayed.nativeReturn-delayed.nativeEntry==ordinary.nativeReturn-ordinary.nativeEntry and
      delayed.nativeReturn-delayed.submitted>ordinary.nativeReturn-ordinary.submitted)
  }
  run outcomeIdentityTest=assert(aDifferentInvocationImageOrRouteIsNotTheSameOutcome)
  run mainPhaseTest=assert(warmupAndCleanupCannotStandInForMain)
  run missingPeakTest=assert(negativePeakIsMissingEvidenceNotZeroBacklog)
  run uncertainPeakTest=assert(unknownOrCrossingPeakUncertaintyCannotPass)
  run impossiblePeakTest=assert(peakCannotExceedThePlannedPopulation)
  run contradictoryCountsTest=assert(contradictoryCountsCannotBeNormalized)
  run completePopulationTest=assert(populationAndSampleCompletenessRemainSeparate)
  run quantileWindowTest=assert(quantilesCannotBeReorderedOrExceedTheirWindow)
  run histogramRoundingTest=assert(bucketRoundingIsExplicitNotAChangedClock)
  run quantileMethodTest=assert(exactAndHistogramQuantilesRequireDifferentEvidence)
  run workloadValidityTest=assert(malformedWorkloadsCannotBeCertified)
  run approvedLimitsTest=assert(relaxationCannotBeSmuggledThroughTheLimitsArgument)
  run arrivalWindowTest=assert(completionCannotPrecedeTheLastPlannedArrival)
  run unknownIdentityTest=assert(emptyIdentityAndInvalidLimitsStayUnknown)
}
