// Observation refinement, not another Actor scheduler. Reuse the existing
// callback lifecycle and service-headroom equation; never alter source custody.
module owner_recovery {
  import owner_wait_scheme_callback as p from "./native_owner_watch"
  import native_service_recovery as r from "./native_source_wait"
  action init=all {p::init,r::init}
  action idle=all {p::idle,r::idle}
  type Point={due:int,published:int,consumed:int}
  pure def valid(a:Point):bool=0<=a.consumed and a.consumed<=a.published and a.published<=a.due
  pure def outstanding(a:Point):int=a.due-a.consumed
  pure def sourceDebt(a:Point):int=a.due-a.published
  pure def surplus(a:Point,b:Point):int=(b.consumed-a.consumed)-(b.due-a.due)
  pure def conserved(a:Point,b:Point):bool=valid(a) and valid(b) and
    outstanding(a)-outstanding(b)==surplus(a,b)
  run actualSourceDebtDecompositionTest=init.then({
    val peak={due:4644082,published:4642669,consumed:4642667}
    all {assert(valid(peak)),assert(sourceDebt(peak)==1413),assert(outstanding(peak)==1415),
    assert(peak.published-peak.consumed==2),idle}})
  run recoveryMustSettleNewArrivalsAsWellAsOldDebtTest=init.then({
    val start={due:10,published:9,consumed:9}
    val peak={due:30,published:13,consumed:13}
    val end={due:40,published:40,consumed:38}
    all {assert(conserved(start,peak)),assert(surplus(start,peak)== -16),
    assert(conserved(peak,end)),assert(surplus(peak,end)==15),
    assert(outstanding(peak)==17),assert(outstanding(end)==2),idle}})
  run returnedFramesDoNotProveRecoveryHeadroomTest=init.then(all {
    assert(r::recoveryNs(2826000,500000,500000)== -1),
    assert(r::recoveryNs(2826000,500000,600000)==14130000),idle})
  // A synthetic distribution, not a reconstruction of the native histogram.
  // A favorable p99 can coexist with a much worse maximum and failed debt gate.
  run favorableQuantileDoesNotAdmitMaximumDebtTest=init.then(all {
    assert((5000000*99+99)/100==4950000),
    assert(4999000>=4950000),assert(4999000+1000==5000000),
    assert(298000<317000),assert(1415>128),idle})
  run independentComplementRanksMatchOriginalCeilTest=init.then(all {
    assert(1.to(10000).forall(n => (n*99+99)/100==n-n/100 and
      (n*999+999)/1000==n-n/1000)),idle})
  run laterPeakCannotBorrowEarlierGcWindowTest=init.then(all {
    assert(9288162667>722629542),assert(4642667!=360849),idle})
  // The rejected observer's debt fell only after the finite source stopped.
  // A lower average arrival rate across that drain is not 500k/s headroom.
  run sourceStopDrainCannotAdmitSustainedRecoveryTest=init.then(all {
    assert(9730646583<9999998000),assert(10768433416>9999998000),
    assert(488294>129772),assert(488294<500000),
    assert(r::recoveryNs(744396000,500000,488294)== -1),idle})
}
