// Evidence/admission refinement only; no duplicate Actor scheduler or lifecycle.
// Verified labels require external source/receipt evidence. A passing model
// cannot manufacture those labels or certify future CPU performance.
module mailbox_research_gate {
  import actor_service_contract as contract from "./actor_service_contract"
  import mailbox_clock_abi as abi from "./mailbox_clock_abi"

  import mailbox_measurement_contract.* from "./mailbox_measurement_contract"
  import mailbox_resource_contract as resource from "./mailbox_resource_contract"
  type Reference={revision:str,work:Workload,p99Ns:int,p999Ns:int,peak:int,
    fullyAdmitted:bool,originalBytesVerified:bool}
  pure val reference6D={revision:"6d4ec27",work:{rate:500000,seconds:10,total:5000000,
    owners:1,workers:4,frames:3,unit:"validated-four-line-order",arrival:"open-planned-to-consumed",
    startup:"paired-same-live-vm",deadlineSeconds:5},p99Ns:317000,p999Ns:431000,peak:351,
    fullyAdmitted:false,originalBytesVerified:false}
  pure val reference7F={revision:"7f915df",work:{rate:1000000,seconds:1,total:1000000,
    owners:1,workers:4,frames:3,unit:"validated-four-line-order",arrival:"open-planned-to-consumed",
    startup:"fresh-process-per-arm",deadlineSeconds:10},p99Ns:6393000,p999Ns:6740000,peak:6746,
    fullyAdmitted:false,originalBytesVerified:true}

  // These are source/operation facts, not filenames, changed hashes, or a list
  // of downstream projects. A repair requires new externally qualified proof.
  pure val retainedFactors=Set("delayed-refill", "new-queue-and-sleep-registration",
    "unhit-probe-stack-or-branch", "partial-source-or-provider-cohort",
    "capture-value-used-as-progress-error", "local-cost-only")
  pure val requiredFacts=Set("same-workload", "complete-link-inputs", "same-startup",
    "same-timing", "real-vm-correctness", "complete-cost-screen", "qualified-control")
  pure val replayFacts=Set("complete-link-inputs", "same-startup", "same-timing")
  pure val diagnosticFacts=replayFacts.union(Set("authorized-single-capture",
    "qualified-unhit-path", "same-request-association", "real-vm-correctness"))
  pure val forbiddenFactors=Set("private-inbox-read", "premature-capacity-return",
    "narrowed-scheme-semantics", "changed-defaults", "relaxed-gates", "sdk-modified")
  type Plan={control:Workload,candidate:Workload,key:RunKey,verified:Set[str],introduced:Set[str],
    repaired:Set[str],repairsQualified:bool,alreadyAttempted:bool,newMechanism:bool,
    newQuestion:bool,explicitReplay:bool,purpose:str,origin:str,originalBytesVerified:bool,scope:str}
  pure def unresolved(s:Research,p:Plan):Set[str]=p.introduced.intersect(s.learned)
    .exclude(if(p.repairsQualified) p.repaired else Set()).union(p.introduced.intersect(forbiddenFactors))
  pure def novelAttempt(p:Plan):bool=not(p.alreadyAttempted) or p.newMechanism
  pure def pairedRunRejections(s:Research,p:Plan):Set[str]=requiredFacts.exclude(p.verified)
    .union(if(validAuthority(s)) Set() else Set("invalid-research-authority"))
    .union(if(validWorkload(p.control) and validWorkload(p.candidate) and validKey(p.key)) Set() else Set("invalid-workload-or-key"))
    .union(unresolved(s,p))
    .union(if(p.purpose=="performance-ab") Set() else Set("not-performance-ab"))
    .union(if(p.control==p.candidate) Set() else Set("workload-mismatch"))
    .union(if(novelAttempt(p)) Set() else Set("unchanged-repeat"))
  pure def pairedRunAllowed(s:Research,p:Plan):bool=pairedRunRejections(s,p)==Set()
  // Reproduction and a single new diagnostic question remain possible. They
  // are not eligible to publish an engine gain, even if the command exits 0.
  pure def replayAllowed(s:Research,p:Plan):bool=validAuthority(s) and validWorkload(p.control) and validKey(p.key) and p.purpose=="baseline-replay" and p.explicitReplay and
    p.control==p.candidate and replayFacts.subseteq(p.verified) and unresolved(s,p)==Set()
  pure def diagnosticAllowed(s:Research,p:Plan):bool=validAuthority(s) and validWorkload(p.control) and validKey(p.key) and p.purpose=="single-diagnostic" and p.newQuestion and
    p.control==p.candidate and diagnosticFacts.subseteq(p.verified) and unresolved(s,p)==Set()
  pure def sameHistoricalWorkload(r:Reference,p:Plan):bool=r.work==p.control and r.work==p.candidate
  pure def identicalHistoricalBytesClaimAllowed(p:Plan):bool=
    p.origin=="original-bytes" and p.originalBytesVerified and replayFacts.subseteq(p.verified)

