// Proposed replacement of two EXISTING leaf calls, not a new consumer,
// queue, timer, clock, capacity authority or runtime evidence certificate.
module mailbox_consume_abi {
  import mailbox_joint_cycle as j from "./mailbox_joint_cycle"
  type Case={eligible:bool,clockClass:str,progressOk:bool}
  pure def hasEvent(xs:List[str],event:str):bool=xs.foldl(false,(found,e)=>found or e==event)
  pure val valid={eligible:true,clockClass:"valid",progressOk:true}
  pure val prefix=List("validate-frame","propagate-worker-failure","sample-consume")
  // Before-plan is a legacy arithmetic path. Do not repair/relabel its clock.
  pure def legacyAfterClock(c:Case):List[str]=
    if(c.clockClass=="expired") List("request-deadline-original")
    else if(c.clockClass=="before-plan") List("legacy-arithmetic-histogram-and-progress")
    else List("histogram-write","sample-progress","check-progress").concat(
      if(c.progressOk) List("publish-progress","existing-progress-hook","scheme-return-frame","immediate-refill")
      else List("sample-original-failure-clock","progress-deadline-original"))
  pure def original(c:Case):List[str]=prefix.concat(legacyAfterClock(c))
  // Decline happens before reading a clock. A numeric fallback instead carries
  // the ALREADY observed full-width clock into the unchanged Scheme suffix.
  pure def replacement(c:Case):List[str]=
    if(not(c.eligible)) List("fused-enter","decline","fused-leave").concat(original(c))
    else if(c.clockClass!="valid") List("validate-frame","propagate-worker-failure","fused-enter","sample-consume","numeric-fallback","fused-leave").concat(legacyAfterClock(c))
    else List("validate-frame","propagate-worker-failure","fused-enter","sample-consume","histogram-write",
      "sample-progress","check-progress").concat(
        if(c.progressOk) List("publish-progress","existing-progress-hook","fused-leave","scheme-return-frame","immediate-refill")
        else List("fused-leave","sample-original-failure-clock","progress-deadline-original"))
  pure def observable(xs:List[str]):List[str]=xs.foldl(List(),(kept,e)=>
    if(Set("fused-enter","fused-leave","decline","numeric-fallback").contains(e)) kept else kept.append(e))
  pure def status(c:Case):str=
    if(not(c.eligible)) "void-decline-before-clock"
    else if(c.clockClass!="valid") "numeric-clock-legacy-suffix"
    else if(c.progressOk) "empty-list-success" else "false-progress-failure"
  pure def ordinaryProfile(owned:bool,observed:bool,watched:bool,scoped:bool):bool=
    owned and not(observed or watched or scoped)
  pure def canReview(semantics:bool,completeCost:bool,vm:bool,control:bool):bool=
    semantics and completeCost and vm and control
  pure def lifetimeSafe(events:List[str]):bool={
    val end=events.foldl({pointer:false,bad:false},(s,e)=>
      if(e=="borrow-histogram") {...s,pointer:true}
      else if(e=="last-histogram-access") {...s,pointer:false}
      else if(Set("gc","scheme-callback","return").contains(e)) {...s,bad:s.bad or s.pointer}
      else s)
    not(end.bad) and not(end.pointer)
  }
  run successPreservesTwoSamplesAndRealFrameReturnTest=assert(
    observable(replacement(valid))==original(valid) and
    original(valid)==List("validate-frame","propagate-worker-failure","sample-consume", "histogram-write",
      "sample-progress","check-progress","publish-progress","existing-progress-hook","scheme-return-frame","immediate-refill") and
    status(valid)=="empty-list-success")
  run earlyProgressOrRemovedSampleIsNotEquivalentTest=assert(
    original(valid)!=List("validate-frame","propagate-worker-failure","sample-consume","sample-progress",
      "check-progress","publish-progress","histogram-write","scheme-return-frame","immediate-refill") and
    not(original(valid)==List("validate-frame","histogram-write","scheme-return-frame")))
  run failedTransactionsKeepOriginalErrorAndHeldCapacityTest={
    val expired={...valid,clockClass:"expired"}
    val progress={...valid,progressOk:false}
    assert(observable(replacement(expired))==original(expired) and observable(replacement(progress))==original(progress) and
      not(hasEvent(original(expired),"histogram-write")) and
      not(hasEvent(original(expired),"scheme-return-frame")) and
      not(hasEvent(original(progress),"publish-progress")) and
      not(hasEvent(original(progress),"scheme-return-frame")))
  }
  run fullWidthAndBeforePlanPathsCannotResampleTheFirstClockTest=assert(
    status({...valid,eligible:false})=="void-decline-before-clock" and
    status({...valid,clockClass:"before-plan"})=="numeric-clock-legacy-suffix" and
    observable(replacement({...valid,clockClass:"before-plan"}))==original({...valid,clockClass:"before-plan"}) and
    Set(0,1152921504606846976,18446744073709551615).forall(stamp=>
      stamp>=0 and stamp<=18446744073709551615))
  run scopeAndStaticCountsCannotAuthorizeRuntimeIntegrationTest=assert(
    ordinaryProfile(true,false,false,false) and
    not(ordinaryProfile(true,true,false,false)) and not(ordinaryProfile(true,false,true,false)) and
    not(ordinaryProfile(true,false,false,true)) and not(ordinaryProfile(false,false,false,false)) and
    not(canReview(true,false,false,false)) and not(canReview(true,true,true,false)))
  run histogramPointerMustEndBeforeConversionGcOrReturnTest=assert(
    lifetimeSafe(List("borrow-histogram","histogram-write","last-histogram-access","sample-progress","gc","return")) and
    not(lifetimeSafe(List("borrow-histogram","gc","last-histogram-access","return"))) and
    not(lifetimeSafe(List("borrow-histogram","return"))))
  // Reuse canonical custody. A native result marker is not a new credit owner.
  run abiFailureCannotReleaseCanonicalFrameTest=j::cpuReady3.then(all {
    assert(j::safe),assert(j::charged==3),
    assert(not(hasEvent(original({...valid,progressOk:false}),"scheme-return-frame"))),j::stay})
    .then(j::consume(1)).then(all {assert(j::safe),assert(j::charged==2),j::stay})

