// Compile-time call-shape study only. Native Mailbox custody, scheduling and
// frame return remain owned by mailbox_joint_cycle; this module adds no queue.
module mailbox_clock_abi {
  pure def fixedClockCall(operation:int,value:int):bool=operation==1 and value==0
  pure def plan(operation:int,value:int):{arity:int,providerOperation:int,providerValue:int}=
    if(fixedClockCall(operation,value)) {arity:0,providerOperation:1,providerValue:0}
    else {arity:2,providerOperation:operation,providerValue:value}

  pure val constantCallKeepsProviderArguments=
    plan(1,0).arity==0 and plan(1,0).providerOperation==1 and plan(1,0).providerValue==0
  // Only the literal call is eligible. Generic argument checks and progress
  // op11 remain in the original entry; this is not a new provider dispatcher.
  pure val otherCallsKeepGenericEntry=
    Set(0,2,5,8,10,11,-1).forall(op=>plan(op,0).arity==2) and
    Set(-1,1,128,1152921504606846975).forall(v=>plan(1,v).arity==2)

  // Read the current binding on every invocation, including continuation
  // reentry. Capturing an old callback while selecting a plan is not valid.
  pure val reentryCannotFreezeProviderBinding={
    val results=List(0,1152921504606846975,1152921504606846976,
      18446744073709551615,9223372036854775808,42)
    val fixed=results.foldl(List(),(items,value)=>items.append({operation:plan(1,0).providerOperation,
      argument:plan(1,0).providerValue,result:value}))
    val generic=results.foldl(List(),(items,value)=>items.append({operation:1,argument:0,result:value}))
    fixed==generic and fixed[0].result!=fixed[5].result
  }

  // The source samples once per drive, once per publication, once per
  // consumption, and at start/end. A successful drive consumes at most one.
  pure def clockCalls(messages:int,drives:int):int=2*messages+drives+2
  pure def foreignCalls(messages:int,drives:int):int=clockCalls(messages,drives)+messages
  pure val completeCycleRemovesOnlyConstantArgumentConversions=
    Set(1,132,1000000).forall(n=>0.to(3).forall(idle=>{
      val drives=n+idle
      val beforeArguments=2*clockCalls(n,drives)
      val afterArguments=0
      beforeArguments-afterArguments==6*n+2*idle+4 and
        foreignCalls(n,drives)==4*n+idle+2
    })) and clockCalls(132,132)==398

  // Keep the same unsigned result conversion, including bignum values. A
  // signed or fixnum-only projection cannot stand in for the original ABI.
  pure val unsignedRangeCannotBeNarrowed={
    val values=Set(0,1152921504606846975,1152921504606846976,
      9223372036854775808,18446744073709551615)
    values.exists(v=>v>1152921504606846975) and
      values.exists(v=>v>9223372036854775807) and
      values.forall(v=>0<=v and v<=18446744073709551615)
  }

  // Identical source under a different trampoline configuration gave 96 vs
  // 80 bytes, while matched native lowering gave 64 vs 80. Neither local
  // comparison is the full Actor symbol or a time/throughput measurement.
  pure val mismatchedLoweringCannotDecideActorCost={
    val unmatched={generic:80,fixed:96}
    val matched={generic:80,fixed:64}
    unmatched.fixed>unmatched.generic and matched.fixed<matched.generic and
      unmatched.fixed!=matched.fixed
  }

  // Reader comparison found the same scheduled body and oracle in a retained
  // diagnostic, but its input and result construction were from another graph.
  pure val sameMailboxBodyDoesNotAuthenticateTheWorkload={
    val frozen={parent:"6991ff02",mailbox:"scheduled",oracle:"independent",
      input:"proven-typed-stores",rankOrder:"sequential-let-star",nativeResult:"guarded-atomic"}
    val legacy={...frozen,parent:"1db4faac",input:"checked-stores",
      rankOrder:"parallel-let",nativeResult:"checked-loop"}
    frozen.mailbox==legacy.mailbox and frozen.oracle==legacy.oracle and frozen!=legacy and
      frozen.parent!=legacy.parent and frozen.input!=legacy.input and
      frozen.rankOrder!=legacy.rankOrder
  }

  // Match the actual mailbox-scoped-access macro, including its unobserved
  // opcode set. scoped=false does not imply native-access=false.
  pure def sourceNativeAccess(operation:int,scoped:bool):bool=
    if(scoped) operation==64 else Set(56,57,58,59,60,62).contains(operation)
  pure val unobservedOp62SelectsTheNativeConstructor={
    val nativeAccess=sourceNativeAccess(62,false)
    val frozen={generic:"checked",native:"guarded-atomic"}
    val legacy={generic:"checked",native:"checked-loop"}
    nativeAccess and (if(nativeAccess) frozen.native else frozen.generic)!=
      (if(nativeAccess) legacy.native else legacy.generic) and
      sourceNativeAccess(64,true) and not(sourceNativeAccess(62,true))
  }
  // Retain the disproven premise, not an executable test that assumes it.
  // The idle guard starts false; dynamic-wind enables the selected profile
  // before worker execution and disables it again on normal/error exit.
  pure val scopedFalseCannotBeUsedAsTheNativeAccessValue={
    val disprovenAssumption=false
    val idle=false
    val entered=sourceNativeAccess(62,false)
    val exited=false
    entered!=disprovenAssumption and not(idle) and entered and not(exited)
  }