  // Limits and learned failures belong to retained research state, not to the
  // proposed candidate. Fixed references cannot be ratcheted by a slow control.
  type Research={six:Reference,seven:Reference,runtimeCandidate:bool,learned:Set[str],approvedLimits:Limits}
  pure val limits={p99Ns:418000,peak:128,wholeNs:3000000000}
  pure val authority={six:reference6D,seven:reference7F,runtimeCandidate:false,
    learned:retainedFactors,approvedLimits:limits}
  pure def validAuthority(s:Research):bool=s.six==reference6D and s.seven==reference7F and
    retainedFactors.subseteq(s.learned) and s.approvedLimits==limits
  pure def engineGainClaimAllowed(s:Research,p:Plan,o:Outcome,l:Limits,effect:str):bool=
    pairedRunAllowed(s,p) and outcomeMatches(o,p.candidate,p.key) and
    limitsWithinApproved(l,s.approvedLimits) and p.scope=="engine" and absolutePassed(o,l) and
    contract::maySeekAdmissionReview({hardFailure:false,relativeEffect:effect,absolutePassed:true})
  pure def rollbackAndLearn(s:Research,factors:Set[str]):Research=
    {...s,runtimeCandidate:false,learned:s.learned.union(retainedFactors).union(factors)}

  pure val goodPlan={control:reference7F.work,candidate:reference7F.work,key:sampleKey,verified:requiredFacts,
    introduced:Set(),repaired:Set(),repairsQualified:false,alreadyAttempted:false,newMechanism:true,
    newQuestion:false,explicitReplay:false,purpose:"performance-ab",origin:"paired-rebuild",
    originalBytesVerified:false,scope:"engine"}
  pure val goodOutcome=sampleOutcome

