// Refinement of existing layout dispatch, not another queue or scheduler.
module mailbox_layout_entry {
  import actor_service_contract as contract from "./actor_service_contract"
  import mailbox_measurement_contract as measurement from "./mailbox_measurement_contract"
  pure def originalRoute(work:bool,specialized:bool):str=
    if(work and specialized) "specialized" else if(work) "generic" else "service-unit"
  pure def directRoute(work:bool,specialized:bool):str=
    if(not(work)) "service-unit" else if(specialized) "specialized" else "generic"
  pure val routesAgree=Set(false,true).forall(w=>Set(false,true).forall(s=>originalRoute(w,s)==directRoute(w,s)))
  // Work=false is not Context=false: no cached plan does not waive a lease.
  pure val missingPlanCannotWaiveTheExistingUnitGate=
    not(contract::unitEntrySafe({snapshotContext:true,currentContext:true,executing:true,leaseHeld:false})) and
    contract::unitEntrySafe({snapshotContext:true,currentContext:true,executing:true,leaseHeld:true})
  pure val snapshotCannotReplaceTheCurrentContextRead=
    not(contract::unitEntrySafe({snapshotContext:false,currentContext:true,executing:true,leaseHeld:false}))
  // No before/after state, lease or root is added by the direct lexical body.
  // An escaped body remains under the ORIGINAL service-unit dynamic extent.
  pure def equivalentEvents(old:List[str],candidate:List[str]):bool=old==candidate
  pure val guardedBodyKeepsObservableOrder=
    equivalentEvents(List("context-read","unit-entry","body","unit-exit"),List("context-read","unit-entry","body","unit-exit")) and
    not(equivalentEvents(List("context-read","unit-entry","body","unit-exit"),List("context-read","body")))
  pure def originalTrace(body:List[str]):List[str]=List("layout-closure","layout-dispatch","wrapper-closure","unit-reread").concat(body)
  pure def candidateTrace(body:List[str]):List[str]=List("direct-closure","unit-reread").concat(body)
  pure def observable(t:List[str]):List[str]=t.foldl(List(),(kept,e)=>if(Set("layout-closure","layout-dispatch","wrapper-closure","direct-closure").contains(e)) kept else kept.concat(List(e)))
  pure val abnormalAndReplayCutsRemainUnitOwned=
    Set(List("entry","raise-original","exit"),List("entry","capture","exit","reenter","body","exit"),
      List("entry","gc","suspend","resume","body","exit")).forall(t=>
        equivalentEvents(observable(originalTrace(t)),observable(candidateTrace(t))))
  // Static allocation/dispatch savings are conditional inputs, not ns or an
  // admission certificate. Any added layout/stack cost must be paid in full.
  pure def netCost(base:int,saved:int,extra:int):int=base-saved+extra
  pure val extraWorkCanEraseTheClaim=
    netCost(100,20,21)>100 and netCost(100,20,20)==100 and netCost(100,20,10)<100
  pure val closureCountIsNotA128DebtProof=
    not(contract::evidenceQualified({cost:"static-sites-only",availability:"unknown",fairness:"unknown",
      retirementTail:"included",sourceMatched:true,readPrivateInbox:false,rewoundCursor:false}))
  // Trial sequence only. This is not a second Actor lifecycle or scheduler.
  // A review-qualified event needs an external current-control receipt; pinning
  // an old image or finishing warmup is not such a receipt.
  pure def trialTraceSafe(events:List[str]):bool={
    val end=events.foldl({started:false,finished:false,reviewed:false,qualified:false,begunB:false,invalid:false},(s,e)=>
      if(e=="control-start") {...s,started:true,invalid:s.invalid or s.started}
      else if(e=="control-finished") {...s,finished:true,invalid:s.invalid or not(s.started) or s.finished}
      else if(e=="review-qualified" or e=="review-unqualified")
        {...s,reviewed:true,qualified:e=="review-qualified",invalid:s.invalid or not(s.finished) or s.reviewed}
      else if(e=="candidate-start") {...s,begunB:true,
        invalid:s.invalid or not(s.finished and s.reviewed and s.qualified) or s.begunB}
      else if(Set("pin-original","warmup-finished").contains(e)) s else {...s,invalid:true})
    not(end.invalid)
  }
  pure val combinedLauncherCannotSkipCurrentControlReview=
    not(trialTraceSafe(List("pin-original","control-start","control-finished","candidate-start"))) and
    not(trialTraceSafe(List("control-start","control-finished","review-unqualified","candidate-start"))) and
    trialTraceSafe(List("control-start","control-finished","review-unqualified")) and
    trialTraceSafe(List("control-start","control-finished","review-qualified","candidate-start"))
  pure val warmupAndOriginalBytesCannotQualifyTheCurrentMain=
    not(trialTraceSafe(List("pin-original","control-start","warmup-finished","review-qualified","candidate-start"))) and
    not(trialTraceSafe(List("control-finished","review-qualified","candidate-start")))
  // Exact retained main observations. No inferred phase CPU or native cause.
  pure val controlObservation={...measurement::sampleOutcome,
    key:{runId:"direct-entry-control",artifact:"7f-original",route:"op62"},
    p99Ns:344430000,p999Ns:344554000,peak:228454,peakUncertainty:-1,wholeNs:1358234583}
  pure val candidateObservation={...measurement::sampleOutcome,
    key:{runId:"direct-entry-candidate",artifact:"isolated-layout-prototype",route:"op62"},
    p99Ns:662645000,p999Ns:662727000,peak:405237,peakUncertainty:-1,wholeNs:1680354041}
  pure val wrapperSavingsAndCorrectnessCannotAdmitThisTrial=
    controlObservation.completed==controlObservation.planned and candidateObservation.completed==candidateObservation.planned and
    controlObservation.peak>measurement::sampleLimits.peak and candidateObservation.peak>measurement::sampleLimits.peak and
    not(measurement::absolutePassed(controlObservation,measurement::sampleLimits)) and
    not(measurement::absolutePassed(candidateObservation,measurement::sampleLimits))
  pure val mostPeakDebtIsUnpublishedWithoutLocatingItsCause=
    controlObservation.peak-3==228451 and candidateObservation.peak-3==405234 and
    controlObservation.peak-3>128 and candidateObservation.peak-3>128
  pure val allRules=and {combinedLauncherCannotSkipCurrentControlReview,
    warmupAndOriginalBytesCannotQualifyTheCurrentMain,wrapperSavingsAndCorrectnessCannotAdmitThisTrial,
    mostPeakDebtIsUnpublishedWithoutLocatingItsCause,routesAgree,missingPlanCannotWaiveTheExistingUnitGate,snapshotCannotReplaceTheCurrentContextRead,
    guardedBodyKeepsObservableOrder,abnormalAndReplayCutsRemainUnitOwned,extraWorkCanEraseTheClaim,closureCountIsNotA128DebtProof}
}
module mailbox_layout_entry_tests {
 import mailbox_layout_entry.*
 run reviewBeforeCandidateTest=assert(combinedLauncherCannotSkipCurrentControlReview)
 run mainControlReceiptTest=assert(warmupAndOriginalBytesCannotQualifyTheCurrentMain)
 run failedCompleteTrialTest=assert(wrapperSavingsAndCorrectnessCannotAdmitThisTrial)
 run sourceDebtLowerBoundTest=assert(mostPeakDebtIsUnpublishedWithoutLocatingItsCause)
 run routePartitionTest=assert(routesAgree)
 run noPlanStillHasLeaseTest=assert(missingPlanCannotWaiveTheExistingUnitGate)
 run rereadContextTest=assert(snapshotCannotReplaceTheCurrentContextRead)
 run keepUnitBoundaryTest=assert(guardedBodyKeepsObservableOrder)
 run replayAndFailureTest=assert(abnormalAndReplayCutsRemainUnitOwned)
 run completeCostTest=assert(extraWorkCanEraseTheClaim)
 run noPerformanceCertificateTest=assert(closureCountIsNotA128DebtProof)
}