  type Marker={tag:str,word:int}
  pure def markerDisposition(m:Marker):str=
    if(m.tag=="empty-list") "success"
    else if(m.tag=="false") "progress-error"
    else if(m.tag=="void") "decline-before-clock" else "legacy-with-original-clock"
  pure def conversionRootsSafe(events:List[str]):bool={
    val end=events.foldl({pins:0,rooted:false,returned:false,bad:false},(s,e)=>
      if(s.returned) {...s,bad:true}
      else if(e=="borrow-result") {...s,pins:s.pins+1}
      else if(e=="install-vm-root") {...s,rooted:true}
      else if(e=="release-result") {...s,pins:s.pins-1,bad:s.bad or s.pins<=0 or not(s.rooted)}
      else if(e=="return") {...s,returned:true} else s)
    not(end.bad) and end.pins==0
  }
  pure def declineHasNoEffects(marker:str,events:List[str]):bool=
    marker!="void" or not(events.foldl(false,(found,e)=>
      found or Set("sample-consume","histogram-write","publish-progress").contains(e)))
  run defaultReturnAfterSideEffectsCannotAuthorizeLegacyReplayTest=assert(
    declineHasNoEffects("void",List()) and
    not(declineHasNoEffects("void",List("sample-consume","histogram-write","publish-progress"))) and
    not(conversionRootsSafe(List("borrow-result","return","install-vm-root","release-result"))))
  run numericZeroAndFalseMustNotAliasSuccessMarkerTest=assert(
    markerDisposition({tag:"empty-list",word:0})=="success" and
    markerDisposition({tag:"numeric",word:0})=="legacy-with-original-clock" and
    markerDisposition({tag:"false",word:0})=="progress-error" and
    markerDisposition({tag:"numeric",word:18446744073709551615})=="legacy-with-original-clock")
  run nativeReturnConversionMustRootThenReleaseItsForeignPinTest=assert(
    conversionRootsSafe(List("borrow-result","install-vm-root","release-result","return")) and
    conversionRootsSafe(List("return-constant")) and
    not(conversionRootsSafe(List("borrow-result","release-result","install-vm-root","return"))) and
    not(conversionRootsSafe(List("borrow-result","install-vm-root","return"))))
  // Caller facts must be established by original validation/custody, not new
  // per-message guards. Moving a conversion/read before the first clock is
  // only equivalent under these preconditions; eligible was not a proof.
  type EntryFacts={frameValidated:bool,workerFailurePropagated:bool,stablePlan:bool,
    privateMutableHistogram:bool,positiveS64Progress:bool,noSchemeCallback:bool}
  pure def entryFactsHold(f:EntryFacts):bool=f.frameValidated and f.workerFailurePropagated and
    f.stablePlan and f.privateMutableHistogram and f.positiveS64Progress and f.noSchemeCallback
  pure val validEntry={frameValidated:true,workerFailurePropagated:true,stablePlan:true,
    privateMutableHistogram:true,positiveS64Progress:true,noSchemeCallback:true}
  run movingConversionsBeforeClockRequiresExistingCallerFactsTest=assert(
    entryFactsHold(validEntry) and
    not(entryFactsHold({...validEntry,stablePlan:false})) and
    not(entryFactsHold({...validEntry,privateMutableHistogram:false})) and
    not(entryFactsHold({...validEntry,positiveS64Progress:false})) and
    not(entryFactsHold({...validEntry,noSchemeCallback:false})) and
    not(entryFactsHold({...validEntry,frameValidated:false})) and
    not(entryFactsHold({...validEntry,workerFailurePropagated:false})))
  // Anonymous c-lambda can add a compiler-owned global even when it introduces
  // no lexical closure. Reusing an old linker inventory is not a valid VM test.
  pure def completeImageSources(aot:bool,c:bool,object:bool,linkInventory:bool):bool=
    aot and c and object and linkInventory
  run staticObjectWithoutUpdatedLinkInventoryCannotAuthorizeVmTest=assert(
    completeImageSources(true,true,true,true) and not(completeImageSources(true,true,true,false)))
  // Abstract cost witnesses, NOT measured nanoseconds. A fast branch saving
  // must also pay code/cache and the actual fallback mix; do not fit them to
  // make a proposed implementation pass. Tail acceptance remains separate.
  pure def screenedCost(count:int,fast:int,fallbackCount:int,fallbackExtra:int,codePenalty:int):int=
    count*fast+fallbackCount*fallbackExtra+codePenalty
  run fewerFastTransfersCannotCertifyUnknownWholeCycleCostsTest=assert(
    screenedCost(100,8,0,20,0)<screenedCost(100,10,0,0,0) and
    screenedCost(100,8,20,20,0)>screenedCost(100,10,0,0,0) and
    screenedCost(100,8,0,20,201)>screenedCost(100,10,0,0,0))