  pure val differentFrozenProfilesCannotBeComparedAsOneRun=
    reference6D.work!=reference7F.work and
    not(pairedRunAllowed(authority,{...goodPlan,control:reference6D.work})) and
    not(sameHistoricalWorkload(reference6D,goodPlan)) and sameHistoricalWorkload(reference7F,goodPlan)
  pure val historicalReferencesRemainPartial=
    not(reference6D.fullyAdmitted) and not(reference7F.fullyAdmitted) and
    reference6D.peak>limits.peak and reference7F.peak>limits.peak
  pure val unitsAndArrivalModesMustMatch=
    not(pairedRunAllowed(authority,{...goodPlan,candidate:{...reference7F.work,unit:"ring-hop"}})) and
    not(pairedRunAllowed(authority,{...goodPlan,candidate:{...reference7F.work,arrival:"closed-loop"}}))
  pure val verifiedLabelsCannotCoverAnOmittedProof={
    val obligations=Set("same-workload", "complete-link-inputs", "same-startup",
      "same-timing", "real-vm-correctness", "complete-cost-screen", "qualified-control")
    obligations.subseteq(requiredFacts) and
      obligations.forall(f=>not(pairedRunAllowed(authority,{...goodPlan,verified:requiredFacts.exclude(Set(f))})))
  }
  pure val matchingActorCannotAuthenticateTheProvider=
    abi::matchingActorDoesNotAuthenticateTheCProvider and
    not(pairedRunAllowed(authority,{...goodPlan,verified:requiredFacts.exclude(Set("complete-link-inputs"))}))
  pure val aRebuildCannotClaimOriginalBytes=
    not(identicalHistoricalBytesClaimAllowed(goodPlan)) and
    identicalHistoricalBytesClaimAllowed({...goodPlan,origin:"original-bytes",originalBytesVerified:true})
  pure val badControlCannotProveAnEngineGain=
    not(engineGainClaimAllowed(authority,{...goodPlan,verified:requiredFacts.exclude(Set("qualified-control"))},
      goodOutcome,limits,"qualified-gain"))
  pure val correctnessAndZeroRejectionDoNotWaiveTails=
    not(absolutePassed({...goodOutcome,p99Ns:107153000,p999Ns:108955000,peak:84335},limits)) and
    not(absolutePassed({...goodOutcome,p99Ns:164041000,p999Ns:167776000,peak:77695},limits))
  pure val localAndHarnessSavingsAreNotEngineAdmission=
    not(pairedRunAllowed(authority,{...goodPlan,introduced:Set("local-cost-only")})) and
    not(engineGainClaimAllowed(authority,{...goodPlan,scope:"harness"},goodOutcome,limits,"qualified-gain"))
  pure val knownMechanismsRequireAnActualQualifiedRepair=
    retainedFactors.forall(f=>not(pairedRunAllowed(authority,{...goodPlan,introduced:Set(f)}))) and
    not(pairedRunAllowed(authority,{...goodPlan,introduced:Set("unhit-probe-stack-or-branch"),
      repaired:Set("unhit-probe-stack-or-branch")})) and
    pairedRunAllowed(authority,{...goodPlan,introduced:Set("unhit-probe-stack-or-branch"),
      repaired:Set("unhit-probe-stack-or-branch"),repairsQualified:true})
  pure val unchangedFailedAttemptCannotBeRepeatedByRenaming=
    not(pairedRunAllowed(authority,{...goodPlan,alreadyAttempted:true,newMechanism:false})) and
    not(pairedRunAllowed(authority,{...goodPlan,alreadyAttempted:true,newMechanism:false,newQuestion:true})) and
    diagnosticAllowed(authority,{...goodPlan,alreadyAttempted:true,newMechanism:false,newQuestion:true,
      purpose:"single-diagnostic",verified:diagnosticFacts})
  pure val explicitReplayIsNotAnEngineGain=
    replayAllowed(authority,{...goodPlan,purpose:"baseline-replay",explicitReplay:true,verified:replayFacts}) and
    not(engineGainClaimAllowed(authority,{...goodPlan,purpose:"baseline-replay",explicitReplay:true},
      goodOutcome,limits,"qualified-gain"))
  pure val rollbackPreservesBothReferencesAndCounterexamples={
    val old={six:reference6D,seven:reference7F,runtimeCandidate:true,approvedLimits:limits,learned:Set("delayed-refill")}
    val rolled=rollbackAndLearn(old,Set("unhit-probe-stack-or-branch"))
    rolled.six==old.six and rolled.seven==old.seven and not(rolled.runtimeCandidate) and
      old.learned.subseteq(rolled.learned) and rolled.learned.contains("unhit-probe-stack-or-branch")
  }
  // A single 0.026% delta cannot produce a qualified-regression label here.
  // That label is external evidence; absolute 128 admission remains separate.
  pure val smallUnconfirmedVariationIsNotAutomaticWithdrawal=
    contract::researchDisposition({hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false})=="retain-isolated" and
    not(contract::maySeekAdmissionReview({hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false})) and
    contract::researchDisposition({hardFailure:false,relativeEffect:"qualified-regression",absolutePassed:false})=="withdraw"
  pure val missingOrCensoredSamplesCannotBecomeAFullP99=
    not(absolutePassed({...goodOutcome,samples:999999},limits)) and
    not(absolutePassed({...goodOutcome,completed:999999,samples:999999},limits)) and
    not(absolutePassed({...goodOutcome,p999Ns:-1},limits))
  pure val validEvidenceCanStillEnterResearchAndSeekReview=
    pairedRunAllowed(authority,goodPlan) and engineGainClaimAllowed(authority,goodPlan,goodOutcome,limits,"qualified-gain") and
    not(engineGainClaimAllowed(authority,goodPlan,goodOutcome,limits,"unconfirmed"))
  pure val forbiddenChangesCannotBeSelfCertifiedAsRepairs=
    forbiddenFactors.forall(f=>not(pairedRunAllowed(authority,{...goodPlan,introduced:Set(f),
      repaired:Set(f),repairsQualified:true})))
  pure val aNewQuestionStillRequiresSameRequestAndUnhitProof={
    val obligations=Set("complete-link-inputs", "same-startup", "same-timing",
      "authorized-single-capture", "qualified-unhit-path", "same-request-association", "real-vm-correctness")
    obligations.subseteq(diagnosticFacts) and
      obligations.forall(f=>not(diagnosticAllowed(authority,{...goodPlan,purpose:"single-diagnostic",
        newQuestion:true,verified:diagnosticFacts.exclude(Set(f))})))
  }
  pure def causeClaimAllowed(facts:Set[str]):bool=
    Set("same-gap", "mechanism-reached", "measurement-interference-qualified",
      "distinguishing-evidence").subseteq(facts)
  pure val learnedNewFailuresFeedTheNextAdmission={
    val old={six:reference6D,seven:reference7F,runtimeCandidate:true,approvedLimits:limits,learned:retainedFactors}
    val restored=rollbackAndLearn(old,Set("newly-observed-progress-side-channel"))
    val candidate={...goodPlan,introduced:Set("newly-observed-progress-side-channel")}
    not(pairedRunAllowed(restored,candidate)) and restored.six==reference6D and restored.seven==reference7F and
      not(restored.runtimeCandidate) and old.learned.subseteq(restored.learned)
  }
  pure val aGoodP99CannotHideTheOriginal351Backlog=
    not(absolutePassed({...goodOutcome,p99Ns:317000,p999Ns:431000,peak:351},limits))
  pure val correlationAndConditionalReachabilityCannotEstablishTheCause={
    val needed=Set("same-gap", "mechanism-reached", "measurement-interference-qualified",
      "distinguishing-evidence")
    not(causeClaimAllowed(Set("owner-runnable", "thread-priority-restored"))) and
      not(causeClaimAllowed(Set("conditional-c-fixture-passed"))) and
      needed.forall(f=>not(causeClaimAllowed(needed.exclude(Set(f))))) and causeClaimAllowed(needed)
  }
  // Historical 5M/s offering is not 5M/s delivered capacity. This row is
  // a retained negative result of 7F, not a replacement reference.
  pure val negative5M={rate:5000000,total:5000000,elapsedNs:4984038584,wholeNs:5038517042,
    p99Ns:3943455000,p999Ns:3980261000,peak:4000404}
  pure def deliveredCapacityAtLeast(completed:int,elapsedNs:int,rate:int):bool=
    completed>0 and elapsedNs>0 and rate>0 and completed*1000000000>=rate*elapsedNs
  pure val offeredRateCannotBeReportedAsDeliveredCapacity=
    not(deliveredCapacityAtLeast(negative5M.total,negative5M.elapsedNs,negative5M.rate)) and
    not(absolutePassed({...goodOutcome,planned:negative5M.total,completed:negative5M.total,
      samples:negative5M.total,work:{...goodOutcome.work,rate:negative5M.rate,total:negative5M.total},p99Ns:negative5M.p99Ns,p999Ns:negative5M.p999Ns,
      peak:negative5M.peak,wholeNs:negative5M.wholeNs},limits))
  pure val candidateCannotSupplyItsOwnEvidenceIdentity=
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,key:{...sampleKey,runId:"old-run"}},limits,"qualified-gain")) and
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,key:{...sampleKey,artifact:"other-image"}},limits,"qualified-gain")) and
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,key:{...sampleKey,route:"op56"}},limits,"qualified-gain")) and
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,phase:"warmup"},limits,"qualified-gain"))
  pure val aSmallValidComponentCannotAuthenticateTheMainPopulation={
    val smallWork={...goodOutcome.work,rate:20000,total:20000}
    val small={...goodOutcome,work:smallWork,planned:20000,completed:20000,samples:20000}
    absolutePassed(small,limits) and not(engineGainClaimAllowed(authority,goodPlan,small,limits,"qualified-gain"))
  }
  pure val callerCannotRelaxApprovedAdmission=
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,peak:351},
      {...limits,peak:351},"qualified-gain")) and
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,p99Ns:6393000,p999Ns:6740000},
      {...limits,p99Ns:10000000},"qualified-gain"))
  pure val forgettingHistoryOrRatchetingReferencesInvalidatesAuthority=
    not(pairedRunAllowed({...authority,learned:Set()},goodPlan)) and
    not(pairedRunAllowed({...authority,six:{...reference6D,p99Ns:100000000}},goodPlan)) and
    not(pairedRunAllowed({...authority,approvedLimits:{...limits,peak:351}},goodPlan))
  pure val negativeOrUncertainPeakCannotPassEngineAdmission=
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,peak:-1},limits,"qualified-gain")) and
    not(engineGainClaimAllowed(authority,goodPlan,{...goodOutcome,peak:127,peakUncertainty:2},limits,"qualified-gain"))
  pure val successiveRollbacksCannotLoseEarlierNewFailures={
    val first=rollbackAndLearn({...authority,runtimeCandidate:true},Set("new-failure-a"))
    val second=rollbackAndLearn({...first,runtimeCandidate:true},Set("new-failure-b"))
    val third=rollbackAndLearn(second,Set())
    first.learned.subseteq(second.learned) and second.learned.subseteq(third.learned) and
      not(pairedRunAllowed(third,{...goodPlan,introduced:Set("new-failure-a")})) and
      not(pairedRunAllowed(third,{...goodPlan,introduced:Set("new-failure-b")})) and
      third.six==authority.six and third.seven==authority.seven and not(third.runtimeCandidate)
  }
  pure val allCounterexamples=and {successiveRollbacksCannotLoseEarlierNewFailures,
    measurementChecks,resource::resourceChecks,candidateCannotSupplyItsOwnEvidenceIdentity,
    aSmallValidComponentCannotAuthenticateTheMainPopulation,callerCannotRelaxApprovedAdmission,
    forgettingHistoryOrRatchetingReferencesInvalidatesAuthority,negativeOrUncertainPeakCannotPassEngineAdmission,
    differentFrozenProfilesCannotBeComparedAsOneRun,historicalReferencesRemainPartial,
    unitsAndArrivalModesMustMatch,verifiedLabelsCannotCoverAnOmittedProof,
    matchingActorCannotAuthenticateTheProvider,aRebuildCannotClaimOriginalBytes,
    badControlCannotProveAnEngineGain,correctnessAndZeroRejectionDoNotWaiveTails,
    localAndHarnessSavingsAreNotEngineAdmission,knownMechanismsRequireAnActualQualifiedRepair,
    unchangedFailedAttemptCannotBeRepeatedByRenaming,explicitReplayIsNotAnEngineGain,
    rollbackPreservesBothReferencesAndCounterexamples,smallUnconfirmedVariationIsNotAutomaticWithdrawal,
    missingOrCensoredSamplesCannotBecomeAFullP99,validEvidenceCanStillEnterResearchAndSeekReview,
    forbiddenChangesCannotBeSelfCertifiedAsRepairs,aNewQuestionStillRequiresSameRequestAndUnhitProof,
    learnedNewFailuresFeedTheNextAdmission,aGoodP99CannotHideTheOriginal351Backlog,
    correlationAndConditionalReachabilityCannotEstablishTheCause,offeredRateCannotBeReportedAsDeliveredCapacity
  }
}
module mailbox_research_gate_tests {
  import mailbox_research_gate.*
  run differentFrozenProfilesTest=assert(differentFrozenProfilesCannotBeComparedAsOneRun)
  run partialReferencesTest=assert(historicalReferencesRemainPartial)
  run workloadUnitsTest=assert(unitsAndArrivalModesMustMatch)
  run everyProofRequiredTest=assert(verifiedLabelsCannotCoverAnOmittedProof)
  run providerProvenanceTest=assert(matchingActorCannotAuthenticateTheProvider)
  run rebuiltBytesTest=assert(aRebuildCannotClaimOriginalBytes)
  run anomalousControlTest=assert(badControlCannotProveAnEngineGain)
  run zeroRejectionIsInsufficientTest=assert(correctnessAndZeroRejectionDoNotWaiveTails)
  run harnessIsNotEngineTest=assert(localAndHarnessSavingsAreNotEngineAdmission)
  run knownFailedMechanismsTest=assert(knownMechanismsRequireAnActualQualifiedRepair)
  run unchangedRepeatTest=assert(unchangedFailedAttemptCannotBeRepeatedByRenaming)
  run explicitReplayTest=assert(explicitReplayIsNotAnEngineGain)
  run preserveReferencesOnRollbackTest=assert(rollbackPreservesBothReferencesAndCounterexamples)
  run normalVariationTest=assert(smallUnconfirmedVariationIsNotAutomaticWithdrawal)
  run censoredSamplesTest=assert(missingOrCensoredSamplesCannotBecomeAFullP99)
  run forbiddenChangesTest=assert(forbiddenChangesCannotBeSelfCertifiedAsRepairs)
  run diagnosticProofTest=assert(aNewQuestionStillRequiresSameRequestAndUnhitProof)
  run learnedFailureFeedbackTest=assert(learnedNewFailuresFeedTheNextAdmission)
  run original351BacklogTest=assert(aGoodP99CannotHideTheOriginal351Backlog)
  run causalAttributionTest=assert(correlationAndConditionalReachabilityCannotEstablishTheCause)
  run offeredCapacityTest=assert(offeredRateCannotBeReportedAsDeliveredCapacity)
  run boundOutcomeIdentityTest=assert(candidateCannotSupplyItsOwnEvidenceIdentity)
  run componentIsNotMainTest=assert(aSmallValidComponentCannotAuthenticateTheMainPopulation)
  run immutableLimitsTest=assert(callerCannotRelaxApprovedAdmission)
  run trustedHistoryTest=assert(forgettingHistoryOrRatchetingReferencesInvalidatesAuthority)
  run validPeakEvidenceTest=assert(negativeOrUncertainPeakCannotPassEngineAdmission)
  run cumulativeRollbackTest=assert(successiveRollbacksCannotLoseEarlierNewFailures)
  run nonVacuousAdmissionTest=assert(validEvidenceCanStillEnterResearchAndSeekReview)
}
