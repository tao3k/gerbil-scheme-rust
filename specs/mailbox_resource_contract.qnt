// Claim prerequisites reuse the established owner lifecycle; no new runtime
// lifecycle or memory allocator is modeled. Evidence fields need real receipts.
module mailbox_resource_contract {
  type Cleanup={roots:int,providerRefs:int,queuedRefs:int,continuationRefs:int,
    ownerExited:bool,tlsReleased:bool,callbacksDrained:bool,error:int,images:int,
    retainedImagesExplained:bool,reuseConverged:bool,longRunChecked:bool}
  pure def reclaimed(c:Cleanup):bool=c.roots==0 and c.providerRefs==0 and c.queuedRefs==0 and
    c.ownerExited and c.tlsReleased and c.callbacksDrained and c.error==0
  pure def unloadEligible(c:Cleanup):bool=reclaimed(c) and c.continuationRefs==0
  pure def memoryClaimEligible(c:Cleanup):bool=unloadEligible(c) and c.images>=0 and
    (c.images==0 or c.retainedImagesExplained) and c.reuseConverged and c.longRunChecked
  type Build={limit:int,peak:int,hasSamples:bool,watcherHealthy:bool,treeComplete:bool,
    allChildrenReaped:bool,exit:int,boundKind:str}
  pure val originalBuildCap=8589934592
  pure def monitoredBuildCompleted(b:Build):bool=b.limit==originalBuildCap and b.peak>=0 and
    b.peak<=b.limit and b.hasSamples and b.watcherHealthy and b.treeComplete and b.allChildrenReaped and b.exit==0
  // Sampled RSS is explicitly weaker than an enforced maximum.
  pure def hardBuildBoundClaimEligible(b:Build):bool=monitoredBuildCompleted(b) and b.boundKind=="enforced-limit"
  pure def terminationClass(signal:int,watchdogConfirmed:bool,capExceeded:bool,userStopped:bool):str=
    if(userStopped) "user-stop" else if(capExceeded) "memory-cap-stop"
    else if(watchdogConfirmed) "watchdog" else "unknown"
  pure val cleanExample={roots:0,providerRefs:0,queuedRefs:0,continuationRefs:0,
    ownerExited:true,tlsReleased:true,callbacksDrained:true,error:0,images:0,
    retainedImagesExplained:false,reuseConverged:true,longRunChecked:true}
  pure val buildExample={limit:8589934592,peak:6000000000,hasSamples:true,watcherHealthy:true,
    treeComplete:true,allChildrenReaped:true,exit:0,boundKind:"sampled-only"}
  pure val zeroRootsAndExitedOwnerDoNotWaiveCleanupError=
    not(reclaimed({...cleanExample,error:-73}))
  pure val callbacksTlsAndProviderRefsIndependentlyBlockReclamation=
    not(reclaimed({...cleanExample,tlsReleased:false})) and
    not(reclaimed({...cleanExample,callbacksDrained:false})) and
    not(reclaimed({...cleanExample,providerRefs:1})) and not(reclaimed({...cleanExample,queuedRefs:1}))
  pure val retainedContinuationBlocksUnload=
    reclaimed({...cleanExample,continuationRefs:1}) and not(unloadEligible({...cleanExample,continuationRefs:1}))
  pure val retainedImagesRequireClassificationNotAutomaticLeakOrPass=
    not(memoryClaimEligible({...cleanExample,images:32})) and
    memoryClaimEligible({...cleanExample,images:32,retainedImagesExplained:true})
  pure val aShortReuseCheckDoesNotProveLongTermConvergence=
    not(memoryClaimEligible({...cleanExample,reuseConverged:false})) and
    not(memoryClaimEligible({...cleanExample,longRunChecked:false}))
  pure val capCannotBeIncreasedToPassTheBuild=
    not(monitoredBuildCompleted({...buildExample,limit:17179869184})) and
    not(monitoredBuildCompleted({...buildExample,peak:8589934593}))
  pure val parentOnlyOrFailedMonitorIsNotTreeEvidence=
    not(monitoredBuildCompleted({...buildExample,treeComplete:false})) and
    not(monitoredBuildCompleted({...buildExample,watcherHealthy:false})) and
    not(monitoredBuildCompleted({...buildExample,allChildrenReaped:false})) and
    not(monitoredBuildCompleted({...buildExample,hasSamples:false}))
  pure val aSampledPeakIsNotAHardMemoryBound=
    monitoredBuildCompleted(buildExample) and not(hardBuildBoundClaimEligible(buildExample)) and
    hardBuildBoundClaimEligible({...buildExample,boundKind:"enforced-limit"})
  pure val aKillSignalAloneDoesNotEstablishOom=
    terminationClass(9,false,false,false)=="unknown" and
    terminationClass(9,false,false,true)=="user-stop" and
    terminationClass(9,false,true,false)=="memory-cap-stop" and
    terminationClass(70,true,false,false)=="watchdog"
  // Installed std/make gerbil-build-cores retains an explicit integer.
  // Only the #t policy reads GERBIL_BUILD_CORES. Effective serial means one.
  // These are construction-policy facts, not Scheme runtime Actor counts.
  pure def producerBuildWorkers(explicit:bool,declared:int,environment:int):int={
    val n=if(explicit) declared else environment
    if(n>1) n else 1
  }
  pure val cargoAndArchiveCapsDoNotOverrideExplicitProducer=
    producerBuildWorkers(true,12,1)==12 and
    producerBuildWorkers(false,12,1)==1 and
    producerBuildWorkers(true,0,12)==1
  // A controlled counterexample, not an attribution of any old observed OOM.
  // An owned child can have another process group; grouping is not ancestry.
  pure val oneProcessGroupPeakCannotAuthenticateTheWholeBuildTree={
    val groupA=6*1073741824
    val groupB=3*1073741824
    val cap=8*1073741824
    groupA<cap and groupB<cap and groupA+groupB>cap and
      not(monitoredBuildCompleted({...buildExample,peak:groupA,treeComplete:false}))
  }
  pure val resourceChecks=and {cargoAndArchiveCapsDoNotOverrideExplicitProducer,
    oneProcessGroupPeakCannotAuthenticateTheWholeBuildTree,zeroRootsAndExitedOwnerDoNotWaiveCleanupError,
    callbacksTlsAndProviderRefsIndependentlyBlockReclamation,retainedContinuationBlocksUnload,
    retainedImagesRequireClassificationNotAutomaticLeakOrPass,aShortReuseCheckDoesNotProveLongTermConvergence,
    capCannotBeIncreasedToPassTheBuild,parentOnlyOrFailedMonitorIsNotTreeEvidence,
    aSampledPeakIsNotAHardMemoryBound,aKillSignalAloneDoesNotEstablishOom}
}
module mailbox_resource_tests {
  import mailbox_resource_contract.*
  run explicitProducerConcurrencyTest=assert(cargoAndArchiveCapsDoNotOverrideExplicitProducer)
  run processGroupIsNotBuildTreeTest=assert(oneProcessGroupPeakCannotAuthenticateTheWholeBuildTree)
  run cleanupErrorTest=assert(zeroRootsAndExitedOwnerDoNotWaiveCleanupError)
  run callbackTlsProviderTest=assert(callbacksTlsAndProviderRefsIndependentlyBlockReclamation)
  run continuationUnloadTest=assert(retainedContinuationBlocksUnload)
  run retainedImagesTest=assert(retainedImagesRequireClassificationNotAutomaticLeakOrPass)
  run convergenceTest=assert(aShortReuseCheckDoesNotProveLongTermConvergence)
  run originalBuildCapTest=assert(capCannotBeIncreasedToPassTheBuild)
  run wholeTreeMonitorTest=assert(parentOnlyOrFailedMonitorIsNotTreeEvidence)
  run sampledMemoryTest=assert(aSampledPeakIsNotAHardMemoryBound)
  run killAttributionTest=assert(aKillSignalAloneDoesNotEstablishOom)
}
