// Result suffix only; canonical Actor custody and retirement stay unchanged.
module mailbox_result_construction {
  import mailbox_research_gate as researchGate from "./mailbox_research_gate"
  import mailbox_layout_entry as layoutEntry from "./mailbox_layout_entry"
  import mailbox_joint_cycle as j from "./mailbox_joint_cycle"
  import actor_service_contract as gate from "./actor_service_contract"
  import actor_service_calculus as budget from "./actor_service_calculus"
  import mailbox_clock_abi as clockAbi from "./mailbox_clock_abi"
  pure val resultFields=0.to(20)
  pure def fastEligible(fixnumFields:Set[int]):bool=resultFields.subseteq(fixnumFields)
  // Actual native constructor allocates after rank returns and initializes all
  // 21 fields before publishing. Tokens abstract fresh native object identity.
  pure def freshResult(rankReturn:int,fixnumFields:Set[int]):{identity:int,initialized:Set[int],mode:str}=
    {identity:rankReturn,initialized:resultFields,mode:if(fastEligible(fixnumFields)) "native" else "original"}
  pure def firstFailure(fitting:Set[int]):int=if(resultFields.subseteq(fitting)) -1 else
    resultFields.exclude(fitting).fold(21,(a,b)=>if(a<b) a else b)
  pure def selectedFailure(fixnums:Set[int],fitting:Set[int]):int=
    if(fastEligible(fixnums)) -1 else firstFailure(fitting)
  run allTwentyOneValuesMustFitTheNativeFastCertificateTest=assert(
    fastEligible(resultFields) and resultFields.forall(i=>not(fastEligible(resultFields.exclude(Set(i))))))
  run initializedNativeResultIsCompleteBeforePublicationTest=assert(
    freshResult(1,resultFields).initialized==resultFields)
  run rankContinuationReturnsAllocateIndependentResultsTest=assert(
    freshResult(1,resultFields).identity!=freshResult(2,resultFields).identity)
  // Reusing a result allocated before a capture would violate that contract.
  run preRankReusableAllocationIsARetainedCounterexampleTest=assert(
    freshResult(1,resultFields).identity==1 and freshResult(2,resultFields).identity!=1)
  run validBignumsKeepOriginalConstructionRatherThanNarrowingTypesTest=assert(
    resultFields.forall(i=>freshResult(1,resultFields.exclude(Set(i))).mode=="original" and
      selectedFailure(resultFields.exclude(Set(i)),resultFields)==-1))
  run invalidFieldRetainsOriginalFirstCheckedFailureTest=assert(
    resultFields.forall(i=>selectedFailure(resultFields.exclude(Set(i)),resultFields.exclude(Set(i)))==i) and
    selectedFailure(resultFields.exclude(Set(2,3)),resultFields.exclude(Set(2,3)))==2)
  run nativeResultDoesNotReturnSourceCapacityTest=j::cpuReady3.then(j::arrive(4)).then(all {
    assert(j::safe),assert(j::charged==3),assert(not(j::canReserveEager(4))),j::stay})
  // A real fixnum certificate comes from the SDK predicate, not a caller flag.
  // The representation premise must hold before trusting unchecked allocation.
  pure def truthfulCertificate(fixnums:Set[int],fitting:Set[int]):bool=fixnums.subseteq(fitting)
  run forgedFastRepresentationCertificateIsNotASafetyPremiseTest=assert(
    not(truthfulCertificate(resultFields,resultFields.exclude(Set(2)))) and
    truthfulCertificate(resultFields,resultFields))
  run observedImprovementRetainsResearchButDoesNotWaiveAbsoluteGatesTest={
    val evidence={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(gate::researchDisposition(evidence)=="retain-isolated" and
      not(gate::maySeekAdmissionReview(evidence)))
  }
  // Both arms completed and shut down, but the paired control itself
  // missed the absolute gates. The observed regression has no causal label.
  run op62FixedSlotPairCannotSupportAdmissionTest = {
    val evidence={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(2023031000>1951767000 and 729888>526903 and
      3128791458>3000000000 and not(gate::maySeekAdmissionReview(evidence)))
  }

  // Each transition has the same checked line order and sum. Only the
  // representation of gross changes from a cell to a tail-call parameter.
  pure def boxedGrossStep(line:int, gross:int, amount:int):{line:int,gross:int} =
    {line:line+1,gross:gross+amount}
  pure def parameterGrossStep(line:int, gross:int, amount:int):{line:int,gross:int} =
    {line:line+1,gross:gross+amount}
  run oracleGrossTransitionRetainsLineAndExactSumTest = assert(
    0.to(3).forall(line => Set(0,1,9999,99990).forall(gross =>
      Set(0,1,9999,39996).forall(amount =>
        boxedGrossStep(line,gross,amount) == parameterGrossStep(line,gross,amount)))))
  // All fields come from an exact signed-64 typed row. The checked
  // product can exceed fixnum or signed-64 range. Reuse the Scheme
  // value only after the original validation and within this row.
  run op62OneExactProductPreservesBothOrderTotalsTest={
    val cases=Set({sku:0,quantity:1,cents:0},{sku:15,quantity:4,cents:9999},
      {sku:3,quantity:9223372036854775807,cents:9223372036854775807},
      {sku:16,quantity:2,cents:10},{sku:1,quantity:0,cents:10})
    pure def valid(row:{sku:int,quantity:int,cents:int}):bool=
      0<=row.sku and row.sku<=15 and row.quantity>0 and row.cents>=0
    assert(cases.forall(row=>{
      if(not(valid(row))) true
      else {
        val product=row.quantity*row.cents
        List(100+product,4+row.quantity,200+product)==
          List(100+row.quantity*row.cents,4+row.quantity,200+row.quantity*row.cents)
      }
    }) and
    9223372036854775807*9223372036854775807>9223372036854775807)
  }

  // Recovered from the Mailbox stash. A checkpoint before later rows
  // may retain their storage even if the ordinal was already copied.
  pure def op62ReentryInputReads(checkpoint:bool,ordinalDetached:bool):Set[int] =
    (if(checkpoint) 5.to(13) else Set()).union(
      if(ordinalDetached) Set() else Set(0))
  run Op62OrdinalSnapshotDoesNotProtectUnreadRowsAfterReentryTest = assert(
    op62ReentryInputReads(false,false)==Set(0) and
    op62ReentryInputReads(false,true)==Set() and
    op62ReentryInputReads(true,true).size()==9 and
    op62ReentryInputReads(true,true).intersect(2.to(13))==5.to(13))
  // Installed Gerbil ran both exact 7F kernel variants. The mutation kept
  // the ordinal and changed only row storage before checkpoint reentry.
  run ReplayedKernelObservedReusedRowsWithTheSameOrdinalTest = {
    val original={id:48,units:19,firstGross:8780,replayGross:16744}
    val mutated={...original,replayGross:16897}
    assert(original.id==mutated.id and original.units==mutated.units and
      original.firstGross==mutated.firstGross and original.replayGross!=mutated.replayGross)
  }

  // The private qualification operation selects both endpoints before frames
  // are published. A raw vector at the public endpoint means a native command.
  type OrderRoute = {op:int, source:str, worker:str}
  pure def mayOmitOrderEnvelope(route:OrderRoute):bool=
    route.op==62 and route.source=="private-short-order" and route.worker=="private-short-order"
  run EnvelopeRemovalRequiresBothPrivateEndpointsTest={
    val route={op:62,source:"private-short-order",worker:"private-short-order"}
    assert(mayOmitOrderEnvelope(route) and
      not(mayOmitOrderEnvelope({...route,worker:"public"})) and
      not(mayOmitOrderEnvelope({...route,source:"public"})) and
      not(mayOmitOrderEnvelope({...route,op:66})))
  }

  // The source constructs a fresh, closed short order. Result construction
  // may specialize only after the original service unit selects its layout.
  pure def sparseOrderEligible(route:OrderRoute,native:bool,originalUnit:bool,
      checkpoint:bool,span:int):bool =
    mayOmitOrderEnvelope(route) and native and originalUnit and not(checkpoint) and span==4
  run SparseOrderRequiresTheLiveNoCheckpointPrivateRouteTest={
    val route={op:62,source:"private-short-order",worker:"private-short-order"}
    assert(sparseOrderEligible(route,true,true,false,4) and
      not(sparseOrderEligible(route,false,true,false,4)) and
      not(sparseOrderEligible(route,true,false,false,4)) and
      not(sparseOrderEligible(route,true,true,true,4)) and
      not(sparseOrderEligible(route,true,true,false,1)) and
      not(sparseOrderEligible({...route,source:"public"},true,true,false,4)))
  }
  pure def shortSku(base:int,line:int):int=(base+line)%16
  run DistinctCyclicShortSkusAllowOneStorePerLineTest={
    val lines=0.to(3)
    val skus=0.to(15)
    assert(skus.forall(base=>lines.map(line=>shortSku(base,line)).size()==4 and
      skus.forall(sku=>{
        val row=(sku+16-base)%16
        val sparse=if(row<4) row+1 else 0
        lines.fold(0,(sum,line)=>sum+(if(shortSku(base,line)==sku) line+1 else 0))==sparse
      })) and
      4*9999==39996 and (1+2+3+4)*9999==99990)
  }
  run DuplicateSkuNeedsTheGenericAccumulatorTest={
    val old=7+11
    val lastWrite=11
    assert(old==18 and old!=lastWrite)
  }

  // A completion identity protects retirement only if checked before writes
  // to reusable frame storage. Model a captured return from the old request.
  type ReplyFrame = {id:int, state:str, payload:str}
  pure def acceptsReply(frame:ReplyFrame, replyId:int, expectedState:str):bool=
    frame.id==replyId and frame.state==expectedState
  pure def writeReply(frame:ReplyFrame, payload:str):ReplyFrame=
    {...frame, state:"ready", payload:payload}

  run OldReplyRejectedAfterItOverwritesReusedFrameTest={
    val reassigned={id:2,state:"queued",payload:"input-two"}
    val overwritten=writeReply(reassigned,"stale-result")
    assert(not(acceptsReply(overwritten,1,"ready")) and overwritten.id==2 and
      overwritten.state=="ready" and overwritten.payload!=reassigned.payload)
  }

  run CheckBeforeFrameWritePreservesReassignedInputAndAllowsLiveReplyTest={
    pure def commit(frame:ReplyFrame, replyId:int, payload:str):ReplyFrame=
      if(acceptsReply(frame,replyId,"running")) writeReply(frame,payload) else frame
    val reassigned={id:2,state:"queued",payload:"input-two"}
    val running={...reassigned,state:"running"}
    assert(commit(reassigned,1,"stale-result")==reassigned and
      commit(running,1,"stale-result")==running and
      commit(reassigned,2,"too-early")==reassigned and
      commit(running,2,"second-result")=={id:2,state:"ready",payload:"second-result"})
  }

  // Archived single-product A/B on an earlier GCC image, distinct from frozen 7F.
  run exactRowProductArchivedFailurePreventsAdmissionTest={
    val candidate1M={completed:1000000,elapsedNs:1256709417,p99Ns:254110000,peak:205244}
    val control1M={completed:1000000,elapsedNs:1275207417,p99Ns:272447000,peak:219241}
    val failure={planned:5000000,published:2729260,completed:2729257,
      active:3,elapsedNs:10546034041,cpuUpperNs:4422863042,phase:"request-deadline"}
    val hardFailure={hardFailure:true,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(candidate1M.completed==control1M.completed and
      candidate1M.elapsedNs<control1M.elapsedNs and candidate1M.p99Ns<control1M.p99Ns and
      candidate1M.p99Ns>418000 and candidate1M.peak>128 and
      failure.published-failure.completed==failure.active and
      failure.planned-failure.published==2270740 and
      failure.elapsedNs-failure.cpuUpperNs==6123170999 and
      failure.phase=="request-deadline" and
      gate::researchDisposition(hardFailure)=="withdraw" and
      not(gate::maySeekAdmissionReview(hardFailure)))
  }

  // Recovered frame-time counting argument. A histogram bucket edge
  // supplies a lower bound after subtracting one full bucket.
  pure def shortHoldFloor(tailCount:int,holdNs:int,elapsedNs:int,frames:int):int =
    if(holdNs<=0 or tailCount<=frames*elapsedNs/holdNs) 0 else tailCount-frames*elapsedNs/holdNs
  pure def p99BeforePublishFloor(total:int,p99UpperNs:int,bucketNs:int,
      elapsedNs:int,holdNs:int,frames:int):{count:int,waitNs:int}={
    val tailCount=total-budget::rank(total,99,100)+1
    val waitNs=p99UpperNs-bucketNs-holdNs
    val count=if(waitNs<=0) 0 else shortHoldFloor(tailCount,holdNs,elapsedNs,frames)
    if(count==0) {count:0,waitNs:0} else {count:count,waitNs:waitNs}
  }
  run FrozenMillionTailAlreadyWaitedBeforePublicationTest=assert(
    p99BeforePublishFloor(1000000,6393000,1000,1004778708,1000000,3)==
      {count:6987,waitNs:5392000} and
    p99BeforePublishFloor(1000000,18922000,1000,1019933458,1000000,3)==
      {count:6942,waitNs:17921000})
  run FailedIntegratedControlStillBoundsItsOwnPostOfferRetirementTest={
    val elapsed=1293533667
    val lastOffer=999999000
    val peak=110030
    val ownerCpuUpper=1093768000
    val gcWallUpper=18307925
    assert((peak+1)*1000000000/(elapsed-lastOffer)==374848 and
      elapsed-ownerCpuUpper-gcWallUpper==181457742 and
      p99BeforePublishFloor(1000000,282234000,1000,elapsed,1000000,3)==
        {count:6121,waitNs:281233000})
  }
  // Complete integral-rate receipts bound post-offer retirements by
  // the sampled maximum plus one. The final publication timestamp
  // is a source stamp after the send, not the mailbox enqueue instant.
  run LatePublicationStampCannotBeExplainedByTheFinalRetirementSuffixTest={
    val lastOffer=999999000
    val stopped=Set(
      {elapsed:1121129458,lastStamp:1121126916,peak:116723,upperRate:963623},
      {elapsed:1181811875,lastStamp:1181809875,peak:154547,upperRate:850039},
      {elapsed:1293533667,lastStamp:1293532292,peak:110030,upperRate:374849})
    assert(stopped.forall(r=>
      r.lastStamp-lastOffer>120000000 and
      r.elapsed-r.lastStamp>0 and r.elapsed-r.lastStamp<3000 and
      budget::ceilDiv((r.peak+1)*1000000000,r.elapsed-lastOffer)==r.upperRate and
      r.upperRate<1000000))
  }
  run FiveMillionTailStillWaitsBeforePublicationDespiteShortFrameHoldsTest={
    val count=5000000
    val elapsed=4984038584
    val upper=3943455000
    val tailCount=count-budget::rank(count,99,100)+1
    assert(p99BeforePublishFloor(count,upper,1000,elapsed,10000000,3)==
      {count:48506,waitNs:3933454000} and
      shortHoldFloor(tailCount,upper-1000-999999800,elapsed,3)==49996)
  }
  // After all arrivals are due, any tail request already published but
  // unable to have consumed by the cut must still own a distinct frame.
  pure def tailSourceFloor(tailCount:int,frames:int,lastOffer:int,cut:int,earliestConsume:int):int =
    if(lastOffer<=cut and cut<earliestConsume and frames<tailCount) tailCount-frames else 0
  run FiveMillionTailKeepsSourceDebtAcrossACommonPostOfferIntervalTest={
    val count=5000000
    val tailCount=count-budget::rank(count,99,100)+1
    val lastOffer=999999800
    val earliestConsume=3943455000-1000
    val cut=earliestConsume-1
    assert(cut-lastOffer==2943454199 and
      tailSourceFloor(tailCount,3,lastOffer,lastOffer,earliestConsume)==49998 and
      tailSourceFloor(tailCount,3,lastOffer,cut,earliestConsume)==49998 and
      tailSourceFloor(tailCount,3,lastOffer,lastOffer-1,earliestConsume)==0 and
      tailSourceFloor(tailCount,3,lastOffer,earliestConsume,earliestConsume)==0)
  }
  run ReplyOnlySpeedupWithFixedPublicationCannotClearRecordedP999TailTest={
    val million=p99BeforePublishFloor(1000000,6393000,1000,1004778708,1000000,3)
    val fiveMillion=p99BeforePublishFloor(5000000,3943455000,1000,4984038584,10000000,3)
    assert(million.count>1000000-budget::rank(1000000,999,1000) and
      million.waitNs==5392000 and
      fiveMillion.count>5000000-budget::rank(5000000,999,1000) and
      fiveMillion.waitNs==3933454000)
  }
  run FastHalfMillionCohortDoesNotInheritTheOtherTailAttributionTest=assert(
    p99BeforePublishFloor(5000000,317000,1000,9999999542,200000,3)=={count:0,waitNs:0})

  // Every verified retirement before the last scheduled tick spends
  // publication-to-return frame time. Holds may overlap only across frames.
  pure def frameTimeForRetired(retired:int,slow:int,fastHold:int,slowHold:int):int =
    (retired-slow)*fastHold+slow*slowHold
  run FixedThreeFramesImposeSeparateResidenceBudgetsTest=assert(
    3*9999998000/(5000000-128)==6000 and
    3*999999000/(1000000-128)==3000 and
    3*999999800/(5000000-128)==600)
  run MoreResidentActorsCannotPayForSuspendedFrameTimeTest={
    val frameBudget=3*999999800
    val required=5000000-128
    assert(frameTimeForRetired(required,500,500,1000000)<=frameBudget and
      frameTimeForRetired(required,501,500,1000000)>frameBudget)
  }

  // Recovered from the Mailbox stash. The numbers are hypothetical
  // exclusive complete-cycle costs, not measured per-request CPU.
  pure def mixedOwnerCost(retired:int,legacy:int,closedNs:int,legacyNs:int):int =
    (retired-legacy)*closedNs+legacy*legacyNs
  run FiveMillionFastPathMustCoverMostVerifiedReturnsTest={
    val required=5000000-128
    val offeredEndNs=999999800
    val maxLegacy=295530
    assert(mixedOwnerCost(required,maxLegacy,150,996)<=offeredEndNs and
      mixedOwnerCost(required,maxLegacy+1,150,996)>offeredEndNs and
      mixedOwnerCost(required,required/4,0,996)>offeredEndNs)
  }
  run OneMillionCanFitTheSameHypotheticalCostWithoutProvingRecoveryTest={
    val required=1000000-128
    val offeredEndNs=999999000
    val base=mixedOwnerCost(required,required,150,996)
    assert(base==995872512 and base<offeredEndNs and
      base+4126489>offeredEndNs)
  }
  run ArchivedLayoutWrapperSavingsChangeSignAcrossMatchedPairsTest={
    val completePerArm=100000
    val firstSaved=118870125-120676375
    val secondSaved=121144333-118812792
    val targetNsPerRequest=41
    assert(firstSaved<0 and secondSaved>0 and
      firstSaved+secondSaved<2*completePerArm*targetNsPerRequest)
  }
  run PrivateOrderSaturatedSavingsDoNotQualifyOpenLoadTest={
    val messages=100000
    val firstSaved=154556542-123556500
    val secondSaved=137693041-115399541
    val nativeOpenLoad={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(firstSaved>0 and secondSaved>0 and
      firstSaved+secondSaved>2*messages*266 and
      not(gate::maySeekAdmissionReview(nativeOpenLoad)))
  }
  run LocalActorPairedCpuSavingDoesNotImplyWallOrRateAdmissionTest={
    val splitFirst={cpu:101574125,wall:88966792}
    val splitLast={cpu:104299541,wall:94850042}
    val localFirst={cpu:95968875,wall:134703500}
    val localLast={cpu:81857750,wall:70396250}
    val evidence={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(localFirst.cpu<splitFirst.cpu and localLast.cpu<splitLast.cpu and
      localFirst.wall>splitFirst.wall and localLast.wall<splitLast.wall and
      localFirst.wall-localFirst.cpu==38734625 and
      not(gate::maySeekAdmissionReview(evidence)))
  }
  run SparseAccumulatorRemovalAndFewerCollectionsDoNotQualifyComponentTest={
    // Total enclosing owner CPU and synthetic-lead-adjusted inner wall.
    val firstControl={cpu:104094292,wall:89599709,gc:4}
    val firstSparse={cpu:100693041,wall:87710542,gc:3}
    val lastSparse={cpu:124667917,wall:121231167,gc:3}
    val lastControl={cpu:102808208,wall:89398917,gc:4}
    val evidence={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(firstSparse.gc<firstControl.gc and lastSparse.gc<lastControl.gc and
      firstSparse.cpu<firstControl.cpu and lastSparse.cpu>lastControl.cpu and
      firstSparse.wall<firstControl.wall and lastSparse.wall>lastControl.wall and
      not(gate::maySeekAdmissionReview(evidence)))
  }
  run LexicalCustodySmallerCodeStillRegressesBothMeasuredPairsTest={
    val framedFirst={cpu:93239667,wall:77046208,codeInstructions:5400}
    val framedLast={cpu:88372167,wall:73999583,codeInstructions:5400}
    val lexicalFirst={cpu:94929333,wall:83061083,codeInstructions:4032}
    val lexicalLast={cpu:91732041,wall:77768042,codeInstructions:4032}
    val evidence={hardFailure:false,relativeEffect:"confirmed-regression",absolutePassed:false}
    assert(lexicalFirst.codeInstructions<framedFirst.codeInstructions and
      lexicalFirst.cpu>framedFirst.cpu and lexicalLast.cpu>framedLast.cpu and
      lexicalFirst.wall>framedFirst.wall and lexicalLast.wall>framedLast.wall and
      not(gate::maySeekAdmissionReview(evidence)))
  }
  run ArchivedShortAggregationComponentCannotCertifyOpenLoadTest={
    val old={messages:200000,cpuNs:232460584,wallNs:208880041}
    val changed={messages:200000,cpuNs:225118874,wallNs:201358667}
    val openControl={completed:1000000,p99Ns:25212000,peak:25191}
    val candidateMainRan=false
    val evidence={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(old.messages==changed.messages and changed.cpuNs<old.cpuNs and
      changed.wallNs<old.wallNs and openControl.p99Ns>418000 and
      openControl.peak>128 and not(candidateMainRan) and
      not(gate::maySeekAdmissionReview(evidence)))
  }

  // Source ordinal debt is outside the bounded physical Actor custody. This
  // identity applies at each observation; maxima need not share a timestamp.
  val dueNotConsumed=j::c.offered.exclude(j::consumedRequests).size()
  val sourceNotPublished=j::c.offered.exclude(j::publishedRequests).size()
  val physicalCustody=j::publishedRequests.exclude(j::consumedRequests).size()
  val debtSplitSafe=j::safe and dueNotConsumed==sourceNotPublished+physicalCustody and
    physicalCustody<=j::charged
  action boot=j::bootMixed
  action step=j::step
  run pendingSourceArrivalsAreNotAnUnboundedMailboxQueueTest=j::cpuReady3
    .then(j::arrive(4)).then(j::arrive(5)).then(j::arrive(6)).then(all {
      assert(debtSplitSafe),assert(dueNotConsumed==6),assert(sourceNotPublished==3),
      assert(physicalCustody==3),j::stay})
    .then(j::consume(1)).then(all {
      assert(debtSplitSafe),assert(dueNotConsumed==5),assert(sourceNotPublished==3),
      assert(physicalCustody==2),j::stay})
  // The request channel contains only a subset of the charged frames.
  // Its full-to-not-full edge is not the source credit return edge.
  run op62SharedChannelRoomDoesNotAuthorizeRefillTest = {
    val limit=3
    val charged=Set(1,2,3)
    val before=charged
    val after=before.exclude(Set(1))
    assert(before.size()==limit and after.size()==limit-1 and charged.size()==limit and
      charged.powerset().forall(live =>
        live.size()==limit or live.powerset().forall(queued => queued.size()<limit)))
  }

  // Replacing broadcast with one signal only on the empty edge can
  // leave a second request unclaimed while the first worker is blocked.
  type PullQueue = {queued:int,waiting:int,woken:int}
  pure def singleEdgePut(q:PullQueue):PullQueue =
    if (q.queued==0 and q.waiting>0)
      {queued:1,waiting:q.waiting-1,woken:q.woken+1}
    else {...q,queued:q.queued+1}

  run op62SingleEdgeSignalCanStrandWorkBehindBlockedClaimTest = {
    val empty={queued:0,waiting:4,woken:0}
    val burst=singleEdgePut(singleEdgePut(empty))
    val claimed={...burst,queued:burst.queued-1,woken:burst.woken-1}
    assert(claimed=={queued:1,waiting:3,woken:0})
  }

  // The claimant passes one wake under the queue lock, before doing
  // business work. A wake by itself never claims an item or returns credit.
  pure def claimAndPassWake(q:PullQueue):PullQueue =
    if (q.queued<=0 or q.woken<=0) q
    else if (q.queued>1 and q.waiting>0)
      {queued:q.queued-1,waiting:q.waiting-1,woken:q.woken}
    else {...q,queued:q.queued-1,woken:q.woken-1}

  run op62PullBatonPassesWakeBeforeHandlerCanBlockTest = {
    val empty={queued:0,waiting:4,woken:0}
    val burst=singleEdgePut(singleEdgePut(empty))
    val first=claimAndPassWake(burst)
    assert(burst=={queued:2,waiting:3,woken:1} and
      first=={queued:1,waiting:2,woken:1} and
      claimAndPassWake(first)=={queued:0,waiting:2,woken:0})
  }

  // A worker can lose the only wake before it enters the claim.
  // The open queue then needs a supervisory wake, even on another put.
  run op62PullBatonCanLoseOnlyWakeBeforeClaimTest = {
    val empty={queued:0,waiting:4,woken:0}
    val burst=singleEdgePut(singleEdgePut(empty))
    val lost={...burst,woken:0}
    val more=singleEdgePut(lost)
    val repaired={...more,waiting:more.waiting-1,woken:1}
    assert(more=={queued:3,waiting:3,woken:0} and
      claimAndPassWake(more)==more and
      claimAndPassWake(repaired)=={queued:2,waiting:1,woken:1})
  }

  // Frame credit is global. After out-of-order returns, ordinal routing
  // can queue two live requests on one worker with spare peers.
  run op62ModuloRouteCanQueueBehindOccupiedWorkerTest = {
    val live=Set(0,3,4)
    val occupied=live.map(id => id % 4)
    assert(live.size()==3 and occupied==Set(0,3) and
      Set(0,1,2,3).exclude(occupied)==Set(1,2))
  }

  // When two same-worker ordinals coexist, the three frame credits
  // require at least two intervening ordinals to have retired first.
  run op62ModuloCollisionRequiresOvertakingTest = {
    val intervening=Set(1,2,3)
    val atCollision=intervening.powerset().filter(live => live.size()+2<=3)
    assert(atCollision.forall(live => intervening.exclude(live).size()>=2)
      and atCollision.contains(Set(3)))
  }

  // Preserve the frozen route unless that worker still owns a frame.
  pure def collisionFallback(id:int, busy:Set[int]):Set[int] = {
    val preferred=id % 4
    if (busy.contains(preferred)) Set(0,1,2,3).exclude(busy)
    else Set(preferred)
  }

  run op62CollisionFallbackPreservesUncontendedRouteTest = {
    val workers=Set(0,1,2,3)
    val masks=workers.powerset().filter(busy => busy.size()<=3)
    assert(0.to(7).forall(id => masks.forall(busy => {
      val eligible=collisionFallback(id,busy)
      eligible.size()>0 and eligible.intersect(busy).size()==0 and
        (busy.contains(id % 4) or eligible==Set(id % 4))
    })) and collisionFallback(4,Set(0,3))==Set(1,2))
  }

  // Source owns the charge. The immutable ordinal and frame identity
  // must both match before a normal reply can release a redirected worker.
  type RouteCharge = {slot:int,ordinal:int,worker:int}
  pure def chargeAfterReply(charge:RouteCharge,slot:int,ordinal:int,verified:bool):Set[RouteCharge] =
    if (verified and charge.slot==slot and charge.ordinal==ordinal) Set()
    else Set(charge)

  run op62RedirectedChargeSurvivesStaleReplyAndUnverifiedProgressTest = {
    val charge={slot:1,ordinal:4,worker:1}
    assert(chargeAfterReply(charge,1,0,true)==Set(charge) and
      chargeAfterReply(charge,2,4,true)==Set(charge) and
      chargeAfterReply(charge,1,4,false)==Set(charge) and
      chargeAfterReply(charge,1,4,true)==Set() and charge.worker!=charge.ordinal % 4)
  }

  // A frame-owned route needs a rotating preference under sparse arrivals.
  // An occupied preferred slot must never block another free slot.
  pure def preferredFreeSlot(id:int,free:Set[int]):int = {
    val first=id % 3
    val second=(first+1) % 3
    val third=(first+2) % 3
    if (free.contains(first)) first
    else if (free.contains(second)) second
    else if (free.contains(third)) third
    else -1
  }

  run op62FrameOwnedRoutePreservesSparseRotationAndFreeSlotFallbackTest = {
    val slots=Set(0,1,2)
    val masks=Set(Set(),Set(0),Set(1),Set(2),Set(0,1),Set(0,2),Set(1,2),slots)
    assert(0.to(2).map(id => preferredFreeSlot(id,slots))==slots and
      0.to(5).forall(id => masks.forall(free =>
        if (free.size()==0) preferredFreeSlot(id,free)== -1
        else free.contains(preferredFreeSlot(id,free)))))
  }

  // With every slot charged, immediate refill can only pick the slot
  // just returned. Rotating the preference cannot rebalance that trace.
  run op62SaturatedRefillFollowsRetirementDespiteRotationTest = {
    val slots=Set(0,1,2)
    val ordinals=0.to(8)
    assert(slots.forall(retired =>
      ordinals.forall(id => preferredFreeSlot(id,Set(retired))==retired)) and
      ordinals.map(id => preferredFreeSlot(id,Set(1)))==Set(1))
  }

  // The fourth entry is the unbound worker. Exchange only after a
  // matching frame retirement and before its next publication.
  pure def exchangeSpare(route:List[int],slot:int):List[int] =
    List(0,1,2,3).foldl(List(), (result,index) =>
      result.append(if (index==slot) route[3]
                    else if (index==3) route[slot] else route[index]))

  run op62SpareExchangePreservesExclusiveWorkerOwnershipTest = {
    val initial=List(0,1,2,3)
    assert(0.to(2).forall(slot => {
      val rotated=exchangeSpare(initial,slot)
      rotated[slot]==3 and rotated[3]==slot and
      0.to(3).map(index => rotated[index])==Set(0,1,2,3) and
      exchangeSpare(rotated,slot)==initial
    }))
  }

  run op62SpareExchangeDoesNotGuaranteeGlobalFairnessTest = {
    val initial=List(0,1,2,3)
    val rotated=exchangeSpare(initial,1)
    assert(rotated==List(0,3,2,1) and exchangeSpare(rotated,1)==initial)
  }

  // A worker can still be between reply send and receive when another
  // slot returns. The spare exchange can then select that worker again.
  run op62SpareCanBeReassignedBeforeNextReceiveTest = {
    val initial=List(0,1,2,3)
    val afterFirst=exchangeSpare(initial,1)
    val afterSecond=exchangeSpare(afterFirst,2)
    val enteredReceive=Set(0,2,3)
    assert(afterFirst[3]==1 and afterSecond[2]==1 and
      0.to(3).map(slot => afterSecond[slot]).size()==4 and
      not(enteredReceive.contains(afterSecond[2])))
  }

  // A late, short debt excursion can be confined to the top one percent.
  // This witness is a metric counterexample, not an observed workload.
  run op62MatchingP99DoesNotQualifyRetirementDebtTest = {
    val total=1000000
    val burst=129
    val timely=total-burst
    val rank99=(99*total+99)/100
    val rank999=(999*total+999)/1000
    assert(timely>=rank99 and timely>=rank999 and burst>128)
  }

  // The source retires at most one reply between consecutive drive samples.
  // A successful run also has a final end sample with zero debt.
  run op62PreRefillPeakCanMissOneOutstandingRequestTest = {
    val guard=128
    val dueBeforeRetire=129
    val retiredBetweenSamples=1
    assert(dueBeforeRetire>guard and dueBeforeRetire-retiredBetweenSamples==guard and
      0.to(129).forall(due => Set(0,1).forall(retired =>
        due <= due-retired+1)))
  }

  // A before-return clock and the next drive sample bracket one credit
  // return. Equal due counts pin the peak; a crossing arrival may not.
  pure def oneReturnPeakBounds(beforeDue:int,afterDue:int,completed:int):List[int] = {
    val before=beforeDue-completed
    val after=afterDue-completed-1
    List(if (before>after) before else after,after+1)
  }

  run op62ExistingClocksBoundButDoNotAlwaysResolvePeakTest = assert(
    oneReturnPeakBounds(168,168,40)==List(128,128) and
    oneReturnPeakBounds(168,169,40)==List(128,129))

  // All closed drive intervals can hide at most one retirement. The
  // request-deadline suffix ends at the sealed clock, before retirement.
  pure def op62SealedPeakBounds(sampled:int,terminalDebt:int):List[int] =
    List(if(sampled>terminalDebt) sampled else terminalDebt,
      if(sampled+1>terminalDebt) sampled+1 else terminalDebt)

  run op62SealedClockAccountsForUnsampledFailureTailTest = assert(
    op62SealedPeakBounds(128,0)==List(128,129) and
    op62SealedPeakBounds(128,400)==List(400,400) and
    op62SealedPeakBounds(929191,263638)==List(929191,929192))

  // At any sampled breach at most three due requests can be in frames.
  // This lower bound does not join independent source and total maxima.
  run op62GuardBreachRequiresUnpublishedDebtTest = assert(
    Set(129,351,6746).forall(dueDebt => 0.to(3).forall(active =>
      dueDebt-active>=126 and dueDebt-active>=dueDebt-3)) and
    351-3==348 and 6746-3==6743)

  run op62IntegralDueQuotientMatchesOriginalClockTest = assert(
    Set(500000,1000000,5000000).forall(rate => {
      val period=1000000000/rate
      Set(0,1,127,128,4999999,5000000).forall(ordinal =>
        Set(0,1,period-1).forall(offset => {
          val now=ordinal*period+offset
          now*rate/1000000000+1==now/period+1
        }))
    }))

  // A progress timestamp is taken before retirement. The next drive uses
  // the post-retirement clock for both eligibility and source accounting.
  run op62ReusingProgressClockCanHideBreachAndDeferSupplyTest = {
    val completedAfterReturn=40
    val published=42
    val dueAtProgress=168
    val dueAtDrive=169
    val sparseDueAtProgress=42
    val sparseDueAtDrive=43
    assert(dueAtProgress-completedAfterReturn==128 and
      dueAtDrive-completedAfterReturn==129 and
      published-completedAfterReturn==2 and
      dueAtProgress-published==126 and dueAtDrive-published==127 and
      sparseDueAtProgress-published==0 and sparseDueAtDrive-published==1)
  }

  run op62ReusingProgressClockCanMissSourceDeadlineTest = {
    val deadline=10000000000
    val plannedAt=1000
    val progressAt=plannedAt+deadline-1
    val driveAt=plannedAt+deadline
    assert(progressAt-plannedAt<deadline and driveAt-plannedAt>=deadline)
  }

  // The last publication was authorized by a drive sample with due=total.
  // After that publication no further source debt can exist.
  run op62StaleArrivalWakeCanOccupyOnlyNotificationSlotTest = {
    val currentGeneration=2
    val queuedGeneration=1
    val due=true
    val freeCredit=true
    val freshEnqueued=queuedGeneration<0
    val staleMayRefill=queuedGeneration==currentGeneration
    assert(due and freeCredit and not(freshEnqueued) and not(staleMayRefill))
  }

  run op62PortSlotBoundDoesNotBoundMailboxTokensTest = {
    val afterForward={pipeBytes:0,mailboxTokens:1}
    val rearmed={...afterForward,pipeBytes:1}
    val forwardedAgain={pipeBytes:0,mailboxTokens:rearmed.mailboxTokens+1}
    assert(rearmed.pipeBytes<=1 and forwardedAgain.pipeBytes<=1 and
      forwardedAgain.mailboxTokens==2)
  }

  run op62SplitDoorbellAckCanLoseLatestArrivalTest = {
    val observed={delivered:1,bytes:1,pending:true}
    val raced={...observed,delivered:2}
    val lateAck={...raced,bytes:0,pending:false}
    assert(raced.delivered>observed.delivered and lateAck.bytes==0 and
      not(lateAck.pending))
  }

  run op62ArrivalDeadlineCannotReplaceHeartbeatDeadlineTest = {
    val heartbeat={deadline:1000,period:1000,kind:"vm-heartbeat"}
    val arrival={deadline:200,period:0,kind:"source-arrival"}
    val replacement=arrival
    assert(replacement.deadline<heartbeat.deadline and
      replacement.period!=heartbeat.period and replacement.kind!=heartbeat.kind)
  }

  run op62ArrivalWakeCannotResetReplyDeadlineTest = {
    val firstWait=100
    val limit=1000
    val arrivalWake=900
    val monotonicNow=1100
    assert(monotonicNow>=firstWait+limit and monotonicNow<arrivalWake+limit)
  }

  run op62FractionalNextEligibilityNeedsCeilingTest = {
    val units=10
    val rate=3
    val ordinal=1
    val planned=ordinal*units/rate
    val eligible=(ordinal*units+rate-1)/rate
    assert(planned==3 and eligible==4 and planned*rate/units<ordinal and
      eligible*rate/units==ordinal)
  }

  run op62ReplyOnlyWaitCanStrandEligibleWorkWithFreeFramesTest = {
    val capacity=3
    val atWait={published:4,completed:3,due:4}
    val atNextArrival={...atWait,due:5}
    val atDeadline={...atWait,due:12}
    assert(atWait.due-atWait.published==0 and
      atNextArrival.published-atNextArrival.completed<capacity and
      atNextArrival.due-atNextArrival.published==1 and
      atDeadline.due-atDeadline.published==8)
  }

  run op62FinalDrainCannotRemoveAnyBacklogBreachTest = {
    val maxFrames=3
    val guard=128
    val total=1000
    assert(0.to(maxFrames).forall(live => {
      val completed=total-live
      val published=total
      val due=total
      due-published==0 and due-completed==live and due-completed<=guard
    }))
  }

  run op62RecoveryRequiresSpareCapacityAboveOfferedRateTest = assert(
    budget::recoveryRatePerSecond(500000,351,128,1)==723000 and
    budget::recoveryRatePerSecond(1000000,6746,128,1)==7618000 and
    budget::recoveryRatePerSecond(1000000,6746,128,10)==1661800)

  // The retained request-deadline receipts are coherent but only expose
  // source and aggregate frame custody, not each sibling's execution phase.
  type DeadlineReceipt = {due:int,published:int,completed:int,elapsed:int,lastPublish:int,maxSourceLag:int}
  pure val frozenControlFailure:DeadlineReceipt =
    {due:1000000,published:936343,completed:936340,elapsed:10938620250,lastPublish:10913902792,maxSourceLag:9977562958}
  pure val accumulatorFailure:DeadlineReceipt =
    {due:1000000,published:791219,completed:791216,elapsed:10799639875,lastPublish:10787582500,maxSourceLag:9996364709}
  pure def sourceDebt(r:DeadlineReceipt):int=r.due-r.published
  pure def outstandingFrames(r:DeadlineReceipt):int=r.published-r.completed
  pure def lastOrdinalLag(r:DeadlineReceipt):int=r.lastPublish-(r.published-1)*1000
  pure def minimumPostPublication(r:DeadlineReceipt):int=r.elapsed-r.lastPublish
  pure def oldestUnpublishedAge(r:DeadlineReceipt):int=r.elapsed-r.published*1000
  run op62DeadlinePreservesSourceAndFrameDebtTest=assert(
    sourceDebt(frozenControlFailure)==63657 and sourceDebt(accumulatorFailure)==208781 and
    Set(frozenControlFailure,accumulatorFailure).forall(r =>
      outstandingFrames(r)==3 and r.due-r.completed==sourceDebt(r)+outstandingFrames(r)))
  run op62DeadlineSpansBothSidesOfPublicationTest=assert(
    lastOrdinalLag(frozenControlFailure)==9977560792 and lastOrdinalLag(accumulatorFailure)==9996364500 and
    Set(frozenControlFailure,accumulatorFailure).forall(r =>
      lastOrdinalLag(r)<=r.maxSourceLag and minimumPostPublication(r)>10000000 and
      oldestUnpublishedAge(r)>10000000000))
  pure def visibleFrames(queued:int, running:int, ready:int):int=queued+running+ready
  run op62TerminalFrameCountCannotAttributeSiblingWaitTest=assert(
    visibleFrames(1,1,1)==outstandingFrames(frozenControlFailure) and
    visibleFrames(0,0,3)==outstandingFrames(frozenControlFailure))

  // The progress deadline fires after adding the current latency sample
  // but before returning the frame. The histogram does not name that reply.
  run op62ProgressDeadlineSampleCannotIdentifyUnretiredReplyTest = {
    val first={published:43,completed:40,samples:41,active:3,ordinal:40}
    val sibling={...first,ordinal:42}
    assert(first.ordinal!=sibling.ordinal and
      List(first.published,first.completed,first.samples,first.active)==
      List(sibling.published,sibling.completed,sibling.samples,sibling.active) and
      first.samples==first.completed+1 and first.active==first.published-first.completed)
  }

  // At the total peak, unpublished debt cannot exceed its own global peak.
  pure def activeAtTotalPeak(sourcePeak:int,totalPeak:int):Set[int] =
    0.to(3).filter(active => 0<=totalPeak-active and totalPeak-active<=sourcePeak)

  run op62MainTotalDebtPeakExcludesZeroActiveTest = assert(
    activeAtTotalPeak(349,351)==Set(2,3) and
    activeAtTotalPeak(6744,6746)==Set(2,3) and
    activeAtTotalPeak(526901,526903)==Set(2,3) and
    activeAtTotalPeak(729886,729888)==Set(2,3))

  // These maxima come from the same drive-loop samples, before supplying.
  // At a source-peak sample the global total peak bounds its total.
  pure def matchesWarmupPeak(sourcePeak:int,totalPeak:int,active:int):bool =
    0 <= active and active <= 3 and sourcePeak+active <= totalPeak
  run op62WarmupEqualPeaksExcludeOccupiedFramesAtSourcePeakTest = assert(
    Set(1,96,131).forall(peak =>
      0.to(3).filter(active => matchesWarmupPeak(peak,peak,active)) == Set(0)))
  pure def oldestDueAgeFloor(sourceDebt:int,period:int):int=(sourceDebt-1)*period
  run op62WarmupSourcePeakBoundsOldestUnpublishedAgeTest = assert(
    oldestDueAgeFloor(131,1000000)==130000000 and
    oldestDueAgeFloor(96,1000000)==95000000)
    // An empty frame set at the sampled peak gives no preceding frame state.
  run op62WarmupPeakCannotDistinguishPreviousWaitPhaseTest = assert(
    matchesWarmupPeak(131,131,0) and
    matchesWarmupPeak(130,131,1) and
    matchesWarmupPeak(130,131,0))

  // A same-owner progress/OS sample observes neither Scheme frame custody
  // nor the boundary from a worker's ready store to Mailbox delivery.
  type WarmupGapState = {
    lastProgress:int, recovery:int, osState:int, queued:int, running:int, ready:int, delivered:int
  }
  pure def sparseGapProjection(s:WarmupGapState):List[int] =
    List(s.lastProgress,s.recovery,s.osState)
  run op62SparseGapCannotDistinguishActorFromReplyTest = {
    val queued = {lastProgress:10,recovery:140,osState:1,queued:1,running:0,ready:0,delivered:0}
    val running = {...queued,queued:0,running:1}
    val beforeSend = {...queued,queued:0,ready:1}
    val delivered = {...beforeSend,delivered:1}
    val idleSource = {...queued,queued:0}
    assert(Set(queued,running,beforeSend,delivered,idleSource).size()==5 and
      Set(queued,running,beforeSend,delivered,idleSource).map(s => sparseGapProjection(s)).size()==1)
  }

  // One isolated gap record. The timestamps alone are insufficient: the
  // trigger must attest custody, and recovery must witness an actual boundary.
  type GapCapture = {owner:int,generation:int,gap:int,ordinal:int,trigger:int,state:str,wakeInjected:bool}
  type GapRecovery = {owner:int,generation:int,gap:int,ordinal:int,dequeuedAt:int,at:int,boundary:str}
  pure def warmupGapClass(active:int,source:str,frame:str,inMailbox:bool):str =
    if (active==0 and source=="yield") "idle-source"
    else if (frame=="queued") "actor-not-started"
    else if (frame=="running") "actor-running"
    else if (frame=="ready" and inMailbox) "reply-in-mailbox"
    else if (frame=="ready") "ready-delivery-unproven"
    else "unknown"
  pure def matchingRecovery(c:GapCapture,r:GapRecovery):bool =
    not(c.wakeInjected) and c.owner==r.owner and c.generation==r.generation and c.gap==r.gap and
    c.ordinal==r.ordinal and r.at>c.trigger and
    (if (c.state=="idle-source") r.boundary=="published"
     else r.boundary=="retired" and
       (if (c.state=="reply-dequeued-credit-held")
          0<=r.dequeuedAt and r.dequeuedAt<=c.trigger
        else Set("actor-not-started","actor-running","reply-in-mailbox").contains(c.state) and
             c.trigger<r.dequeuedAt and r.dequeuedAt<r.at))
  pure def recoveryDisposition(phase:str,c:GapCapture,r:GapRecovery):str =
    if (phase=="triggered" and matchingRecovery(c,r)) "sealed" else phase
  run op62BootstrapWitnessCannotSealLaterProgressGapTest = {
    val bootstrap={owner:1,generation:2,gap:0,ordinal:0,trigger:100,
      state:"reply-in-mailbox",wakeInjected:false}
    val firstReturn={owner:1,generation:2,gap:0,ordinal:0,dequeuedAt:110,at:120,
      boundary:"retired"}
    val laterGap={...bootstrap,gap:1,ordinal:1,trigger:130}
    assert(matchingRecovery(bootstrap,firstReturn) and
      not(matchingRecovery(laterGap,firstReturn)))
  }

  run op62OneShotRequiresActualRecoveryAndMatchingGenerationTest = {
    val c={owner:1,generation:2,gap:3,ordinal:41,trigger:130,state:"reply-in-mailbox",wakeInjected:false}
    val r={owner:1,generation:2,gap:3,ordinal:41,dequeuedAt:135,at:140,boundary:"retired"}
    assert(recoveryDisposition("triggered",c,r)=="sealed" and
      recoveryDisposition("triggered",c,{...r,boundary:"verified"})=="triggered" and
      recoveryDisposition("triggered",c,{...r,generation:3})=="triggered" and
      recoveryDisposition("triggered",c,{...r,dequeuedAt:-1})=="triggered" and
      recoveryDisposition("triggered",c,{...r,dequeuedAt:129})=="triggered" and
      recoveryDisposition("triggered",c,{...r,dequeuedAt:141})=="triggered" and
      recoveryDisposition("triggered",{...c,wakeInjected:true},r)=="triggered" and
      recoveryDisposition("triggered",{...c,state:"unknown"},r)=="triggered" and
      recoveryDisposition("triggered",{...c,state:"ready-delivery-unproven"},r)=="triggered" and
      recoveryDisposition("closed",c,r)=="closed" and
      recoveryDisposition("sealed",c,r)=="sealed")
  }
  run op62OneShotCanSealAlreadyDequeuedCreditHeldTest = {
    val held={owner:1,generation:2,gap:3,ordinal:41,trigger:130,
      state:"reply-dequeued-credit-held",wakeInjected:false}
    val retired={owner:1,generation:2,gap:3,ordinal:41,dequeuedAt:129,at:140,boundary:"retired"}
    assert(recoveryDisposition("triggered",held,retired)=="sealed" and
      recoveryDisposition("triggered",held,{...retired,dequeuedAt:131})=="triggered" and
      recoveryDisposition("triggered",held,{...retired,dequeuedAt:-1})=="triggered" and
      recoveryDisposition("triggered",held,{...retired,ordinal:42})=="triggered")
  }

  run op62OneShotSiblingProgressCannotSealCapturedFrameTest = {
    val frame={owner:1,generation:2,gap:3,ordinal:41,trigger:130,state:"reply-in-mailbox",wakeInjected:false}
    val sibling={owner:1,generation:2,gap:3,ordinal:42,dequeuedAt:135,at:140,boundary:"retired"}
    val source={...frame,state:"idle-source"}
    assert(recoveryDisposition("triggered",frame,sibling)=="triggered" and
      recoveryDisposition("triggered",frame,{...sibling,ordinal:41})=="sealed" and
      recoveryDisposition("triggered",source,{...sibling,boundary:"published"})=="triggered" and
      recoveryDisposition("triggered",source,{...sibling,ordinal:41,boundary:"published"})=="sealed")
  }
  run op62OneShotDoesNotInferDeliveryFromReadyTagTest = assert(
    warmupGapClass(0,"yield","free",false)=="idle-source" and
    warmupGapClass(1,"receive","queued",false)=="actor-not-started" and
    warmupGapClass(1,"receive","running",false)=="actor-running" and
    warmupGapClass(1,"receive","ready",false)=="ready-delivery-unproven" and
    warmupGapClass(1,"receive","ready",true)=="reply-in-mailbox")

  // Public call boundaries bound the transitions; overlap with a call
  // leaves its internal transition unknown. All events describe one ordinal.
  type ReplyBounds = {
    sendEnter:int,sendReturn:int,receiveEnter:int,receiveReturn:int,retire:int
  }
  pure def custodyAt(b:ReplyBounds,t:int):str =
    if (t>b.retire) "retired"
    else if (t>b.receiveReturn and t<b.retire) "dequeued"
    else if (t>b.sendReturn and t<b.receiveEnter) "in-mailbox"
    else if (t<b.sendEnter) "before-send"
    else "unknown"

  run op62PublicBoundariesAttestOnlyOutsideAmbiguousIntervalsTest = {
    val b={sendEnter:10,sendReturn:20,receiveEnter:30,receiveReturn:40,retire:50}
    assert(custodyAt(b,9)=="before-send" and custodyAt(b,15)=="unknown" and
      custodyAt(b,25)=="in-mailbox" and custodyAt(b,35)=="unknown" and
      custodyAt(b,45)=="dequeued" and custodyAt(b,50)=="unknown" and
      custodyAt(b,51)=="retired")
  }

  run op62BlockedReceiveAndRescheduledSenderLeaveNoPublicInMailboxWindowTest = {
    val b={sendEnter:20,sendReturn:60,receiveEnter:10,receiveReturn:30,retire:40}
    assert(custodyAt(b,25)=="unknown" and custodyAt(b,35)=="dequeued" and
      custodyAt(b,45)=="retired")
  }

  // A cold terminal ordinal witnesses dequeue before the deadline check.
  // It gives no new observation at the earlier gap trigger.
  run op62DeadlineOrdinalDoesNotIdentifyEarlierMailboxCustodyTest = {
    val early={sendEnter:10,sendReturn:20,receiveEnter:40,receiveReturn:50,retire:80}
    val late={...early,sendEnter:35,sendReturn:39}
    assert(custodyAt(early,30)=="in-mailbox" and custodyAt(late,30)=="before-send" and
      custodyAt(early,60)=="dequeued" and custodyAt(late,60)=="dequeued")
  }

  // A recovery probe only supplies an upper bound on enqueue. A
  // pre-trigger send-return receipt narrows that bound for a reply still
  // in the FIFO at recovery, provided its identity cannot be re-enqueued.
  run op62RecoveryProbeNeedsIndependentPreTriggerSendReceiptTest = {
    val early={sendReturn:20,trigger:30,probe:40,ordinal:41,retire:50}
    val late={...early,sendReturn:35}
    assert(List(early.probe,early.ordinal,early.retire)==
      List(late.probe,late.ordinal,late.retire) and
      early.sendReturn<early.trigger and late.trigger<late.sendReturn)
  }

  // Both frozen loops publish a clear failure flag. Only the worker's
  // exception path sets it, and the source raises before normal retirement.
  pure def op62FailureFlagAfterSuccess(flag:bool, reset:bool):bool =
    if (reset) false else flag

  run op62SuccessOnlyFailureFlagResetIsIdempotentTest = {
    val workerFlags=Set(false,true)
    val admittedToRetirement=workerFlags.filter(flag => not(flag))
    assert(admittedToRetirement==Set(false) and
      admittedToRetirement.forall(flag =>
        op62FailureFlagAfterSuccess(flag,false)==op62FailureFlagAfterSuccess(flag,true)) and
      op62FailureFlagAfterSuccess(true,false)!=op62FailureFlagAfterSuccess(true,true))
  }

  // Between consecutive successful progress timestamps on one source,
  // only the earlier frame retires. The later frame still holds credit.
  // Use the full offered interval, before arrival clipping at the end.
  pure def op62DebtAfterProgressGap(initial:int,gapNs:int,periodNs:int):int =
    initial + gapNs / periodNs - 1

  run op62ProgressGapCanForceBacklogDespiteNormalRetirementTest = {
    val guard=128
    assert(op62DebtAfterProgressGap(0,258000,2000)==guard and
      op62DebtAfterProgressGap(0,260000,2000)>guard and
      op62DebtAfterProgressGap(0,129000,1000)==guard and
      op62DebtAfterProgressGap(0,130000,1000)>guard and
      op62DebtAfterProgressGap(32,98000,1000)>guard)
  }

  run op62LowRateGapDoesNotQualifyProductionGuardTest = {
    val historicalGapNs=8107875
    assert(op62DebtAfterProgressGap(0,historicalGapNs,1000000)==7 and
      op62DebtAfterProgressGap(0,historicalGapNs,2000)==4052 and
      op62DebtAfterProgressGap(0,historicalGapNs,1000)==8106)
  }

  pure def op62ReplyDisposition(identityOk:bool, state:str):str =
    if (not(identityOk)) "frame-identity"
    else if (state=="ready") "consume"
    else if (state=="failed-ready") "worker-exception"
    else "frame-identity"

  run op62FailureTagRequiresReplyIdentityBeforeExceptionTest = {
    val states=Set("free","queued","running","ready","failed-ready")
    assert(states.forall(state => op62ReplyDisposition(false,state)=="frame-identity") and
      states.filter(state => op62ReplyDisposition(true,state)=="consume")==Set("ready") and
      states.filter(state => op62ReplyDisposition(true,state)=="worker-exception")==Set("failed-ready"))
  }

  // Conditional counterexample: an escaped worker continuation could
  // write through the recycled frame before its old reply is rejected.
  run op62StaleReplyGuardCannotUndoLateWorkerWriteTest = {
    val oldOrdinal=0
    val recycled={ordinal:3,state:"running",payload:"new-request"}
    val afterReplay={...recycled,state:"ready",payload:"old-result"}
    assert(op62ReplyDisposition(oldOrdinal==afterReplay.ordinal,afterReplay.state)=="frame-identity" and
      afterReplay.payload!=recycled.payload)
  }

  run op62ReadyStateEnvelopeCannotExplainFullOwnerGapTest = {
    val frozenInnerWall=9761178791
    val candidateInnerWall=10792667209
    val frozenCpuUpper=1635004125
    val candidateCpuUpper=1133220708
    val frozenGcWallUpper=152960539
    val candidateGcWallUpper=155998469
    assert(frozenInnerWall-frozenCpuUpper-frozenGcWallUpper==7973214127 and
      candidateInnerWall-candidateCpuUpper-candidateGcWallUpper==9503448032)
  }

  // After the last send no new credit can enter. A sealed full frame
  // count therefore rules out any retirement in that suffix.
  run op62ReadyStateFullTerminalCreditsExcludeSuffixRetirementTest = {
    val terminalActive=3
    val capacity=3
    val feasibleRetirements=0.to(capacity).filter(n => terminalActive+n<=capacity)
    assert(feasibleRetirements==Set(0) and
      10792667209-10735661459==57005750 and
      10792667209-(736365-1)*1000==10056303209)
  }

  // The sealed counters do not report the outstanding ordinals.
  run op62FullTerminalCreditsCannotIdentifyWorkerCollisionTest = {
    val published=736365
    val last=published-1
    val possible=Set(Set(last-2,last-1,last),Set(last-4,last-1,last),Set(last-8,last-4,last))
    assert(possible.forall(live => live.contains(last) and published-live.size()==736362) and
      possible.map(live => live.map(id => id % 4).size())==Set(1,2,3))
  }

  // Slot ordinals are source-owned. The free sentinel is not a route.
  run op62TerminalSlotOrdinalsCanDistinguishWorkerRoutesTest = {
    val oneLive=Set(0,-1).exclude(Set(-1))
    val collided=Set(736356,736360,736364)
    val spread=Set(736362,736363,736364)
    assert(oneLive==Set(0) and collided.map(id => id % 4)==Set(0) and
      spread.map(id => id % 4)==Set(0,2,3))
  }

  run op62PostSealPeerTagDoesNotWitnessDeadlineStateTest = {
    val first={ordinal:736363,atDeadline:"queued",afterSeal:"ready"}
    val second={...first,atDeadline:"ready"}
    assert(first.ordinal==second.ordinal and first.afterSeal==second.afterSeal and
      first.atDeadline!=second.atDeadline)
  }

  run op62ReadyStateDeadlinePrefixCannotPassAdmissionTest = {
    val due=1000000
    val published=736365
    val completed=736362
    val peak=929191
    val decision={hardFailure:true,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(published-completed==3 and due-published==263635 and
      due-completed==263638 and peak>128 and
      gate::researchDisposition(decision)=="withdraw" and
      not(gate::maySeekAdmissionReview(decision)))
  }

  // A queued reference to a mutable frame (or its reusable wrapper)
  // observes the new ordinal after the slot is recycled.
  run op62ReusableReplyEnvelopeCannotRejectStaleReferenceTest = {
    val retired={slot:1,ordinal:41,state:"ready"}
    val recycled={...retired,ordinal:44}
    val queuedOrdinal=retired.ordinal
    assert(retired.slot==recycled.slot and recycled.state=="ready" and
      queuedOrdinal!=recycled.ordinal)
  }

  // Moving the one immutable pair to request publication keeps both
  // Mailbox handoffs. It makes the pair live during Actor execution.
  run op62RoundTripEnvelopeMovesAllocationAndExtendsLifetimeTest = {
    val replyOnly={pairs:1,sends:2,receives:2,allocatedAt:30,retiredAt:40}
    val roundTrip={...replyOnly,allocatedAt:10}
    val handlerRunningAt=20
    assert(roundTrip.pairs==replyOnly.pairs and roundTrip.sends==replyOnly.sends and
      roundTrip.receives==replyOnly.receives and
      roundTrip.allocatedAt<handlerRunningAt and handlerRunningAt<replyOnly.allocatedAt and
      roundTrip.retiredAt==replyOnly.retiredAt)
  }

  run op62EvidenceRejectsPrivateInboxAndCursorRewindTest = {
    val passive={cost:"qualified-upper",availability:"qualified-envelope",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:true,
      readPrivateInbox:false,rewoundCursor:false}
    assert(gate::evidenceQualified(passive) and
      not(gate::evidenceQualified({...passive,readPrivateInbox:true})) and
      not(gate::evidenceQualified({...passive,rewoundCursor:true})) and
      not(gate::evidenceQualified({...passive,retirementTail:"unknown"})))
  }

  // A one-shot write budget does not bound the cost of requests that miss
  // the ordinal guard. These are static costs from the isolated op62 image.
  pure def op62MissIsUnchanged(stackReads:int, comparisons:int, branches:int):bool =
    stackReads==0 and comparisons==0 and branches==0

  run op62OneShotRecordStillChangesUnmatchedReplyPathTest = {
    val unmatchedSend={writes:0,stackReads:1,comparisons:1,branches:1}
    val unmatchedReceive={writes:0,stackReads:0,comparisons:1,branches:1}
    val unmatchedRetire=unmatchedSend
    assert(Set(unmatchedSend,unmatchedReceive,unmatchedRetire).forall(miss =>
      miss.writes==0 and
      not(op62MissIsUnchanged(miss.stackReads,miss.comparisons,miss.branches))))
  }

  // Even phase-labelled copies of the existing source clock reads leave
  // the reply enqueue and actual credit return unobserved.
  run op62ExistingClockSitesCannotLocateReplyCustodyTest = {
    val clockSites=List(10,20,100)
    val lateEnqueue={clocks:clockSites,enqueue:90,dequeue:95,retire:105}
    val earlyEnqueue={...lateEnqueue,enqueue:30}
    assert(lateEnqueue.clocks==earlyEnqueue.clocks and
      earlyEnqueue.enqueue<60 and 60<lateEnqueue.enqueue and
      lateEnqueue.dequeue<lateEnqueue.clocks[2] and
      lateEnqueue.clocks[2]<lateEnqueue.retire)
  }

  // Observer maxima are selected independently. A matching ordinal still
  // cannot make pre-send handoff and verified progress custody receipts.
  run op62ObserverMaximaCannotSealGapTest = {
    val capture={owner:1,generation:2,gap:3,ordinal:41,trigger:130,state:"reply-in-mailbox",wakeInjected:false}
    val sibling={owner:1,generation:2,gap:3,ordinal:42,dequeuedAt:135,at:150,boundary:"retired"}
    val beforeReturn={...sibling,ordinal:41,boundary:"verified"}
    assert(not(matchingRecovery(capture,sibling)) and
      not(matchingRecovery(capture,beforeReturn)) and
      matchingRecovery(capture,{...beforeReturn,boundary:"retired"}))
  }

  // A join deadline is an observation failure, not a transfer of custody.
  // Closing drains unread replies without counting a successful retirement.
  pure def closeDisposition(joined:Set[int],workers:Set[int],unread:int):str =
    if (joined!=workers) "hold-frames"
    else if (unread>0) "drain-replies"
    else "clear-frames"

  run op62CloseTimeoutPreservesFramesUntilAllWorkersJoinTest = {
    val workers=Set(0,1,2,3)
    val joinedAfterDeadline=Set(0,2,3)
    assert(closeDisposition(joinedAfterDeadline,workers,0)=="hold-frames" and
      closeDisposition(joinedAfterDeadline,workers,1)=="hold-frames" and
      closeDisposition(workers,workers,1)=="drain-replies" and
      closeDisposition(workers,workers,0)=="clear-frames")
  }

  // The post-send exception fixture leaves published at zero even
  // though the worker owns a live frame and can still reply.
  run op62UncommittedSendCannotAuthorizeEarlyClearTest = {
    val beforeSend={published:0,workerOwnsFrame:false,unread:0}
    val afterSend={...beforeSend,workerOwnsFrame:true}
    val lateReply={...afterSend,unread:1}
    assert(beforeSend.published==afterSend.published and
      beforeSend.workerOwnsFrame!=afterSend.workerOwnsFrame and
      closeDisposition(Set(),Set(1),afterSend.unread)=="hold-frames" and
      closeDisposition(Set(1),Set(1),lateReply.unread)=="drain-replies" and
      closeDisposition(Set(1),Set(1),0)=="clear-frames")
  }

  // The same public waiting-for identity is returned on each blocking
  // receive by one thread. It does not identify the request it awaits.
  run op62PublicWaitObjectCannotProveTargetReplyWasNotDequeuedTest = {
    val before={mailbox:1,dequeued:false,creditHeld:true}
    val after={mailbox:1,dequeued:true,creditHeld:true}
    assert(before!=after and before.mailbox==after.mailbox and
      before.creditHeld==after.creditHeld)
  }

  // The paired candidate completed all ordinals, but fewer generated
  // instructions and lower enclosing CPU did not admit it.
  run op62PreSupplyGatePairedReceiptsFailAbsoluteAdmissionTest = {
    val frozen={elapsed:1123607792,p99:124275000,peak:124230,cpu:1086108043}
    val candidate={elapsed:1612427292,p99:614913000,peak:615067,cpu:1043485460}
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false}
    assert(candidate.cpu<frozen.cpu and candidate.elapsed>frozen.elapsed and
      candidate.p99>frozen.p99 and frozen.p99>6393000 and
      candidate.peak>frozen.peak and frozen.peak>128 and
      not(gate::maySeekAdmissionReview(decision)))
  }

  // The CPU and GC totals enclose the measured prefix and the timed
  // service interval. Subtracting both overcounts any shared time.
  run op62PreSupplyGateHasWallOutsideCpuAndGcBeforeLastPublicationTest = {
    val frozen={lastPublish:1123606375,cpu:1086108043,gc:15872241}
    val candidate={lastPublish:1612425875,cpu:1043485460,gc:14918566}
    assert(frozen.lastPublish-frozen.cpu-frozen.gc==21626091 and
      candidate.lastPublish-candidate.cpu-candidate.gc==554021849)
  }

  // At most three frames can charge wall time after publication.
  // Intersect the nearest-rank tail with requests below the 1 ms cutoff.
  run op62PreSupplyGateTailNeedsPrepublicationTimeOutsideGcTest = {
    val tailCount=1000000-budget::rank(1000000,99,100)+1
    val frozen={elapsed:1123607792,p99:124275000,gc:15872241}
    val candidate={elapsed:1612427292,p99:614913000,gc:14918566}
    val cutoff=1000000
    assert(tailCount-3*frozen.elapsed/cutoff==6631 and
      tailCount-3*candidate.elapsed/cutoff==5164 and
      frozen.p99-1000-cutoff-frozen.gc==107401759 and
      candidate.p99-1000-cutoff-candidate.gc==598993434)
  }

  // The public snapshot can allocate a fresh timeout object on each read
  // even when condition identity and the original receive are unchanged.
  run op62PublicTimeoutObjectIdentityCannotCountReceivesTest={
    val first={receive:1,condition:7,absoluteDeadline:100,object:21}
    val repeat={...first,object:22}
    assert(first.receive==repeat.receive and first.condition==repeat.condition and
      first.absoluteDeadline==repeat.absoluteDeadline and first.object!=repeat.object)
  }

  // On this runtime, both an empty condition signal and a business send
  // can move an as-yet-unresumed receiver to processor wait with no timeout.
  run op62ProcessorWaitWithoutTimeoutCannotAttestEnqueueTest={
    val emptyWake={waitingFor:"processor",hasTimeout:false,enqueued:false,received:false,retired:false}
    val queuedReply={...emptyWake,enqueued:true}
    assert(List(emptyWake.waitingFor==queuedReply.waitingFor,
                emptyWake.hasTimeout==queuedReply.hasTimeout,
                emptyWake.received==queuedReply.received,
                emptyWake.retired==queuedReply.retired)==List(true,true,true,true) and
      emptyWake.enqueued!=queuedReply.enqueued)
  }

  // A calibrated condition wait can exclude a completed wake only for
  // the sole FIFO consumer on the same cut. It never proves send-entry
  // or insertion did not happen before an unfinished send.
  type Op62ConditionCut={sameCut:bool,soleConsumer:bool,ordinaryFifo:bool,
    calibratedCondition:bool,heldTarget:bool,fifoWakeContract:bool}
  pure def completedSendExcludedAtCut(c:Op62ConditionCut):bool =
    c.sameCut and c.soleConsumer and c.ordinaryFifo and c.calibratedCondition and
    c.heldTarget and c.fifoWakeContract

  run op62OrdinaryConditionExclusionRequiresLiveFifoCutTest={
    val fifo={sameCut:true,soleConsumer:true,ordinaryFifo:true,calibratedCondition:true,
      heldTarget:true,fifoWakeContract:true}
    assert(completedSendExcludedAtCut(fifo) and
      not(completedSendExcludedAtCut({...fifo,ordinaryFifo:false})) and
      not(completedSendExcludedAtCut({...fifo,calibratedCondition:false})) and
      not(completedSendExcludedAtCut({...fifo,sameCut:false})) and
      not(completedSendExcludedAtCut({...fifo,heldTarget:false})) and
      not(completedSendExcludedAtCut({...fifo,fifoWakeContract:false})))
  }

  run op62SelectiveReceiveCanReparkWithACompletedReplySendTest={
    val empty={condition:7,deadline:100,replyQueued:false,completedSend:false}
    val skipped={...empty,replyQueued:true,completedSend:true}
    assert(empty.condition==skipped.condition and empty.deadline==skipped.deadline and
      skipped.replyQueued and skipped.completedSend and not(empty.replyQueued))
  }

  // Original request age precedes histogram mutation. Original heartbeat
  // age follows it, and both checks precede verified frame return.
  pure def op62RetirementClockDecision(planned:int,previous:int,
      latencySample:int,progressSample:int):{phase:str,sampled:bool,heartbeat:int,held:bool} =
    if (latencySample-planned>=10000000000)
      {phase:"request-deadline",sampled:false,heartbeat:previous,held:true}
    else if (progressSample-previous>=10000000000)
      {phase:"progress-deadline",sampled:true,heartbeat:previous,held:true}
    else {phase:"retired",sampled:true,heartbeat:progressSample,held:false}

  run op62SingleRetirementClockCanChangeOriginalDeadlineAndCreditTest={
    val early=10000000099
    val late=early+1
    val originalProgress=op62RetirementClockDecision(101,100,early,late)
    val originalSuccess=op62RetirementClockDecision(100,101,early,late)
    assert(originalProgress=={phase:"progress-deadline",sampled:true,heartbeat:100,held:true} and
      op62RetirementClockDecision(101,100,early,early).phase=="retired" and
      originalSuccess=={phase:"retired",sampled:true,heartbeat:late,held:false} and
      op62RetirementClockDecision(100,101,late,late)==
        {phase:"request-deadline",sampled:false,heartbeat:101,held:true})
  }

  run op62EarlyClockAlsoChangesSuccessfulHeartbeatTest={
    val original=op62RetirementClockDecision(101,100,102,103)
    val reused=op62RetirementClockDecision(101,100,102,102)
    assert(original.phase==reused.phase and original.heartbeat==reused.heartbeat+1)
  }

  // The worker has no write authority after successful publication,
  // including when a scheduler poll resumes its normal send continuation.
  run op62WorkerPostSendClearCanCorruptAnImmediatelyRefilledSlotTest={
    val atSend={slot:0,ordinal:41,payload:"old-result",state:"ready"}
    val atReturn={...atSend,ordinal:42,payload:"new-input",state:"queued"}
    val staleClear={...atReturn,payload:"cleared"}
    val sentOrdinal=atSend.ordinal
    assert(atSend.slot==atReturn.slot and sentOrdinal!=atReturn.ordinal and
      staleClear.ordinal==atReturn.ordinal and staleClear.state==atReturn.state and
      staleClear.payload!=atReturn.payload)
  }

  // Successful integral-rate FIFO source. The same owner publishes in
  // ordinal order. A worst-lag sample may come from any ordinal.
  pure def op62LaterPublicationsFloor(total:int,period:int,last:int,lag:int):int = {
    val drop=lag-(last-(total-1)*period)
    if (drop>0) budget::ceilDiv(drop,period) else 0
  }

  pure def op62SuffixRetirementRateFloor(total:int,period:int,end:int,lag:int):int =
    if (end>lag and lag>end-(total-1)*period) (total-1)*1000000000/(end-lag)
    else 0

  run op62LagRecoverySuffixBoundAcrossWorstOrdinalPositionsTest=assert(
    2.to(12).forall(total => 1.to(total-1).forall(worst => {
      val lag=10
      val period=4
      val end=(total-1)*period+9
      val remaining=total-worst
      val suffix=end-((worst-1)*period+lag)
      suffix>0 and remaining>=op62LaterPublicationsFloor(total,period,end,lag) and
        remaining*1000000000/suffix>=op62SuffixRetirementRateFloor(total,period,end,lag)
    })))

  run op62FrozenSuffixRecoveryDiffersAtOneAndFiveMillionTest=assert(
    op62LaterPublicationsFloor(5000000,2000,9999998500,697625)==349 and
    op62SuffixRetirementRateFloor(5000000,2000,9999999542,697625)==500034 and
    op62LaterPublicationsFloor(1000000,1000,1004776833,6743416)==1966 and
    op62SuffixRetirementRateFloor(1000000,1000,1004778708,6743416)==1001967 and
    op62LaterPublicationsFloor(5000000,200,4984036084,3984036284)==0 and
    op62SuffixRetirementRateFloor(5000000,200,4984038584,3984036284)==0)

  // The next drive sample can hide at most one return. A complete call
  // also has a final zero-debt end cut.
  pure def op62OfferedPrefixReturnedFloor(total:int,peak:int):int =
    if(total>peak+1) total-peak-1 else 0

  // If the entire offered window is shorter than the quantile lower
  // edge, each request in that tail needs at least excess after its
  // publication to have published within the offered window.
  pure def op62OfferedPrefixUnpublishedFloor(tailCount:int,excess:int,elapsed:int,frames:int):int =
    if(excess<=0 or tailCount<=frames*elapsed/excess) 0 else tailCount-frames*elapsed/excess

  run op62FiveMillionHasUnpublishedTailAtEndOfOffersTest={
    val total=5000000
    val tailCount=total-budget::rank(total,99,100)+1
    val excess=3943455000-1000-999999800
    assert(tailCount==50001 and excess==2943454200 and
      op62OfferedPrefixUnpublishedFloor(tailCount,excess,4984038584,3)==49996 and
      op62OfferedPrefixReturnedFloor(total,4000404)==999595 and
      total-tailCount==4949999 and tailCount>128)
  }

  run op62OfferedPrefixFloorRetainsEachFrozenWorkloadTest=assert(
    op62OfferedPrefixReturnedFloor(5000000,351)==4999648 and
    op62OfferedPrefixReturnedFloor(1000000,6746)==993253 and
    op62OfferedPrefixUnpublishedFloor(10001,-993607000,1004778708,3)==0)

  // A post-send sample is a positive eligibility certificate for the
  // ordinals it covers. It cannot wake the original pending receive.
  run op62ArrivalAfterPostSendSampleCanLeaveCreditUnusedTest={
    val sample={postSend:99,active:1,capacity:3,published:1,now:99,
      replyQueued:false,waitingForReply:false}
    val nextDue=100
    val parked={...sample,now:100,replyQueued:false,waitingForReply:true}
    val reply={...parked,now:102,replyQueued:true,waitingForReply:false}
    assert(sample.postSend<nextDue and nextDue<=parked.now and
      parked.active<parked.capacity and parked.waitingForReply and
      not(parked.replyQueued) and parked.published==sample.published and
      reply.published==parked.published and reply.replyQueued)
  }

  // A fresh timed receive can take a queued reply even at a past
  // deadline. Once the real timeout is returned, that case is sealed.
  pure def op62MailboxWakeDisposition(replyQueued:bool,arrived:bool,
      actualTimeout:bool,terminalSealed:bool):str =
    if(terminalSealed) "cleanup-only"
    else if(replyQueued) "verify-reply"
    else if(actualTimeout) "seal-timeout"
    else if(arrived) "supply"
    else "wait"

  run op62NativeArrivalHintCannotReopenOriginalTimeoutTest=assert(
    op62MailboxWakeDisposition(true,true,true,false)=="verify-reply" and
    op62MailboxWakeDisposition(false,true,true,false)=="seal-timeout" and
    op62MailboxWakeDisposition(true,true,true,true)=="cleanup-only" and
    op62MailboxWakeDisposition(false,false,false,false)=="wait" and
    op62MailboxWakeDisposition(false,true,false,false)=="supply")

  // An empty receive returning early gives no arrival authority. A
  // late return can spend the whole backlog allowance even with no
  // preexisting debt or additional service cost.
  run op62NativeMailboxHintDoesNotQualifyMonotonicProgressTest={
    val oneMillion={periodNs:1000,burst:1,maxDebt:128,latencyNs:1000000}
    val fiveMillion={...oneMillion,periodNs:200}
    val corpus=5000000
    assert(op62MailboxWakeDisposition(false,false,false,false)=="wait" and
      gate::noConsumptionGapFits(oneMillion,0,127999,0,corpus) and
      not(gate::noConsumptionGapFits(oneMillion,0,128000,0,corpus)) and
      gate::noConsumptionGapFits(fiveMillion,0,25599,0,corpus) and
      not(gate::noConsumptionGapFits(fiveMillion,0,25750,0,corpus)))
  }

  // The isolated hint route is sparse in the source, but the full
  // ARM64 route still compares the tagged frame count and branches.
  // Gambit's return continuation additionally checks the hint marker.
  run op62SparseHintDispatchChangesFullyChargedReceiveTest={
    val frozen={timeout:10,scheduledInstructions:5620,extraCompares:0,extraBranches:0}
    val candidate={...frozen,scheduledInstructions:5666,extraCompares:1,extraBranches:1}
    assert(frozen.timeout==candidate.timeout and
      candidate.scheduledInstructions>frozen.scheduledInstructions and
      not(op62MissIsUnchanged(0,candidate.extraCompares,candidate.extraBranches)))
  }

  // Reusing the existing debt comparison does not preserve the receive
  // continuation layout when the two paths share a local retire closure.
  run op62DebtRoutedReceiveStillCopiesFullCreditFrameTest={
    val frozen={timeout:10,markerChecks:0,stackCopies:0,frameWords:0,instructions:5620}
    val routed={...frozen,stackCopies:13,frameWords:16,instructions:5826}
    assert(routed.timeout==frozen.timeout and routed.markerChecks==frozen.markerChecks and
      routed.instructions>frozen.instructions and
      not(op62MissIsUnchanged(routed.stackCopies,0,0)) and
      routed.frameWords*8==128)
  }

  // An idle helper has no pending reply deadline. It can consume a
  // bounded hint internally, but cannot return a terminal result
  // until a genuine reply wait has been created.
  pure def op62ReplyOnlyBoundary(held:int,reply:bool,terminalTimeout:bool):str =
    if(reply and held>0) "retire"
    else if(terminalTimeout and held>0) "seal-timeout"
    else "stay-in-helper"

  run op62ReplyOnlyHelperDoesNotExportArrivalOrIdleTimeoutTest=assert(
    op62ReplyOnlyBoundary(0,false,false)=="stay-in-helper" and
    op62ReplyOnlyBoundary(0,false,true)=="stay-in-helper" and
    op62ReplyOnlyBoundary(1,false,false)=="stay-in-helper" and
    op62ReplyOnlyBoundary(1,true,false)=="retire" and
    op62ReplyOnlyBoundary(1,false,true)=="seal-timeout")

  run op62ReplyOnlyShapeAvoidsStackCopiesButChangesBranchLayoutTest={
    val control={instructions:5620,copies:0,markerChecks:0,activeGate:6,takenToReceive:false}
    val shape={...control,instructions:5605,takenToReceive:true}
    assert(shape.activeGate==control.activeGate and shape.copies==0 and
      shape.markerChecks==0 and shape.instructions<control.instructions and
      shape.takenToReceive!=control.takenToReceive)
  }

  // The hint does not seal the original receive. If its continuation
  // resumes after the original deadline, identical public state can
  // conceal opposite enqueue orders across that deadline.
  run op62HintOverrunCannotClassifyAlreadyQueuedReplyTest={
    val deadline=100
    val resumed=120
    val early={enqueued:90,queued:true,terminalSealed:false}
    val late={...early,enqueued:110}
    assert(early.enqueued<deadline and deadline<late.enqueued and
      late.enqueued<resumed and early.queued==late.queued and
      early.terminalSealed==late.terminalSealed and
      op62MailboxWakeDisposition(early.queued,false,true,early.terminalSealed)=="verify-reply" and
      op62MailboxWakeDisposition(late.queued,false,true,late.terminalSealed)=="verify-reply" and
      op62MailboxWakeDisposition(late.queued,false,true,true)=="cleanup-only")
  }

  pure def op62SpareLeaseAfterSeal(lease:str):str =
    if(lease=="preparing") "revoked" else lease

  run op62SpareLeaseMustSettleBeforeTerminalFrameClearTest=assert(
    op62SpareLeaseAfterSeal("preparing")=="revoked" and
    op62SpareLeaseAfterSeal("committing")=="committing" and
    closeDisposition(Set(),Set(1),0)=="hold-frames" and
    closeDisposition(Set(1),Set(1),1)=="drain-replies" and
    closeDisposition(Set(1),Set(1),0)=="clear-frames")

  // The source's actual receive can return false before its continuation
  // revokes a preparing lease. A lock around the lease cannot see that cut.
  run op62SpareLeaseLockCannotObserveUnrecordedTimeoutTest={
    val waiting={lease:"preparing",timeoutReturned:false,terminalRecorded:false}
    val resumed={...waiting,timeoutReturned:true}
    val published={...resumed,lease:"published"}
    assert(waiting.lease==resumed.lease and waiting.terminalRecorded==resumed.terminalRecorded and
      waiting.timeoutReturned!=resumed.timeoutReturned and
      op62SpareLeaseAfterSeal(published.lease)=="published" and
      op62MailboxWakeDisposition(true,false,true,true)=="cleanup-only")
  }

  // The original oracle checks empty bins in ascending SKU order after
  // identity, occupied-bin totals and gross. The two intervals below
  // preserve that ordered domain, including when the occupied span wraps.
  pure def op62EmptySkuRange(first:int, sku:int):bool = {
    val end=first+4
    val low=if(end<=16) 0 else end-16
    (low<=sku and sku<first) or (end<=sku and sku<16)
  }

  run op62EmptySkuRangesPreserveOrderedOracleDomainTest={
    val bins=0.to(15)
    assert(bins.forall(first => {
      val stock=bins.filter(sku => (sku-first+16)%16>=4)
      val split=bins.filter(sku => op62EmptySkuRange(first,sku))
      stock==split and split.size()==12
    }))
  }

  // The isolated full-module GCC lowering replaces the loop with two
  // loop bodies. An exact oracle with fewer iterations can still grow
  // materially, and does not remove a Mailbox handoff or credit transition.
  run op62EmptySkuRangeLoweringIsNotAnEngineAcceptanceTest={
    val stock={visits:16,checks:12,instructions:1209,handoffs:2}
    val split={visits:12,checks:12,instructions:1775,handoffs:2}
    assert(split.visits<stock.visits and split.checks==stock.checks and
      split.instructions>stock.instructions and split.handoffs==stock.handoffs)
  }

  type Op62HistogramAlias={enqueue:int,dequeue:int,ageSample:int,
    nextAgeSample:int,returnAt:int}

  // Both ordinals finish. The receipt loses the permutation of their
  // latency buckets, even when the selected ordinal and heartbeat match.
  pure def op62HistogramAliasReceipt(h:Op62HistogramAlias):List[int]={
    val a=h.ageSample/1000
    val b=(h.nextAgeSample-10000)/1000
    List(0,0,3000,5000,if(a<b) a else b,if(a>b) a else b)
  }

  pure def op62HistogramAliasPhase(h:Op62HistogramAlias):str =
    if (h.enqueue>3000) "not-enqueued"
    else if (h.dequeue>3000) "queued"
    else if (h.returnAt>3000) "dequeued-held"
    else "returned"

  pure def op62HistogramAliasValid(h:Op62HistogramAlias):bool =
    h.enqueue<=h.dequeue and h.dequeue<=h.ageSample and
    h.ageSample<5000 and 5000<h.returnAt and h.returnAt<10000 and
    10000<=h.nextAgeSample

  run op62OccupiedHistogramAliasesAllThreeMailboxCustodyPhasesTest={
    val dequeued={enqueue:1500,dequeue:1900,ageSample:2000,nextAgeSample:14000,returnAt:5100}
    val queued={...dequeued,dequeue:3900,ageSample:4000,nextAgeSample:12000}
    val missing={...queued,enqueue:3800}
    val receipt=List(0,0,3000,5000,2,4)
    val histories=Set(dequeued,queued,missing)
    assert(histories.forall(h => op62HistogramAliasValid(h) and op62HistogramAliasReceipt(h)==receipt)
      and histories.map(h => op62HistogramAliasPhase(h))==
        Set("not-enqueued","queued","dequeued-held"))
  }

  // A completed latency sample follows publication and precedes credit
  // return. Across the whole successful call these post-publication
  // intervals consume at most frames * elapsed units of frame time.
  // Charge that entire budget to the selected tail for a lower bound.
  pure def op62TailBeforePublicationFloor(tailCount:int,quantileFloor:int,
      threshold:int,frames:int,elapsed:int):int = {
    val needed=quantileFloor-threshold
    if (needed<=0) 0 else {
      val possiblyLater=frames*elapsed/needed
      if (possiblyLater>=tailCount) 0 else tailCount-possiblyLater
    }
  }

  run op62TailBeforePublicationBudgetCoversFiniteCutsTest={
    val candidates=0.to(4)
    val spans=candidates.map(a => candidates.map(b => candidates.map(c => List(a,b,c))))
      .flatten().flatten()
    assert(spans.filter(post => post[0]+post[1]+post[2]<=4).forall(post => {
      val earlyCount=0.to(2).filter(i => 4-post[i]>=2).size()
      earlyCount>=op62TailBeforePublicationFloor(3,4,2,1,4)
    }))
  }

  run op62ArchivedTailBeforePublicationRetainsSeparateCohortsTest={
    val millionTail=1000000-budget::rank(1000000,99,100)+1
    val fiveMillionTail=5000000-budget::rank(5000000,99,100)+1
    assert(op62TailBeforePublicationFloor(fiveMillionTail,316000,128000,3,9999999542)==0 and
      op62TailBeforePublicationFloor(millionTail,6392000,1000000,3,1004778708)==9442 and
      op62TailBeforePublicationFloor(millionTail,6392000,5000000,3,1004778708)==7836 and
      op62TailBeforePublicationFloor(millionTail,6392000,6000000,3,1004778708)==2312 and
      op62TailBeforePublicationFloor(fiveMillionTail,3943454000,3000000000,3,4984038584)==49986 and
      op62TailBeforePublicationFloor(millionTail,27601000,26000000,3,1017108875)==8096)
  }

  type Op62RouteFrame={ordinal:int,slot:int}
  pure def op62WorkerFor(f:Op62RouteFrame,frameOwned:bool):int =
    if(frameOwned) f.slot else f.ordinal%4

  run op62FrameOwnedWorkerEliminatesLiveOrdinalRouteCollisionTest={
    val live=Set({ordinal:0,slot:0},{ordinal:3,slot:1},{ordinal:4,slot:2})
    assert(live.map(f=>op62WorkerFor(f,false))==Set(0,3) and
      live.map(f=>op62WorkerFor(f,true))==Set(0,1,2) and
      0.to(2).forall(slot => live.filter(f=>op62WorkerFor(f,true)==slot).size()==1))
  }

  // Changing destinations is observable for lane-owned state. An
  // identified stateless profile can use frame identity instead.
  pure def op62MayBindWorkerToFrame(internalShortOnly:bool,
      callerLaneState:bool,escapingActivation:bool):bool =
    internalShortOnly and not(callerLaneState) and not(escapingActivation)

  run op62FrameOwnedWorkerMustKeepGeneralLaneOwnershipTest={
    val replacement={ordinal:4,slot:2}
    assert(op62WorkerFor(replacement,false)!=op62WorkerFor(replacement,true) and
      op62MayBindWorkerToFrame(true,false,false) and
      not(op62MayBindWorkerToFrame(true,true,false)) and
      not(op62MayBindWorkerToFrame(true,false,true)) and
      not(op62MayBindWorkerToFrame(false,false,false)))
  }

  // The matched 1M/s route experiment cleared every request but missed
  // both absolute gates. Its directional gain over a slow control
  // does not establish a qualified relative effect or permit admission.
  run op62FrameOwnedWorkerSlowControlCannotAdmitRouteTest={
    val historicalP99=6393000
    val backlogGate=128
    val frozen={completed:1000000,rejected:0,timeouts:0,
      p99:225625000,peak:208133,elapsed:1226629209}
    val frameOwned={...frozen,p99:217520000,peak:197965,elapsed:1220195333}
    val passes=frameOwned.p99<=historicalP99 and frameOwned.peak<=backlogGate
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:passes}
    val tailCount=frozen.completed-budget::rank(frozen.completed,99,100)+1
    assert(frameOwned.completed==frozen.completed and frameOwned.rejected==0 and
      frameOwned.timeouts==0 and frameOwned.p99<frozen.p99 and frameOwned.peak<frozen.peak and
      frameOwned.elapsed<frozen.elapsed and not(passes) and
      gate::researchDisposition(decision)=="retain-isolated" and
      not(gate::maySeekAdmissionReview(decision)) and
      op62TailBeforePublicationFloor(tailCount,frozen.p99-1000,200000000,3,frozen.elapsed)==9858 and
      op62TailBeforePublicationFloor(tailCount,frameOwned.p99-1000,200000000,3,frameOwned.elapsed)==9793)
  }

  // A fixed ABI changes the argument conversion, not the operation
  // or its callback selection. Progress keeps the original operation.
  pure def op62ClockRequest(nullary:bool,op:int,arg:int):List[int]=
    if(nullary) List(1,0) else List(op,arg)

  run op62NullaryClockKeepsIndependentProgressDeadlineTest={
    val original=List(op62ClockRequest(false,1,0),op62ClockRequest(false,11,7))
    val specialized=List(op62ClockRequest(true,1,0),op62ClockRequest(false,11,7))
    val deadline=10000000101
    assert(original==specialized and
      op62ClockRequest(true,11,7)!=op62ClockRequest(false,11,7) and
      op62RetirementClockDecision(101,101,102,deadline).phase=="progress-deadline" and
      op62RetirementClockDecision(101,101,102,102).phase=="retired")
  }

  // Removing argument conversions preserves the deadline contract,
  // but the linked matched pair still failed the original acceptance.
  // Lower bounds on source delay describe each cohort separately.
  run op62NullaryClockConversionRemovalDidNotAdmitCycleTest={
    val historicalP99=6393000
    val backlogGate=128
    val frozen={completed:1000000,rejected:0,timeouts:0,
      p99:64942000,peak:61664,sourcePeak:61662,elapsed:1065431208}
    val candidate={...frozen,p99:87169000,peak:84665,sourcePeak:84663,elapsed:1088033583}
    val tailCount=frozen.completed-budget::rank(frozen.completed,99,100)+1
    val passes=candidate.p99<=historicalP99 and candidate.peak<=backlogGate
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:passes}
    assert(candidate.completed==frozen.completed and candidate.rejected==0 and
      candidate.timeouts==0 and candidate.p99>frozen.p99 and candidate.peak>frozen.peak and
      candidate.elapsed>frozen.elapsed and not(gate::maySeekAdmissionReview(decision)) and
      oldestDueAgeFloor(frozen.sourcePeak-tailCount+1,1000)==51661000 and
      oldestDueAgeFloor(candidate.sourcePeak-tailCount+1,1000)==74662000)
  }

  // A successful ordinary drive returns at most one frame before the
  // next sample. At the first cut after final arrival, all are due.
  // With no new arrivals, the recorded backlog then caps the count
  // that could have returned during the whole post-offer drain.
  pure def op62PostOfferReturnCeiling(total:int,peak:int):int =
    if(total<=peak+1) total else peak+1

  pure def op62PostOfferRateCeiling(total:int,peak:int,drainNs:int):int =
    budget::ceilDiv(op62PostOfferReturnCeiling(total,peak)*1000000000,drainNs)

  run op62PostOfferCeilingIncludesOneReturnBeforeNextSampleTest={
    val total=8
    val peak=3
    val sampledCompleted=5
    val completedAtCutoff=4
    assert(total-sampledCompleted==peak and sampledCompleted-completedAtCutoff==1 and
      total-completedAtCutoff==op62PostOfferReturnCeiling(total,peak) and
      op62PostOfferReturnCeiling(2,3)==2)
  }

  run op62ArchivedPostOfferDrainCouldNotMatchArrivalRateTest={
    val million=1000000
    val fiveMillion=5000000
    assert(op62PostOfferRateCeiling(fiveMillion,4000404,3984038784)==1004108 and
      op62PostOfferRateCeiling(million,6746,4779708)==1411593 and
      op62PostOfferRateCeiling(million,208133,226630209)==918386 and
      op62PostOfferRateCeiling(million,197965,220196333)==899044 and
      op62PostOfferRateCeiling(million,61664,65432208)==942426 and
      op62PostOfferRateCeiling(million,84665,88034583)==961736)
  }

  // An initialized actor may receive before it is scheduled. Proof of
  // a private, stable destination and live source publication authority
  // is needed before bypassing the general target validation.
  type Op62OwnedSend={privateWorker:bool,initialized:bool,sourceOpen:bool,
    stableWorker:bool,normalFrameCustody:bool}
  pure def op62MayUseOwnedSend(p:Op62OwnedSend):bool =
    p.privateWorker and p.initialized and p.sourceOpen and
    p.stableWorker and p.normalFrameCustody

  run op62OwnedSendDoesNotInheritPublicTargetValidationTest={
    val live={privateWorker:true,initialized:true,sourceOpen:true,
      stableWorker:true,normalFrameCustody:true}
    assert(op62MayUseOwnedSend(live) and
      not(op62MayUseOwnedSend({...live,initialized:false})) and
      not(op62MayUseOwnedSend({...live,privateWorker:false})) and
      not(op62MayUseOwnedSend({...live,sourceOpen:false})) and
      not(op62MayUseOwnedSend({...live,stableWorker:false})) and
      not(op62MayUseOwnedSend({...live,normalFrameCustody:false})))
  }

  // A lower-level send entry preserved lifecycle correctness but did
  // not meet either of the unchanged performance gates in this pair.
  run op62OwnedSendWrapperRemovalDidNotAdmitCycleTest={
    val total=1000000
    val historicalP99=6393000
    val backlogGate=128
    val frozen={completed:total,rejected:0,timeouts:0,
      p99:104327000,peak:98566,elapsed:1105239334,drain:105240334}
    val candidate={...frozen,p99:159058000,peak:134545,elapsed:1159915958,drain:159916958}
    val passes=candidate.p99<=historicalP99 and candidate.peak<=backlogGate
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:passes}
    assert(candidate.completed==total and candidate.rejected==0 and candidate.timeouts==0 and
      candidate.p99>frozen.p99 and candidate.peak>frozen.peak and candidate.elapsed>frozen.elapsed and
      not(gate::maySeekAdmissionReview(decision)) and
      op62PostOfferRateCeiling(total,frozen.peak,frozen.drain)==936590 and
      op62PostOfferRateCeiling(total,candidate.peak,candidate.drain)==841350)
  }

  // Actual FIFO timeout, application seal and a later public state
  // sample are separate cuts. The two forced schedules give the
  // same failure receipt and a later completed send/repark witness.
  type TimeoutSealHistory={timeoutAt:int,sendStart:int,sendReturn:int,
    sealAt:int,queryAt:int,retired:int}
  pure def timeoutSealProjection(h:TimeoutSealHistory):List[int] =
    List(1,h.retired,1,if(h.sendReturn<=h.queryAt) 1 else 0)

  run op62TimeoutSealCannotDateLaterReadyReparkTest={
    val before={timeoutAt:100,sendStart:110,sendReturn:115,sealAt:120,
      queryAt:140,retired:0}
    val after={...before,sendStart:125,sendReturn:130}
    assert(before.timeoutAt<before.sendStart and after.timeoutAt<after.sendStart and
      before.sendReturn<before.sealAt and after.sealAt<after.sendStart and
      timeoutSealProjection(before)==timeoutSealProjection(after) and
      timeoutSealProjection(after)==List(1,0,1,1) and
      op62MailboxWakeDisposition(true,false,true,true)=="cleanup-only")
  }

  // Input storage reuse improved the direction of this matched pair.
  // It neither clears the original gates nor locates a recovery cut
  // before the last scheduled arrival.
  run op62PrivateInputReuseRecoveryDoesNotAdmitCycleTest={
    val total=1000000
    val frozen={completed:total,rejected:0,timeouts:0,gc:39,
      p99:103344000,peak:97586,elapsed:1105839583,drain:105840583}
    val candidate={...frozen,gc:34,p99:76458000,peak:76879,elapsed:1070436125,drain:70437125}
    val passes=candidate.p99<=6393000 and candidate.peak<=128
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:passes}
    assert(candidate.completed==total and candidate.rejected==0 and candidate.timeouts==0 and
      candidate.gc<frozen.gc and candidate.p99<frozen.p99 and candidate.peak<frozen.peak and
      candidate.elapsed<frozen.elapsed and gate::researchDisposition(decision)=="retain-isolated" and
      not(gate::maySeekAdmissionReview(decision)) and
      op62PostOfferRateCeiling(total,frozen.peak,frozen.drain)==922019 and
      op62PostOfferRateCeiling(total,candidate.peak,candidate.drain)==1091470 and
      op62LaterPublicationsFloor(total,1000,1070433709,76876417)==6442 and
      op62SuffixRetirementRateFloor(total,1000,candidate.elapsed,76876417)==1006481)
  }

  // The rooted fallback runs under the real inherited service budget.
  // Selecting storage at entry does not grant a worker lease, and the
  // ambient budget must be restored and idle on either exit.
  pure def op62InputStorageAtEntry(inheritedBudget:bool):str =
    if(inheritedBudget) "fresh" else "private-reuse"

  type Op62InheritedExit={storage:str,grantSeen:bool,idleAtExit:bool,contextRestored:bool}
  pure def op62InheritedExitValid(r:Op62InheritedExit):bool =
    r.storage=="fresh" and r.grantSeen and r.idleAtExit and r.contextRestored

  run op62RootedInheritedBudgetUsesFreshInputAndReleasesLeaseTest={
    val receipt={storage:op62InputStorageAtEntry(true),grantSeen:true,idleAtExit:true,contextRestored:true}
    assert(op62InheritedExitValid(receipt) and
      not(op62InheritedExitValid({...receipt,storage:"private-reuse"})) and
      not(op62InheritedExitValid({...receipt,grantSeen:false})) and
      not(op62InheritedExitValid({...receipt,idleAtExit:false})) and
      not(op62InheritedExitValid({...receipt,contextRestored:false})) and
      op62InputStorageAtEntry(false)=="private-reuse")
  }

  // The rank callback follows the last row read. A work checkpoint can
  // precede later row reads, so detaching just the ordinal is insufficient.

  run op62OrdinalSnapshotDoesNotCloseEarlierCheckpointInputTest={
    assert(op62ReentryInputReads(false,false)==Set(0) and
      op62ReentryInputReads(false,true)==Set() and
      op62ReentryInputReads(true,true)==5.to(13) and
      op62ReentryInputReads(true,true).size()==9)
  }

  // Both one-driver lowerings choose storage at invocation entry.
  // Their steady-state costs differ: one retains a callable stack
  // root, the other checks the frame's optional envelope each time.
  run op62OneInputDriverDoesNotPreserveReuseHotPathTest={
    val reuse={scheduled:5731,helpers:680,indirect:false,nullableCheck:false}
    val indirect={...reuse,scheduled:5805,helpers:785,indirect:true}
    val nullable={...reuse,scheduled:5779,helpers:679,nullableCheck:true}
    assert(indirect.indirect and nullable.nullableCheck and
      indirect.scheduled+indirect.helpers>nullable.scheduled+nullable.helpers and
      nullable.scheduled+nullable.helpers>reuse.scheduled+reuse.helpers)
  }

  // The closed typed input constructor sets four quantities once.
  // Skipping their stores relies on initialized private storage and
  // the handler never mutating those fields during frame custody.
  pure val op62QuantityFields=0.to(3).map(line=>3+3*line)
  pure val op62VaryingFields=Set(0).union(0.to(3).map(line=>2+3*line))
    .union(0.to(3).map(line=>4+3*line))
  pure def op62FixedQuantitiesValid(initialized:Set[int],writes:Set[int],
      privateClosed:bool):bool =
    privateClosed and op62QuantityFields.subseteq(initialized) and
    writes.intersect(op62QuantityFields)==Set()

  run op62FixedQuantityRefillRequiresOriginalInitializationAndCustodyTest={
    assert(op62QuantityFields==Set(3,6,9,12) and op62VaryingFields.size()==9 and
      op62FixedQuantitiesValid(op62QuantityFields,op62VaryingFields,true) and
      not(op62FixedQuantitiesValid(Set(3,6,9),op62VaryingFields,true)) and
      not(op62FixedQuantitiesValid(op62QuantityFields,op62VaryingFields.union(Set(6)),true)) and
      not(op62FixedQuantitiesValid(op62QuantityFields,op62VaryingFields,false)))
  }

  // Fewer input stores and a smaller helper do not admit the full
  // service cycle. These are matched observations, not a causal claim
  // about the stores or a replacement for the frozen 7F reference.
  run op62FixedQuantityStoresRemovalDidNotAdmitCycleTest={
    val total=1000000
    val reuse={completed:total,rejected:0,timeouts:0,gc:33,
      p99:48590000,peak:43477,elapsed:1051277667,drain:51278667}
    val fixed={...reuse,p99:109518000,peak:101141,elapsed:1108543291,drain:108544291}
    val decision={hardFailure:false,relativeEffect:"unconfirmed",
      absolutePassed:fixed.p99<=6393000 and fixed.peak<=128}
    assert(reuse.completed==total and fixed.completed==total and
      fixed.rejected==0 and fixed.timeouts==0 and fixed.gc==reuse.gc and
      fixed.p99>reuse.p99 and fixed.peak>reuse.peak and fixed.elapsed>reuse.elapsed and
      op62PostOfferRateCeiling(total,reuse.peak,reuse.drain)==847877 and
      op62PostOfferRateCeiling(total,fixed.peak,fixed.drain)==931804 and
      not(gate::maySeekAdmissionReview(decision)))
  }

  // Local lexical delivery can reach the same retirement decision. It
  // does not create an enqueue/dequeue witness for a Mailbox gap.
  run op62LocalRetirementKeepsDeadlineAndCannotSealMailboxGapTest={
    val c={owner:1,generation:2,gap:3,ordinal:41,trigger:130,
      state:"reply-in-mailbox",wakeInjected:false}
    val actualReply={owner:1,generation:2,gap:3,ordinal:41,dequeuedAt:131,at:135,boundary:"retired"}
    val local={...actualReply,dequeuedAt:-1}
    val requestLimit=op62RetirementClockDecision(100,100,10000000100,10000000100)
    val progressLimit=op62RetirementClockDecision(101,100,10000000099,10000000100)
    assert(matchingRecovery(c,actualReply) and not(matchingRecovery(c,local)) and
      not(requestLimit.sampled) and requestLimit.held and
      progressLimit.sampled and progressLimit.held and
      op62RetirementClockDecision(100,100,133,134).phase=="retired")
  }

  // A closed local turn still admits before execution and retires after
  // sampling and progress. A completion sample cannot stand in for local
  // admission, and local admission cannot attest a public Mailbox enqueue.
  type Op62LocalCuts={due:int,admitted:int,businessStart:int,latency:int,
    progress:int,retired:int,refill:int}
  pure def op62LocalCutsOrdered(c:Op62LocalCuts):bool =
    c.due<=c.admitted and c.admitted<=c.businessStart and
    c.businessStart<c.latency and c.latency<=c.progress and
    c.progress<=c.retired and c.retired<=c.refill

  run op62LocalTurnCannotChargeExecutionAsSourceAdmissionTest={
    val normal={due:100,admitted:102,businessStart:103,latency:110,
      progress:112,retired:113,refill:114}
    val late={...normal,admitted:normal.latency}
    assert(op62LocalCutsOrdered(normal) and not(op62LocalCutsOrdered(late)) and
      not(op62LocalCutsOrdered({...normal,refill:normal.progress})) and
      normal.admitted-normal.due==2 and late.admitted-late.due==10 and
      normal.latency-normal.due==late.latency-late.due)
  }

  // Real one-million/s pair. Removing the two Mailbox legs and reply
  // pair did not admit the closed ring. Enclosing CPU and fewer GCs
  // do not identify the reason for the wall-time or source debt change.
  run op62ClosedRingDidNotAdmitTheWholeCycleTest={
    val total=1000000
    val frozen={completed:total,rejected:0,timeout:0,gc:39,p99:219135000,
      peak:176543,sourcePeak:176541,elapsed:1220718750,drain:220719750}
    val ring={...frozen,gc:31,p99:297791000,peak:209591,sourcePeak:209589,
      elapsed:1325393000,drain:325394000}
    val passes=ring.p99<=6393000 and ring.peak<=128
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:passes}
    val tailCount=total-budget::rank(total,99,100)+1
    assert(ring.completed==total and ring.rejected==0 and ring.timeout==0 and
      ring.gc<frozen.gc and ring.p99>frozen.p99 and ring.peak>frozen.peak and
      ring.elapsed>frozen.elapsed and not(gate::maySeekAdmissionReview(decision)) and
      op62PostOfferRateCeiling(total,frozen.peak,frozen.drain)==799856 and
      op62PostOfferRateCeiling(total,ring.peak,ring.drain)==644118 and
      oldestDueAgeFloor(frozen.sourcePeak-tailCount+1,1000)==166540000 and
      oldestDueAgeFloor(ring.sourcePeak-tailCount+1,1000)==199588000)
  }

  // An armed global send entry sees the next call, not necessarily the
  // selected reply. Even the selected call exposes entry, not insertion.
  type DispatchArm={ordinal:int,arm:int,targetSend:int,firstAfterArm:int,
    sendReturn:int,dequeued:int,retired:int}
  pure def op62OneShotDispatchCoverage(t:DispatchArm):str =
    if(t.targetSend<t.arm) "target-already-started"
    else if(t.firstAfterArm!=t.ordinal) "intercepted-unrelated"
    else "target-send-entry"

  run op62GlobalSendArmDoesNotGuaranteeSelectedReplyOrInBoxWitnessTest={
    val target={ordinal:41,arm:100,targetSend:110,firstAfterArm:41,
      sendReturn:125,dequeued:120,retired:130}
    val other={...target,firstAfterArm:42}
    val earlier={...target,targetSend:80,sendReturn:85,firstAfterArm:-1}
    val cut=115
    val possibleEnqueues=Set(112,118)
    assert(op62OneShotDispatchCoverage(target)=="target-send-entry" and
      op62OneShotDispatchCoverage(other)=="intercepted-unrelated" and
      op62OneShotDispatchCoverage(earlier)=="target-already-started" and
      target.dequeued<target.retired and other.dequeued==target.dequeued and
      earlier.retired==target.retired and
      possibleEnqueues.forall(at=>target.targetSend<at and at<target.dequeued) and
      possibleEnqueues.exists(at=>at<cut) and possibleEnqueues.exists(at=>at>cut))
  }

  // A CPU delta enclosing the service interval supplies only an upper
  // bound until a wall interval encloses both CPU reads on that owner.
  type Op62CpuEnvelope={sameOwner:bool,wallStart:int,cpuStart:int,
    innerStart:int,innerEnd:int,cpuEnd:int,wallEnd:int,cpuDelta:int}
  pure def op62InnerCpuFloor(e:Op62CpuEnvelope):int =
    if(not(e.sameOwner) or e.wallStart>e.cpuStart or e.cpuStart>e.innerStart or
       e.innerStart>e.innerEnd or e.innerEnd>e.cpuEnd or e.cpuEnd>e.wallEnd or
       e.cpuDelta<0 or e.cpuDelta>e.cpuEnd-e.cpuStart)
      -1
    else {
      val lower=e.cpuDelta-(e.innerStart-e.wallStart)-(e.wallEnd-e.innerEnd)
      if(lower>0) lower else 0
    }

  run op62OwnerCpuOutsideTheInnerClockCannotProveRecurringOverloadTest={
    val inner={sameOwner:true,wallStart:0,cpuStart:1,innerStart:2,innerEnd:27,
      cpuEnd:28,wallEnd:29,cpuDelta:25}
    val before={...inner,innerStart:26,innerEnd:51,cpuEnd:52,wallEnd:53}
    val crossed={...before,wallStart:25}
    assert(inner.cpuDelta==before.cpuDelta and
      inner.innerEnd-inner.innerStart==before.innerEnd-before.innerStart and
      op62InnerCpuFloor(inner)==21 and op62InnerCpuFloor(before)==0 and
      op62InnerCpuFloor(crossed)==-1 and
      op62InnerCpuFloor({...inner,sameOwner:false})==-1)
  }

  // Existing frozen reader paths at 16 requests/s. CPU floors use
  // integer CPU clocks on the fixed owner around each native call.
  // op59 differs in planning as well as wait policy.
  run op62SparseNativeCpuEnvelopeDoesNotTransferToMainRateTest={
    val scheduled={inner:937519709,outer:954497000,cpu:920721750}
    val parked={inner:945449791,outer:960686209,cpu:6947000}
    val fault={inner:31416,outer:20707292,cpu:12130542}
    val scheduledFloor=scheduled.cpu-(scheduled.outer-scheduled.inner)
    val parkedUpper=parked.cpu
    assert(scheduledFloor==903744459 and parkedUpper==6947000 and
      scheduledFloor-parkedUpper==896797459 and fault.cpu>fault.inner and
      fault.cpu-(fault.outer-fault.inner)<0 and
      not(gate::maySeekAdmissionReview({
        hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false})))
  }

  // Same op62 plan and frame cycle, changing only the zero-active wait.
  // A sparse CPU gain does not qualify the relative timeout timebase
  // or turn a missing natural gap into a passing workload.
  run op62IdleParkRemovesSparseCpuWithoutQualifyingMainRateTest={
    val originalCpuFloors=Set(903744459,914534459)
    val parkedCpuUppers=Set(11258125,1688333)
    val planned=1000
    val wallBackstep=250
    val delayWithBackstep=planned+wallBackstep
    assert(originalCpuFloors.forall(before =>
      parkedCpuUppers.forall(after => before-after>=892486334)) and
      delayWithBackstep>planned and
      not(gate::maySeekAdmissionReview({
        hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:false})))
  }

  // Each above-window ordinary drive retires a distinct request in a
  // complete run: there must be active work or due work to publish.
  // This witness counts drive turns, not time or individual latency.
  pure def op62AboveWindowRequiresWork(debt:int,active:int):bool =
    if(debt>=0 and 0<=active and active<=3 and debt+active>128) {
      val free=3-active
      val issued=if(debt<free) debt else free
      active+issued>0
    } else true

  run op62MonotonicIdlePortGainStillMissesFrozenReaderGatesTest={
    val original={total:1000000,p99:43872000,peak:44073,above:998605,elapsed:1043682917}
    val port={...original,p99:5890000,peak:6158,above:998051,elapsed:1002592791}
    val historicalReference=6393000
    val readerP99Limit=418000
    val passed=port.p99<=readerP99Limit and port.peak<=128
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:passed}
    assert(port.p99<historicalReference and port.p99<original.p99 and
      port.peak<original.peak and port.elapsed<original.elapsed and
      port.p99>readerP99Limit and port.peak>128 and
      port.total-port.above==1949 and not(gate::maySeekAdmissionReview(decision)) and
      0.to(3).forall(active=>Set(125,126,127,128,129,6156).forall(debt=>
        op62AboveWindowRequiresWork(debt,active))))
  }

  // Complete ordinary op62 runs, one retirement per above-window
  // drive, with due = min(total, 1 + elapsed/period). A saturated due
  // count cannot supply an exclusive first-crossing time upper bound.
  pure def op62FirstAboveUpper(total:int,above:int,peak:int,period:int):int =
    if(total-above+peak<total) (total-above+peak)*period else -1
  pure def op62LastAboveLower(above:int,limit:int,period:int):int =
    (above+limit-1)*period

  run op62ArchivedPortBreachStartedEarlyAndWasStillPresentLateTest=assert(
    op62FirstAboveUpper(1000000,998051,6158,1000)==8107000 and
    op62LastAboveLower(998051,128,1000)==998178000 and
    op62FirstAboveUpper(1000000,998605,44073,1000)==45468000 and
    op62LastAboveLower(998605,128,1000)==998732000 and
    op62FirstAboveUpper(4,2,3,1000)== -1)

  // A cold initialization can cross the next eligibility time even
  // though the original pre-open check authorized a wait.
  pure def op62ColdPortMayArm(at:int,before:int,openedAt:int,cold:bool):bool =
    before<at and (not(cold) or openedAt<at)

  run op62ColdPortMustNotWaitForArrivalThatBecameDueWhileOpeningTest=assert(
    op62ColdPortMayArm(100,90,100,false) and
    not(op62ColdPortMayArm(100,90,100,true)) and
    not(op62ColdPortMayArm(100,90,101,true)) and
    op62ColdPortMayArm(100,90,99,true) and
    not(op62ColdPortMayArm(100,100,100,true)))

  // Keeping the ordinal live across cold open also changed the
  // ordinary idle helper's stack before the cold/warm branch.
  run op62ColdOrdinalWitnessAddsWarmStackStoreTest={
    val oldEntry={stores:2,ordinalRetained:false}
    val ordinalEntry={...oldEntry,stores:3,ordinalRetained:true}
    assert(ordinalEntry.stores-oldEntry.stores==1 and
      ordinalEntry.ordinalRetained and not(oldEntry.ordinalRetained))
  }

  // Eligibility timestamps are distinct at the admitted rates, but
  // only identify a request within the same owner invocation.
  pure def op62EligibilityId(epoch:int,ordinal:int,rate:int):int =
    epoch+budget::ceilDiv(ordinal*1000000000,rate)

  run op62ColdEligibilityIdDoesNotRequireAnExtraLiveOrdinalTest=assert(
    Set(3,16,500000,1000000,5000000).forall(rate =>
      Set(0,1,2,3,999999,4999999).forall(ordinal =>
        op62EligibilityId(100,ordinal,rate)<op62EligibilityId(100,ordinal+1,rate))) and
    op62EligibilityId(100,1,1000000)==op62EligibilityId(1100,0,1000000))

  run op62MonotonicPortTailIncludesPrepublicationDebtTest={
    val tailCount=1000000-budget::rank(1000000,99,100)+1
    val cutoff=1000000
    val original={elapsed:1043682917,p99:43872000}
    val port={elapsed:1002592791,p99:5890000}
    assert(tailCount==10001 and tailCount-3*original.elapsed/cutoff==6870 and
      tailCount-3*port.elapsed/cutoff==6994 and original.p99-1000-cutoff==42871000 and
      port.p99-1000-cutoff==4889000)
  }

  // Both reports retain the same post-send clock sample. The separate
  // request latency and watchdog samples are outside this identity.
  run op62DirectPublishTimestampPreservesReportArithmeticTest=assert(
    Set(0,1000,9223372036854775807).forall(epoch =>
      Set(0,1,10000000000).forall(offset =>
        Set(0,1,1000000).forall(late => {
          val planned=epoch+offset
          val postSend=planned+late
          val elapsed=postSend-epoch
          epoch+elapsed-planned==postSend-planned and postSend-planned==late
        }))))

  // One completed A/B. The same port image moved far from its
  // earlier result. Neither static savings nor a small matched
  // direction admits the alternative or explains the control drift.
  run op62PublishClockAlgebraDoesNotAdmitCycleTest={
    val total=1000000
    val previous={p99:5890000,peak:6158}
    val control={p99:83558000,peak:80142,elapsed:1083839209,drain:83840209,cpuUpper:1072023542}
    val direct={p99:82346000,peak:68988,elapsed:1082757708,drain:82758708,cpuUpper:1036833041}
    val passes=direct.p99<=418000 and direct.peak<=128
    val decision={hardFailure:false,relativeEffect:"unconfirmed",absolutePassed:passes}
    assert(control.p99>previous.p99 and direct.p99>previous.p99 and
      direct.p99<control.p99 and direct.peak<control.peak and direct.elapsed<control.elapsed and
      direct.cpuUpper<control.cpuUpper and not(gate::maySeekAdmissionReview(decision)) and
      op62PostOfferRateCeiling(total,control.peak,control.drain)==955902 and
      op62PostOfferRateCeiling(total,direct.peak,direct.drain)==833617)
  }

  // At an integral arrival period, an interval of d periods contains at
  // most d new arrivals. Retirement can only decrease the true backlog.
  // Thus a peak P above L witnesses a continuous pre-peak interval with
  // at least L+1 due requests still unreturned, independent of sampling.
  pure def op62AboveLimitBeforePeak(peak:int,limit:int,period:int):int =
    if(peak>limit) (peak-limit-1)*period else 0

  run op62PeakCertifiesContinuousDebtWithoutIdentifyingAMailboxGapTest={
    val limit=3
    val period=4
    assert(4.to(8).forall(peak => {
      val span=op62AboveLimitBeforePeak(peak,limit,period)
      0.to(span).forall(distance =>
        0.to(period-1).forall(phase =>
          0.to(3).forall(retired => {
            val arrivals=(phase+distance)/period
            val earlierDebt=peak-arrivals+retired
            earlierDebt>limit
          })))
    }) and op62AboveLimitBeforePeak(4,3,4)==0 and
      op62AboveLimitBeforePeak(3,3,4)==0)
  }

  run op62ArchivedBacklogSpansAreIndependentOfTheFinalReplyDrainTest=assert(
    op62AboveLimitBeforePeak(351,128,2000)==444000 and
    op62AboveLimitBeforePeak(6746,128,1000)==6617000 and
    op62AboveLimitBeforePeak(6158,128,1000)==6029000 and
    op62AboveLimitBeforePeak(27769,128,1000)==27640000 and
    op62AboveLimitBeforePeak(80142,128,1000)==80013000 and
    op62AboveLimitBeforePeak(68988,128,1000)==68859000 and
    1083839209-1083837209==2000 and 1082757708-1082756083==1625)

  // Only a successful, ordinary (non-coalesced) drive can contribute
  // another pre-supply sample. It either yielded with zero active or
  // verified and returned one frame from a receive with 1..3 active.
  pure def op62FollowingDriveActive(beforeReceive:int):int =
    if(beforeReceive==0) 0 else beforeReceive-1

  pure def op62OrdinaryPeakCredits(sourcePeak:int,totalPeak:int):Set[int] =
    activeAtTotalPeak(sourcePeak,totalPeak)
      .intersect(0.to(3).map(op62FollowingDriveActive))

  run op62PeakCutProvesTwoHeldAndPreviousFullReceiveTest={
    val peaks=Set({source:349,total:351},{source:6744,total:6746},
      {source:6156,total:6158},{source:27767,total:27769},
      {source:80140,total:80142},{source:68986,total:68988})
    assert(peaks.forall(p => op62OrdinaryPeakCredits(p.source,p.total)==Set(2)
      and p.total-2==p.source) and
      0.to(3).filter(n=>op62FollowingDriveActive(n)==2)==Set(3) and
      op62OrdinaryPeakCredits(129,132)==Set())
  }

  // The predecessor may start with zero or two active frames even
  // though eager supply makes both receives full. Its terminal count
  // carries no evidence about a prior idle wait or reply queue state.
  run op62PeakCutDoesNotExcludeEarlierEmptySourceTest={
    val completedBefore=1
    val dueBefore=4
    val nextDue=353
    assert(Set(1,3).forall(publishedBefore => {
      val activeBefore=publishedBefore-completedBefore
      val refill=dueBefore-publishedBefore
      val beforeReceive=activeBefore+refill
      val completed=completedBefore+1
      val published=publishedBefore+refill
      beforeReceive==3 and op62FollowingDriveActive(beforeReceive)==2 and
      nextDue-published==349 and nextDue-completed==351
    }))
  }

  // Choosing a destination for a free credit can be work conserving
  // without moving any live frame. Static partitions can strand a
  // free frame while the only runnable peer has eligible work.
  run op62StaticPartitionCanStrandFreeCreditBesideSuspendedActorTest={
    val slots=Set(0,1,2)
    val parked={slot:0,ordinal:41,worker:0}
    val healthy={slot:2,ordinal:42,worker:1}
    val held=Set(parked,healthy)
    val free=slots.exclude(held.map(frame=>frame.slot))
    val fixedEligible=free.intersect(Set(2))
    val sharedEligible=free
    val filled=held.union(Set({slot:1,ordinal:43,worker:1}))
    val returned=chargeAfterReply(healthy,2,42,true)
    val stale=chargeAfterReply(parked,0,40,true)
    val afterRefill=filled.exclude(Set(healthy)).union(Set({slot:2,ordinal:44,worker:1}))
    assert(fixedEligible==Set() and sharedEligible==Set(1) and
      filled.map(frame=>frame.slot)==slots and returned==Set() and stale==Set(parked) and
      afterRefill.contains(parked) and afterRefill.map(frame=>frame.slot)==slots and
      afterRefill.size()==slots.size())
  }

  // A bounded active hook can ignore a different frame even at the
  // same ordinal. It must disarm on budget exhaustion. A match still
  // only brackets FIFO insertion inside the original send call.
  type Op62ReplyKey={owner:int,ordinal:int,slot:int}
  pure def op62BoundedDispatchMatch(target:Op62ReplyKey,calls:List[Op62ReplyKey],budget:int):bool =
    0.to(calls.length()-1).exists(i=>i<budget and calls[i]==target)

  run op62BoundedDispatchIdentityDoesNotAttestOverlappingCutTest={
    val target={owner:1,ordinal:41,slot:0}
    val sibling={...target,slot:1}
    val otherOwner={...target,owner:2}
    val b={sendEnter:10,sendReturn:20,receiveEnter:30,receiveReturn:40,retire:50}
    assert(op62BoundedDispatchMatch(target,List(sibling,target),2) and
      not(op62BoundedDispatchMatch(target,List(sibling,otherOwner,target),2)) and
      not(op62BoundedDispatchMatch(target,List(sibling),2)) and
      custodyAt(b,15)=="unknown" and custodyAt(b,25)=="in-mailbox" and
      custodyAt({...b,receiveEnter:5},25)=="unknown")
  }

  // At a full-credit heartbeat, the normal return is still pending.
  // The first later committed publication following exactly one return
  // must reuse that frame. The validated reply identity is independent
  // evidence; it is never obtained from the reused frame or counters.
  type Op62RefillCut={owner:int,generation:int,gap:int,at:int,published:int,
    completed:int,progress:int,phase:str}
  pure def op62RefillCertifiesReturn(heartbeat:Op62RefillCut,refill:Op62RefillCut,
      expectedOrdinal:int,validatedOrdinal:int):bool =
    heartbeat.phase=="verified-heartbeat" and refill.phase=="normal-post-send" and
    heartbeat.owner==refill.owner and heartbeat.generation==refill.generation and
    heartbeat.gap==refill.gap and heartbeat.at<refill.at and
    heartbeat.published-heartbeat.completed==3 and
    refill.published==heartbeat.published+1 and refill.completed==heartbeat.completed+1 and
    refill.progress==heartbeat.progress and
    0<=expectedOrdinal and expectedOrdinal==validatedOrdinal and
    validatedOrdinal<heartbeat.published

  run op62OneNormalRefillBoundsFullCreditReturnForKnownReplyTest={
    val heartbeat={owner:1,generation:2,gap:3,at:100,published:43,
      completed:40,progress:41,phase:"verified-heartbeat"}
    val refill={...heartbeat,at:103,published:44,completed:41,phase:"normal-post-send"}
    assert(op62RefillCertifiesReturn(heartbeat,refill,41,41) and
      op62RefillCertifiesReturn(heartbeat,refill,42,42) and
      not(op62RefillCertifiesReturn(heartbeat,refill,41,42)) and
      not(op62RefillCertifiesReturn(heartbeat,{...refill,generation:3},41,41)) and
      not(op62RefillCertifiesReturn(heartbeat,{...refill,phase:"cleanup"},41,41)) and
      not(op62RefillCertifiesReturn(heartbeat,{...refill,progress:42},41,41)) and
      not(op62RefillCertifiesReturn(heartbeat,{...refill,completed:40},41,41)) and
      not(op62RefillCertifiesReturn(heartbeat,refill,43,43)))
  }

  type Op62WitnessKey={invocation:int,owner:int,generation:int,gap:int,ordinal:int}
  type Op62TimeBound={key:Op62WitnessKey,lo:int,hi:int}
  pure def op62Earlier(a:int,b:int):int=if(a<=b) a else b
  pure def op62Later(a:int,b:int):int=if(a>=b) a else b

  // Compose public, independently established bounds only for one
  // invocation and target. Order tightens each bound but cannot create
  // a unique custody phase when a transition can straddle the cut.
  pure def op62ComposedCustody(enqueue:Op62TimeBound,dequeue:Op62TimeBound,
      retire:Op62TimeBound,cut:Op62TimeBound):str =
    if(enqueue.key!=cut.key or dequeue.key!=cut.key or retire.key!=cut.key) "unknown"
    else {
      val enqueueHi=op62Earlier(enqueue.hi,op62Earlier(dequeue.hi,retire.hi))
      val dequeueLo=op62Later(enqueue.lo,dequeue.lo)
      val dequeueHi=op62Earlier(dequeue.hi,retire.hi)
      val retireLo=op62Later(dequeueLo,retire.lo)
      if(enqueue.lo>enqueueHi or dequeueLo>dequeueHi or retireLo>retire.hi or cut.lo>cut.hi)
        "inconsistent"
      else if(cut.hi<enqueue.lo) "before-enqueue"
      else if(enqueueHi<cut.lo and cut.hi<dequeueLo) "in-mailbox"
      else if(dequeueHi<cut.lo and cut.hi<retireLo) "dequeued"
      else if(retire.hi<cut.lo) "retired"
      else "unknown"
    }

  run op62JointBoundsCannotMergeSeparateRoundsOrForceAThreadPhaseTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:41}
    val enqueue={key:key,lo:10,hi:20}
    val dequeue={key:key,lo:30,hi:40}
    val retire={key:key,lo:50,hi:60}
    val cut={key:key,lo:23,hi:27}
    assert(op62ComposedCustody(enqueue,dequeue,retire,cut)=="in-mailbox" and
      op62ComposedCustody(enqueue,dequeue,retire,{...cut,lo:15})=="unknown" and
      op62ComposedCustody(enqueue,dequeue,retire,{...cut,lo:20})=="unknown" and
      op62ComposedCustody(enqueue,dequeue,retire,{...cut,lo:44,hi:48})=="dequeued" and
      op62ComposedCustody(enqueue,dequeue,retire,{...cut,lo:7,hi:9})=="before-enqueue" and
      op62ComposedCustody(enqueue,dequeue,{...retire,lo:5,hi:9},cut)=="inconsistent" and
      op62ComposedCustody(enqueue,dequeue,{...retire,key:{...key,invocation:2}},cut)=="unknown" and
      op62ComposedCustody(enqueue,{...dequeue,key:{...key,ordinal:42}},retire,cut)=="unknown")
  }

  // Clock offsets from the selected opening heartbeat in one real
  // three-request VM run. The terminal ledger retired every request,
  // and its last progress timestamp equals the selected closing one.
  run op62ThreeRequestVmReceiptClosesKnownBeforeEnqueueCutTest={
    val key={invocation:1,owner:0,generation:1,gap:1,ordinal:2}
    val enqueue={key:key,lo:333301708,hi:333327750}
    val dequeue=enqueue
    val retire={key:key,lo:333327750,hi:333429625}
    val sample={key:key,lo:5594125,hi:5594125}
    assert(op62ComposedCustody(enqueue,dequeue,retire,sample)=="before-enqueue" and
      enqueue.lo-sample.hi==327707583 and dequeue.hi-enqueue.lo==26042 and
      retire.hi-retire.lo==101875 and
      op62ComposedCustody(enqueue,dequeue,retire,{...sample,lo:enqueue.lo,hi:enqueue.lo})=="unknown")
  }

  // Keep startup off the generic clock dispatch. The existing verified
  // progress event can spend the startup ticket only after the original
  // deadline check. An asynchronous sample still needs the current key.
  pure def op62ProgressObserverAction(operation:str,verified:bool,event:str,
      previous:int,key:int):str =
    if(operation!="progress" or not(verified)) "preserve"
    else if(event=="start") "start-once"
    else if(event=="sample" and previous==key) "capture-once"
    else if(event=="sample") "discard"
    else "idle"

  run op62ObserverStartupDoesNotInterceptSourceClockOrReplaceDeadlineTest={
    val oldClockWrapper={instructions:14,stackPairs:4}
    val progressOnlyClock={instructions:0,stackPairs:0}
    assert(op62ProgressObserverAction("clock",true,"start",100,100)=="preserve" and
      op62ProgressObserverAction("progress",false,"start",100,100)=="preserve" and
      op62ProgressObserverAction("progress",true,"start",100,100)=="start-once" and
      op62ProgressObserverAction("progress",true,"quiet",100,100)=="idle" and
      op62ProgressObserverAction("progress",true,"sample",101,100)=="discard" and
      op62ProgressObserverAction("progress",true,"sample",102,102)=="capture-once" and
      oldClockWrapper.instructions>progressOnlyClock.instructions and
      oldClockWrapper.stackPairs>progressOnlyClock.stackPairs)
  }

  // Creation failure owns no thread. Once a thread exists, even a
  // late entry after close can still read the borrowed progress word.
  pure def op62WatcherCanRelease(created:bool,exited:bool,joined:bool):bool =
    not(created) or (exited and joined)

  run op62WatcherLateEntryAndCreationFailureKeepBorrowOwnershipTest={
    val createFailed={created:false,exited:false,joined:false}
    val joinBegun={created:true,exited:false,joined:false}
    val workerExited={...joinBegun,exited:true}
    val joined={...workerExited,joined:true}
    assert(op62WatcherCanRelease(createFailed.created,createFailed.exited,createFailed.joined) and
      not(op62WatcherCanRelease(joinBegun.created,joinBegun.exited,joinBegun.joined)) and
      not(op62WatcherCanRelease(workerExited.created,workerExited.exited,workerExited.joined)) and
      op62WatcherCanRelease(joined.created,joined.exited,joined.joined))
  }

  // The real isolated VM fails closed on a join error. Even when
  // business succeeded, the borrowed progress word cannot be returned
  // to the caller as if observation cleanup had succeeded.
  pure def op62WatcherCloseDisposition(created:bool,joinStatus:int,exited:bool):str =
    if(created and joinStatus!=0) "abort-unverified"
    else if(op62WatcherCanRelease(created,exited,joinStatus==0)) "release"
    else "hold"

  run op62InjectedJoinFailureCannotReturnASuccessfulReceiptTest=assert(
    op62WatcherCloseDisposition(true,22,false)=="abort-unverified" and
    op62WatcherCloseDisposition(true,22,true)=="abort-unverified" and
    op62WatcherCloseDisposition(true,0,false)=="hold" and
    op62WatcherCloseDisposition(true,0,true)=="release" and
    op62WatcherCloseDisposition(false,-1,false)=="release")

  // An invocation-wide wait budget is stronger than a correction
  // budget per polling iteration. The price is explicit lost coverage.
  pure def op62TwoDeadlineDisposition(firstStable:bool,secondStable:bool,
      crossedQuery:bool,cancelled:bool):{waitAttempts:int,queries:int,captured:bool} =
    if(cancelled) {waitAttempts:1,queries:0,captured:false}
    else if(firstStable) {waitAttempts:1,queries:1,captured:not(crossedQuery)}
    else if(secondStable) {waitAttempts:2,queries:1,captured:not(crossedQuery)}
    else {waitAttempts:2,queries:0,captured:false}

  run op62TwoDeadlineObserverBoundsWorkButMissesLaterGapsTest={
    val quiet=op62TwoDeadlineDisposition(true,false,false,false)
    val corrected=op62TwoDeadlineDisposition(false,true,false,false)
    val moved=op62TwoDeadlineDisposition(false,false,false,false)
    val crossed=op62TwoDeadlineDisposition(true,false,true,false)
    val stopped=op62TwoDeadlineDisposition(true,false,false,true)
    assert(quiet=={waitAttempts:1,queries:1,captured:true} and
      corrected=={waitAttempts:2,queries:1,captured:true} and
      moved=={waitAttempts:2,queries:0,captured:false} and
      crossed=={waitAttempts:1,queries:1,captured:false} and
      stopped=={waitAttempts:1,queries:0,captured:false} and
      Set(200,1000,2000).forall(period=>128*period<4000000))
  }

  // A single fixed-work observer-off/on fixture: little observer CPU
  // and nearly matched owner CPU do not qualify wall-time interference.
  run op62ObserverCpuDoesNotQualifyOwnerWallInterferenceTest={
    val off={updates:1000000,wall:561130792,cpu:326499459,observerCpu:0}
    val on={updates:1000000,wall:799490583,cpu:327097000,observerCpu:30041}
    val pairedOutsideDelta=(on.wall-on.cpu)-(off.wall-off.cpu)
    val evidence={cost:"unknown",availability:"qualified-envelope",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:true,
      readPrivateInbox:false,rewoundCursor:false}
    assert(on.updates==off.updates and on.wall-off.wall==238359791 and
      on.cpu-off.cpu==597541 and pairedOutsideDelta==237762250 and
      pairedOutsideDelta>on.observerCpu and not(gate::evidenceQualified(evidence)))
  }

  // The latest occupied bin (or the latest unscanned bin) is an
  // upper bound on a matched target's sample, never its identity.
  pure def op62LatestPossibleHistogramBin(occupied:Set[int],first:int,last:int,budget:int):int = {
    val cut=op62Later(first,last-budget+1)
    val inspected=occupied.filter(bin=>cut<=bin and bin<=last)
    if(inspected.size()>0) inspected.fold(-1,(top,bin)=>op62Later(top,bin))
    else if(cut>first) cut-1
    else -1
  }

  run op62TerminalReverseScanKeepsConservativeUpperUnderBudgetTest={
    val bins=0.to(7)
    assert(bins.powerset().exclude(Set(Set())).forall(occupied =>
      1.to(8).forall(budget =>
        occupied.forall(bin=>bin<=op62LatestPossibleHistogramBin(occupied,0,7,budget)))) and
      op62LatestPossibleHistogramBin(Set(2),1,7,2)==5 and
      op62LatestPossibleHistogramBin(Set(2),1,7,8)==2 and
      op62LatestPossibleHistogramBin(Set(2),5,7,8)== -1 and
      op62LatestPossibleHistogramBin(Set(2,7),1,7,8)==7)
  }

  // The request-deadline cut is after validation but before the
  // failed reply's histogram update. Earlier timed replies retired.
  // The progress-deadline cut can include an unreturned timed reply.
  pure def op62TimedCompletedPrefix(phase:str,timed:int,completed:int):bool =
    phase=="request-deadline" and timed>0 and timed==completed

  run op62ArchivedFailedRunProvesLongPrechargeWaitForCompletedTailTest={
    val timed=627083
    val elapsed=10125416833
    val threshold=9000000000
    val tail99=timed-budget::rank(timed,99,100)+1
    val tail999=timed-budget::rank(timed,999,1000)+1
    assert(op62TimedCompletedPrefix("request-deadline",timed,timed) and
      not(op62TimedCompletedPrefix("progress-deadline",timed+1,timed)) and
      not(op62TimedCompletedPrefix("request-deadline",timed+1,timed)) and
      tail99==6271 and tail999==628 and
      op62TailBeforePublicationFloor(tail99,9934359000,threshold,3,elapsed)==6239 and
      op62TailBeforePublicationFloor(tail999,9998563000,threshold,3,elapsed)==598)
  }

  // The raw Mach reads have a matching port, but their increment is
  // physically larger than the wall envelope. Even the loosest
  // possible query bracket rejects them. Custody evidence is separate.
  run op62MatchedMachCounterCanFailOwnerCpuEnvelopeTest={
    val origin=115941041220416
    val outer=115941366778000-origin
    val delta=682797000-351729000
    val envelope={sameOwner:true,wallStart:0,cpuStart:0,
      innerStart:115941366607582-origin,innerEnd:115941366636750-origin,
      cpuEnd:outer,wallEnd:outer,cpuDelta:delta}
    assert(outer==325557584 and delta==331068000 and
      delta-outer==5510416 and op62InnerCpuFloor(envelope)== -1 and
      115941366607582<115941366636581 and 115941366636581<115941366636750)
  }

  // Standalone Darwin calibration: both remote interfaces returned
  // the same counters. Neither can replace the bounded self clock.
  run op62RemoteCpuEnvelopeExcessDoesNotRequireGerbilOrMixedReadersTest={
    val owner={sameOwner:true,wallStart:0,cpuStart:0,innerStart:0,
      innerEnd:11898583,cpuEnd:11898583,wallEnd:11898583,cpuDelta:11859000}
    val mach={...owner,innerEnd:11701333,cpuEnd:11701333,wallEnd:11701333,cpuDelta:11730000}
    val proc={...mach,innerEnd:11707500,cpuEnd:11707500,wallEnd:11707500}
    assert(op62InnerCpuFloor(owner)==11859000 and
      op62InnerCpuFloor(mach)== -1 and op62InnerCpuFloor(proc)== -1 and
      mach.cpuDelta-mach.wallEnd==28667 and proc.cpuDelta-proc.wallEnd==22500)
  }

  // A valid broad self-clock bracket can bound a progress gap
  // while saying nothing about its narrower after-eligibility stage.
  run op62MatchedSelfClockBoundsGapButNotEligibleWorkTest={
    val origin=116299279895000
    val closing=116299974797666-origin
    val gapOpen=116299614135375-origin
    val eligible=116299946508541-origin
    val outer=116299975810791-origin
    val delta=168501666-34654625
    val gap={sameOwner:true,wallStart:0,cpuStart:0,
      innerStart:gapOpen,innerEnd:closing,cpuEnd:outer,wallEnd:outer,cpuDelta:delta}
    val work={...gap,innerStart:eligible}
    assert(op62InnerCpuFloor(gap)==0 and op62InnerCpuFloor(work)==0 and
      closing-gapOpen==360662291 and closing-gapOpen-delta==226815250 and
      closing-eligible==28289125 and delta>closing-eligible)
  }

  // The final source publication scalar belongs to the final
  // admitted ordinal. It is a lower bound on this reply's dequeue,
  // not on reply insertion, which could precede the source clock.
  pure def op62CanUseLastPublication(selected:int,planned:int,retired:int,rejected:int):bool =
    planned>0 and selected==planned-1 and retired==planned and rejected==0

  run op62FinalOrdinalReceiptTightensDequeueAndNormalRetirementTest={
    val planned=116459117082791
    val sourceEpoch=planned-(2*1000000000/3)
    val postSend=sourceEpoch+666676666
    val dequeueUpper=116459117111790
    val closing=116459117112791
    val innerEnd=sourceEpoch+666697291
    assert(op62CanUseLastPublication(2,3,3,0) and
      not(op62CanUseLastPublication(1,3,3,0)) and
      not(op62CanUseLastPublication(2,3,2,0)) and
      not(op62CanUseLastPublication(2,3,3,1)) and
      postSend-planned==10000 and dequeueUpper-postSend==18999 and
      innerEnd-closing==625 and
      Set(postSend-1,postSend+1).forall(enqueue=>
        planned<=enqueue and enqueue<=dequeueUpper))
  }

  // A timestamp between two equal monotonic heartbeat reads needs
  // no OS thread-state query. Later progress can still invalidate
  // an armed witness before the owner claims its closing reply.
  pure def op62HeartbeatCutMatches(before:int,sample:int,after:int,
      closingPrevious:int,closing:int,stopped:bool):bool =
    not(stopped) and before==after and before==closingPrevious and
    before<=sample and sample<closing

  run op62HeartbeatOnlySampleRejectsCrossedAndStaleCutsTest=assert(
    op62HeartbeatCutMatches(10,15,10,10,20,false) and
    not(op62HeartbeatCutMatches(10,15,11,10,20,false)) and
    not(op62HeartbeatCutMatches(10,15,10,11,20,false)) and
    not(op62HeartbeatCutMatches(10,15,10,10,15,false)) and
    not(op62HeartbeatCutMatches(10,15,10,10,20,true)))

  // Real gated VM receipt. The original histogram gives a valid
  // upper before the accepted cut, and the original closing clock
  // still precedes normal frame return. No remote thread query.
  run op62HeartbeatOnlyVmCanProveDequeuedCreditStillHeldTest={
    val key={invocation:1,owner:0,generation:1,gap:1,ordinal:2}
    val enqueued={key:key,lo:0,hi:561999}
    val dequeued={key:key,lo:75001,hi:561999}
    val retired={key:key,lo:1575876,hi:1621459}
    val sample={key:key,lo:1529959,hi:1529959}
    assert(op62ComposedCustody(enqueued,dequeued,retired,sample)=="dequeued" and
      sample.lo-dequeued.hi==967960 and retired.hi-retired.lo==45583 and
      op62ComposedCustody(enqueued,dequeued,retired,{...sample,key:{...key,generation:2}})=="unknown")
  }

  // In the linked VM, the bounded observer can choose a non-final
  // reply. Whole-run success proves normal return by the terminal
  // cut, but does not make the final publication sample its own.
  run op62LinkedBoundedHeartbeatMaySelectNonFinalOrdinalTest={
    val receipt={attempts:1,timedWaits:1,remoteQueries:0,
      cancelReleased:true,ordinal:1,planned:3,retired:3,rejected:0}
    assert(receipt.attempts<=2 and receipt.timedWaits<=receipt.attempts and
      receipt.remoteQueries==0 and receipt.cancelReleased and receipt.retired==receipt.planned and
      not(op62CanUseLastPublication(receipt.ordinal,receipt.planned,receipt.retired,receipt.rejected)))
  }

  // The real isolated close contracts used a pending wait. Signal
  // success wakes it, and signal failure leaves its timed fallback.
  // In either case join remains the release boundary.
  run op62SignalFailureDoesNotSkipWaitExitOrBorrowJoinTest={
    val woken={waitStatus:0,exited:true,joined:true,released:true,attempts:1,samples:0}
    val fallback={...woken,waitStatus:49}
    assert(Set(woken,fallback).forall(r=>
      r.attempts==1 and r.samples==0 and r.released and
      op62WatcherCanRelease(true,r.exited,r.joined)) and
      not(op62WatcherCanRelease(true,true,false)))
  }

  // On a fixed owner, conditional exclusive costs cannot overlap. The
  // budget gives a necessary coverage screen, even with zero overhead
  // and no pauses. Do not substitute aggregate wall averages for costs
  // unless they are explicitly being used as hypothetical inputs.
  pure def op62MixedServiceCost(retired:int, legacy:int, closedNs:int,
      legacyNs:int):int = (retired-legacy)*closedNs+legacy*legacyNs

  run op62FiveMillionBatchedPathMustCoverMostVerifiedReturnsTest={
    val required=5000000-128
    val availableNs=1000000000
    val legacyNs=996
    val closedNs=150
    val maxLegacy=(availableNs-required*closedNs)/(legacyNs-closedNs)
    assert(maxLegacy==295530 and
      op62MixedServiceCost(required,maxLegacy,closedNs,legacyNs)<=availableNs and
      op62MixedServiceCost(required,maxLegacy+1,closedNs,legacyNs)>availableNs and
      op62MixedServiceCost(required,required/4,0,legacyNs)>availableNs)
  }

  // A cheap average and a fully drained cohort can conceal an earlier
  // breach. Keep original arrival debt when classifying fallback bursts.
  run op62FallbackBurstCanFailPeakWithEqualFinalThroughputTest={
    val gate=128
    val planned=List(1000,2000)
    val even=List(1000,2000)
    val burst=List(871,2000)
    assert(even[1]==burst[1] and planned[1]-burst[1]==0 and
      planned[0]-even[0]<=gate and planned[0]-burst[0]==gate+1)
  }

  // Normal op62 has one receiver and cannot start the next receive
  // before returning the currently validated reply. A different
  // live reply whose send already returned must still be queued.
  // The ready/repark witness must identify the target and span this cut.
  pure def op62PeerTurnProvesQueued(target:Op62WitnessKey,
      peer:Op62WitnessKey, soleOrdinaryConsumer:bool, normalPeerHeld:bool,
      targetReadyAndReparked:bool):bool =
    soleOrdinaryConsumer and normalPeerHeld and targetReadyAndReparked and
    target.ordinal!=peer.ordinal and target=={...peer,ordinal:target.ordinal}

  run op62HeldPeerDisambiguatesAReparkedTargetsDequeueTest={
    val target={invocation:1,owner:0,generation:2,gap:3,ordinal:42}
    val peer={...target,ordinal:41}
    val cut={key:target,lo:120,hi:120}
    val enqueue={key:target,lo:80,hi:119}
    val originalDequeue={key:target,lo:90,hi:135}
    val retire={key:target,lo:135,hi:140}
    val proved=op62PeerTurnProvesQueued(target,peer,true,true,true)
    val narrowed=if(proved) {...originalDequeue,lo:121} else originalDequeue
    assert(op62ComposedCustody(enqueue,originalDequeue,retire,cut)=="unknown" and
      op62ComposedCustody(enqueue,narrowed,retire,cut)=="in-mailbox")
  }

  run op62HeldPeerCannotAttestStaleTargetOrExceptionalDrainTest={
    val target={invocation:1,owner:0,generation:2,gap:3,ordinal:42}
    val peer={...target,ordinal:41}
    assert(not(op62PeerTurnProvesQueued(target,peer,true,true,false)) and
      not(op62PeerTurnProvesQueued(target,peer,true,false,true)) and
      not(op62PeerTurnProvesQueued(target,peer,false,true,true)) and
      not(op62PeerTurnProvesQueued(target,target,true,true,true)) and
      not(op62PeerTurnProvesQueued(target,{...peer,generation:3},true,true,true)) and
      not(op62PeerTurnProvesQueued(target,{...peer,gap:4},true,true,true)))
  }

  // With three exclusive credits, a known queued reply can have one
  // earlier queued predecessor while a different reply is held by
  // the source. Immediate refill appends behind that queued reply.
  run op62QueuedTargetRetiresWithinTwoFurtherNormalTurnsTest={
    val target=42
    val peer=41
    val predecessor=43
    val refill=44
    val legalQueueCuts=Set(List(target),List(target,predecessor),List(predecessor,target))
    assert(legalQueueCuts.forall(queue => {
      val retirementOrder=List(peer).concat(queue).concat(List(refill))
      retirementOrder[1]==target or retirementOrder[2]==target
    }) and List(peer,predecessor,target,refill)[2]==target)
  }

  // A turn bound requires FIFO and a unique reply per charged frame.
  // In particular it is not a wall bound, and cleanup is not a turn.
  run op62QueuedTargetTurnBoundCannotTimeRecoveryOrAdmitDuplicateTest={
    val order=List(41,43,42,44)
    val early={order:order,at:120,returned:140,normal:true}
    val late={...early,returned:5000120}
    val duplicate=List(41,43,43,42,44)
    val failed={...late,normal:false}
    assert(early.order==late.order and late.returned-late.at>1000000 and
      duplicate[1]!=42 and duplicate[2]!=42 and duplicate[3]==42 and
      failed.order==late.order and not(failed.normal))
  }

  // The single comparison can enter a slow arm without shortening
  // the original progress deadline. Timeout must never publish or
  // spend the witness. Selection only applies to an already closed gap.
  pure def op62OwnerClosedGapStatus(last:int, now:int, openingFloor:int,
      sameWord:bool, alreadySelected:bool):str =
    if(now<last or now-last>=10000000000) "original-timeout"
    else if(now-last>=4000000 and last>openingFloor and sameWord and not(alreadySelected))
      "closed-gap"
    else "ordinary"

  run op62OwnerClosedGapPreservesWatchdogAndSelectionScopeTest=assert(
    op62OwnerClosedGapStatus(100,4000100,100,true,false)=="ordinary" and
    op62OwnerClosedGapStatus(101,4000100,100,true,false)=="ordinary" and
    op62OwnerClosedGapStatus(101,4000101,100,true,false)=="closed-gap" and
    op62OwnerClosedGapStatus(101,4000101,100,false,false)=="ordinary" and
    op62OwnerClosedGapStatus(101,4000101,100,true,true)=="ordinary" and
    op62OwnerClosedGapStatus(101,10000000100,100,true,false)=="closed-gap" and
    op62OwnerClosedGapStatus(101,10000000101,100,true,false)=="original-timeout" and
    op62OwnerClosedGapStatus(101,100,100,true,false)=="original-timeout")

  // The observer-free receipt closes on the normal progress event,
  // after dequeue. Its endpoints do not distinguish whether a target
  // was enqueued at an earlier timestamp in the same interval.
  run op62ClosedGapEndpointsCannotReplaceAnInteriorCustodyCutTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:42}
    val enqueue={key:key,lo:101,hi:4000099}
    val dequeue={key:key,lo:102,hi:4000100}
    val retire={key:key,lo:4000101,hi:4000110}
    val interior={key:key,lo:2000000,hi:2000000}
    assert(op62OwnerClosedGapStatus(101,4000101,100,true,false)=="closed-gap" and
      op62ComposedCustody(enqueue,dequeue,retire,interior)=="unknown")
  }

  // GCC 16 -O2 merged the op1 epilogue in both full owner adapter
  // variants. The extra taken branch is on the ordinary clock path.
  run op62ClosedGapLoweringMustIncludeOrdinaryClockExitTest={
    val original={clockBranch:0,stackBytes:64,heartbeatExtraLoads:0}
    val direct={...original,clockBranch:1}
    val sharedReturn={...direct}
    assert(Set(direct,sharedReturn).forall(shape =>
      shape.stackBytes==original.stackBytes and
      shape.heartbeatExtraLoads==original.heartbeatExtraLoads and
      shape.clockBranch>original.clockBranch))
  }

  // Keeping the original failure return in the adapter preserves
  // the ordinary op1 inline exit in the GCC 16 whole-object build.
  // The linked, ABI-matched sparse contract sealed exactly one cut.
  run op62RetainedDeadlineExitKeepsClockAndSelectsOneClosedVmGapTest={
    val control={op1ExitBranch:0,op11ExtraLoads:0,stackBytes:64}
    val retained={...control}
    val receipt={opening:0,planned:333306666,closing:341325125,
      ordinal:1,retired:3,rejected:0,selected:1}
    assert(retained==control and receipt.selected==1 and
      receipt.closing-receipt.planned==8018459 and
      op62OwnerClosedGapStatus(receipt.opening,receipt.closing,-1,true,false)=="closed-gap" and
      receipt.retired==3 and receipt.rejected==0 and
      not(op62CanUseLastPublication(receipt.ordinal,3,receipt.retired,receipt.rejected)))
  }

  // Same image and owner. The second call stages ordinal 1 before
  // its third original progress check fails. The failed call emits
  // no successful receipt and the next open resets both staged
  // scalar sets even when it later selects the same ordinal.
  run op62ClosedGapAbortDiscardsSelectedReplyBeforeOwnerReuseTest={
    val rounds=List(
      {owner:0,generation:1,selected:1,emitted:1,failed:false,published:3,retired:3,timed:3},
      {owner:0,generation:2,selected:1,emitted:0,failed:true,published:3,retired:2,timed:3},
      {owner:0,generation:3,selected:1,emitted:1,failed:false,published:3,retired:3,timed:3})
    val reset={opening:0,closing:0,staged:false}
    assert(rounds[1].published-rounds[1].retired==1 and
      rounds[1].timed==rounds[1].retired+1 and
      rounds[1].selected==rounds[2].selected and
      rounds[1].generation!=rounds[2].generation and
      rounds[1].owner==rounds[2].owner and
      rounds[1].failed and rounds[1].emitted==0 and
      reset.opening==0 and reset.closing==0 and not(reset.staged) and
      rounds[2].emitted==1 and rounds[2].retired==rounds[2].published)
  }

  // Installed Gambit permits Scheme execution after restoring an old
  // callback continuation. The dead foreign return is detected later.
  run op62ForeignReturnRejectionDoesNotFenceRestoredBodyTest={
    val events=List("old-c-returned","new-c-entry","restored-old-body",
      "stale-side-effect","dead-c-return-rejected")
    assert(events[2]=="restored-old-body" and events[3]=="stale-side-effect" and
      events[4]=="dead-c-return-rejected")
  }

  // An active wrapper and even a reused address can belong to a newer
  // call. The first guarded effect must use the activation's identity.
  run op62OldActivationCannotBorrowNewForeignCallTest={
    val origin={owner:0,generation:1,address:128}
    val current={...origin,generation:2}
    val samePointer=origin.owner==current.owner and origin.address==current.address
    val foreignEntries=2
    val restoredBodyEntries=3
    val guarded=if(origin==current) "stale-side-effect" else "resume-current"
    assert(samePointer and origin!=current and foreignEntries<restoredBodyEntries and
      guarded=="resume-current")
  }

  pure def op62RelevantGapNs(opening:int, closing:int, planned:int):int={
    val relevant=if(planned>opening) planned else opening
    if(closing>=relevant) closing-relevant else 0
  }

  // The first tentative cut is rejected by its own eligibility,
  // without spending the next candidate's one-shot selection.
  run op62ClosedGapSelectionClipsEligibilityWithoutSpendingWitnessTest={
    val first={closing:334333333,planned:333333333}
    val last={opening:first.closing,closing:676135333,planned:666666666,
      postSend:666756500,normalReturnUpper:676141708,ordinal:2}
    val afterFirst=op62RelevantGapNs(0,first.closing,first.planned)>=4000000
    val afterLast=not(afterFirst) and op62RelevantGapNs(last.opening,last.closing,last.planned)>=4000000
    assert(not(afterFirst) and afterLast and
      op62RelevantGapNs(last.opening,last.closing,last.planned)==9468667 and
      op62RelevantGapNs(5000000,7000000,0)==2000000 and
      op62RelevantGapNs(0,5000000,6000000)==0 and
      op62CanUseLastPublication(last.ordinal,3,3,0) and
      last.planned<last.postSend and last.postSend<last.closing and
      last.normalReturnUpper-last.closing==6375)
  }

  // The native row is inside the same run and relevant interval,
  // but ends before the source's final request post-send clock.
  // Its CPU bracket encloses more than its wall bracket.
  run op62ClosedGapSelectCannotAccountForPostSendFixtureDelayTest={
    val receipt={planned:0,postSend:147126,closing:9326209,retirementUpper:9327501}
    val row={start:11042,end:53459,cpu:84167,kind:4,requested:0}
    val overlapBeforeClosing=op62Earlier(receipt.closing,row.end)-op62Later(receipt.planned,row.start)
    assert(overlapBeforeClosing==42417 and row.cpu>overlapBeforeClosing and
      row.end<receipt.postSend and receipt.closing-receipt.postSend==9179083 and
      receipt.closing-overlapBeforeClosing==9283792 and
      receipt.retirementUpper-receipt.closing==1292)
  }

  // The sole ordinary source must return the held frame before it can
  // reach another verified progress call. A timestamp from another
  // scope or a non-verified update cannot close this return bound.
  pure def op62NextProgressBoundsReturn(selected:Op62WitnessKey, observed:Op62WitnessKey,
      selectedClose:int, previous:int, successor:int, normal:bool, soleSource:bool):bool =
    selected=={...observed,ordinal:selected.ordinal} and
    selected.ordinal!=observed.ordinal and normal and soleSource and
    selectedClose==previous and successor>previous

  run op62NextNativeProgressBoundsNonFinalReplyReturnTest={
    val selected={invocation:1,owner:0,generation:2,gap:3,ordinal:1}
    val observed={...selected,ordinal:2}
    val closed=119615437263916
    val successor=119615762918916
    assert(op62NextProgressBoundsReturn(selected,observed,closed,closed,successor,true,true) and
      successor-closed==325655000 and
      not(op62CanUseLastPublication(selected.ordinal,3,3,0)) and
      not(op62NextProgressBoundsReturn(selected,{...observed,generation:3},closed,closed,successor,true,true)) and
      not(op62NextProgressBoundsReturn(selected,observed,closed,closed,successor,false,true)) and
      not(op62NextProgressBoundsReturn(selected,observed,closed,closed+1,successor,true,true)) and
      not(op62NextProgressBoundsReturn(selected,observed,closed,closed,successor,true,false)))
  }

  // First delivery on one original modulo-four worker. The distinct
  // prefix never reads the frame after send. A late seal still gives
  // an upper bound but cannot classify an earlier cut as in-mailbox.
  run op62FirstWorkerSealIsImmutableAcrossEarlyReuseTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:1}
    val cut={key:key,lo:120,hi:120}
    val enqueue={key:key,lo:100,hi:119}
    val dequeue={key:key,lo:130,hi:130}
    val retire={key:key,lo:135,hi:135}
    val frameReuse={ordinal:5,at:136}
    val delayed={ordinal:key.ordinal,seal:140}
    val afterReuse={...enqueue,hi:delayed.seal}
    assert(key.ordinal%4==frameReuse.ordinal%4 and
      frameReuse.at<delayed.seal and delayed.ordinal!=frameReuse.ordinal and
      op62ComposedCustody(enqueue,dequeue,retire,cut)=="in-mailbox" and
      op62ComposedCustody(afterReuse,dequeue,retire,cut)=="unknown")
  }

  run op62FirstWorkerWitnessCannotCoverLaterUnselectedGapTest={
    val armed=1
    val route=1
    val following=5
    assert(armed%4==route and following%4==route and
      0.to(3).filter(id=>id%4==route)==Set(armed) and
      following!=armed)
  }

  // The local worker proof did not extend to the full generated
  // scheduled symbol. Changing register allocation at source supply
  // keeps observer cost unqualified even with one possible seal.
  run op62FirstWorkerFullLoweringChangesOrdinarySourceTest={
    val local={controlWords:300,candidateWords:300}
    val scheduled={controlWords:5616,candidateWords:6175,
      sourceCounterRegisterBefore:10,sourceCounterRegisterAfter:6}
    val evidence={cost:"unknown",availability:"qualified-envelope",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:true,
      readPrivateInbox:false,rewoundCursor:false}
    assert(local.controlWords==local.candidateWords and
      scheduled.candidateWords-scheduled.controlWords==559 and
      scheduled.sourceCounterRegisterBefore!=scheduled.sourceCounterRegisterAfter and
      not(gate::evidenceQualified(evidence)))
  }

  // Outlining removed the first worker body from the scheduled
  // symbol. The startup branch still changes ordinary lowering.
  run op62OutlinedFirstWorkerDoesNotRecoverFrozenSourceLayoutTest={
    val words={control:5616,inline:6175,outlined:5724}
    val register={control:10,inline:6,outlined:6}
    val evidence={cost:"unknown",availability:"qualified-envelope",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:true,
      readPrivateInbox:false,rewoundCursor:false}
    assert(words.inline-words.outlined==451 and words.outlined-words.control==108 and
      register.outlined!=register.control and not(gate::evidenceQualified(evidence)))
  }

  // The original continuation restores the loop's lane and receiver.
  // Those remain equal on later jobs on that lane. A slow poll
  // reached after send does not restore the sent reply's ordinal.
  run op62ReplyReturnPollArgumentsAliasLaterOrdinalTest={
    val first={ordinal:2,lane:2,receiver:1,site:71}
    val later={...first,ordinal:6}
    def snapshot(a)={lane:a.lane,receiver:a.receiver,site:a.site}
    assert(first.ordinal%4==first.lane and later.ordinal%4==later.lane and
      first.ordinal!=later.ordinal and snapshot(first)==snapshot(later))
  }

  type Op62SpawnKey={owner:int,code:int,lane:int,receiver:int}
  pure def op62StartupArmMatches(armed:bool, expected:Op62SpawnKey,
      observed:Op62SpawnKey):bool = armed and expected==observed

  run op62StartupSpawnArmRequiresCodeOwnerLaneAndReceiverTest={
    val expected={owner:1,code:68,lane:2,receiver:1}
    assert(op62StartupArmMatches(true,expected,expected) and
      not(op62StartupArmMatches(true,expected,{...expected,code:69})) and
      not(op62StartupArmMatches(true,expected,{...expected,owner:2})) and
      not(op62StartupArmMatches(true,expected,{...expected,lane:0})) and
      not(op62StartupArmMatches(true,expected,{...expected,receiver:2})) and
      not(op62StartupArmMatches(false,expected,expected)))
  }

  // Optimized Gerbil fixture. The target matched on the third
  // startup call. A detached hook could only delegate afterward.
  run op62StartupSpawnBindingRestoresBeforeTargetAndPartialFailureTest={
    val normal={hits:3,seals:1,bindingRestored:true,detachedSeals:0}
    val failed={hits:2,seals:0,bindingRestored:true,exactFailure:true}
    assert(normal.bindingRestored and normal.seals==1 and normal.detachedSeals==0 and
      failed.bindingRestored and failed.exactFailure and failed.seals==0 and
      failed.hits<normal.hits)
  }

  // The full image ran the three-request contract, but its diagnostic
  // output did not contain an authenticated startup closure record.
  run op62FullStartupSuccessWithoutClosureReceiptCannotAuthorizeBindingTest={
    val receipt={planned:3,retired:3,rejected:0,closed:true,closureRows:0}
    val scheduled={controlWords:5616,observerWords:5620,
      originalCounterRegister:10,observerCounterRegister:10}
    val evidence={cost:"unknown",availability:"unknown",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:false,
      readPrivateInbox:false,rewoundCursor:false}
    assert(receipt.planned==receipt.retired and receipt.rejected==0 and receipt.closed and
      receipt.closureRows==0 and scheduled.observerWords-scheduled.controlWords==4 and
      scheduled.originalCounterRegister==scheduled.observerCounterRegister and
      not(gate::evidenceQualified(evidence)))
  }

  // If target still owns a frame, each later published member of
  // its FIFO worker lane owns one too. Publishing the next ordinal
  // after those credits fill requires target's normal retirement.
  pure def op62StartupPublishProofOrdinal(target:int, lanes:int, credits:int):int =
    target + (credits-1)*lanes + 1

  run op62FourWorkerStartupWitnessEndsWithinTwelveRetirementTurnsTest={
    val startup=0.to(3)
    val proofs=startup.map(i=>op62StartupPublishProofOrdinal(i,4,3))
    assert(proofs==Set(9,10,11,12) and startup.forall(i=>{
      val third=i+8
      val held=i.to(third).filter(j=>j%4==i%4)
      held.size()==3 and not(held.contains(op62StartupPublishProofOrdinal(i,4,3)))
    }))
  }

  // The archived above-window turns have distinct normal returns.
  // A startup witness cannot explain these as time spent holding
  // that startup frame. It may still see the initial debt trigger.
  run op62ArchivedBreachTurnsOutlastAllStartupWitnessesTest={
    val proof=op62StartupPublishProofOrdinal(3,4,3)
    val rounds=Set(
      {breaches:83042,after:83030,period:2000,maxLag:697625,latest:721625},
      {breaches:998385,after:998373,period:1000,maxLag:6743416,latest:6755416},
      {breaches:4999857,after:4999845,period:200,maxLag:3984036284,latest:3984038684},
      {breaches:999845,after:999833,period:1000,maxLag:27767125,latest:27779125})
    assert(rounds.forall(r=>r.breaches-proof==r.after and
      proof*r.period+r.maxLag==r.latest and r.latest>128*r.period))
  }

  // The source executes supply and its post-send clock before
  // entering receive. Capture the latest such clock in that reply's
  // cold continuation. A terminal clock from a later turn is not
  // interchangeable with the value at the selected turn.
  pure def op62SelectedPostSendDequeueLower(planned:int, closing:int,
      postSend:int, sameKey:bool, capturedAtSelectedTurn:bool):int =
    if(sameKey and capturedAtSelectedTurn and planned<=postSend and postSend<=closing)
      postSend
    else planned

  run op62SelectedTurnPublicationCanTightenNonFinalDequeueLowerTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:41}
    val cut={key:key,lo:120,hi:120}
    val enqueue={key:key,lo:110,hi:119}
    val dequeue={key:key,lo:110,hi:150}
    val retire={key:key,lo:150,hi:170}
    val lower=op62SelectedPostSendDequeueLower(110,150,125,true,true)
    assert(lower==125 and
      op62ComposedCustody(enqueue,dequeue,retire,cut)=="unknown" and
      op62ComposedCustody(enqueue,{...dequeue,lo:lower},retire,cut)=="in-mailbox" and
      op62SelectedPostSendDequeueLower(110,150,125,true,false)==110 and
      op62SelectedPostSendDequeueLower(110,150,125,false,true)==110 and
      op62SelectedPostSendDequeueLower(110,150,151,true,true)==110)
  }

  // GCC 16 matched every relocated direct intra-function edge
  // outside the accepted-capture block. This is a static screen
  // against the parent isolate, not an observed timing bound.
  run op62DequeueLowerStagingKeepsOrdinaryDirectEdgesTest={
    val control={words:5656,cold:11,padding:1}
    val staged={words:5666,cold:22,padding:0}
    val cfg={prefix:3395,suffix:2249,directEdges:800,intoChangedBlock:0}
    assert(control.words-control.cold-control.padding==cfg.prefix+cfg.suffix and
      staged.words-staged.cold-staged.padding==cfg.prefix+cfg.suffix and
      cfg.directEdges==800 and cfg.intoChangedBlock==0)
  }

  run op62StagedSourceClockDefersValidationUntilSuccessfulCloseTest={
    val planned=110
    val close=150
    val epoch=100
    val offset=25
    assert(op62SelectedPostSendDequeueLower(planned,close,epoch+offset,true,true)==125 and
      op62SelectedPostSendDequeueLower(planned,close,epoch+offset,false,true)==planned and
      op62SelectedPostSendDequeueLower(planned,close,epoch+offset,true,false)==planned and
      op62SelectedPostSendDequeueLower(planned,close,epoch+51,true,true)==planned)
  }

  // The scheduled source advances the integral plan after its request
  // send and before its clock. The generated plan has a Gambit poll.
  // Even for the same ordinal, the worker can send its reply before
  // that source clock if the source yields the processor there.
  run op62SameOrdinalSourcePostSendCannotBoundReplyEnqueueFromBelowTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:41}
    val source={requestSent:110,poll:114,postSend:130,dequeue:145,progress:150,retire:155}
    val scheduledWorker={handler:116,enqueue:120}
    val laterWorker={handler:135,enqueue:140}
    val cut={key:key,lo:125,hi:125}
    val e={key:key,lo:source.requestSent,hi:source.dequeue}
    val d={key:key,lo:source.dequeue,hi:source.dequeue}
    val r={key:key,lo:source.retire,hi:source.retire}
    val dequeueLower=op62SelectedPostSendDequeueLower(100,source.progress,source.postSend,true,true)
    assert(source.requestSent<source.poll and source.poll<scheduledWorker.handler and
      scheduledWorker.handler<scheduledWorker.enqueue and scheduledWorker.enqueue<cut.lo and
      cut.hi<source.postSend and source.postSend<laterWorker.handler and
      laterWorker.handler<laterWorker.enqueue and laterWorker.enqueue<source.dequeue and
      dequeueLower==source.postSend and
      op62ComposedCustody({...e,hi:scheduledWorker.enqueue},d,r,cut)=="in-mailbox" and
      op62ComposedCustody({...e,lo:laterWorker.enqueue},d,r,cut)=="before-enqueue" and
      op62ComposedCustody(e,d,r,cut)=="unknown" and
      op62ComposedCustody({...e,lo:source.postSend},d,r,cut)=="before-enqueue")
  }

  // Public FIFO with exactly one delivery per ordinal lets an
  // ordinary business reply bound another reply's insertion. Use
  // verified receive order, not the order in which work was offered.
  pure def op62EnqueueFromFifoPeer(target:Op62TimeBound,peer:Op62TimeBound,
      targetTurn:int,peerTurn:int,uniqueOrdinaryFifo:bool):Op62TimeBound =
    if(not(uniqueOrdinaryFifo) or targetTurn<0 or peerTurn<0 or
       target.key.ordinal==peer.key.ordinal or
       target.key!={...peer.key,ordinal:target.key.ordinal} or peer.lo>peer.hi)
      target
    else if(targetTurn<peerTurn) {...target,hi:op62Earlier(target.hi,peer.hi)}
    else if(peerTurn<targetTurn) {...target,lo:op62Later(target.lo,peer.lo)}
    else target

  run op62BusinessPeerBoundsReplyWithoutInsertingAMarkerTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:43}
    val target={key:key,lo:100,hi:140}
    val peer={key:{...key,ordinal:41},lo:110,hi:119}
    val cut={key:key,lo:120,hi:120}
    val dequeue={key:key,lo:135,hi:140}
    val retire={key:key,lo:141,hi:150}
    val upper=op62EnqueueFromFifoPeer(target,peer,5,6,true)
    val lower=op62EnqueueFromFifoPeer(target,{...peer,lo:125,hi:130},6,5,true)
    assert(op62ComposedCustody(target,dequeue,retire,cut)=="unknown" and
      op62ComposedCustody(upper,dequeue,retire,cut)=="in-mailbox" and
      op62ComposedCustody(lower,dequeue,retire,cut)=="before-enqueue" and
      op62EnqueueFromFifoPeer(target,peer,6,5,true).hi==target.hi and
      op62EnqueueFromFifoPeer(target,peer,5,6,false)==target and
      op62EnqueueFromFifoPeer(target,peer,5,5,true)==target and
      op62EnqueueFromFifoPeer(target,peer,-1,6,true)==target and
      op62EnqueueFromFifoPeer(target,peer,5,-1,true)==target and
      op62EnqueueFromFifoPeer(target,{...peer,key:{...peer.key,gap:4}},5,6,true)==target and
      op62EnqueueFromFifoPeer(target,{...peer,key:{...peer.key,generation:3}},5,6,true)==target and
      op62EnqueueFromFifoPeer(target,{...peer,lo:120},5,6,true)==target)
  }

  // The opening reply is included in completedBeforePeer. The source
  // sends a contiguous prefix and returns every ordinary credit once.
  // All ordinals below that distinct completion count were published
  // by the opening heartbeat, including any later-dequeued ordinal.
  run op62CompletedCardinalityBoundsAlreadyPublishedTargetTest={
    val ordinals=0.to(7)
    assert(ordinals.forall(published =>
      ordinals.filter(id => id<published).powerset().forall(verified =>
        ordinals.filter(id => id<verified.size()).forall(id => id<published))) and
      Set(1,2).size()==2 and not(Set(1,2).contains(0)))
  }

  // No more than capacity frames are charged just before the first
  // post-opening retirement. Its ordinal is therefore below the
  // retired count plus capacity. If it was not yet due at opening,
  // opening debt after the opening return was at most capacity - 1.
  run op62FutureFifoPeerCannotWitnessASaturatedOpeningTest={
    val capacity=3
    val positions=0.to(16)
    assert(positions.forall(completed =>
      positions.filter(published => completed<=published and published<=completed+capacity)
        .forall(published =>
          positions.filter(peer => peer<published).forall(peer =>
            positions.filter(dueAtOpening => dueAtOpening<=peer).forall(dueAtOpening =>
              dueAtOpening-completed<=capacity-1)))))
  }

  run op62FuturePeerCanClassifyALowDebtPublishedTargetTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:3}
    val completedBeforePeer=5
    val target={key:key,lo:60,hi:170}
    val peer={key:{...key,ordinal:7},lo:125,hi:160}
    val cut={key:key,lo:120,hi:120}
    val dequeue={key:key,lo:161,hi:170}
    val retire={key:key,lo:170,hi:180}
    val withPeer=op62EnqueueFromFifoPeer(target,peer,6,5,true)
    assert(target.key.ordinal<completedBeforePeer and
      peer.key.ordinal<completedBeforePeer+3 and
      op62ComposedCustody(target,dequeue,retire,cut)=="unknown" and
      op62ComposedCustody(withPeer,dequeue,retire,cut)=="before-enqueue")
  }

  // Every above-window drive leads to one progress event at least as
  // late as its sample. Its next peer cannot be future-eligible at
  // that opening. Omit the final retirement, which has no successor.
  run op62FuturePeerCoverageIsLimitedByArchivedBreachTurnsTest={
    val cohorts=Set(
      {total:5000000,breach:83042,eligibleUpper:4916958},
      {total:1000000,breach:998385,eligibleUpper:1615},
      {total:1000000,breach:999845,eligibleUpper:155},
      {total:5000000,breach:4999857,eligibleUpper:143})
    assert(cohorts.forall(c => c.total-1-(c.breach-1)==c.eligibleUpper))
  }

  // The installed Gambit sender appends first. Its heap slow path
  // and blocked-receiver wake are conditional after insertion. A
  // source that is still supplying can have no waiting receive.
  run op62QuietSendCanCrossTheCutWithoutHeapOrWakeEvidenceTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:41}
    val cut={key:key,lo:120,hi:120}
    val dequeue={key:key,lo:145,hi:145}
    val retire={key:key,lo:150,hi:150}
    val early={key:key,lo:115,hi:115}
    val late={...early,lo:135,hi:135}
    val candidate={key:key,lo:110,hi:145}
    val noHeapOrWake={heapSlowPaths:0,wakes:0}
    val histories=Set({enqueue:early,events:noHeapOrWake},{enqueue:late,events:noHeapOrWake})
    assert(histories.map(h => h.events).size()==1 and
      histories.map(h => op62ComposedCustody(h.enqueue,dequeue,retire,cut))==
        Set("before-enqueue","in-mailbox") and
      op62ComposedCustody(candidate,dequeue,retire,cut)=="unknown")
  }

  // A public ready/repark snapshot only certifies its own cut. A
  // scheduler poll between progress and state acquisition can let
  // the worker append and repark before the source reads its state.
  run op62QueryPollCannotBackdateReplyAtClosedGapTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:41}
    val peer={...key,ordinal:42}
    val close={key:key,lo:120,hi:120}
    val sampled={key:key,lo:130,hi:130}
    val dequeue={key:key,lo:145,hi:145}
    val retire={key:key,lo:150,hi:150}
    val beforeClose={key:key,lo:115,hi:115}
    val afterClose={key:key,lo:125,hi:125}
    val observed={key:key,lo:110,hi:129}
    assert(op62PeerTurnProvesQueued(key,peer,true,true,true) and
      op62ComposedCustody(observed,dequeue,retire,sampled)=="in-mailbox" and
      op62ComposedCustody(observed,dequeue,retire,close)=="unknown" and
      op62ComposedCustody(beforeClose,dequeue,retire,close)=="in-mailbox" and
      op62ComposedCustody(afterClose,dequeue,retire,close)=="before-enqueue")
  }

  // Allocating the waiting-state result can happen after the fixed
  // snapshot. This does not move the snapshot, but it can still
  // stall the held frame's return and the next available credit.
  run op62StateSnapshotAndRecoveryCostAreSeparateTest={
    val original={close:120,snapshot:120,returnAt:125,nextDequeue:140}
    val slow={...original,returnAt:1025,nextDequeue:1040}
    assert(original.snapshot==slow.snapshot and
      original.close==slow.close and
      slow.returnAt-original.returnAt==900 and
      slow.nextDequeue-original.nextDequeue==900)
  }

  // The original worker receives its next request only after its
  // preceding public send returns. A later live frame on that lane
  // proves the earlier reply was sent once it reaches running.
  type Op62SiblingCut = {key:Op62WitnessKey, slot:int, phase:str}
  pure def op62SameLaneSiblingProvesQueued(target:Op62SiblingCut,
      later:Op62SiblingCut, held:Op62SiblingCut, originalHeldTurn:bool,
      sameOwnerWithoutYield:bool):bool =
    originalHeldTurn and sameOwnerWithoutYield and
    Set(target.slot,later.slot,held.slot)==Set(0,1,2) and
    Set(target.key.ordinal,later.key.ordinal,held.key.ordinal).size()==3 and
    target.key.ordinal>=0 and held.key.ordinal>=0 and
    target.key=={...later.key,ordinal:target.key.ordinal} and
    target.key=={...held.key,ordinal:target.key.ordinal} and
    target.phase=="ready" and held.phase=="dequeued" and
    (later.phase=="running" or later.phase=="ready") and
    target.key.ordinal<later.key.ordinal and
    target.key.ordinal%4==later.key.ordinal%4

  run op62SameLaneRunningWitnessNeedsNoPublicStateQueryTest={
    val key={invocation:1,owner:0,generation:2,gap:3,ordinal:0}
    val target={key:key,slot:0,phase:"ready"}
    val later={key:{...key,ordinal:4},slot:2,phase:"running"}
    val held={key:{...key,ordinal:3},slot:1,phase:"dequeued"}
    assert(op62SameLaneSiblingProvesQueued(target,later,held,true,true) and
      op62SameLaneSiblingProvesQueued(target,{...later,phase:"ready"},held,true,true) and
      not(op62SameLaneSiblingProvesQueued(target,{...later,phase:"queued"},held,true,true)) and
      not(op62SameLaneSiblingProvesQueued(target,{...later,key:{...key,ordinal:5}},held,true,true)) and
      not(op62SameLaneSiblingProvesQueued(target,{...later,key:{...later.key,generation:3}},held,true,true)) and
      not(op62SameLaneSiblingProvesQueued(target,later,{...held,slot:2},true,true)) and
      not(op62SameLaneSiblingProvesQueued(target,later,held,false,true)) and
      not(op62SameLaneSiblingProvesQueued(target,later,held,true,false)))
  }

  // At this positive cut, all three credits are accounted for.
  // The held reply is already out, and the other sibling cannot
  // send before the target. A post-return refill also appends later.
  run op62SameLaneSiblingIdentifiesImmediateSuccessorTest={
    val target=0
    val later=4
    val refill=5
    val suffixes=Set(List(),List(later),List(refill),List(later,refill),List(refill,later))
    assert(suffixes.forall(suffix => List(target).concat(suffix)[0]==target) and
      List(later,target)[0]!=target)
  }

  // The matched parent rebuild reproduces the scheduled words.
  // A source-level cold snapshot still changes earlier lowering.
  run op62SiblingSnapshotColdSourceChangesOrdinaryMachineCodeTest={
    val parent={words:5666,coldStart:3395}
    val rebuilt={words:5666,matchedWords:5666}
    val staged={words:5724,firstChanged:934}
    assert(parent.words==rebuilt.words and rebuilt.words==rebuilt.matchedWords and
      staged.words-parent.words==58 and staged.firstChanged<parent.coldStart)
  }

  // A cold C call in the generated host still makes GCC save LR on
  // the ordinary host entry. Static stack change fails the screen.
  run op62OutlinedSiblingSnapshotAddsOrdinaryNativeStackTest={
    val control={words:5666,stackOperands:0,entryFrame:0}
    val outlined={words:5687,stackOperands:13,entryFrame:96}
    assert(outlined.words-control.words==21 and
      outlined.stackOperands>control.stackOperands and
      outlined.entryFrame>control.entryFrame)
  }

  // The bridge saves the caller label in STK(3), then adjusts by
  // eight words. The old scheduled STK(-19) becomes PSSTK(-27).
  // Only the exact saved return label authenticates those slots.
  run op62ExistingFfiFrameKeepsCallerIdentityAcrossAdjustmentTest={
    val caller=200
    val bridge=caller-8
    val frameVector=caller-(-19)
    val callerReturn=caller-3
    val expectedReturn=6702+150
    val ledger=Map(frameVector -> 41,callerReturn -> expectedReturn,bridge -> 8613)
    assert(bridge-(-27)==frameVector and bridge-(-5)==callerReturn and
      ledger.get(bridge-(-5))==expectedReturn and
      ledger.get(bridge)!=expectedReturn and
      ledger.get(bridge-(-27))==41)
  }

  // OS owner, processor, return label and numeric generation can
  // coincide for a different green thread. The source activation
  // must authenticate before any caller vector access.
  type Op62BridgeIdentity={processor:int,owner:int,source:int,generation:int,returnLabel:int}
  pure def op62BridgeMayRead(expected:Op62BridgeIdentity,actual:Op62BridgeIdentity,
      armed:bool,spent:bool):bool =
    armed and not(spent) and expected==actual

  run op62ExistingBridgeRejectsAnotherGreenThreadAndSameLabelReplayTest={
    val source={processor:1,owner:2,source:3,generation:7,returnLabel:13}
    val other={...source,source:4}
    assert(op62BridgeMayRead(source,source,true,false) and
      not(op62BridgeMayRead(source,other,true,false)) and
      not(op62BridgeMayRead(source,{...source,generation:6},true,false)) and
      not(op62BridgeMayRead(source,{...source,returnLabel:9},true,false)) and
      not(op62BridgeMayRead(source,source,true,true)) and
      not(op62BridgeMayRead(source,source,false,false)))
  }

  // Link-time address changes are covered by matching object
  // relocations or byte-identical renamed literals. Static equality
  // does not bound the added cold call or qualify the capture.
  run op62SiblingBridgeLinkedOrdinaryPathDoesNotQualifyColdCostTest={
    val ordinary=Set(
      {words:119,relocated:6,unmatched:0},
      {words:5666,relocated:26,unmatched:0},
      {words:64,relocated:8,unmatched:0},
      {words:246,relocated:29,unmatched:0})
    val evidence={cost:"unknown",availability:"qualified-envelope",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:true,
      readPrivateInbox:false,rewoundCursor:false}
    assert(ordinary.forall(s => s.words>s.relocated and s.unmatched==0) and
      not(gate::evidenceQualified(evidence)))
  }

  // The admitted three-request contract can validate frame access.
  // Its ordinals occupy distinct lanes and cannot prove same-lane send.
  run op62ThreeOrdinalBridgeContractCannotProduceSiblingSendWitnessTest={
    val ordinals=Set(0,1,2)
    assert(ordinals.map(id=>id%4).size()==ordinals.size() and
      not(ordinals.exists(a=>ordinals.exists(b=>a<b and a%4==b%4))))
  }

  // Source-driven five-message Gerbil fixture. The earlier delayed
  // reply is eligible for the generic gap detector. A fixture must
  // select the actual held-peer turn instead of spending it there.
  run op62FiveMessageSiblingContractNeedsTheThirdReplyCutTest={
    val previous=1400000000
    val early={ordinal:2,planned:1400000000,progress:1805000000}
    val chosen={ordinal:3,planned:1600000000,progress:1810000000}
    val fifo=List(1,2,3,0,4)
    val successorAt=1810001000
    val upper=1810002000
    assert(early.progress-op62Later(previous,early.planned)==405000000 and
      chosen.progress-op62Later(early.progress,chosen.planned)==5000000 and
      fifo[2]==chosen.ordinal and fifo[3]==0 and fifo[4]==4 and
      chosen.progress<successorAt and successorAt<upper and
      not(op62OwnerClosedGapStatus(early.progress,chosen.progress,previous,true,true)=="closed-gap"))
  }

  // The compiled five-message fixture reads the selected slot at the
  // original progress FFI boundary and verifies its later release.
  // Its observed clock bracket excludes surrounding fixture and bridge work.
  run op62FiveMessageCompiledBridgeDoesNotQualifyFullImageCostTest={
    val held={ordinal:3,readyPeer:0,runningPeer:4}
    val targetSlot=1
    val dequeued={ordinal:0,slot:targetSlot}
    val retired={lastProgress:4,slot:targetSlot,free:true}
    val evidence={cost:"unknown",availability:"qualified-envelope",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:false,
      readPrivateInbox:false,rewoundCursor:false}
    assert(held.readyPeer%4==held.runningPeer%4 and
      held.ordinal%4!=held.readyPeer%4 and
      dequeued.ordinal==held.readyPeer and dequeued.slot==retired.slot and
      retired.lastProgress==held.runningPeer and retired.free and
      not(gate::evidenceQualified(evidence)) and
      not(gate::evidenceQualified({...evidence,sourceMatched:true})))
  }

  // The linked bridge runs before the planned-time filter. Rejected
  // candidates may incur another bridge call, while any accepted
  // gap prevents the later sibling snapshot even if it was uninformative.
  run op62LinkedFirstAcceptedGapCannotInheritFixtureOrdinalFilterTest={
    val floor=0
    val early={opening:1400000000,closing:1805000000,due:1400000000}
    val sibling={opening:early.closing,closing:1810000000,due:1600000000}
    val clipped={...early,due:early.closing-1}
    val earlierAccepted=early.closing-op62Later(early.opening,early.due)>=4000000
    assert(op62OwnerClosedGapStatus(early.opening,early.closing,floor,true,false)=="closed-gap" and
      earlierAccepted and
      op62OwnerClosedGapStatus(sibling.opening,sibling.closing,floor,true,earlierAccepted)=="ordinary" and
      op62OwnerClosedGapStatus(clipped.opening,clipped.closing,floor,true,false)=="closed-gap" and
      clipped.closing-op62Later(clipped.opening,clipped.due)<4000000 and
      op62OwnerClosedGapStatus(sibling.opening,sibling.closing,floor,true,false)=="closed-gap")
  }

  // Put the initial long interval on the existing opening-floor
  // exclusion. Then the first accepted cut can be the sibling cut
  // without a fixture-only ordinal selector.
  run op62OpeningFloorCanExcludeBootstrapWithoutFilteringOrdinalsTest={
    val initial=1000000000
    val first=1805000000
    val second=first+1000
    val selected=1810000000
    assert(op62OwnerClosedGapStatus(initial,first,initial,true,false)=="ordinary" and
      op62OwnerClosedGapStatus(first,second,initial,true,false)=="ordinary" and
      op62OwnerClosedGapStatus(second,selected,initial,true,false)=="closed-gap" and
      selected-op62Later(second,1600000000)>=4000000)
  }

  // A matched five-message fixture covers startup, the accepted helper,
  // a later frame-free observation, and cleanup. The admitted image
  // does not publish a watcher-start event. The dynamic fixture still
  // has different bridge lowering and exercises fewer raw offers.
  run op62MatchedRecoveryFixtureDoesNotQualifyLinkedBridgeCostTest={
    val fixture={stack:64,attempts:1}
    val possibleImage={stack:48,attempts:3}
    val evidence={cost:"unknown",availability:"qualified-envelope",
      fairness:"qualified-progress",retirementTail:"included",sourceMatched:false,
      readPrivateInbox:false,rewoundCursor:false}
    assert(fixture.stack!=possibleImage.stack and
      fixture.attempts<possibleImage.attempts and not(gate::evidenceQualified(evidence)))
  }

  // Disjoint monotonic heartbeat intervals bound raw bridge attempts
  // by elapsed time divided by 4 ms, not by the one accepted cut.
  run op62ClippedGapAttemptsAreChargedBeforeOneAcceptedCutTest={
    val intervals=List(
      {opening:1000000000,closing:1004000000,due:1003999999},
      {opening:1004000000,closing:1008000000,due:1007999999},
      {opening:1008000000,closing:1012000000,due:1008000000})
    val attempts=Set(intervals[0],intervals[1],intervals[2]).filter(i =>
      op62OwnerClosedGapStatus(i.opening,i.closing,999999999,true,false)=="closed-gap")
    val accepted=attempts.filter(i => i.closing-op62Later(i.opening,i.due)>=4000000)
    assert(intervals[0].closing==intervals[1].opening and
      intervals[1].closing==intervals[2].opening and attempts.size()==3 and
      accepted==Set(intervals[2]) and
      attempts.size()*4000000==intervals[2].closing-intervals[0].opening)
  }

  // The private scheduled invocation resets the event word, admits the
  // borrowed clock and root, and publishes only the successor event.
  // Observer-only sample stores cannot start a previously inactive watcher.
  run op62SiblingAdmissionDoesNotPublishWatcherStartTest={
    val scopeEvents=List(0,0,3,0)
    val fullDispatcherEvents=Set(0,1,2,3)
    assert(scopeEvents[2]==3 and fullDispatcherEvents.contains(2) and
      scopeEvents.foldl(false,(started,event) => started or event==2)==false)
  }

  // Both complete isolated images ran the same forced clock contract.
  // The first raw offer clipped to 1 ms; only the final offer staged
  // a receipt. A one-shot reservation at raw entry loses that receipt.
  run op62FullImageFirstRawAttemptWouldLoseSelectedReceiptTest={
    val offers=List({ordinal:1,eligibleNs:1000000},{ordinal:2,eligibleNs:7315959})
    val bridgeCallAt=offers[0].ordinal
    val stageAt=offers[1].ordinal
    assert(offers[0].eligibleNs<4000000 and offers[1].eligibleNs>=4000000 and
      bridgeCallAt!=stageAt)
  }

  // Matching complete images closes the ABI difference for this
  // forced terminal case, with no same-lane peer or natural envelope.
  run op62FullImageForcedTerminalRecoveryIsNotNaturalCostUpperTest={
    val control={inner:1334,outer:109459}
    val candidate={inner:7083,outer:245083}
    val evidence={cost:"unknown",availability:"unknown",fairness:"unknown",
      retirementTail:"included",sourceMatched:true,readPrivateInbox:false,rewoundCursor:false}
    assert(control.inner<=control.outer and candidate.inner<=candidate.outer and
      candidate.inner-control.inner==5749 and candidate.outer-control.outer==135624 and
      not(gate::evidenceQualified(evidence)))
  }

  // Deferred projection is scalar-only. A private reader takes it
  // after return while both input and result roots still belong to
  // the same owner. Entry invalidation discards any earlier receipt.
  run op62DeferredReceiptNeedsOwnerAndBothLiveRootTokensTest={
    val stored={owner:11,input:7,result:8}
    val stale={...stored,result:0}
    val subsequent={owner:11,input:10,result:11}
    val callers=Set(stored,{...stored,owner:12},{...stored,input:9},
      {...stored,result:9},stale,subsequent)
    pure def readable(current:{owner:int,input:int,result:int},
        request:{owner:int,input:int,result:int}):bool =
      current.result>0 and current==request
    assert(callers.filter(k=>readable(stored,k))==Set(stored) and
      not(readable(stale,stored)) and not(readable(subsequent,stored)) and readable(subsequent,subsequent))
  }

  // The real VM reader checks both live root tokens after C return.
  // Deliberate foreign-owner and replay checks are outside that bracket.
  run op62DeferredVmTakeChecksAreOutsideRecoveryBracketTest={
    val trace={retired:3,rejected:0,inner:5917,wrapperReturn:269000,readerReturn:459792}
    val scope={clipped:true,selected:2,taken:true,foreignRejected:true,replayRejected:true}
    val evidence={cost:"unknown",availability:"unknown",fairness:"unknown",
      retirementTail:"included",sourceMatched:true,readPrivateInbox:false,rewoundCursor:false}
    assert(trace.retired==3 and trace.rejected==0 and trace.inner<=trace.wrapperReturn and
      trace.readerReturn-trace.wrapperReturn==190792 and scope.clipped and scope.selected==2 and
      scope.taken and scope.foreignRejected and scope.replayRejected and
      not(gate::evidenceQualified(evidence)))
  }

  // The shutdown status callback is still inside the rooted wrapper.
  // Correct owner and root tokens used to spend the staged receipt
  // there. Active-call exclusion leaves it readable after return.
  run op62DeferredReceiptCannotBeConsumedDuringWrapperReportTest={
    val report={active:true,ownerMatches:true,tokensMatch:true,pending:true}
    val returned={...report,active:false}
    pure def mayTake(state:{active:bool,ownerMatches:bool,tokensMatch:bool,pending:bool},
        fence:bool):bool =
      state.ownerMatches and state.tokensMatch and state.pending and
      (not(fence) or not(state.active))
    assert(mayTake(report,false) and not(mayTake({...returned,pending:false},false)) and
      not(mayTake(report,true)) and mayTake(returned,true) and
      not(mayTake({...returned,pending:false},true)))
  }

  // The full image accepts the five-request header and preserves the
  // receipt after three clipped offers. Ordinals sharing a route are
  // insufficient when the selected cut contains no overlapping peer.
  run op62FiveRequestAdmissionDoesNotImplyLiveSameLanePeerTest={
    val admitted=Set(0,1,2,3,4)
    val clipped=Set(1,2,3)
    val staged=Set({ordinal:4,phase:"held"},{ordinal:-1,phase:"free"})
    assert(admitted.contains(0) and admitted.contains(4) and 0%4==4%4 and
      clipped.size()==3 and staged.filter(frame=>frame.phase=="ready").size()==0 and
      not(staged.exists(frame=>frame.ordinal==0)))
  }

  // The linked Actor qualification fixture forces this live overlap
  // while retaining the ordinary public Mailbox and typed oracle.
  // Its synthetic clock and scheduling gates supply no cost bound.
  run op62FullActorSameLaneFixturePreservesVerifiedReturnOrderTest={
    val key={invocation:1,owner:0,generation:1,gap:3,ordinal:0}
    val target={key:key,slot:0,phase:"ready"}
    val later={key:{...key,ordinal:4},slot:2,phase:"running"}
    val held={key:{...key,ordinal:3},slot:1,phase:"dequeued"}
    val retired=List(1,2,3,0,4)
    val evidence={cost:"unknown",availability:"unknown",fairness:"unknown",
      retirementTail:"included",sourceMatched:false,readPrivateInbox:false,rewoundCursor:false}
    assert(op62SameLaneSiblingProvesQueued(target,later,held,true,true) and
      retired[2]==held.key.ordinal and retired[3]==target.key.ordinal and
      retired[4]==later.key.ordinal and not(gate::evidenceQualified(evidence)))
  }

  // One linked, controlled Actor invocation projects the live siblings
  // and the exact next verified progress. The fixture independently
  // checks the original slot was free before the final progress.
  run op62FullActorDeferredReceiptBindsSuccessorAndReturnTest={
    val key={invocation:1,owner:0,generation:1,gap:3,ordinal:0}
    val target={key:key,slot:0,phase:"ready"}
    val later={key:{...key,ordinal:4},slot:2,phase:"running"}
    val held={key:{...key,ordinal:3},slot:1,phase:"dequeued"}
    val times={opening:805001000,closing:810000000,successor:810001000,retirementUpper:810002000}
    val evidence={cost:"unknown",availability:"unknown",fairness:"unknown",
      retirementTail:"included",sourceMatched:true,readPrivateInbox:false,rewoundCursor:false}
    assert(op62SameLaneSiblingProvesQueued(target,later,held,true,true) and
      times.opening<times.closing and times.closing<times.successor and
      times.successor<times.retirementUpper and not(gate::evidenceQualified(evidence)))
  }

  // Both fully linked arms execute the same controlled Actor schedule.
  // A lower observed candidate interval still does not bound the
  // bridge under natural recovery or repeated clipped offers.
  run op62FullActorMatchedRecoveryDoesNotQualifyNaturalUpperTest={
    val control={nextNs:70000,returnNs:141958,wrapperNs:12480250,cpuReturnNs:141292}
    val candidate={nextNs:46167,returnNs:92000,wrapperNs:9390000,cpuReturnNs:91083}
    val evidence={cost:"unknown",availability:"unknown",fairness:"unknown",
      retirementTail:"included",sourceMatched:true,readPrivateInbox:false,rewoundCursor:false}
    assert(control.nextNs<=control.returnNs and control.returnNs<=control.wrapperNs and
      candidate.nextNs<=candidate.returnNs and candidate.returnNs<=candidate.wrapperNs and
      candidate.returnNs<control.returnNs and candidate.cpuReturnNs<control.cpuReturnNs and
      not(gate::evidenceQualified(evidence)))
  }

  // Direct cold-bridge wall brackets in the complete five-request VM.
  // Four distinct raw offers are charged even though only one stages.
  run op62FullImageClippedBridgeBracketsDoNotQualifyNaturalBudgetTest={
    val clipped=List(416,375,375)
    val stagedNs=791
    val totalNs=clipped.foldl(stagedNs,(sum,n)=>sum+n)
    val evidence={cost:"unknown",availability:"unknown",fairness:"unknown",
      retirementTail:"included",sourceMatched:true,readPrivateInbox:false,rewoundCursor:false}
    assert(clipped.length()+1==4 and totalNs==1957 and totalNs>stagedNs and
      // Arrival phases restart for disjoint pauses; summing wall time
      // before rounding undercounts the worst alignment.
      budget::ceilDiv(totalNs,1000)==2 and budget::ceilDiv(totalNs,200)==10 and
      clipped.foldl(budget::ceilDiv(stagedNs,1000),(sum,n)=>sum+budget::ceilDiv(n,1000))==4 and
      clipped.foldl(budget::ceilDiv(stagedNs,200),(sum,n)=>sum+budget::ceilDiv(n,200))==11 and
      not(gate::evidenceQualified(evidence)))
  }

  // In a complete ordinary run each over-window drive has its own
  // verified retirement index. The final limit indices cannot breach
  // even if all planned arrivals are due. They cannot be used as
  // startup headroom or as separators between breach runs.
  pure def op62NonBreachTurnsBeforeTerminalTail(total:int,above:int,limit:int):int =
    total-above-limit

  run op62TerminalDrainTightensFirstBreachAndFullRefillBoundsTest={
    val limit=128
    val diagnostic={total:1000000,above:999845,period:1000,peak:27769}
    val historic={total:5000000,above:4999857,period:200,peak:4000404}
    val prefix=op62NonBreachTurnsBeforeTerminalTail(diagnostic.total,diagnostic.above,limit)
    val largePrefix=op62NonBreachTurnsBeforeTerminalTail(historic.total,historic.above,limit)
    assert((diagnostic.total-limit).to(diagnostic.total-1).forall(i=>diagnostic.total-i<=limit) and
      prefix==27 and largePrefix==15 and (prefix-1+limit)*diagnostic.period==154000 and
      (prefix+diagnostic.peak)*diagnostic.period==27796000 and
      (largePrefix-1+limit)*historic.period==28400 and
      diagnostic.above-(prefix+1)==999817 and historic.above-(largePrefix+1)==4999841 and
      budget::ceilDiv(diagnostic.above,prefix+1)==35709 and
      budget::ceilDiv(historic.above,largePrefix+1)==312492)
  }

  // The executable drive/retirement replay produces the same
  // terminal counters and latency quantiles for all three schedules.
  // A later selected gap need not be the first debt breach.
  run op62SameArchivedSummaryAllowsEarlyLateOrNoLongGapTest={
    val summary={completed:1000000,above:999845,peak:27769,source:27767,
      p99:27602000,p999:27745000,elapsed:1017108875}
    val short={ledger:summary,first:27,longAt:-1,gap:128099}
    val early={...short,longAt:27,gap:4173099}
    val late={...short,longAt:10000,gap:4100000}
    assert(short.ledger==early.ledger and short.ledger==late.ledger and
      short.gap<4000000 and early.gap>=4000000 and late.gap>=4000000 and
      early.longAt==early.first and late.longAt>late.first)
  }

  // The fixed owner may encode logical zero as the maximum fixnum
  // only in an isolated copy. The existing overflow edge consumes
  // it once. Both terminal projections must decode an unspent arm.
  run op62OverflowArmUsesExistingColdEdgeExactlyOnceTest={
    val sentinel=1152921504606846975
    pure def step(count:int,outstanding:int):int =
      if(outstanding<=128) count else if(count==sentinel) 1 else count+1
    pure def project(count:int):int = if(count==sentinel) 0 else count
    val first=step(sentinel,129)
    val subsequent=step(first,130)
    assert(5000000*10<sentinel and step(sentinel,127)==sentinel and
      step(sentinel,128)==sentinel and project(step(sentinel,128))==0 and
      first==1 and subsequent==2 and project(first)==1 and project(subsequent)==2 and
      step(0,129)==1)
  }

  // The sampled due frontier brackets the original source clock
  // only while unsaturated. The wall clock need not be read again.
  run op62FirstOverflowCanUseDueBucketWithoutNewClockTest={
    val due=156
    val period=1000
    val total=1000000
    val lower=(due-1)*period
    val upper=due*period
    pure def dueAt(elapsed:int):int =
      if(1+elapsed/period<total) 1+elapsed/period else total
    assert(due<total and lower==155000 and upper==156000 and
      155000.to(155999).forall(t=>dueAt(t)==due) and dueAt(upper)==due+1 and
      dueAt((total-1)*period)==total and dueAt((total+4)*period)==total)
  }

  // The full Actor object preserves the ordinary instructions, but
  // its unmodified overflow successor computes an exact integer.
  // Decoding the initial value at the output boundary cannot fix it.
  run op62OverflowSeedAloneFailsFirstBreachSettlementTest={
    val sentinel=1152921504606846975
    pure def project(count:int):int = if(count==sentinel) 0 else count
    val originalOverflow=sentinel+1
    assert(project(sentinel)==0 and project(originalOverflow)!=1 and
      project(originalOverflow+1)!=2)
  }

  // A replayable Scheme counter and the owner generation's right to
  // publish are separate. An old frame cannot spend a new generation.
  run op62OverflowReplayNeedsUnspentOwnerGenerationTest={
    val armed={owner:1,generation:8,pending:true}
    val first={owner:1,generation:8}
    val stale={owner:1,generation:7}
    val foreign={owner:2,generation:8}
    pure def mayPublish(
      permit:{owner:int,generation:int,pending:bool},
      frame:{owner:int,generation:int}
    ):bool =
      permit.pending and permit.owner==frame.owner and
        permit.generation==frame.generation
    val spent={...armed,pending:false}
    val newer={...armed,generation:9}
    assert(mayPublish(armed,first) and not(mayPublish(spent,first)) and
      not(mayPublish(armed,stale)) and not(mayPublish(armed,foreign)) and
      not(mayPublish(newer,first)) and mayPublish(newer,{owner:1,generation:9}))
  }

  // The overflow successor can reuse the already committed peak to
  // reconstruct the current prefix only at the first threshold crossing.
  run op62FirstOverflowPeakReconstructsPresupplyPrefixesTest={
    val cut={due:156,debt:128,outstanding:129,previousPeak:128}
    val repeat={...cut,due:158,previousPeak:133}
    pure def peak(r:{due:int,debt:int,outstanding:int,previousPeak:int}):int =
      if(r.outstanding>r.previousPeak) r.outstanding else r.previousPeak
    val published=cut.due-cut.debt
    val retired=cut.due-peak(cut)
    assert(cut.previousPeak<=128 and cut.outstanding>128 and
      peak(cut)==cut.outstanding and published==28 and retired==27 and
      published-retired==1 and
      repeat.due-peak(repeat)!=repeat.due-repeat.outstanding)
  }

  // Matching owner, generation and ordinal is necessary, but a
  // later ready witness alone does not establish readiness at breach.
  run op62LaterReadyWitnessDoesNotBackdateFirstOverflowCustodyTest={
    val breach={lo:155000,hi:156000,due:156,published:28,retired:27}
    val observed={owner:1,generation:8,ordinal:27,publishedAt:154000,
      readyBy:158000,dequeuedAt:160000,retiredAt:161000}
    val early=154800
    val late=156200
    pure def fits(enqueued:int):bool =
      observed.publishedAt<enqueued and enqueued<=observed.readyBy and
        observed.readyBy<observed.dequeuedAt and observed.dequeuedAt<observed.retiredAt
    assert(fits(early) and fits(late) and
      early<breach.lo and breach.hi<=late and
      breach.due-breach.retired==129 and breach.published-breach.retired==1)
  }

  // The complete-VM fault occurs after the latency histogram write
  // and before verified progress returns frame custody. An encoded
  // diagnostic count also needs explicit decoding on both outputs.
  run op62OverflowVmFaultDoesNotRetireTimedReplyOrNormalizeRawCountTest={
    val sentinel=1152921504606846975
    val noBreach={raw:sentinel,logical:0,samples:1,retired:0}
    val breached={raw:1,logical:1,samples:1,retired:0}
    val recovered={raw:sentinel,logical:0,samples:3,retired:3}
    assert(noBreach.samples>noBreach.retired and breached.samples>breached.retired and
      noBreach.raw!=noBreach.logical and breached.raw==breached.logical and
      recovered.retired==recovered.samples and recovered.raw!=recovered.logical)
  }

  // Reentry after staging must invalidate the pending outer publication.
  // Clearing sealed alone is undone if the outer return reads staged.
  run op62FirstCutNestedEntryCannotResealOuterReceiptTest={
    val staged={generation:8,staged:true,pending:false,sealed:false,cancelled:false}
    val weakNested={...staged,sealed:false}
    val invalidated={...staged,staged:false,pending:false,sealed:false,cancelled:true}
    pure def maySeal(generation:int,s:{generation:int,staged:bool,pending:bool,sealed:bool,cancelled:bool}):bool =
      s.generation==generation and s.staged and not(s.cancelled)
    assert(weakNested.staged and maySeal(8,staged) and
      not(maySeal(8,invalidated)) and not(maySeal(7,staged)) and
      not(maySeal(8,{...staged,generation:9})))
  }

  // The complete-VM in-flight cut distinguishes source debt from
  // occupied frames. Ready business values supply no inbox timestamp.
  run op62FirstCutSeparatesSourceDebtAndReadyFrameCustodyTest={
    val cut={due:132,published:3,retired:1}
    val source=cut.due-cut.published
    val active=cut.published-cut.retired
    val outstanding=cut.due-cut.retired
    val frames=Set({id:-1,phase:"free"},{id:1,phase:"ready"},{id:2,phase:"ready"})
    val occupied=frames.filter(f=>f.phase!="free")
    val finalRetired=132
    assert(source==129 and active==2 and outstanding==131 and
      source+active==outstanding and occupied.size()==active and
      occupied.forall(f=>f.id>=cut.retired and f.id<cut.published) and
      finalRetired>cut.retired and frames.contains({id:-1,phase:"free"}))
  }

  // A second existing overflow entry may certify the immediately
  // following consumption only if the retired prefix advances by one
  // and a previously occupied slot is free before any refill.
  run op62SecondOverflowNeedsOneActualReturnAndSameSlotTest={
    val before={generation:8,published:3,retired:1,ids:List(-1,1,2)}
    val after={generation:8,published:4,retired:2,ids:List(3,-1,2)}
    val skipped={...after,retired:3}
    val other={...after,generation:9}
    pure def provesReturned(
      x:{generation:int,published:int,retired:int,ids:List[int]},
      y:{generation:int,published:int,retired:int,ids:List[int]},
      slot:int
    ):bool =
      x.generation==y.generation and y.retired==x.retired+1 and
        y.published>=x.published and y.published-x.published<=1 and
        x.ids[slot]>=0 and y.ids[slot]== -1
    pure def decoded(stage:int,raw:int):int =
      if(raw==1152921504606846975) stage else raw
    assert(provesReturned(before,after,1) and
      not(provesReturned(before,after,0)) and
      not(provesReturned(before,skipped,1)) and
      not(provesReturned(before,other,1)) and
      decoded(0,1152921504606846975)==0 and
      decoded(1,1152921504606846975)==1 and decoded(2,2)==2)
  }

  // Both zero crossings and one crossing without a second overflow
  // keep the same raw arm. The private invocation phase disambiguates it.
  run op62MissingSecondOverflowMustDecodeOneCrossingTest={
    val raw=1152921504606846975
    pure def decode(phase:int,value:int):int =
      if(value==1152921504606846975) if(phase==1) 1 else 0 else value
    assert(decode(0,raw)==0 and decode(1,raw)==1 and
      decode(2,2)==2 and decode(2,3)==3)
  }

  // An inner call cancels parent capture, but the parent's private
  // counter phase survives. Its next existing overflow still writes two.
  run op62NestedCancellationDoesNotResetParentCounterPhaseTest={
    val parent={generation:7,phase:1,observe:true}
    val cancelled={...parent,observe:false}
    val nested={generation:8,phase:0,observe:false}
    val resumed={...cancelled,phase:2}
    val lastParent={input:99,result:102,generation:7}
    pure def matches(generation:int):bool=lastParent.generation==generation
    assert(cancelled.phase==1 and not(cancelled.observe) and
      resumed.phase+1==3 and nested.phase==0 and
      matches(parent.generation) and not(matches(nested.generation)))
  }

  // The first peak is not the second cut's outstanding count.
  // Reusing it loses the newly verified return in the paired receipt.
  run op62SecondOverflowCannotReuseHistoricalPeakForRetirementTest={
    val due=132
    val firstPeak=131
    val actualRetired=2
    val currentOutstanding=due-actualRetired
    assert(currentOutstanding==130 and due-firstPeak==1 and
      due-firstPeak!=actualRetired and due-currentOutstanding==actualRetired)
  }

  // Public receive and original frame return lie after the first
  // pre-supply clock and before the next one. A send may have occurred
  // on either side of the opening cut.
  run op62OriginalClockBoundsDoNotBackdateEnqueueTest={
    val first=1000000
    val second=1001000
    val planned=900000
    val early={enqueue:999900,dequeue:1000500,retire:1000800}
    val late={enqueue:1000200,dequeue:1000500,retire:1000800}
    assert(Set(early,late).forall(r=>
      planned<=r.enqueue and r.enqueue<=r.dequeue and
      first<=r.dequeue and r.dequeue<=r.retire and r.retire<=second) and
      early.enqueue<first and late.enqueue>first)
  }

  // Roots held for the whole call may add collection work outside
  // the measured cuts. Selected roots must live inside the enclosing
  // physical interval for that interval to cover their trace work.
  run op62WholeCallRootsEscapeSelectedCostWindowTest={
    val opening=0
    val closing=10
    pure def live(begin:int,end:int,t:int):bool=begin<=t and t<end
    val gcCuts=Set(-1,1,5,9,11)
    assert(live(-2,20,-1) and not(opening<= -1) and live(-2,20,11) and
      gcCuts.forall(t=>not(live(2,8,t)) or (opening<=t and t<=closing)) and
      live(2,8,5) and not(live(2,8,11)))
  }

  // First binding is authorized by the frozen rooted entry path,
  // not merely by a header that resembles an op62 command.
  run op62HeaderShapeAloneCannotAuthorizeFirstRootBindingTest={
    pure def allowed(privateEntry:bool,activeScope:bool,sameProcessor:bool,
      rootedArgument:bool,header:bool):bool =
      privateEntry and activeScope and sameProcessor and rootedArgument and header
    assert(allowed(true,true,true,true,true) and
      not(allowed(false,true,true,true,true)) and
      not(allowed(true,true,true,false,true)) and
      not(allowed(true,false,true,true,true)))
  }

  // The controlled source clock and the physical cost brackets
  // share an ordering scenario but are different numeric domains.
  run op62ControlledClockSpanIsNotPhysicalObserverCostTest={
    val logical={first:1000000000,second:1000001000}
    val physical={first:0,firstPost:4667,second:5208,secondPost:6250}
    assert(logical.second-logical.first==1000 and
      physical.secondPost-physical.first==6250 and
      (physical.firstPost-physical.first)+(physical.second-physical.firstPost)+
        (physical.secondPost-physical.second)==6250 and
      logical.second-logical.first!=physical.secondPost-physical.first)
  }

  // The typed projection reserves consumption before creating its result.
  // A factory failure restores permission only for the same generation.
  run op62FailedTypedProjectionCanRetryOnlyItsOwnGenerationTest={
    val initial={generation:4,reserved:0,sealed:true,roots:0}
    val reserved={...initial,reserved:4}
    val failed={...reserved,reserved:0}
    val retry={...failed,reserved:4,roots:1,sealed:false}
    assert(failed==initial and retry.roots==initial.roots+1 and
      retry.reserved==retry.generation and not(retry.sealed))
  }

  // Output allocation may reenter the same owner. Discard the stale output
  // root without clearing a newer invocation's sealed receipt or permission.
  run op62ReenteredTypedProjectionMustReleaseOnlyItsStaleRootTest={
    val old={generation:7,reserved:7,roots:0}
    val projected={...old,roots:1}
    val newer={generation:8,reserved:0,sealed:true,roots:projected.roots}
    val discarded={...newer,roots:newer.roots-1}
    val wrongReset={...discarded,sealed:false}
    assert(discarded.generation==8 and discarded.reserved==0 and
      discarded.sealed and discarded.roots==old.roots and not(wrongReset.sealed))
  }

  // Native op11 publishes verified progress before the Scheme frame is freed.
  // The atomic word alone therefore never authorizes capacity reuse.
  run op62VerifiedProgressDoesNotAuthorizeFrameReuseTest={
    val ordered={validate:10,progress:11,clearResult:12,free:13,retire:14}
    pure def reusable(at:int):bool=at>=ordered.retire
    assert(ordered.validate<ordered.progress and ordered.progress<ordered.free and
      not(reusable(ordered.progress)) and reusable(ordered.retire))
  }

  // An initialization timestamp can precede the source clock. Clip only
  // the source interval; do not pretend the initial stamp is a consumption.
  run op62InitialProgressStampIsNotAnEarlierSourceConsumptionTest={
    val epoch=1000
    val sample=1400
    pure def sourceSpan(last:int):int=sample-(if(last>epoch) last else epoch)
    assert(sourceSpan(900)==400 and sample-900==500 and
      sourceSpan(1200)==200 and sourceSpan(900)!=sample-900)
  }

  // Same-invocation natural capture. Both ready results precede the selected
  // return, and one frame is freed between the two original clock samples.
  // This is a local progress fact, not attribution of the later p99 or peak.
  run op62FirstNaturalBreachCanMakeProgressWhileSourceDebtGrowsTest={
    val a={due:492,published:364,retired:362,active:2,clock:491250,verified:491209}
    val b={due:495,published:365,retired:363,active:2,clock:494084,verified:494042}
    assert(a.due-a.retired==130 and a.due-a.published==128 and
      a.active==a.published-a.retired and b.active==b.published-b.retired and
      b.retired==a.retired+1 and b.clock-a.clock==2834 and
      a.clock-a.verified==41 and b.due-b.published>a.due-a.published and
      235962>a.due-a.retired)
  }

  // Extending the private record changes its word count. A size-only parser
  // or a version-only parser accepts incompatible projections.
  run op62TypedReportRequiresMatchingVersionAndShapeTest={
    pure def accepts(version:int,words:int):bool=version==2 and words==43
    assert(accepts(2,43) and not(accepts(1,41)) and
      not(accepts(2,41)) and not(accepts(1,43)))
  }

  // Complete, zero-rejection natural diagnostics still fail both budgets.
  // Root cleanup and the small selected bracket cannot waive the service gate.
  run op62NaturalCompletionCannotWaiveLatencyOrBacklogAcceptanceTest={
    val receipt={planned:1000000,completed:1000000,rejected:0,p99:292061000,
      peak:235962,whole:1334709750,clean:true}
    pure def accepted(p99:int,peak:int):bool=
      receipt.completed==receipt.planned and receipt.rejected==0 and
      receipt.whole<=3000000000 and p99<1000000 and peak<=128
    assert(receipt.clean and not(accepted(receipt.p99,receipt.peak)) and
      not(accepted(999999,receipt.peak)) and not(accepted(receipt.p99,128)))
  }

  run op62FixedClockKeepsProviderArgumentsTest=assert(clockAbi::constantCallKeepsProviderArguments)
  run op62ClockSpecializationKeepsGenericProgressAndErrorsTest=assert(clockAbi::otherCallsKeepGenericEntry)
  run op62ClockReentryReadsCurrentBindingTest=assert(clockAbi::reentryCannotFreezeProviderBinding)
  run op62ClockArgumentSavingsDoNotRemoveSamplesOrForeignCallsTest=assert(clockAbi::completeCycleRemovesOnlyConstantArgumentConversions)
  run op62ClockResultStillRequiresUnsignedBignumConversionTest=assert(clockAbi::unsignedRangeCannotBeNarrowed)
  run op62ClockPrototypeRequiresMatchedTrampolineLoweringTest=assert(clockAbi::mismatchedLoweringCannotDecideActorCost)
  run op62MatchingMailboxBodyDoesNotAuthenticateWorkloadSourceTest=assert(clockAbi::sameMailboxBodyDoesNotAuthenticateTheWorkload)
  run op62UnobservedProfileSelectsNativeResultConstructionTest=assert(clockAbi::unobservedOp62SelectsTheNativeConstructor)
  run op62ScopedFalseNativeAccessAssumptionIsCounterexampleTest=assert(clockAbi::scopedFalseCannotBeUsedAsTheNativeAccessValue)
  run op62NewClockGlobalRequiresMatchingNativeLinkerMetadataTest=assert(clockAbi::newClockProcedureRequiresMatchingLinkerMetadata)
  run op62WholeModuleCostIncludesPrivateProcedureStorageTest=assert(clockAbi::wholeModuleCostCannotBeReplacedByLocalStackSavings)
  run op62ClockCalibrationCannotReplaceMailboxAcceptanceTest=assert(clockAbi::clockCalibrationCannotReplaceMailboxAcceptance)
  run op62ProviderRestoration1Test=assert(clockAbi::matchingActorDoesNotAuthenticateTheCProvider)
  run op62ProviderRestoration2Test=assert(clockAbi::selectedCaptureValueCannotUseTheSuccessErrorChannel)
  run op62ProviderRestoration3Test=assert(clockAbi::originalProgressBoundaryAndPublicationOrderArePreserved)
  run op62ProviderRestoration4Test=assert(clockAbi::removedDiagnosticSourceCheckDoesNotProveCheaperUnhitMachinePath)
  run mailboxLayoutEntryCompletionTest=assert(layoutEntry::allRules)
  run mailboxHistoricalAdmissionCounterexamplesTest=assert(researchGate::allCounterexamples)
}