  // Adding the private C procedure introduced two generated global symbols.
  // Defining their names without regenerating owned linker metadata is not a
  // valid repair. The standard linker consumes the matching module interface.
  pure val newClockProcedureRequiresMatchingLinkerMetadata={
    val old=Set("generic-clock","other-existing-globals")
    val required=old.union(Set("private-clock","generated-procedure-47"))
    pure def valid(provided:Set[str],moduleInput:str,linkerInput:str):bool=
      required.subseteq(provided) and moduleInput==linkerInput
    not(valid(old,"new-module","old-module")) and
      not(valid(required,"new-module","old-module")) and
      valid(required,"new-module","new-module")
  }

  // Full lowering reduced local code and frames but increased static module
  // storage. These are separate costs and neither is a request-latency result.
  pure val wholeModuleCostCannotBeReplacedByLocalStackSavings={
    val old={scheduledWords:5620,clockFrame:80,text:702132,data:431688,bss:3112}
    val replacement={scheduledWords:5608,clockFrame:48,text:702560,data:431816,bss:3120}
    replacement.scheduledWords<old.scheduledWords and replacement.clockFrame<old.clockFrame and
      replacement.text-old.text==428 and replacement.data-old.data==128 and
      replacement.bss-old.bss==8
  }
  // The single source-matched pair completes every offered message and exits
  // cleanly. Local ABI savings and improved relative tails still do not
  // discharge the independent absolute Mailbox latency and backlog gates.
  pure val clockCalibrationCannotReplaceMailboxAcceptance={
    pure def admitted(done:int,rejected:int,p99:int,peak:int,whole:int):bool=
      done==1000000 and rejected==0 and whole<=3000000000 and
        p99>0 and p99<=418000 and peak<=128
    val control={done:1000000,rejected:0,p99:164041000,peak:77695,whole:1182591250}
    val candidate={done:1000000,rejected:0,p99:107153000,peak:84335,whole:1120632917}
    candidate.p99<control.p99 and candidate.whole<control.whole and
      candidate.peak>control.peak and
      not(admitted(control.done,control.rejected,control.p99,control.peak,control.whole)) and
      not(admitted(candidate.done,candidate.rejected,candidate.p99,candidate.peak,candidate.whole))
  }

  // The linked provider is part of the measured cohort too. The source-
  // matched Actor alone cannot authenticate the original progress callback.
  pure val matchingActorDoesNotAuthenticateTheCProvider={
    val frozen={actor:"6991ff02",provider:"original-case11"}
    val diagnostic={...frozen,provider:"four-ms-capture-case11"}
    frozen.actor==diagnostic.actor and frozen.provider!=diagnostic.provider
  }

  type ProgressOutcome={result:int,events:List[str]}
  pure def providerProgress(delta:int,selected:bool,diagnostic:bool):ProgressOutcome=
    if(delta>=10000000000) {result:-1,events:List("clock","load")}
    else {result:if(diagnostic and selected and delta>=4000000) 1 else 0,
      events:List("clock","load","publish","callback")}

  // The original Scheme caller maps ANY nonzero result to progress failure.
  // This is a conditional compatibility counterexample, not a claim that an
  // historical instrumented caller used that unchanged predicate.
  pure val selectedCaptureValueCannotUseTheSuccessErrorChannel={
    val original=providerProgress(5000000,true,false)
    val selected=providerProgress(5000000,true,true)
    original.result==0 and selected.result!=0 and
      selected.events==original.events and selected.events==List("clock","load","publish","callback")
  }

  pure val originalProgressBoundaryAndPublicationOrderArePreserved=
    Set(0,1,3999999,4000000,5000000,9999999999,10000000000,10000000001).forall(delta=>{
      val original=providerProgress(delta,true,false)
      val unarmed=providerProgress(delta,false,true)
      original==unarmed and
        (if(delta<10000000000) original.result==0 and
          original.events==List("clock","load","publish","callback")
         else original.result==-1 and original.events==List("clock","load"))
    })

  // GCC already moves the second deadline comparison to the cold arm. The
  // source restoration keeps one ordinary-path comparison but materializes
  // the wider ten-second constant with one additional instruction.
  pure val removedDiagnosticSourceCheckDoesNotProveCheaperUnhitMachinePath={
    val diagnostic={ordinaryComparisons:1,thresholdMaterialization:2,cFrame:64}
    val original={ordinaryComparisons:1,thresholdMaterialization:3,cFrame:64}
    original.ordinaryComparisons==diagnostic.ordinaryComparisons and
      original.thresholdMaterialization==diagnostic.thresholdMaterialization+1 and
      original.cFrame==diagnostic.cFrame
  }

}
