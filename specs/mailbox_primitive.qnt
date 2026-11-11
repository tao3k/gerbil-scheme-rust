// Refinement of one existing Gambit Mailbox, not another runtime queue.
// Source counterpart: Gerbil 1cfb032 -> Gambit ea114fc3; non-SMP only.
// Admission/lifecycle, processor leases and application credit remain canonical.
module mailbox_primitive_protocol {
  const MESSAGES:Set[int]
  const OBJECTS:int->int
  const TIMED:bool
  const SELECTIVE:bool
  type Mailbox={queue:List[int],insertions:List[int],received:List[int],objects:List[int],
    sends:int->str,phase:str,cursor:int,probe:int,registered:bool,timer:bool,
    deadlineToken:int,timeoutConversions:int,wakes:int,selections:int,closed:bool}
  var b:Mailbox
  pure def members(xs:List[int]):Set[int]=xs.foldl(Set(),(s,m)=>s.union(Set(m)))
  pure def without(xs:List[int],m:int):List[int]=xs.foldl(List(),(r,x)=>if(x==m) r else r.append(x))
  pure def prefix(xs:List[int],ys:List[int]):bool=xs.length()<=ys.length() and
    (xs.length()==0 or 0.to(xs.length()-1).forall(i=>xs[i]==ys[i]))
  val inserted=MESSAGES.filter(m=>Set("inserted","signaled","returned").contains(b.sends.get(m)))
  val pendingSignals=MESSAGES.filter(m=>b.sends.get(m)=="inserted")
  val available=b.cursor<b.queue.length()
  val queueNodes=b.insertions.length()
  val signals=MESSAGES.filter(m=>Set("signaled","returned").contains(b.sends.get(m))).size()
  // This is a notification obligation, NOT unconditional liveness. Senders
  // must finish; the scheduler and eventual selection need separate evidence.
  val notificationCovered=not(b.registered and available and pendingSignals==Set())
  val safety=and {
    members(b.queue).intersect(members(b.received))==Set(),
    members(b.queue).union(members(b.received))==inserted,
    b.queue.length()==members(b.queue).size(),b.received.length()==members(b.received).size(),
    b.insertions.length()==inserted.size(),members(b.insertions)==inserted,
    b.objects==b.received.foldl(List(),(xs,m)=>xs.append(OBJECTS.get(m))),
    0<=b.cursor,b.cursor<=b.queue.length(),
    b.registered==(b.phase=="blocked"),b.timer implies b.registered,
    notificationCovered,SELECTIVE or prefix(b.received,b.insertions),
    b.closed implies b.queue==List() and not(b.registered) and pendingSignals==Set()
  }
  action init=all {b'={queue:List(),insertions:List(),received:List(),objects:List(),
    sends:MESSAGES.mapBy(_=>"new"),phase:"idle",cursor:0,probe:0,registered:false,
    timer:false,deadlineToken:0,timeoutConversions:0,wakes:0,selections:0,closed:false}}
  action stay=all {b'=b}
  // Heap checking may pause before insertion; it is not evidence of enqueue.
  action prepareSend(m:int):bool=all {not(b.closed),b.sends.get(m)=="new",
    b'={...b,sends:b.sends.set(m,"checked")}}
  action failBeforeInsertion(m:int):bool=all {b.sends.get(m)=="checked",
    b'={...b,sends:b.sends.set(m,"failed")}}
  // Protected FIFO mutation is one step. No GC/interruption is inserted
  // BETWEEN the queue links; _thread.scm checks heap before the lock.
  action insert(m:int):bool=all {not(b.closed),b.sends.get(m)=="checked",
    b'={...b,queue:b.queue.append(m),insertions:b.insertions.append(m),sends:b.sends.set(m,"inserted")}}
  action signal(m:int):bool=all {b.sends.get(m)=="inserted",
    b'={...b,sends:b.sends.set(m,"signaled"),
      phase:if(b.registered) "woken" else b.phase,wakes:b.wakes+(if(b.registered) 1 else 0),registered:false,timer:false}}
  action returnSend(m:int):bool=all {b.sends.get(m)=="signaled",
    b'={...b,sends:b.sends.set(m,"returned")}}
  action send(m:int):bool=prepareSend(m).then(insert(m)).then(signal(m)).then(returnSend(m))
  action beginReceive=all {not(b.closed),Set("idle","returned","timedout").contains(b.phase),
    b'={...b,phase:"reading",probe:0,deadlineToken:b.deadlineToken+1}}
  action observeEmpty=all {b.phase=="reading",not(available),b'={...b,phase:"empty"}}
  // Native timeout conversion precedes the protected second check. Already
  // ready receive does not take this path. False timeout does not read time.
  action prepareWait=all {b.phase=="empty",b'={...b,phase:"rechecking",
    timeoutConversions:b.timeoutConversions+(if(TIMED) 1 else 0)}}
  action registerWait=all {b.phase=="rechecking",not(available),
    b'={...b,phase:"blocked",registered:true,timer:TIMED}}
  action take=all {Set("reading","rechecking").contains(b.phase),available,
    b'={...b,queue:without(b.queue,b.queue[b.cursor]),received:b.received.append(b.queue[b.cursor]),
      objects:b.objects.append(OBJECTS.get(b.queue[b.cursor])),phase:"returned",cursor:0,probe:0}}
  action selectReceiver=all {Set("woken","timeout-woken").contains(b.phase),
    b'={...b,phase:if(b.phase=="timeout-woken") "timedout" else "rechecking",selections:b.selections+1}}
  action timeout=all {b.phase=="blocked",b.timer,
    b'={...b,phase:"timeout-woken",registered:false,timer:false}}
  // A spurious/interrupt wake retries with the SAME converted deadline.
  action interrupt=all {b.phase=="blocked",
    b'={...b,phase:"woken",registered:false,timer:false}}
  action peek=all {SELECTIVE,Set("reading","rechecking").contains(b.phase),available,
    b'={...b,probe:b.queue[b.cursor],cursor:b.cursor+1,phase:"probing"}}
  action skip=all {SELECTIVE,b.phase=="probing",b'={...b,phase:"reading",probe:0}}
  action extractAndRewind=all {SELECTIVE,b.phase=="probing",b.cursor>0,
    b'={...b,queue:without(b.queue,b.probe),received:b.received.append(b.probe),
      objects:b.objects.append(OBJECTS.get(b.probe)),phase:"returned",cursor:0,probe:0}}
  action rewind=all {SELECTIVE,Set("reading","probing","timedout").contains(b.phase),
    b'={...b,cursor:0,probe:0,phase:if(b.phase=="probing") "reading" else b.phase}}
  // Library admission cut only: native thread-send is NOT claimed to check
  // this service-close rule. Canonical service drain/join is also required.
  action close=all {not(b.closed),b.queue==List(),not(b.registered),
    Set("idle","returned","timedout").contains(b.phase),
    MESSAGES.forall(m=>Set("new","returned","failed").contains(b.sends.get(m))),b'={...b,closed:true}}
  // Mutants are never admitted into step.
  action unsafeRegisterWithoutRecheck=all {b.phase=="rechecking",
    b'={...b,phase:"blocked",registered:true,timer:TIMED}}
  action unsafeForgetSignal(m:int):bool=all {b.sends.get(m)=="inserted",b'={...b,sends:b.sends.set(m,"returned")}}
  action step={nondet m=MESSAGES.oneOf()
    any {prepareSend(m),failBeforeInsertion(m),insert(m),signal(m),returnSend(m),beginReceive,
      observeEmpty,prepareWait,registerWait,take,selectReceiver,timeout,interrupt,
      peek,skip,extractAndRewind,rewind,close,stay}}
}
module mailbox_primitive_check {
  import mailbox_primitive_protocol(MESSAGES=Set(1,2),OBJECTS=Map(1->7,2->7),TIMED=true,SELECTIVE=false).*
}
module mailbox_selective_check {
  import mailbox_primitive_protocol(MESSAGES=Set(1,2),OBJECTS=Map(1->7,2->8),TIMED=true,SELECTIVE=true).*
}
module mailbox_primitive_tests {
  import mailbox_primitive_protocol(MESSAGES=Set(1,2),OBJECTS=Map(1->7,2->7),TIMED=true,SELECTIVE=false).*
  run readyReceiveSkipsTimeoutConversionAndPreservesReferenceTest=init.then(send(1)).then(beginReceive).then(take)
    .then(all {assert(safety),assert(b.objects==List(7)),assert(b.timeoutConversions==0),assert(queueNodes==1),assert(signals==1),
      assert(b.wakes==0),assert(b.selections==0),stay})
  run arrivalBeforeWaitRegistrationUsesProtectedRecheckTest=init.then(beginReceive).then(observeEmpty)
    .then(send(1)).then(prepareWait).then(take)
    .then(all {assert(safety),assert(not(b.registered)),assert(b.timeoutConversions==1),stay})
  run removingProtectedRecheckLosesNotificationTest=init.then(beginReceive).then(observeEmpty)
    .then(send(1)).then(prepareWait).then(unsafeRegisterWithoutRecheck)
    .then(all {assert(not(notificationCovered)),assert(not(safety)),stay})
  run registeredReceiverNeedsSignalAndLaterSelectionTest=init.then(beginReceive).then(observeEmpty)
    .then(prepareWait).then(registerWait).then(prepareSend(1)).then(insert(1))
    .then(all {assert(safety),assert(b.phase=="blocked"),assert(pendingSignals==Set(1)),stay})
    .then(signal(1)).then(all {assert(b.phase=="woken"),assert(b.received==List()),assert(not(b.timer)),assert(b.wakes==1),assert(b.selections==0),assert(safety),stay})
    .then(selectReceiver).then(take).then(returnSend(1)).then(all {assert(safety),stay})
  run enqueueCanBeObservedBeforeSendReturnsTest=init.then(beginReceive).then(prepareSend(1)).then(insert(1))
    .then(take).then(all {assert(safety),assert(b.sends.get(1)=="inserted"),assert(b.received==List(1)),stay})
    .then(signal(1)).then(returnSend(1)).then(all {assert(safety),stay})
  run insertionWithoutRequiredSignalIsRetainedCounterexampleTest=init.then(beginReceive).then(observeEmpty)
    .then(prepareWait).then(registerWait).then(prepareSend(1)).then(insert(1)).then(unsafeForgetSignal(1))
    .then(all {assert(not(notificationCovered)),assert(not(safety)),stay})
  run timeoutDoesNotBecomeSuccessWhenLaterMessageArrivesTest=init.then(beginReceive).then(observeEmpty)
    .then(prepareWait).then(registerWait).then(timeout).then(send(1)).then(selectReceiver)
    .then(all {assert(safety),assert(b.phase=="timedout"),assert(b.received==List()),assert(b.queue==List(1)),stay})
  run interruptedWaitRetainsConvertedDeadlineTest=init.then(beginReceive).then(observeEmpty).then(prepareWait)
    .then(registerWait).then(interrupt).then(selectReceiver).then(registerWait)
    .then(all {assert(safety),assert(b.deadlineToken==1),assert(b.timeoutConversions==1),stay})
  run heapPreparationAndFailureDoNotPublishMailboxEntryTest=init.then(prepareSend(1))
    .then(all {assert(b.queue==List()),assert(inserted==Set()),assert(safety),stay})
    .then(failBeforeInsertion(1)).then(close).then(all {assert(safety),assert(b.closed),stay})
  run fifoOrderAndAliasedPayloadAreSeparateIdentitiesTest=init.then(send(1)).then(send(2))
    .then(beginReceive).then(take).then(beginReceive).then(take).then(close)
    .then(all {assert(safety),assert(b.received==List(1,2)),assert(b.objects==List(7,7)),stay})
}
module mailbox_selective_tests {
  import mailbox_primitive_protocol(MESSAGES=Set(1,2),OBJECTS=Map(1->7,2->8),TIMED=true,SELECTIVE=true).*
  run unmatchedMessagesCanRemainWhileReceiveWaitsTest=init.then(send(1)).then(beginReceive).then(peek).then(skip)
    .then(observeEmpty).then(prepareWait).then(registerWait)
    .then(all {assert(safety),assert(b.queue==List(1)),assert(not(available)),assert(b.phase=="blocked"),stay})
    .then(send(2)).then(selectReceiver).then(peek).then(extractAndRewind)
    .then(all {assert(safety),assert(b.received==List(2)),assert(b.queue==List(1)),assert(b.cursor==0),stay})
  run selectiveExtractionIsNotRawFifoReceiveTest=init.then(send(1)).then(send(2)).then(beginReceive)
    .then(peek).then(skip).then(peek).then(extractAndRewind)
    .then(all {assert(safety),assert(b.received==List(2)),assert(b.queue==List(1)),assert(not(prefix(b.received,b.insertions))),stay})
    .then(beginReceive).then(take).then(close).then(all {assert(safety),assert(b.received==List(2,1)),stay})
}
module mailbox_reply_refinement {
  import mailbox_packet_protocol(ACTORS=Set(1,2),REQUESTS=Set(1,2,3),LIMIT=16) as app from "./mailbox_packets"
  import mailbox_primitive_protocol(MESSAGES=Set(1,2,3),OBJECTS=Map(1->7,2->8,3->9),TIMED=true,SELECTIVE=false) as box
  action stay=all {app::stay,box::stay}
  action boot=all {app::initPackets,box::init}.then(all {app::create(1),box::stay})
    .then(all {app::create(2),box::stay}).then(all {app::credits(3),box::stay}).then(all {app::open,box::stay})
  action ready=boot.then(all {app::offer(1),box::stay}).then(all {app::stage(1,1),box::stay})
    .then(all {app::send(List(1),1),box::stay}).then(all {app::cancel(1),box::stay})
    .then(all {app::decode(1),box::stay}).then(all {app::unit(1),box::stay})
    .then(all {app::proceed(1),box::stay}).then(all {app::finish(1,1,1,false),box::stay})
  // Keep the other module at EVERY microstep of a composed scenario.
  action replySend(m:int):bool=all {app::stay,box::prepareSend(m)}
    .then(all {app::stay,box::insert(m)}).then(all {app::stay,box::signal(m)})
    .then(all {app::stay,box::returnSend(m)})
  // This wrapper refines the EXISTING consumption action, not a new credit.
  def canConsumeReceived(m:int):bool=box::members(box::b.received).contains(m) and app::ready.contains(m)
  action consumeReceived(m:int):bool=all {canConsumeReceived(m),app::consume(m),box::stay}
  // Primitive scheduling is supplied by its caller, not a new CPU model.
  action selectSource=all {app::runningActors==Set(),app::stay,box::selectReceiver}
  run notificationDoesNotSelectSourceAheadOfCurrentActorTest=ready
    .then(all {app::offer(2),box::beginReceive}).then(all {app::stage(2,2),box::observeEmpty})
    .then(all {app::send(List(2),1),box::prepareWait}).then(all {app::decode(2),box::registerWait})
    .then(replySend(1)).then(all {
      assert(app::safe),assert(box::safety),assert(box::b.phase=="woken"),
      assert(app::runningActors==Set(2)),assert(app::outstanding==Set(1,2)),stay})
    .then(all {app::unit(2),box::stay}).then(selectSource).then(all {app::stay,box::take})
    .then(all {assert(app::safe),assert(box::safety),assert(app::outstanding==Set(1,2)),stay})
    .then(consumeReceived(1)).then(all {assert(app::outstanding==Set(2)),assert(app::safe),stay})
  run dequeueAndCallerCancellationDoNotReturnFrameTest=ready.then(replySend(1))
    .then(all {app::stay,box::beginReceive}).then(all {app::stay,box::take})
    .then(all {assert(app::safe),assert(box::safety),assert(app::outstanding==Set(1)),
      assert(app::actorState.consumed==Set()),assert(app::actorState.timedOut==Set(1)),
      assert(not(canConsumeReceived(2))),stay})
    .then(consumeReceived(1)).then(all {assert(app::safe),assert(app::outstanding==Set()),
      assert(not(canConsumeReceived(1))),stay})
}
module mailbox_untimed_tests {
  import mailbox_primitive_protocol(MESSAGES=Set(1,2),OBJECTS=Map(1->7,2->8),TIMED=false,SELECTIVE=false).*
  run untimedWorkerWaitStillUsesNativeNotificationTest=init.then(beginReceive).then(observeEmpty)
    .then(prepareWait).then(registerWait).then(all {
      assert(safety),assert(b.registered),assert(not(b.timer)),assert(b.timeoutConversions==0),stay})
    .then(send(1)).then(selectReceiver).then(take).then(all {assert(safety),assert(b.received==List(1)),stay})
}
