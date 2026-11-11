// Gerbil 1cfb032 std/actor/{interaction,message}.ss. Pure observations of the
// standard layer, not another Actor lifecycle, matcher, clock or routing engine.
module gerbil_actor_transport {
  type MatchView={expired:bool,protocolMatches:bool,continuationMatches:bool,payloadMatches:bool}
  // Selection results assume the user predicate returned normally. Its Scheme
  // effects, exception and continuation semantics are not replaced by this view.
  pure def receiveDecision(e:MatchView):str=
    if(e.expired) "discard-expired" else
    if(e.protocolMatches and e.continuationMatches and e.payloadMatches) "accept" else "skip"
  pure def sendRoute(peer:str):str=
    if(peer=="LocalActor") "thread-send" else if(peer=="Handle") "host-send" else
    if(peer=="ActorGroup") "host-broadcast" else "bad-argument"
  pure def expiredAt(expirySeconds:int,wallSeconds:int):bool=expirySeconds<=wallSeconds
  pure val fullMatch={expired:false,protocolMatches:true,continuationMatches:true,payloadMatches:true}
  run localHandleAndBroadcastDoNotShareTheSameTransportTest=assert(
    sendRoute("LocalActor")=="thread-send" and sendRoute("Handle")=="host-send" and
    sendRoute("ActorGroup")=="host-broadcast" and sendRoute("unknown")=="bad-argument")
  run payloadEqualityCannotRemoveStandardEnvelopeSemanticsTest=assert(
    receiveDecision(fullMatch)=="accept" and
    receiveDecision({...fullMatch,protocolMatches:false})=="skip" and
    receiveDecision({...fullMatch,continuationMatches:false})=="skip" and
    receiveDecision({...fullMatch,payloadMatches:false})=="skip" and
    receiveDecision({...fullMatch,expired:true})=="discard-expired")
  run expirationIsCheckedBeforeProtocolMatchingTest=assert(
    receiveDecision({...fullMatch,expired:true,protocolMatches:false})=="discard-expired")
  run actorEnvelopeExpiryIsNotTheMonotonicBenchmarkClockTest=assert(
    expiredAt(2000,3000) and not(expiredAt(2000,100)))
}
