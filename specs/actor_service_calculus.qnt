// Pure service-budget algebra. Units are abstract ticks, not measured CPU ns.
module actor_service_calculus {
  type Cost = { source:int,construct:int,send:int,execute:int,verify:int,consume:int,schedule:int,observe:int }
  pure def validCost(c:Cost):bool = c.source>=0 and c.construct>=0 and c.send>=0 and
    c.execute>0 and c.verify>=0 and c.consume>=0 and c.schedule>=0 and c.observe>=0
  // One processor: sum the phases. Logical transfers do not determine actual
  // green dispatches. Retained cycleCost explicitly assumes two of each.
  pure def wholeCycleCost(c:Cost,messages:int,sends:int,dispatches:int,extra:int):int=
    messages*(c.source+c.construct+c.execute+c.verify+c.consume+c.observe)+
      sends*c.send+dispatches*c.schedule+extra
  // Counts in the successful unobserved op62 driver, not a CPU-time price.
  // Each verified message has one publication, latency, and fresh progress
  // read; one successful drive reads source time. Idle drives add one read.
  pure def driverClockCalls(messages:int,idleDrives:int):int=4*messages+idleDrives+2
  // Projection of the successful original op62 supply transaction, not a
  // second scheduler. Caller proves next-completed=active, due>=next and
  // exhaustive free-frame scanning; no construction/receive exception.
  pure def activeAfterOriginalSupply(due:int,published:int,active:int,capacity:int):int=
    active+(if(due-published<capacity-active) due-published else capacity-active)
  pure def dispatchChanges(threads:List[int]):int=
    threads.foldl({previous:0,started:false,changes:0},(s,t)=>{previous:t,started:true,
      changes:s.changes+(if(s.started and s.previous!=t) 1 else 0)}).changes
  pure def cycleCost(c:Cost):int=wholeCycleCost(c,1,2,2,0)
  pure def cycleCostWithWaits(c:Cost,waits:int):int=wholeCycleCost(c,1,2,2+2*waits,0)
  pure def ceilDiv(n:int,d:int):int = (n+d-1)/d
  pure def planned(m:int,period:int,burst:int):int = if(m<=burst) 0 else (m-burst)*period
  pure def arrivals(t:int,period:int,burst:int,limit:int):int = if(burst+t/period<limit) burst+t/period else limit
  pure def service(t:int,cycle:int,latency:int):int = if(t<=latency) 0 else (t-latency)/cycle
  // Include the empty initial prefix A(0-)=0; packet arrivals at time 0 are legal.
  pure def minPlus(t:int,period:int,burst:int,limit:int,cycle:int,latency:int):int =
    0.to(t).fold(service(t,cycle,latency),(bound,s)=>{
      val candidate = arrivals(s,period,burst,limit)+service(t-s,cycle,latency)
      if(candidate<bound) candidate else bound
    })
  pure def hasRecoveryHeadroom(period:int,cycle:int):bool = period>cycle and cycle>0
  // Conditional packet bound: at least floor(t/cycle) completions while
  // backlogged; at most ceil(t/period) NEW arrivals. Round to a completion
  // opportunity, not just an integer tick. Unknown/invalid headroom is -1.
  // This is time to reach the target deficit at some cut, not a guarantee
  // of staying empty or of never exceeding an earlier peak. Native use needs
  // an independently justified service curve, not a measured average rate.
  pure def recoveryTicks(debt:int,period:int,cycle:int):int =
    if(debt<0 or not(hasRecoveryHeadroom(period,cycle))) -1
    else cycle*ceilDiv(debt*period,period-cycle)
  // Necessary average completions per second over a whole-millisecond
  // recovery window with continuous integer-rate arrivals.
  pure def recoveryRatePerSecond(rate:int,debt:int,target:int,windowMs:int):int =
    rate+ceilDiv((if(debt>target) debt-target else 0)*1000,windowMs)
  // Packetized conservative bounds; require cycle<=period and a VALID service curve.
  pure def backlogBound(period:int,burst:int,cycle:int,latency:int):int = burst+ceilDiv(latency+cycle,period)
  // A per-request interpretation additionally requires FIFO or per-request fairness.
  pure def delayBound(burst:int,cycle:int,latency:int):int = latency+cycle+burst*cycle
  // Illustrative constant-cadence serial FIFO, no pauses or cost measurement.
  // Finite-batch mean elapsed is NOT evidence that a native service has this curve.
  pure def constantFifoLatency(period:int,cycle:int,ordinal:int):int =
    if(period<=0 or cycle<=0 or ordinal<=0) -1
    else cycle+(ordinal-1)*(if(cycle>period) cycle-period else 0)
  pure def rank(n:int,num:int,den:int):int = ceilDiv(n*num,den)
}

