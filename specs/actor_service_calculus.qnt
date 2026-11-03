// Pure service-budget algebra. Units are abstract ticks, not measured CPU ns.
module actor_service_calculus {
  type Cost = { source:int,construct:int,send:int,execute:int,verify:int,consume:int,schedule:int,observe:int }
  pure def validCost(c:Cost):bool = c.source>=0 and c.construct>=0 and c.send>=0 and
    c.execute>0 and c.verify>=0 and c.consume>=0 and c.schedule>=0 and c.observe>=0
  // Source and worker phases share ONE processor. Do not take max(stage costs).
  pure def cycleCost(c:Cost):int = c.source+c.construct+2*c.send+c.execute+c.verify+c.consume+2*c.schedule+c.observe
  pure def cycleCostWithWaits(c:Cost,waits:int):int = cycleCost(c)+2*waits*c.schedule
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
  // Packetized conservative bounds; require cycle<=period and a VALID service curve.
  pure def backlogBound(period:int,burst:int,cycle:int,latency:int):int = burst+ceilDiv(latency+cycle,period)
  // A per-request interpretation additionally requires FIFO or per-request fairness.
  pure def delayBound(burst:int,cycle:int,latency:int):int = latency+cycle+burst*cycle
  pure def rank(n:int,num:int,den:int):int = ceilDiv(n*num,den)
}
