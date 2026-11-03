// Result suffix only; canonical Actor custody and retirement stay unchanged.
module mailbox_result_construction {
  import mailbox_joint_cycle as j from "./mailbox_joint_cycle"
  import actor_service_contract as gate from "./actor_service_contract"
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
}
