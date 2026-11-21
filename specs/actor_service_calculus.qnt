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
  pure def rank(n:int,num:int,den:int):int = ceilDiv(n*num,den)
}

module actor_service_cost_tests {
  import actor_service_calculus.*
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
