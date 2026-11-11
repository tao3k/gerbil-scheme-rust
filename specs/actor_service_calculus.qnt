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
  // Conservative integer rounding; valid only with positive spare service.
  pure def recoveryTicks(debt:int,period:int,cycle:int):int = ceilDiv(debt*period*cycle,period-cycle)
  // Necessary average completions per second over a whole-millisecond
  // recovery window with continuous integer-rate arrivals.
  pure def recoveryRatePerSecond(rate:int,debt:int,target:int,windowMs:int):int =
    rate+ceilDiv((if(debt>target) debt-target else 0)*1000,windowMs)
  // Packetized conservative bounds; require cycle<=period and a VALID service curve.
  pure def backlogBound(period:int,burst:int,cycle:int,latency:int):int = burst+ceilDiv(latency+cycle,period)
  // A per-request interpretation additionally requires FIFO or per-request fairness.
  pure def delayBound(burst:int,cycle:int,latency:int):int = latency+cycle+burst*cycle
  // Official ring source: one circulating token, n parameterized threads.
  // Timed region also includes neighbor setup, starts and join. Abstract
  // costs are external inputs, never fitted historical native nanoseconds.
  pure def ringHops(n:int,m:int):int=n*m
  pure def ringTimedCost(n:int,m:int,start:int,neighbor:int,hop:int,join:int):int=
    n*start+n*neighbor+ringHops(n,m)*hop+join
  // Necessary CPU feasibility only. More resident Actors do not divide the
  // CPU work of one non-SMP owner; suspension overlap can reduce idle time,
  // but must not erase construction, validation, sends or retirement CPU.
  pure def cpuWorkFits(c:Cost,messages:int,sends:int,dispatches:int,extra:int,available:int):bool=
    validCost(c) and messages>=0 and sends>=0 and dispatches>=0 and extra>=0 and available>=0 and
      wholeCycleCost(c,messages,sends,dispatches,extra)<=available
  // A coarse conservation projection of the canonical protocol, NOT a new
  // queue or lifecycle. Inputs are increments at cuts of the SAME run. Only
  // real consumption may enter consumed; publication/wake/dequeue may not.
  type Cut={arrived:int,consumed:int}
  pure def debtCuts(cuts:List[Cut]):{arrived:int,consumed:int,peak:int,valid:bool}=
    cuts.foldl({arrived:0,consumed:0,peak:0,valid:true},(s,x)=>{
      val a=s.arrived+x.arrived
      val d=s.consumed+x.consumed
      val debt=a-d
      {arrived:a,consumed:d,peak:if(debt>s.peak) debt else s.peak,
        valid:s.valid and x.arrived>=0 and x.consumed>=0 and d<=a}
    })
  // Necessary no-consumption-gap budget at a known cut. Period, phase and
  // initial debt are external evidence; no p99 or mean rate supplies them.
  pure def pauseDebt(debt:int,phase:int,gap:int,period:int):int=debt+(phase+gap)/period
  pure def pauseFits(debt:int,phase:int,gap:int,period:int,limit:int):bool=
    debt>=0 and phase>=0 and period>0 and phase<period and gap>=0 and limit>=0 and
      pauseDebt(debt,phase,gap,period)<=limit
  pure def rank(n:int,num:int,den:int):int = ceilDiv(n*num,den)
}

module actor_service_cost_tests {
  import actor_service_calculus.*
  run officialRingSetupCannotBeChargedAsPureSteadyHopCostTest={
    val small=ringTimedCost(10,100,3,2,1,7)
    val large=ringTimedCost(1000,1,3,2,1,7)
    assert(ringHops(10,100)==ringHops(1000,1) and small==1057 and large==6007 and
      small>ringHops(10,100) and large>small)
  }
  run ringDefaultRoundsDoNotPromiseAMillionHopsForEveryPopulationTest=assert(
    ringHops(1000000,1000000/1000000)==1000000 and
    ringHops(600000,1000000/600000)==600000 and
    ringHops(1000,1000000/1000)==1000000)
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
  run residentPopulationCannotMultiplyOneOwnerCpuBudgetTest={
    val c={source:1,construct:2,send:1,execute:3,verify:2,consume:1,schedule:1,observe:0}
    // Grouped dispatch reduces actual switches; it does not parallelize CPU.
    val work=wholeCycleCost(c,100,200,20,0)
    assert(work==1120 and cpuWorkFits(c,100,200,20,0,1120) and
      not(cpuWorkFits(c,100,200,20,0,1119)) and
      Set(1,4,1000000).forall(population=>population>0 and work==1120))
  }
  run eventualDrainAndSameMeanDoNotCertifyPeakDebtTest={
    val smooth=debtCuts(List({arrived:64,consumed:0},{arrived:0,consumed:64},
      {arrived:65,consumed:0},{arrived:0,consumed:65}))
    val stalled=debtCuts(List({arrived:129,consumed:0},{arrived:0,consumed:129}))
    assert(smooth.valid and stalled.valid and smooth.arrived==stalled.arrived and
      smooth.consumed==stalled.consumed and smooth.peak==65 and stalled.peak==129 and
      smooth.peak<=128 and not(stalled.peak<=128))
  }
  run invalidConsumptionCountersCannotCertifyDebtTest=assert(
    not(debtCuts(List({arrived:1,consumed:2})).valid) and
    not(debtCuts(List({arrived:-1,consumed:0})).valid) and
    debtCuts(List({arrived:129,consumed:0})).peak==129)
  run pauseBudgetIncludesInitialDebtAndArrivalPhaseTest=assert(
    pauseFits(0,0,128000,1000,128) and not(pauseFits(0,0,129000,1000,128)) and
    pauseFits(127,0,1999,1000,128) and not(pauseFits(127,999,1001,1000,128)) and
    not(pauseFits(129,0,0,1000,128)))
  run lowerSteadyCpuCannotPayForAnUnboundedPauseTest={
    val original={source:1,construct:2,send:1,execute:3,verify:2,consume:1,schedule:1,observe:0}
    val saving={...original,construct:1}
    assert(wholeCycleCost(saving,100,200,20,0)<wholeCycleCost(original,100,200,20,0) and
      not(pauseFits(0,0,129000,1000,128)) and
      recoveryRatePerSecond(1000000,129,128,1)==1001000)
  }

}