  // Caller POLL-site counts do not account for opportunities in removed
  // callees. Abstract ticks illustrate the obligation, not SDK timings.
  type SpanPart={cpu:int,opportunityAfter:bool}
  pure def nativeSpan(parts:List[SpanPart]):{total:int,open:int,largest:int,valid:bool}=
    parts.foldl({total:0,open:0,largest:0,valid:true},(s,p)=>{
      val span=s.open+p.cpu
      {total:s.total+p.cpu,open:if(p.opportunityAfter) 0 else span,
        largest:if(span>s.largest) span else s.largest,valid:s.valid and p.cpu>=0}
    })
  run fewerCpuTicksAndSameCallerPollsDoNotBoundRemovedCalleeOpportunitiesTest={
    val original=nativeSpan(List({cpu:5,opportunityAfter:true},{cpu:3,opportunityAfter:true},
      {cpu:5,opportunityAfter:true}))
    val fused=nativeSpan(List({cpu:4,opportunityAfter:false},{cpu:3,opportunityAfter:false},
      {cpu:4,opportunityAfter:true}))
    assert(original.valid and fused.valid and original.open==0 and fused.open==0 and
      fused.total<original.total and original.largest==5 and fused.largest==11 and
      original.largest<=10 and not(fused.largest<=10))
  }

}
