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
  // Matched PR op62 images, one million planned arrivals and a five-second
  // deadline. GC measurements enclose the owner call and are not a tail gate.
  pure val lowerCollectionCostDoesNotAdmitTheHistogramCandidate={
    val work={...sampleWork,deadlineSeconds:5}
    val control={outcome:{...sampleOutcome,work:work,
      key:{...sampleKey,artifact:"pr18-matched-control"},
      p99Ns:2993000,p999Ns:3088000,peak:3113,wholeNs:1011121917},
      collections:74,gcWallNs:27418613}
    val candidate={outcome:{...sampleOutcome,work:work,
      key:{...sampleKey,artifact:"pr18-matched-histogram"},
      p99Ns:6562000,p999Ns:6782000,peak:6789,wholeNs:1015049875},
      collections:39,gcWallNs:15584707}
    outcomeMatches(control.outcome,work,control.outcome.key) and
    outcomeMatches(candidate.outcome,work,candidate.outcome.key) and
    candidate.collections<control.collections and candidate.gcWallNs<control.gcWallNs and
    candidate.outcome.p99Ns>control.outcome.p99Ns and candidate.outcome.peak>control.outcome.peak and
    not(absolutePassed(control.outcome,sampleLimits)) and
    not(absolutePassed(candidate.outcome,sampleLimits))
  }
  // Total owner CPU includes the zero-inflight source yield loop. Without
  // component-exclusive accounting, payload cost is not identifiable.
  pure def cpuDecompositionValid(total:int,payload:int,idle:int,other:int):bool=
    total>=0 and payload>=0 and idle>=0 and other>=0 and total==payload+idle+other
  pure val identicalOwnerCpuCannotIdentifyPayloadCpu=
    cpuDecompositionValid(1000000000,1000000,998000000,1000000) and
    cpuDecompositionValid(1000000000,900000000,99000000,1000000)
  // Only a thread-CPU window that ENCLOSES the wall interval supplies an
  // upper bound for its CPU. The successful reader's larger returned whole
  // field also contains report copying after the second CPU snapshot.
  pure def enclosedNonCpuLowerNs(wall:int,cpuUpper:int,error:int,sameOwner:bool,enclosed:bool):int=
    if(wall>=0 and cpuUpper>=0 and error>=0 and sameOwner and enclosed)
      if(wall>cpuUpper+error) wall-cpuUpper-error else 0
    else -1
  pure val sourceCpuEnclosureProvidesOnlyAnAggregateLowerBound=
    enclosedNonCpuLowerNs(1239239875,953310960,1000,true,true)==285927915 and
    Set(0,1,100).forall(cpu=>Set(0,1,100).forall(blocked=>
      Set(0,1,100).forall(overhead=>
        enclosedNonCpuLowerNs(cpu+blocked,cpu+overhead,1,true,true)<=blocked)))
  pure val reportOutsideCpuSnapshotCannotBorrowTheEnclosure=
    enclosedNonCpuLowerNs(1333752083,953310960,1000,true,false)==-1 and
    enclosedNonCpuLowerNs(1239239875,953310960,1000,false,true)==-1 and
    enclosedNonCpuLowerNs(1000,2000,1,true,true)==0 and
    enclosedNonCpuLowerNs(1000,-1,1,true,true)==-1
  // Equal aggregate counters permit all-blocked, all-runnable-wait, and many
  // disjoint-pause explanations. They cannot identify a cause or a max gap.
  pure def sourceTimeBudget(wall:int,cpu:int,blocked:int,runnableWait:int):bool=
    cpu>=0 and blocked>=0 and runnableWait>=0 and wall==cpu+blocked+runnableWait
  pure val aggregateNonCpuCannotIdentifyBlockedOrRunnableWait=
    sourceTimeBudget(1239239875,953310959,285928916,0) and
    sourceTimeBudget(1239239875,953310959,0,285928916) and
    285928916==142964458+142964458
  // Conditional source-loop proof, not a generic sampler guarantee. The
  // reviewed standard owned-source loop samples before supply/receive, and
  // increments completed at most once before the next sample. Due is monotone.
  pure def oneCompletionInterval(laterDue:int,priorCompleted:int,step:int):bool=
    priorCompleted>=0 and 0<=step and step<=1 and priorCompleted+step<=laterDue
  pure def sampledAfterCompletion(laterDue:int,priorCompleted:int,step:int):int=
    laterDue-priorCompleted-step
  pure def closedOwnedSourcePeakUpper(sampled:int,total:int,complete:bool,protocolKnown:bool):int=
    if(complete and protocolKnown and total>0 and 0<=sampled and sampled<=total)
      if(sampled+1<total) sampled+1 else total
    else -1
  pure val oneCompletionSamplingGapHasOneMessageUncertainty=
    Set(0,1000,999999).forall(c=>Set(0,1,127,128,129,1000000).forall(debt=>
      Set(0,1).forall(step=>not(oneCompletionInterval(c+debt,c,step)) or
        c+debt-c<=sampledAfterCompletion(c+debt,c,step)+1)))
  pure val raw128SampleCannotCertifyTheTrue128Peak=
    sampledAfterCompletion(129,0,1)==128 and
    closedOwnedSourcePeakUpper(128,1000000,true,true)==129 and
    closedOwnedSourcePeakUpper(127,1000000,true,true)==128 and
    not(oneCompletionInterval(129,0,2)) and
    129>sampledAfterCompletion(129,0,2)+1
  pure val incompleteOrUnqualifiedSourceCannotBorrowThisBound=
    closedOwnedSourcePeakUpper(3460,1000000,true,true)==3461 and
    closedOwnedSourcePeakUpper(1,1,true,true)==1 and
    closedOwnedSourcePeakUpper(128,1000000,false,true)==-1 and
    closedOwnedSourcePeakUpper(128,1000000,true,false)==-1 and
    closedOwnedSourcePeakUpper(-1,1000000,true,true)==-1
  pure val measurementChecks=and {sourceCpuEnclosureProvidesOnlyAnAggregateLowerBound,reportOutsideCpuSnapshotCannotBorrowTheEnclosure,aggregateNonCpuCannotIdentifyBlockedOrRunnableWait,oneCompletionSamplingGapHasOneMessageUncertainty,raw128SampleCannotCertifyTheTrue128Peak,incompleteOrUnqualifiedSourceCannotBorrowThisBound,identicalOwnerCpuCannotIdentifyPayloadCpu,negativeAllocationWithNoGcCannotBeClampedIntoEvidence,
    lowerCollectionCostDoesNotAdmitTheHistogramCandidate,
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
  run sourceCpuEnclosureTest=assert(sourceCpuEnclosureProvidesOnlyAnAggregateLowerBound)
  run reportOutsideCpuSnapshotTest=assert(reportOutsideCpuSnapshotCannotBorrowTheEnclosure)
  run aggregateWaitAmbiguityTest=assert(aggregateNonCpuCannotIdentifyBlockedOrRunnableWait)
  run signedAllocationTest=assert(negativeAllocationWithNoGcCannotBeClampedIntoEvidence)
  run ownerCpuIsNotPayloadCpuTest=assert(identicalOwnerCpuCannotIdentifyPayloadCpu)
  run ownedSourceOneCompletionGapTest=assert(oneCompletionSamplingGapHasOneMessageUncertainty)
  run sampled128IsNotTrue128Test=assert(raw128SampleCannotCertifyTheTrue128Peak)
  run conditionalPeakBoundTest=assert(incompleteOrUnqualifiedSourceCannotBorrowThisBound)
  run gcWallIsNotCpuTest=assert(gcWallEnvelopeIsNotExclusiveCpuOrAStallCause)
  run gcReductionDoesNotAdmitHistogramTest=assert(lowerCollectionCostDoesNotAdmitTheHistogramCandidate)
  run componentCountTest=assert(countCannotBeSubstitutedByAComponent)
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