module actor_service_cost_tests {
  import actor_service_calculus.*
  run NearCapacityTailAmplificationDoesNotRequireHugePerMessageCostTest={
    val ordinal=990000
    assert(constantFifoLatency(1000,1013,ordinal)==12871000 and
      constantFifoLatency(1000,1032,ordinal)==31681000 and
      constantFifoLatency(2000,1032,ordinal)==1032 and
      1032-1013==19 and not(hasRecoveryHeadroom(1000,1013)))
  }
  run OriginalDebtForcesReceiveAndNotIdleYieldTest={
    assert(0.to(12).forall(due=>0.to(due).forall(published=>0.to(published).forall(completed=>{
      val active=published-completed
      val outstanding=due-completed
      active<=3 implies {
        val after=activeAfterOriginalSupply(due,published,active,3)
        after==(if(outstanding<3) outstanding else 3) and
          (outstanding>0 implies after>0) and
          (after==0 implies outstanding==0)
      }
    }))) and activeAfterOriginalSupply(132,3,3,3)==3)
  }
  run FrozenAboveWindowCountsDoNotCountIdleDrivesTest={
    val n=1000000
    val firstAbove=998145
    val secondAbove=998960
    // One receive retires one request. Above-limit samples cannot be idle;
    // non-above receive counts do NOT count or time empty pacing loops.
    assert(n-firstAbove==1855 and n-secondAbove==1040 and
      driverClockCalls(n,0)!=driverClockCalls(n,4000000))
  }
  run FluidRecoveryRoundingCanMissTheNextCompleteMessageTest={
    val debt=1
    val period=7
    val cycle=5
    val fluid=ceilDiv(debt*period*cycle,period-cycle)
    val packet=recoveryTicks(debt,period,cycle)
    assert(fluid==18 and fluid/cycle-ceilDiv(fluid,period)==0 and
      packet==20 and packet/cycle-ceilDiv(packet,period)>=debt)
  }
  run RecoveryBoundPaysForWholeCompletionOpportunitiesTest={
    assert(2.to(30).forall(period=>1.to(period-1).forall(cycle=>0.to(16).forall(debt=>{
      val time=recoveryTicks(debt,period,cycle)
      time>=0 and time%cycle==0 and
        time/cycle-ceilDiv(time,period)>=debt and
        (debt>0 implies (time-cycle)/cycle-ceilDiv(time-cycle,period)<debt)
    }))))
  }
  run MissingRecoveryHeadroomCannotBecomeZeroOrNegativeSuccessTest=assert(
    recoveryTicks(1,5,5)== -1 and recoveryTicks(1,5,6)== -1 and
    recoveryTicks(1,5,0)== -1 and recoveryTicks(-1,7,5)== -1 and
    recoveryTicks(0,7,5)==0)
  run CompleteDriverClockAccountingRetainsFreshProgressTest={
    val n=1000000
    val idle=17
    val source=n+idle
    val publication=n
    val latency=n
    val freshProgress=n
    val boundaries=2
    assert(driverClockCalls(n,0)==4000002 and
      driverClockCalls(n,idle)==source+publication+latency+freshProgress+boundaries)
  }
  run MessageCountsAloneCannotPricePacingCpuTest={
    // Counterexample only; neither idle count is claimed for an actual run.
    assert(driverClockCalls(1000,4000000)>driverClockCalls(1000000,0))
  }
  run ClockLoopGrossDeltaDoesNotDetermineExclusiveBridgeCostTest={
    // One authorized component cohort, rounded total process CPU ns.
    // Both decompositions fit the observed pair; neither is native attribution.
    val baseline=23434000
    val reference=15791000
    val delta=baseline-reference
    val mostlyBridge={common:reference,bridge:delta,schemeLoop:0}
    val mostlyLoop={common:reference,bridge:1000,schemeLoop:delta-1000}
    assert(delta==7643000 and
      mostlyBridge.common+mostlyBridge.bridge+mostlyBridge.schemeLoop==baseline and
      mostlyLoop.common+mostlyLoop.bridge+mostlyLoop.schemeLoop==baseline and
      mostlyBridge.bridge!=mostlyLoop.bridge)
  }
  run transfersDoNotFixDispatchesTest={
    val c={source:1,construct:1,send:2,execute:3,verify:2,consume:1,schedule:5,observe:0}
    val grouped=List(0,0,0,1,1,2,2,3,3,0,0,0)
    val separate=List(0,1,0,2,0,3,0)
    assert(dispatchChanges(grouped)==4 and dispatchChanges(separate)==6 and
      wholeCycleCost(c,3,6,4,0)<wholeCycleCost(c,3,6,6,0))
  }
  run historicalCostAssumptionIsExplicitTest={
    val c={source:1,construct:2,send:3,execute:4,verify:5,consume:6,schedule:7,observe:8}
    assert(cycleCost(c)==46 and cycleCostWithWaits(c,2)==74 and
      wholeCycleCost(c,3,6,6,0)==3*cycleCost(c) and dispatchChanges(List())==0)
  }
  run packingMustPayItsExtraWakeCostsTest={
    val c={source:1,construct:1,send:10,execute:3,verify:2,consume:1,schedule:5,observe:0}
    val baseline=wholeCycleCost(c,3,6,4,0)
    assert(wholeCycleCost(c,3,6,4,6)>baseline and wholeCycleCost(c,3,4,4,6)<baseline and
      wholeCycleCost(c,3,4,8,6)>baseline)
  }
}
