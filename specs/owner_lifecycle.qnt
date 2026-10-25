// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
// Fixed-owner lifetime refinement. bridge_handoff owns queue/receiver semantics;
// this module adds close-flag publication and lazy-executor retirement races.
// Native execution, GC, FFI ownership and successful joins remain assumptions.
// serviceRoots covers the internal task table, then its temporary native root
// during take/projection. Actor affinity and continuation semantics are native
// qualification obligations. Notifications model delivery, not OS latency or
// an unconditional liveness guarantee; enqueue/notify publication is split.
module owner_lifecycle {
  const JOBS: Set[int]
  const ASSIGNED: int -> int
  type State = {
    phase: str,
    selected: int -> int, committed: int -> int,
    accepted: Set[int], completed: Set[int], ready: Set[int],
    roots: Set[int], registry: Set[int], timedOut: Set[int],
    executor: str, filesPresent: bool, imagesRetained: bool,
    codeLeases: Set[int], unloadAuthorized: bool, codeQuarantined: bool,
    unsafeCodeRelease: bool,
    capacityLimit: int, capacityHeld: Set[int], reserved: Set[int],
    workLimit: int, workHeld: Set[int],
    poolStaged: Set[int], poolReady: bool, poolPremature: bool,
    poolStreaming: bool, poolSealed: bool, poolInputFailed: bool, poolPublished: Set[int], poolInputLimit: int,
    producerLimit: int, producers: Set[int], staged: Set[int], producerJobs: Set[int],
    servicePending: Set[int], serviceReady: Set[int], serviceRoots: Set[int], serviceFailed: Set[int],
    inbox: Set[int], wakeOwed: Set[int], notified: Set[int], parked: Set[int],
    capacityArmed: Set[int], capacitySignalled: Set[int],
    shortWaiters: List[int], shortSignalled: Set[int],
    offerSlots: int -> int, offers: Set[int], offerEpoch: int -> int, receivedEpoch: int -> int,
    progressDone: int -> Set[int], progressYielded: Set[int], shortDebt: int -> int,
    latePublication: bool, lateExecutor: bool, foreignUse: bool
  }
  var s: State
  val shortWaiterSet = s.shortWaiters.foldl(Set(), (seen, o) => seen.union(Set(o)))
  def waitersWithout(owner: int): List[int] = s.shortWaiters.foldl(List(), (kept, o) => if (o == owner) kept else kept.append(o))

  val initial = {
    selected: ASSIGNED, committed: JOBS.mapBy(_ => 0),
    phase: "open", accepted: Set(), completed: Set(), ready: Set(),
    roots: Set(), registry: Set(), timedOut: Set(), executor: "absent",
    filesPresent: true, imagesRetained: true,
    codeLeases: Set(), unloadAuthorized: false, codeQuarantined: false,
    unsafeCodeRelease: false,
    capacityLimit: JOBS.size(), capacityHeld: Set(), reserved: Set(),
    workLimit: JOBS.size() + 7, workHeld: Set(),
    poolStaged: Set(), poolReady: false, poolPremature: false,
    poolStreaming: false, poolSealed: true, poolInputFailed: false, poolPublished: Set(), poolInputLimit: JOBS.size()+7,
    producerLimit: JOBS.size(), producers: Set(), staged: Set(), producerJobs: Set(),
    servicePending: Set(), serviceReady: Set(), serviceRoots: Set(), serviceFailed: Set(),
    inbox: Set(), wakeOwed: Set(), notified: Set(), parked: Set(),
    capacityArmed: Set(), capacitySignalled: Set(),
    shortWaiters: List(), shortSignalled: Set(),
    offerSlots: Set(1, 2).mapBy(_ => 0), offers: Set(), offerEpoch: JOBS.mapBy(_ => 0), receivedEpoch: JOBS.mapBy(_ => -1),
    progressDone: JOBS.mapBy(_ => Set()), progressYielded: Set(), shortDebt: Set(1, 2).mapBy(_ => 0),
    latePublication: false, lateExecutor: false, foreignUse: false
  }
  action init = s' = initial
  def ownerOf(r: int): int = s.selected.get(r)
  // Only unreserved, unpublished stateless requests can select another owner.
  // Affine requests use the existing fixed-owner path and never this action.
  action selectOwner(r: int, owner: int): bool = all {
    s.phase == "open", JOBS.contains(r), Set(1, 2).contains(owner),
    not(s.workHeld.contains(r)), not(s.offers.contains(r)), not(s.accepted.contains(r)), not(s.producerJobs.contains(r)),
    s' = { ...s, selected: s.selected.set(r, owner) }
  }
  action unsafeOwnerMove(r: int, owner: int): bool = all {
    s.accepted.contains(r), ownerOf(r) != owner,
    s' = { ...s, selected: s.selected.set(r, owner) }
  }
  // Job 1 is a batch of eight; the other requests have unit cost.
  def units(r: int): int = if (r == 1) 8 else 1
  def workUsed(owner: int): int = {
    val held = s.workHeld.filter(job => ownerOf(job) == owner)
    held.size() + (if (held.contains(1)) 7 else 0)
  }
  def hasSlot(r: int): bool =
    s.capacityHeld.filter(job => ownerOf(job) == ownerOf(r)).size() < s.capacityLimit
  def hasWork(r: int): bool = workUsed(ownerOf(r)) + units(r) <= s.workLimit
  def hasCapacity(r: int): bool = hasSlot(r) and hasWork(r)
  // Full work acquisition can precede slot acquisition. Partial semaphore
  // allocation is an implementation detail covered by native cancellation tests.
  action reserveWork(r: int): bool = all {
    s.phase == "open", JOBS.contains(r), not(s.poolStaged.contains(r)), not(s.offers.contains(r)), not(s.workHeld.contains(r)), not(s.accepted.contains(r)), hasWork(r),
    s' = { ...s, workHeld: s.workHeld.union(Set(r)) }
  }
  action reserveSlot(r: int): bool = all {
    s.phase == "open", s.workHeld.contains(r), not(s.capacityHeld.contains(r)), not(s.accepted.contains(r)), hasSlot(r),
    s' = { ...s, reserved: s.reserved.union(Set(r)), capacityHeld: s.capacityHeld.union(Set(r)) }
  }
  action cancelWork(r: int): bool = all {
    s.workHeld.contains(r), not(s.capacityHeld.contains(r)), not(s.accepted.contains(r)),
    s' = { ...s, workHeld: s.workHeld.exclude(Set(r)), wakeOwed: s.wakeOwed.union(Set(ownerOf(r))) }
  }
  action configureWork(limit: int): bool = all {
    s.phase == "open", s.capacityHeld.size() == 0, s.workHeld.size() == 0, s.producers.size() == 0, limit > 0,
    s' = { ...s, workLimit: limit }
  }
  action reserve(r: int): bool = all {
    s.phase == "open", JOBS.contains(r), not(s.accepted.contains(r)), not(s.poolStaged.contains(r)), not(s.offers.contains(r)),
    not(s.reserved.contains(r)), not(s.workHeld.contains(r)), hasCapacity(r),
    s' = { ...s, reserved: s.reserved.union(Set(r)), capacityHeld: s.capacityHeld.union(Set(r)), workHeld: s.workHeld.union(Set(r)) }
  }
  action admitReserved(r: int): bool = all {
    s.phase == "open", s.reserved.contains(r), not(s.offers.contains(r)), canProducerEnter(r),
    s' = { ...s, accepted: s.accepted.union(Set(r)), committed: s.committed.set(r, ownerOf(r)), inbox: s.inbox.union(Set(r)), wakeOwed: s.wakeOwed.union(Set(ownerOf(r))), staged: s.staged.exclude(Set(r)), reserved: s.reserved.exclude(Set(r)) }
  }
  action releaseReservation(r: int): bool = all {
    s.reserved.contains(r), not(s.offers.contains(r)),
    s' = { ...s, reserved: s.reserved.exclude(Set(r)), capacityHeld: s.capacityHeld.exclude(Set(r)), workHeld: s.workHeld.exclude(Set(r)), wakeOwed: s.wakeOwed.union(Set(ownerOf(r))) }
  }
  action configureCapacity(limit: int): bool = all {
    s.phase == "open", s.capacityHeld.size() == 0, s.workHeld.size() == 0, s.producers.size() == 0, limit > 0,
    s' = { ...s, capacityLimit: limit }
  }
  action admitUnchecked(r: int): bool = all {
    s.phase == "open", JOBS.contains(r), not(s.accepted.contains(r)), not(s.poolStaged.contains(r)), not(s.offers.contains(r)),
    not(s.reserved.contains(r)), not(s.workHeld.contains(r)), hasCapacity(r),
    s' = { ...s, accepted: s.accepted.union(Set(r)), committed: s.committed.set(r, ownerOf(r)), inbox: s.inbox.union(Set(r)), wakeOwed: s.wakeOwed.union(Set(ownerOf(r))), staged: s.staged.exclude(Set(r)), capacityHeld: s.capacityHeld.union(Set(r)), workHeld: s.workHeld.union(Set(r)) }
  }
  action admit(r: int): bool = all { canProducerEnter(r), admitUnchecked(r) }
  def producerActive(owner: int): int = s.producerJobs.intersect(s.accepted).exclude(s.completed).filter(r => ownerOf(r) == owner).size()
  def canProducerEnter(r: int): bool = not(s.producerJobs.contains(r)) or producerActive(ownerOf(r)) < s.producerLimit
  action configureProducer(limit: int): bool = all {
    s.phase == "open", s.producers.size() == 0, limit > 0,
    s' = { ...s, producerLimit: limit }
  }
  action registerProducer(owner: int): bool = all {
    s.phase == "open", Set(1, 2).contains(owner), not(s.producers.contains(owner)),
    s' = { ...s, producers: s.producers.union(Set(owner)) }
  }
  action stage(r: int): bool = all {
    s.phase == "open", JOBS.contains(r), not(s.poolStaged.contains(r)), s.producers.contains(ownerOf(r)),
    not(s.accepted.contains(r)), not(s.offers.contains(r)), not(s.staged.contains(r)), not(s.workHeld.contains(r)),
    s' = { ...s, staged: s.staged.union(Set(r)), producerJobs: s.producerJobs.union(Set(r)) }
  }
  action cancelProducer(owner: int): bool = all {
    Set("open", "closing").contains(s.phase), s.producers.contains(owner),
    all {
      val pending = s.staged.filter(job => ownerOf(job) == owner)
      s' = { ...s, offerSlots: s.offerSlots.set(owner, 0), offers: s.offers.filter(r => ownerOf(r) != owner), staged: s.staged.exclude(pending), producerJobs: s.producerJobs.exclude(pending),
        reserved: s.reserved.exclude(pending), capacityHeld: s.capacityHeld.exclude(pending), workHeld: s.workHeld.exclude(pending) }
    }
  }
  action finishProducer(owner: int): bool = all {
    Set("open", "closing").contains(s.phase), s.producers.contains(owner),
    (not(s.poolStreaming) or s.poolSealed),
    s.poolStaged.size() == 0, s.offers.size() == 0, s.producerJobs.filter(job => ownerOf(job) == owner).subseteq(s.completed),
    s' = { ...s, producers: s.producers.exclude(Set(owner)),
      producerJobs: s.producerJobs.filter(job => ownerOf(job) != owner) }
  }
  // Completion is an externally validated owner action, not Rust computation.
  action complete(r: int): bool = all {
    Set("open", "closing").contains(s.phase),
    s.accepted.contains(r), not(s.completed.contains(r)),
    not(s.servicePending.contains(r)) or s.serviceReady.contains(r),
    s' = { ...s, servicePending: s.servicePending.exclude(Set(r)), serviceReady: s.serviceReady.exclude(Set(r)), serviceRoots: s.serviceRoots.exclude(Set(r)), inbox: s.inbox.exclude(Set(r)), completed: s.completed.union(Set(r)), capacityHeld: s.capacityHeld.exclude(Set(r)), workHeld: s.workHeld.exclude(Set(r)),
      roots: s.roots.union(Set(r)), ready: s.ready.union(Set(r)), wakeOwed: s.wakeOwed.union(Set(ownerOf(r))) }
  }
  // Observation timeout leaves accepted native work and roots unchanged.
  action observeTimeout(r: int): bool = all {
    s.accepted.contains(r), not(s.timedOut.contains(r)),
    s' = { ...s, timedOut: s.timedOut.union(Set(r)) }
  }
  val canJoin = s.phase == "closing" and s.accepted.subseteq(s.completed) and s.producers.size() == 0 and s.serviceRoots.size() == 0
  def canPublish(r: int): bool = s.phase == "open" and s.ready.contains(r)
  def canUse(r: int, owner: int): bool = and {
    s.phase == "open", s.registry.contains(r), s.roots.contains(r),
    ownerOf(r) == owner
  }
  // This helper deliberately exposes the missing-close-check mutation below.
  action publishUnchecked(r: int): bool = all {
    s.ready.contains(r), not(s.registry.contains(r)),
    s' = { ...s, registry: s.registry.union(Set(r)), ready: s.ready.exclude(Set(r)),
      latePublication: s.latePublication or s.phase != "open" }
  }
  action publish(r: int): bool = all { canPublish(r), publishUnchecked(r) }
  action use(r: int, owner: int): bool = all { canUse(r, owner), s' = s }
  action release(r: int, owner: int): bool = all {
    Set("open", "closing").contains(s.phase), s.roots.contains(r), ownerOf(r) == owner,
    s' = { ...s, roots: s.roots.exclude(Set(r)), registry: s.registry.exclude(Set(r)),
      ready: s.ready.exclude(Set(r)) }
  }
  action createExecutorUnchecked = all {
    s.executor != "alive",
    s' = { ...s, executor: "alive", lateExecutor: s.lateExecutor or s.phase != "open" }
  }
  action createExecutor = all { s.phase == "open", createExecutorUnchecked }
  action close = all { s.phase == "open", s' = { ...s, phase: "closing", notified: Set(1, 2), shortWaiters: List(), shortSignalled: Set() } }
  action joinUnchecked = all {
    s.phase == "closing", s' = { ...s, phase: "joined", roots: Set(), parked: Set(), notified: Set(), wakeOwed: Set() }
  }
  action join = all { canJoin, joinUnchecked }
  // Metadata can briefly outlive native roots during close. No API use or
  // publication is allowed then; an already returned RetainedValue is a copy.
  action clearRegistry = all {
    s.phase == "joined", s' = { ...s, registry: Set() }
  }
  action retire = all {
    s.phase == "joined", s.registry.size() == 0,
    s' = { ...s, phase: "closed", executor: "retired", filesPresent: false }
  }
  // Code leases are separate from Scheme heap roots. Existing lease holders
  // may clone after owner exit; an empty retired image cannot acquire a lease.
  action codeLease(r: int): bool = all {
    s.imagesRetained, JOBS.contains(r), not(s.codeLeases.contains(r)),
    s.phase == "open" or s.codeLeases.size() > 0,
    s' = { ...s, codeLeases: s.codeLeases.union(Set(r)) }
  }
  action releaseCodeLease(r: int): bool = all {
    s.codeLeases.contains(r), s' = { ...s, codeLeases: s.codeLeases.exclude(Set(r)) }
  }
  action authorizeCodeRelease = all {
    s.phase == "open", not(s.unloadAuthorized),
    s' = { ...s, unloadAuthorized: true }
  }
  action quarantineCode = all {
    s.imagesRetained, not(s.codeQuarantined), s' = { ...s, codeQuarantined: true }
  }
  // Join assumes successful native cleanup; failure quarantine is qualified
  // natively. This flag represents the backend loader reference, not every
  // possible dlopen reference held by another host library.
  val canReleaseCode = and {
    s.imagesRetained, Set("joined", "closed").contains(s.phase),
    s.unloadAuthorized, s.codeLeases.size() == 0, not(s.codeQuarantined)
  }
  action releaseCodeUnchecked = all {
    s.imagesRetained,
    s' = { ...s, imagesRetained: false,
      unsafeCodeRelease: s.unsafeCodeRelease or not(canReleaseCode) }
  }
  action releaseCode = all { canReleaseCode, releaseCodeUnchecked }
  // Strong Scheme task graphs persist until native result projection/retirement.
  action submitService(r: int): bool = all {
    Set("open", "closing").contains(s.phase), s.inbox.contains(r),
    not(s.servicePending.contains(r)), not(s.completed.contains(r)),
    s' = { ...s, inbox: s.inbox.exclude(Set(r)), servicePending: s.servicePending.union(Set(r)), serviceRoots: s.serviceRoots.union(Set(r)) }
  }
  // Internal worker acknowledgement does not create host-actionable work.
  // The existing ready/input/control transitions own the native wake level.
  action ackServiceDispatch(r: int): bool = all {
    Set("open", "closing").contains(s.phase), s.servicePending.contains(r), not(s.serviceReady.contains(r)),
    s' = s
  }
  action readyService(r: int, failed: bool): bool = all {
    s.servicePending.contains(r), not(s.serviceReady.contains(r)),
    s' = { ...s, serviceReady: s.serviceReady.union(Set(r)), notified: s.notified.union(Set(ownerOf(r))),
      serviceFailed: if (failed) s.serviceFailed.union(Set(r)) else s.serviceFailed }
  }
  // Enqueue and OS notification publication are deliberately separate actions.
  action notify(owner: int): bool = all {
    s.wakeOwed.contains(owner), s' = { ...s, wakeOwed: s.wakeOwed.exclude(Set(owner)), notified: s.notified.union(Set(owner)) }
  }
  action deliverWake(owner: int): bool = all {
    s.notified.contains(owner), s' = { ...s, parked: s.parked.exclude(Set(owner)) }
  }
  action clearWake(owner: int): bool = all {
    not(s.parked.contains(owner)), s' = { ...s, notified: s.notified.exclude(Set(owner)) }
  }
  def canPark(owner: int): bool = and {
    s.phase == "open", not(s.notified.contains(owner)),
    not(s.inbox.exists(r => ownerOf(r) == owner)),
    not(s.serviceReady.exists(r => ownerOf(r) == owner)),
    not(s.offers.exists(r => ownerOf(r) == owner and offerRunnable(r)))
  }
  action park(owner: int): bool = all { canPark(owner), s' = { ...s, parked: s.parked.union(Set(owner)) } }
  action unsafePark(owner: int): bool = s' = { ...s, parked: s.parked.union(Set(owner)) }
  action unsafeServiceRoot(r: int): bool = all {
    s.servicePending.contains(r), s' = { ...s, serviceRoots: s.serviceRoots.exclude(Set(r)) }
  }
  action unsafeParkStep = { nondet owner = Set(1, 2).oneOf(); any { step, unsafePark(owner) } }
  action unsafeServiceRootStep = { nondet r = JOBS.oneOf(); any { step, unsafeServiceRoot(r) } }
  action stutter = s' = s
  action step = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    any { submitService(r), readyService(r, false), readyService(r, true), notify(owner), deliverWake(owner), clearWake(owner), park(owner), admit(r), complete(r), observeTimeout(r), publish(r), use(r, owner),
      release(r, owner), createExecutor, close, join, clearRegistry, retire,
      codeLease(r), releaseCodeLease(r), authorizeCodeRelease, quarantineCode, releaseCode,
      registerProducer(owner), stage(r), cancelProducer(owner), finishProducer(owner),
      reserveWork(r), reserveSlot(r), cancelWork(r), configureWork(8), configureWork(9), reserve(r), admitReserved(r), releaseReservation(r), configureCapacity(1), configureCapacity(2), stutter }
  }
  val safety = and {
    shortWaiterSet.subseteq(Set(1, 2)), s.shortSignalled.subseteq(Set(1, 2)),
    s.shortWaiters.length() == shortWaiterSet.size(),
    shortWaiterSet.intersect(s.shortSignalled).size() == 0,
    s.capacityArmed.subseteq(Set(1, 2)), s.capacitySignalled.subseteq(s.capacityArmed),
    JOBS.forall(r => Set(1, 2).contains(ownerOf(r))),
    s.accepted.forall(r => ownerOf(r) == s.committed.get(r)),
    s.servicePending.subseteq(s.accepted.exclude(s.completed)), s.serviceRoots == s.servicePending,
    s.serviceReady.subseteq(s.servicePending), s.serviceFailed.subseteq(s.accepted),
    s.inbox.subseteq(s.accepted.exclude(s.completed)), s.inbox.intersect(s.servicePending).size() == 0,
    s.parked.subseteq(Set(1, 2)), s.notified.subseteq(Set(1, 2)), s.wakeOwed.subseteq(Set(1, 2)),
    s.parked.forall(owner => implies(
      s.inbox.exists(r => ownerOf(r) == owner) or s.serviceReady.exists(r => ownerOf(r) == owner),
      s.notified.contains(owner) or s.wakeOwed.contains(owner))),
    not(s.poolPremature), s.poolStaged.subseteq(JOBS), s.poolStaged.intersect(s.accepted.union(s.producerJobs).union(s.workHeld)).size() == 0,
    implies(s.poolReady and s.poolStaged.size() > 0 and not(s.poolStreaming), s.producers == Set(1, 2)),
    implies(s.poolInputFailed, s.poolSealed and s.poolStaged.size() == 0),
    s.poolPublished.subseteq(JOBS),
    implies(s.poolStreaming, s.poolPublished.fold(0,(n,r) => n+units(r)) <= s.poolInputLimit),
    s.producerLimit > 0, Set(1, 2).forall(owner => producerActive(owner) <= s.producerLimit),
    s.producers.subseteq(Set(1, 2)), s.staged.subseteq(s.producerJobs), s.producerJobs.subseteq(JOBS),
    s.producerJobs.subseteq(s.staged.union(s.accepted)), s.staged.intersect(s.accepted).size() == 0,
    s.producerJobs.forall(job => s.producers.contains(ownerOf(job))),
    s.workLimit > 0, s.workHeld.subseteq(JOBS), s.capacityHeld.subseteq(s.workHeld),
    s.workHeld.intersect(s.completed).size() == 0,
    Set(1, 2).forall(owner => workUsed(owner) <= s.workLimit),
    s.capacityLimit > 0, s.capacityHeld.subseteq(JOBS), s.reserved.subseteq(JOBS),
    s.reserved.intersect(s.accepted).size() == 0,
    s.capacityHeld == s.reserved.union(s.accepted.exclude(s.completed)),
    Set(1, 2).forall(owner => s.capacityHeld.filter(job => ownerOf(job) == owner).size() <= s.capacityLimit),
    s.accepted.subseteq(JOBS), s.completed.subseteq(s.accepted),
    s.timedOut.subseteq(s.accepted), s.ready.subseteq(s.completed),
    s.roots.subseteq(s.completed), s.registry.subseteq(s.completed),
    not(s.latePublication), not(s.lateExecutor), not(s.foreignUse),
    implies(s.phase == "open", s.registry.subseteq(s.roots)),
    implies(Set("joined", "closed").contains(s.phase), and {
      s.accepted.subseteq(s.completed), s.roots.size() == 0, s.producers.size() == 0, s.serviceRoots.size() == 0 }),
    implies(s.phase == "closed", and { s.executor == "retired",
      not(s.filesPresent), s.registry.size() == 0 }),
    s.codeLeases.subseteq(JOBS), not(s.unsafeCodeRelease),
    implies(not(s.imagesRetained), and {
      Set("joined", "closed").contains(s.phase), s.unloadAuthorized,
      s.codeLeases.size() == 0, not(s.codeQuarantined) })
  }

  action unsafeProducerExit(owner: int): bool = all {
    s.producers.contains(owner), s.producerJobs.filter(job => ownerOf(job) == owner).size() > 0,
    s' = { ...s, producers: s.producers.exclude(Set(owner)) }
  }
  action unsafeProducerExitStep = { nondet owner = Set(1, 2).oneOf(); any { step, unsafeProducerExit(owner) } }
  action unsafeCreditReturn(r: int): bool = all {
    s.accepted.contains(r), not(s.completed.contains(r)), s.timedOut.contains(r),
    s' = { ...s, capacityHeld: s.capacityHeld.exclude(Set(r)) }
  }
  action unsafeCreditReturnStep = { nondet r = JOBS.oneOf(); any { step, unsafeCreditReturn(r) } }
  action unsafePublicationStep = { nondet r = JOBS.oneOf(); any { step, publishUnchecked(r) } }
  action unsafeExecutorStep = any { step, createExecutorUnchecked }
  action unsafeJoinStep = any { step, joinUnchecked }
  action unsafeCodeReleaseStep = any { step, releaseCodeUnchecked }
  action unsafeForeignUse = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    all { s.phase == "open", s.registry.contains(r), ownerOf(r) != owner,
      s' = { ...s, foreignUse: true } }
  }
  action unsafeForeignStep = any { step, unsafeForeignUse }

  run serviceWakeRecheckTest = init.then(admit(1)).then(notify(1)).then(clearWake(1))
    .then(all { assert(not(canPark(1))), assert(safety), stutter })
    .then(submitService(1)).then(park(1)).then(readyService(1, false)).then(deliverWake(1)).then(complete(1))
    .then(close).then(join).then(all { assert(safety), stutter })
  run serviceTimeoutCloseDrainTest = init.then(admit(1)).then(submitService(1)).then(observeTimeout(1)).then(close)
    .then(all { assert(s.serviceRoots.contains(1)), assert(s.workHeld.contains(1)), assert(not(canJoin)), assert(safety), stutter })
    .then(readyService(1, true)).then(complete(1)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(s.serviceFailed.contains(1)), assert(safety), stutter })
  // Bounded class retirement may leave a ready graph behind. Reuse the same
  // root/wake protocol; native tests separately check FIFO and class turns.
  run servicePartialRetirementTest = init
    .then(admit(1)).then(submitService(1)).then(readyService(1, false))
    .then(admit(2)).then(submitService(2)).then(readyService(2, true))
    .then(complete(2)).then(clearWake(ownerOf(1)))
    .then(all { assert(s.serviceRoots == Set(1)), assert(s.serviceReady == Set(1)),
      assert(not(canPark(ownerOf(1)))), assert(safety), stutter })
    .then(close).then(complete(1)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(safety), stutter })
  run serviceEarlyRootMutationTest = init.then(admit(1)).then(submitService(1)).then(unsafeServiceRoot(1))
    .then(all { assert(not(safety)), stutter })
  run serviceLostWakeMutationTest = init.then(admit(1)).then(notify(1)).then(clearWake(1)).then(unsafePark(1))
    .then(all { assert(not(safety)), stutter })
  run producerNativeRefillTest = init.then(configureWork(8)).then(registerProducer(1)).then(stage(1)).then(stage(3)).then(admit(1))
    .then(all { assert(not(hasCapacity(3))), assert(safety), stutter }).then(complete(1)).then(admit(3)).then(complete(3))
    .then(finishProducer(1)).then(all { assert(s.producers.size() == 0), assert(safety), stutter })
  run producerCloseRejectsUnadmittedTest = init.then(configureWork(8)).then(registerProducer(1)).then(stage(1)).then(stage(3)).then(admit(1))
    .then(observeTimeout(1)).then(close).then(cancelProducer(1))
    .then(all { assert(s.workHeld.contains(1)), assert(not(s.accepted.contains(3))), assert(safety), stutter })
    .then(complete(1)).then(finishProducer(1)).then(join).then(all { assert(safety), stutter })
  run producerPartialWorkCloseTest = init.then(configureWork(8)).then(registerProducer(1)).then(stage(1)).then(reserveWork(1))
    .then(close).then(cancelProducer(1)).then(finishProducer(1)).then(join)
    .then(all { assert(s.workHeld.size() == 0), assert(s.accepted.size() == 0), assert(safety), stutter })
  run earlyProducerExitMutationTest = init.then(registerProducer(1)).then(stage(1)).then(unsafeProducerExit(1))
    .then(all { assert(not(safety)), stutter })
  run workAndSlotsAreIndependentTest = init.then(configureCapacity(4)).then(configureWork(8)).then(admit(1))
    .then(all { assert(hasSlot(3)), assert(not(hasWork(3))), assert(safety), stutter })
    .then(observeTimeout(1)).then(all { assert(workUsed(1) == 8), assert(safety), stutter })
    .then(complete(1)).then(all { assert(hasCapacity(3)), assert(safety), stutter })
  run workBeforeSlotCancellationTest = init.then(configureWork(8)).then(reserveWork(1))
    .then(all { assert(s.capacityHeld.size() == 0), assert(not(hasWork(3))), assert(safety), stutter })
    .then(close).then(cancelWork(1)).then(all { assert(s.workHeld.size() == 0), assert(safety), stutter })
  run workThenSlotTest = init.then(configureWork(8)).then(reserveWork(1)).then(reserveSlot(1))
    .then(admitReserved(1)).then(complete(1)).then(all { assert(s.workHeld.size() == 0), assert(safety), stutter })
  run timeoutKeepsSharedCreditTest = init.then(configureCapacity(1)).then(admit(1))
    .then(observeTimeout(1)).then(all { assert(s.capacityHeld.contains(1)), assert(not(hasCapacity(3))), assert(safety), stutter })
    .then(complete(1)).then(all { assert(hasCapacity(3)), assert(safety), stutter })
  run reservedCloseCannotAdmitTest = init.then(configureCapacity(1)).then(reserve(1)).then(close)
    .then(releaseReservation(1)).then(join).then(all { assert(s.capacityHeld.size() == 0), assert(safety), stutter })
  run nativeCompletionReturnsCreditTest = init.then(configureCapacity(1)).then(reserve(1)).then(admitReserved(1))
    .then(complete(1)).then(configureCapacity(2)).then(all { assert(safety), stutter })
  run earlyCreditReturnMutationTest = init.then(configureCapacity(1)).then(admit(1)).then(observeTimeout(1))
    .then(unsafeCreditReturn(1)).then(all { assert(not(safety)), stutter })
  run codeLeasePreventsReleaseTest = init.then(authorizeCodeRelease).then(codeLease(1))
    .then(close).then(join)
    .then(all { assert(not(canReleaseCode)), assert(s.imagesRetained), stutter })
    .then(codeLease(2)).then(releaseCodeLease(1))
    .then(all { assert(not(canReleaseCode)), stutter })
    .then(releaseCodeLease(2)).then(releaseCode)
    .then(all { assert(not(s.imagesRetained)), assert(safety), stutter })
  run policyRetainsCodeTest = init.then(close).then(join)
    .then(all { assert(not(canReleaseCode)), assert(s.imagesRetained), assert(safety), stutter })
  run quarantineRetainsCodeTest = init.then(authorizeCodeRelease).then(close)
    .then(join).then(quarantineCode)
    .then(all { assert(not(canReleaseCode)), assert(s.imagesRetained), assert(safety), stutter })
  run earlyCodeReleaseMutationTest = init.then(authorizeCodeRelease).then(codeLease(1))
    .then(close).then(join).then(releaseCodeUnchecked)
    .then(all { assert(not(safety)), stutter })
  run timeoutIsNotCancellationTest = init.then(admit(1)).then(observeTimeout(1)).then(close)
    .then(all { assert(not(canJoin)), assert(s.accepted.contains(1)), stutter })
    .then(complete(1)).then(join).then(clearRegistry).then(retire)
    .then(all { assert(safety), assert(s.imagesRetained), stutter })
  run ownerBoundHandleTest = init.then(admit(1)).then(complete(1)).then(publish(1))
    .then(all { assert(canUse(1, 1)), assert(not(canUse(1, 2))), stutter })
    .then(release(1, 1)).then(all { assert(not(canUse(1, 1))), assert(safety), stutter })
  run closingRejectsPublicationTest = init.then(admit(1)).then(complete(1)).then(close)
    .then(all { assert(not(canPublish(1))), assert(safety), stutter })
  run lateReplyAfterRetirementTest = init.then(admit(1)).then(complete(1)).then(close)
    .then(join).then(clearRegistry).then(retire)
    .then(all { assert(s.ready.contains(1)), assert(not(canPublish(1))), assert(safety), stutter })
  run latePublicationMutationTest = init.then(admit(1)).then(complete(1)).then(close)
    .then(join).then(clearRegistry).then(retire).then(publishUnchecked(1))
    .then(all { assert(not(safety)), stutter })
  run lateExecutorMutationTest = init.then(close).then(join).then(clearRegistry).then(retire)
    .then(createExecutorUnchecked).then(all { assert(not(safety)), stutter })
  run earlyJoinMutationTest = init.then(admit(1)).then(close).then(joinUnchecked)
    .then(all { assert(not(safety)), stutter })
  run foreignHandleMutationTest = init.then(admit(1)).then(complete(1)).then(publish(1))
    .then(unsafeForeignUse).then(all { assert(not(safety)), stutter })
  // A bounded uncommitted source has no owner charge. Claim atomically selects
  // a fixed owner only after its active/native capacities and barrier are ready.
  // Speculative partial credits and OS wake delivery are native obligations.
  action stagePool(r: int): bool = all {
    s.phase == "open", (not(s.poolReady) or (s.poolStreaming and not(s.poolSealed) and not(s.poolInputFailed))), JOBS.contains(r),
    not(s.poolPublished.contains(r)),
    not(s.poolStreaming) or s.poolPublished.union(Set(r)).fold(0,(n,j) => n+units(j)) <= s.poolInputLimit,
    not(s.poolStaged.union(s.offers).union(s.accepted).union(s.producerJobs).union(s.workHeld).contains(r)),
    s' = { ...s, poolStaged: s.poolStaged.union(Set(r)), poolPublished: s.poolPublished.union(Set(r)) }
  }
  action enablePool = all {
    s.phase == "open", not(s.poolReady), s.producers == Set(1, 2),
    s' = { ...s, poolReady: true }
  }
  action claimPoolUnchecked(r: int, owner: int): bool = all {
    s.phase == "open", s.poolStaged.contains(r), s.producers.contains(owner),
    producerActive(owner) < s.producerLimit,
    s.capacityHeld.filter(job => ownerOf(job) == owner).size() < s.capacityLimit,
    workUsed(owner) + units(r) <= s.workLimit,
    s' = { ...s, progressYielded: s.progressYielded.exclude(Set(owner)), shortDebt: s.shortDebt.set(owner, 0), poolPremature: s.poolPremature or not(s.poolReady), selected: s.selected.set(r, owner), committed: s.committed.set(r, owner),
      poolStaged: s.poolStaged.exclude(Set(r)), producerJobs: s.producerJobs.union(Set(r)),
      shortWaiters: waitersWithout(owner), shortSignalled: if (s.poolStaged.size() == 1) Set() else s.shortSignalled.exclude(Set(owner)),
      accepted: s.accepted.union(Set(r)), capacityHeld: s.capacityHeld.union(Set(r)), workHeld: s.workHeld.union(Set(r)),
      inbox: s.inbox.union(Set(r)), wakeOwed: s.wakeOwed.union(Set(owner)) }
  }
  action claimPool(r: int, owner: int): bool = all { s.poolReady, not(s.poolInputFailed), claimPoolUnchecked(r, owner) }
  // Cancellation owns only the still-unassigned Rust payload, never a claim.
  action cancelShort(r: int): bool = all {
    s.poolStaged.contains(r),
    s' = { ...s, poolStaged: s.poolStaged.exclude(Set(r)),
      shortSignalled: if (s.poolStaged.size() == 1) Set() else s.shortSignalled }
  }
  action unsafeQueuedCredit(r: int): bool = all {
    s.poolStaged.contains(r), s' = { ...s, workHeld: s.workHeld.union(Set(r)) }
  }
  action cancelPool = all {
    Set("open", "closing", "joined", "closed").contains(s.phase), s.poolStaged.size() > 0,
    s' = { ...s, poolStaged: Set(), shortSignalled: Set() }
  }
  action poolStep = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    any { streamStep, stagePool(r), enablePool, claimPool(r, owner), cancelPool }
  }
  // An opt-in open source reuses the producer set and native capacity. The
  // cumulative publication budget is independent of native per-owner work.
  action streamingInputOpen = all {
    s.phase == "open", not(s.poolStreaming), s.accepted.size() == 0,
    s' = { ...s, poolStreaming: true, poolSealed: false, poolReady: true }
  }
  action streamingInputSeal = all {
    s.poolStreaming, not(s.poolSealed),
    s' = { ...s, poolSealed: true }
  }
  action streamingInputAbort = all {
    s.poolStreaming,
    s' = { ...s, poolSealed: true, poolInputFailed: true, poolStaged: Set(), shortSignalled: Set() }
  }
  action streamingInputBudget(limit: int): bool = all {
    s.poolPublished.size() == 0, limit > 0,
    s' = { ...s, poolInputLimit: limit }
  }
  action unsafeStreamingPublish(r: int): bool = all {
    s.poolInputFailed, s' = { ...s, poolStaged: s.poolStaged.union(Set(r)) }
  }
  action unsafeStreamingFinish(owner: int): bool = all {
    s.poolStreaming, not(s.poolSealed), s.producers.contains(owner),
    s' = { ...s, producers: s.producers.exclude(Set(owner)), poolPremature: true }
  }
  action streamingStart = init.then(streamingInputOpen)
  action streamingInputStep = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1,2).oneOf()
    any { streamStep, stagePool(r), claimPool(r,owner), streamingInputSeal, streamingInputAbort }
  }
  run streamingInputPartialRegistrationTest = init.then(streamingInputOpen).then(registerProducer(1)).then(stagePool(3))
    .then(claimPool(3,1)).then(submitService(3))
    .then(all { assert(s.producers == Set(1)), assert(s.serviceRoots == Set(3)), assert(not(s.poolSealed)), assert(safety), stutter })
    .then(registerProducer(2)).then(streamingInputSeal).then(readyService(3,false)).then(complete(3))
    .then(finishProducer(1)).then(finishProducer(2)).then(all { assert(safety), stutter })
  run streamingInputEmptyOpenTest = init.then(streamingInputOpen).then(registerProducer(1))
    .then(all { assert(s.poolStaged.size() == 0), assert(not(s.poolSealed)), assert(safety), stutter })
    .then(stagePool(3)).then(claimPool(3,1)).then(complete(3)).then(streamingInputSeal).then(finishProducer(1))
    .then(all { assert(s.producers.size() == 0), assert(safety), stutter })
  // Native execution/retirement must not await future Rust publication or seal.
  run streamingInputOpenExecutionRetirementTest = init.then(streamingInputOpen).then(registerProducer(1))
    .then(stagePool(1)).then(claimPool(1,1)).then(submitService(1))
    .then(readyService(1,false)).then(complete(1))
    .then(all { assert(not(s.poolSealed)), assert(s.producers == Set(1)),
      assert(s.serviceRoots.size() == 0), assert(s.servicePending.size() == 0),
      assert(s.capacityHeld.size() == 0), assert(s.workHeld.size() == 0), assert(s.ready == Set(1)), assert(safety), stutter })
    .then(streamingInputSeal).then(finishProducer(1)).then(all { assert(safety), stutter })
  run streamingInputAbortAcceptedDrainTest = init.then(streamingInputOpen).then(registerProducer(1)).then(stagePool(1)).then(stagePool(3))
    .then(claimPool(1,1)).then(submitService(1)).then(streamingInputAbort).then(close)
    .then(all { assert(s.serviceRoots == Set(1)), assert(s.capacityHeld == Set(1)), assert(s.poolStaged.size() == 0), assert(safety), stutter })
    .then(readyService(1,true)).then(complete(1)).then(finishProducer(1)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(safety), stutter })
  run streamingInputSeparateBudgetTest = init.then(configureWork(8)).then(streamingInputBudget(9)).then(streamingInputOpen)
    .then(registerProducer(1)).then(stagePool(1)).then(stagePool(3)).then(claimPool(1,1))
    .then(all { assert(s.poolPublished == Set(1,3)), assert(workUsed(1) == 8), assert(s.poolInputLimit == 9), assert(safety), stutter })
  run streamingInputLatePublicationMutationTest = init.then(streamingInputOpen).then(registerProducer(1))
    .then(streamingInputAbort).then(unsafeStreamingPublish(3)).then(all { assert(not(safety)), stutter })
  run streamingInputPrematureFinishMutationTest = init.then(streamingInputOpen).then(registerProducer(1))
    .then(unsafeStreamingFinish(1)).then(all { assert(not(safety)), stutter })
  action poolRegistrationMutationInit = s' = { ...initial, poolStaged: Set(3), producers: Set(1) }
  action unsafePoolRegistrationStep = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    any { poolStep, claimPoolUnchecked(r, owner) }
  }
  run poolBarrierMutationTest = init.then(stagePool(3)).then(registerProducer(1)).then(claimPoolUnchecked(3, 1))
    .then(all { assert(not(safety)), stutter })
  // Refill one released slot while the other ready result keeps its root and
  // credit. Reuse the pool claim and retirement transitions, not a new scheduler.
  run poolRetirementRefillTest = init.then(configureCapacity(2)).then(configureProducer(2))
    .then(stagePool(1)).then(stagePool(2)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(claimPool(1, 1)).then(submitService(1)).then(readyService(1, false))
    .then(claimPool(2, 1)).then(submitService(2)).then(readyService(2, false))
    .then(all { assert(producerActive(1) == 2), assert(safety), stutter })
    .then(complete(1)).then(claimPool(3, 1)).then(submitService(3))
    .then(all { assert(s.serviceRoots == Set(2, 3)), assert(s.serviceReady == Set(2)),
      assert(producerActive(1) == 2), assert(safety), stutter })
    .then(close).then(readyService(3, true)).then(complete(3)).then(complete(2))
    .then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(safety), stutter })
  run poolRetirementSnapshotRefillTest = init.then(configureCapacity(2)).then(configureProducer(2))
    .then(stagePool(1)).then(stagePool(2)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(claimPool(1, 1)).then(submitService(1)).then(readyService(1, false))
    .then(claimPool(2, 1)).then(submitService(2)).then(readyService(2, false))
    .then(all { assert(producerActive(1) == 2), assert(safety), stutter })
    .then(complete(1)).then(complete(2)).then(claimPool(3, 1)).then(submitService(3))
    .then(all { assert(s.serviceRoots == Set(3)), assert(s.serviceReady.size() == 0),
      assert(producerActive(1) == 1), assert(safety), stutter })
    .then(close).then(readyService(3, true)).then(complete(3))
    .then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(safety), stutter })
  run poolBusyOwnerBypassTest = init.then(configureCapacity(1)).then(admit(1))
    .then(stagePool(3)).then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(claimPool(3, 2)).then(submitService(3))
    .then(all { assert(ownerOf(3) == 2), assert(s.serviceRoots == Set(3)), assert(safety), stutter })
    .then(readyService(3, false)).then(complete(3)).then(complete(1))
    .then(finishProducer(1)).then(finishProducer(2)).then(all { assert(safety), stutter })
  run poolCancelAcceptedDrainTest = init.then(configureProducer(1)).then(stagePool(1)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool).then(claimPool(1, 2)).then(submitService(1))
    .then(cancelPool).then(close)
    .then(all { assert(s.poolStaged.size() == 0), assert(s.serviceRoots == Set(1)), assert(safety), stutter })
    .then(readyService(1, true)).then(complete(1)).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(safety), stutter })
  // Unregistered Rust inputs can outlive native join while their host future is
  // unpolled. They own no native root/credit; a later failure/drop rejects them.
  run poolUnregisteredCloseTest = init.then(stagePool(3)).then(close).then(join)
    .then(all { assert(s.accepted.size() == 0), assert(s.serviceRoots.size() == 0), assert(safety), stutter })
    .then(cancelPool).then(all { assert(s.poolStaged.size() == 0), assert(safety), stutter })
  run poolPartialRegistrationCancelTest = init.then(stagePool(1)).then(registerProducer(1))
    .then(all { assert(not(s.poolReady)), assert(s.accepted.size() == 0), assert(safety), stutter })
    .then(cancelPool).then(finishProducer(1)).then(all { assert(s.producers.size() == 0), assert(safety), stutter })
  run poolAcceptedMoveMutationTest = init.then(stagePool(3)).then(registerProducer(1)).then(registerProducer(2))
    .then(enablePool).then(claimPool(3, 2)).then(unsafeOwnerMove(3, 1))
    .then(all { assert(not(safety)), stutter })
  // Shared-capacity notification refines this lifecycle, not a second credit
  // model. Scanner IDs represent the two class gates, not native owners.
  // No permit is reserved while a scanner waits. Registration precedes the
  // live scan; every credit return/close publishes before the producer blocks.
  action armCapacity(scanner: int): bool = all {
    Set(1, 2).contains(scanner), not(s.capacityArmed.contains(scanner)),
    s' = { ...s, capacityArmed: s.capacityArmed.union(Set(scanner)), capacitySignalled: s.capacitySignalled.exclude(Set(scanner)) }
  }
  action publishCapacity = s' = { ...s, capacitySignalled: s.capacitySignalled.union(s.capacityArmed) }
  action disarmCapacity(scanner: int): bool = all {
    s.capacityArmed.contains(scanner),
    s' = { ...s, capacityArmed: s.capacityArmed.exclude(Set(scanner)), capacitySignalled: s.capacitySignalled.exclude(Set(scanner)) }
  }
  action capacityProtocolStep = any { step, armCapacity(1), armCapacity(2), publishCapacity, disarmCapacity(1), disarmCapacity(2) }
  run capacityArmScanReturnWaitTest = init.then(configureCapacity(1)).then(admit(1))
    .then(armCapacity(1)).then(armCapacity(2))
    .then(all { assert(not(hasCapacity(3))), stutter }).then(complete(1)).then(publishCapacity).then(publishCapacity)
    .then(all { assert(hasCapacity(3)), assert(s.capacitySignalled == Set(1, 2)), assert(safety), stutter })
    .then(disarmCapacity(1)).then(reserve(3)).then(admitReserved(3)).then(complete(3)).then(publishCapacity)
    .then(all { assert(s.capacitySignalled == Set(2)), assert(safety), stutter })
  run capacityCancelAndCloseTest = init.then(armCapacity(1)).then(armCapacity(2)).then(disarmCapacity(1))
    .then(close).then(publishCapacity).then(all { assert(s.capacitySignalled == Set(2)), assert(safety), stutter })
  // Negative ordering control: scan first, then return/publish, then register
  // has no notification. The implementation must never use this ordering.
  run capacityScanBeforeArmLosesWakeTest = init.then(configureCapacity(1)).then(admit(1))
    .then(all { assert(not(hasCapacity(3))), stutter }).then(complete(1)).then(publishCapacity).then(armCapacity(1))
    .then(all { assert(hasCapacity(3)), assert(not(s.capacitySignalled.contains(1))), assert(safety), stutter })
  // A dynamic short offer reuses the existing unassigned source and claim
  // boundary. producers abstracts installed owner claim endpoints here; this
  // facility does not consume the native bulk producer registration semaphore.
  action offerShort(r: int): bool = all {
    s.phase == "open", s.poolReady, s.producers == Set(1, 2), units(r) == 1,
    not(s.poolStaged.union(s.offers).union(s.accepted).union(s.producerJobs).union(s.workHeld).contains(r)),
    s' = { ...s, poolStaged: s.poolStaged.union(Set(r)) }
  }
  // Wait registration and notification duty are distinct from committed owner
  // and native credit ownership. A running owner may claim a signalled peer's
  // payload; no OS scheduling bound or equal placement is assumed.
  action armShort(owner: int): bool = all {
    s.phase == "open", s.producers.contains(owner),
    not(shortWaiterSet.union(s.shortSignalled).contains(owner)),
    s' = { ...s, shortDebt: s.shortDebt.set(owner, 0), shortWaiters: s.shortWaiters.append(owner) }
  }
  action resumeShort(owner: int): bool = all {
    shortWaiterSet.union(s.shortSignalled).contains(owner),
    s' = { ...s, shortDebt: s.shortDebt.set(owner, 0), shortWaiters: waitersWithout(owner), shortSignalled: s.shortSignalled.exclude(Set(owner)) }
  }
  def shortEligible(owner: int): bool = and {
    s.producers.contains(owner), producerActive(owner) < s.producerLimit,
    s.capacityHeld.filter(r => ownerOf(r) == owner).size() < s.capacityLimit,
    workUsed(owner) + 1 <= s.workLimit }
  action selectiveShortWake = {
    val eligible = s.shortWaiters.foldl(List(), (kept, o) => if (shortEligible(o)) kept.append(o) else kept)
    all {
      s.phase == "open", s.poolStaged.size() > s.shortSignalled.size(), eligible.length() > 0,
      s' = { ...s, shortWaiters: waitersWithout(eligible.head()),
        shortSignalled: s.shortSignalled.union(Set(eligible.head())), wakeOwed: s.wakeOwed.union(Set(eligible.head())) }
    }
  }
  // Visible admissible work is claimed before the owner enters its wait path.
  // Registration still handles publication racing a failed scan.
  action claimOrArmShort(r: int, owner: int): bool =
    if (s.poolStaged.contains(r) and units(r) == 1 and shortEligible(owner))
      claimPool(r, owner) else armShort(owner)
  run selectiveVisibleWorkBeforeWaitTest = init.then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerShort(3)).then(claimOrArmShort(3, 2))
    .then(all { assert(s.accepted == Set(3)), assert(s.committed.get(3) == 2), assert(s.shortSignalled.size() == 0), assert(safety), stutter })
  action selectiveShortStep = {
    nondet owner = Set(1, 2).oneOf()
    any { shortPoolStep, armShort(owner), resumeShort(owner), selectiveShortWake, claimOrArmShort(3, owner) }
  }
  run selectiveWakeDoesNotBindTest = init.then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(armShort(1)).then(armShort(2)).then(offerShort(3)).then(selectiveShortWake)
    .then(all { assert(s.shortSignalled == Set(1)), assert(s.committed.get(3) == 0), assert(not(s.workHeld.contains(3))), assert(safety), stutter })
    .then(claimPool(3, 2)).then(submitService(3)).then(observeTimeout(3)).then(close)
    .then(readyService(3, false)).then(complete(3)).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(safety), stutter })
  run selectiveInterruptedWaitRelaysTest = init.then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(armShort(1)).then(armShort(2)).then(offerShort(3)).then(selectiveShortWake).then(resumeShort(1)).then(selectiveShortWake)
    .then(all { assert(s.shortSignalled == Set(2)), assert(s.accepted.size() == 0), assert(safety), stutter })
    .then(cancelPool).then(close).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(s.shortSignalled.size() == 0), assert(s.shortWaiters.length() == 0), assert(safety), stutter })
  run selectiveArmAfterPublishTest = init.then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerShort(3)).then(armShort(2)).then(selectiveShortWake)
    .then(all { assert(s.shortSignalled == Set(2)), assert(safety), stutter })
  // Native service progress is advisory: zero residual never returns capacity,
  // root, producer or code ownership. Stable unit IDs are idempotent on replay.
  def residual(owner: int): int = s.accepted.filter(r => ownerOf(r) == owner
    and not(s.completed.union(s.serviceReady).contains(r))).fold(0,
      (total, r) => total + units(r) - s.progressDone.get(r).size())
    + s.offers.filter(r => ownerOf(r) == owner).fold(0, (total, r) => total + units(r))
  action reportProgress(r: int, unit: int): bool = all {
    s.servicePending.contains(r), not(s.serviceReady.contains(r)), 0 <= unit, unit < units(r),
    s' = { ...s, progressDone: s.progressDone.set(r, s.progressDone.get(r).union(Set(unit))) }
  }
  def eligibleFor(owner: int, amount: int): bool = and {
    shortEligible(owner), s.parked.contains(owner), workUsed(owner) + amount <= s.workLimit }
  action yieldProgress(r: int, owner: int, peer: int): bool = all {
    s.phase == "open", s.poolStaged.contains(r), owner != peer,
    s.producers.contains(owner), shortEligible(owner),
    workUsed(owner) + units(r) <= s.workLimit, eligibleFor(peer, units(r)),
    not(s.progressYielded.contains(owner)), residual(owner) > residual(peer) + units(r),
    s' = { ...s, progressYielded: s.progressYielded.union(Set(owner)),
      wakeOwed: s.wakeOwed.union(Set(peer)) }
  }
  // Unlike historical yieldProgress, this commits an actual target inbox entry.
  // residual includes inbox units already, then native progress replaces them.
  action routeProgress(r: int, from: int, target: int): bool = all {
    from != target, s.producers.contains(from),
    residual(from) > residual(target) + units(r), claimPool(r, target)
  }
  action claimProgressPool(r: int, owner: int): bool = all {
    s.progressYielded.contains(owner) or Set(1, 2).forall(peer => peer == owner
      or not(eligibleFor(peer, units(r))) or residual(owner) <= residual(peer) + units(r)),
    claimPool(r, owner)
  }
  action progressShortWake = {
    val eligible = s.shortWaiters.foldl(List(), (kept, o) => if (shortEligible(o)) kept.append(o) else kept)
    val promoted = eligible.foldl(List(), (kept, o) => if (s.shortDebt.get(o) >= 2) kept.append(o) else kept)
    nondet chosen = Set(1, 2).oneOf()
    all {
      s.phase == "open", s.poolStaged.size() > s.shortSignalled.size(), eligible.length() > 0,
      eligible.foldl(Set(), (seen, o) => seen.union(Set(o))).contains(chosen),
      if (promoted.length() > 0) chosen == promoted.head()
      else eligible.foldl(Set(), (seen, o) => seen.union(Set(o))).forall(o => residual(chosen) <= residual(o)),
      s' = { ...s, shortWaiters: waitersWithout(chosen),
        shortDebt: Set(1, 2).mapBy(o => if (o == chosen) 0 else
          if (shortWaiterSet.contains(o) and shortEligible(o)) s.shortDebt.get(o) + 1 else s.shortDebt.get(o)),
        shortSignalled: s.shortSignalled.union(Set(chosen)), wakeOwed: s.wakeOwed.union(Set(chosen)) }
    }
  }
  // Dedicated Rust offer slots never consume native active/work/request credit.
  // A source head uses canonical input identity order in these finite domains.
  def sourcePrice(entries: Set[int]): int = {
    val first = entries.fold(0, (old, r) => if (old == 0 or r < old) r else old)
    if (first == 0) 0 else units(first)
  }
  action offerProgress(r: int, target: int): bool = all {
    s.phase == "open", s.poolReady, s.poolStaged.contains(r), s.producers.contains(target),
    s.offerEpoch.get(r) < 2, s.offerSlots.get(target) == 0,
    s' = { ...s, offerSlots: s.offerSlots.set(target, r), offers: s.offers.union(Set(r)),
      offerEpoch: s.offerEpoch.set(r, s.offerEpoch.get(r) + 1),
      selected: s.selected.set(r, target), poolStaged: s.poolStaged.exclude(Set(r)),
      wakeOwed: s.wakeOwed.union(Set(target)) }
  }
  action receiveOffer(r: int): bool = all {
    s.offers.contains(r), s' = { ...s, receivedEpoch: s.receivedEpoch.set(r, s.offerEpoch.get(r)) }
  }
  action withdrawOffer(r: int): bool = all {
    s.offers.contains(r), not(s.accepted.contains(r)),
    s' = { ...s, offerSlots: s.offerSlots.set(ownerOf(r), 0),
      offers: s.offers.exclude(Set(r)), poolStaged: s.poolStaged.union(Set(r)) }
  }
  def offerRunnable(r: int): bool = and { s.phase == "open", s.offers.contains(r),
    producerActive(ownerOf(r)) < s.producerLimit, hasCapacity(r) }
  def canEnterOffer(r: int): bool = and { offerRunnable(r),
    s.receivedEpoch.get(r) == s.offerEpoch.get(r) }
  // The running owner acquires all native credits at the unique entry fence.
  // Roots conservatively include the whole constructor/submit operation.
  action enterOffer(r: int): bool = all {
    canEnterOffer(r),
    all {
      val before = s.poolStaged.union(s.offers)
      val repriced = sourcePrice(before) != sourcePrice(before.exclude(Set(r)))
      s' = { ...s, offerSlots: s.offerSlots.set(ownerOf(r), 0), offers: s.offers.exclude(Set(r)),
        accepted: s.accepted.union(Set(r)), producerJobs: s.producerJobs.union(Set(r)),
        capacityHeld: s.capacityHeld.union(Set(r)), workHeld: s.workHeld.union(Set(r)),
        committed: s.committed.set(r, ownerOf(r)), servicePending: s.servicePending.union(Set(r)),
        serviceRoots: s.serviceRoots.union(Set(r)),
        wakeOwed: if (repriced) s.wakeOwed.union(s.producers) else s.wakeOwed }
    }
  }
  action unsafeOfferHoldCredit(r: int): bool = all {
    s.offers.contains(r), s' = { ...s, workHeld: s.workHeld.union(Set(r)) }
  }
  action unsafeRetainOfferSlot(r: int): bool = all {
    s.offers.contains(r), s' = { ...s, offers: s.offers.exclude(Set(r)), poolStaged: s.poolStaged.union(Set(r)) }
  }
  action unsafeStaleOfferEntry(r: int): bool = all {
    s.receivedEpoch.get(r) > 0, not(s.offers.contains(r)), not(s.accepted.contains(r)),
    s' = { ...s, accepted: s.accepted.union(Set(r)), committed: s.committed.set(r, ownerOf(r)),
      servicePending: s.servicePending.union(Set(r)), serviceRoots: s.serviceRoots.union(Set(r)) }
  }
  val progressSafety = and { safety,
    s.offers.intersect(s.poolStaged.union(s.producerJobs).union(s.workHeld).union(s.capacityHeld).union(s.accepted).union(s.serviceRoots)).size() == 0,
    implies(s.offers.size() > 0, s.producers == Set(1, 2)),
    s.parked.forall(o => implies(s.offers.exists(r => ownerOf(r) == o and offerRunnable(r)),
      s.notified.contains(o) or s.wakeOwed.contains(o))),
    Set(1, 2).forall(o => if (s.offerSlots.get(o) == 0) not(s.offers.exists(r => ownerOf(r) == o))
      else s.offers.filter(r => ownerOf(r) == o) == Set(s.offerSlots.get(o))),
    s.offers.forall(r => s.committed.get(r) == 0), s.progressYielded.subseteq(Set(1, 2)),
    JOBS.forall(r => s.progressDone.get(r).forall(unit => 0 <= unit and unit < units(r))),
    Set(1, 2).forall(o => residual(o) >= 0 and s.shortDebt.get(o) <= 2) }
  action progressStep = {
    nondet r = JOBS.oneOf(); nondet unit = Set(0, 1, 2, 3, 4, 5, 6, 7).oneOf()
    nondet owner = Set(1, 2).oneOf(); nondet peer = Set(1, 2).oneOf()
    any { selectiveShortStep, offerProgress(r, owner), receiveOffer(r), withdrawOffer(r), enterOffer(r), reportProgress(r, unit), yieldProgress(r, owner, peer), routeProgress(r, owner, peer), claimProgressPool(r, owner), progressShortWake }
  }
  action unsafeProgressRelease(r: int): bool = all {
    s.servicePending.contains(r), s.progressDone.get(r).size() > 0,
    s' = { ...s, capacityHeld: s.capacityHeld.exclude(Set(r)), workHeld: s.workHeld.exclude(Set(r)) }
  }
  action unsafeProgressReleaseStep = { nondet r = JOBS.oneOf(); any { progressStep, unsafeProgressRelease(r) } }
  run progressReceiptKeepsCreditTest = init.then(configureCapacity(1)).then(configureWork(8))
    .then(admit(1)).then(submitService(1)).then(reportProgress(1, 0)).then(reportProgress(1, 0))
    .then(all { assert(residual(1) == 7), assert(workUsed(1) == 8), assert(s.serviceRoots == Set(1)), assert(progressSafety), stutter })
    .then(observeTimeout(1)).then(close).then(readyService(1, true)).then(complete(1)).then(join)
    .then(all { assert(progressSafety), stutter })
  run progressPeerOpportunityTest = init.then(stagePool(1)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(claimPool(1, 1)).then(submitService(1)).then(park(2)).then(yieldProgress(3, 1, 2))
    .then(all { assert(not(s.capacityHeld.contains(3))), assert(not(s.workHeld.contains(3))), assert(s.committed.get(3) == 0), assert(progressSafety), stutter })
    .then(claimProgressPool(3, 1)).then(submitService(3))
    .then(all { assert(ownerOf(3) == 1), assert(s.progressYielded.size() == 0), assert(progressSafety), stutter })
    .then(close).then(readyService(1, false)).then(readyService(3, false)).then(complete(1)).then(complete(3))
    .then(finishProducer(1)).then(finishProducer(2)).then(join).then(all { assert(progressSafety), stutter })
  run progressBusyPeerClaimTest = init.then(stagePool(1)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(claimPool(1, 1)).then(submitService(1))
    .then(all { assert(not(s.parked.contains(2))), assert(residual(1) > residual(2)), stutter })
    .then(claimProgressPool(3, 1)).then(submitService(3))
    .then(all { assert(ownerOf(3) == 1), assert(s.progressYielded.size() == 0), assert(progressSafety), stutter })
    .then(close).then(readyService(1, false)).then(readyService(3, false)).then(complete(1)).then(complete(3))
    .then(finishProducer(1)).then(finishProducer(2)).then(join).then(all { assert(progressSafety), stutter })
  run progressActualPeerDeliveryTest = init.then(stagePool(1)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(claimPool(1, 1)).then(submitService(1)).then(routeProgress(3, 1, 2))
    .then(all { assert(s.inbox.contains(3)), assert(ownerOf(3) == 2),
      assert(s.capacityHeld.contains(3)), assert(workUsed(2) == 1), assert(residual(2) == 1),
      assert(s.servicePending == Set(1)), assert(s.progressYielded.size() == 0), assert(progressSafety), stutter })
    .then(observeTimeout(3)).then(close).then(submitService(3))
    .then(readyService(3, true)).then(complete(3)).then(readyService(1, false)).then(complete(1))
    .then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(progressSafety), stutter })
  run progressOfferWithdrawReclaimTest = init.then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(3, 2)).then(receiveOffer(3))
    .then(all { assert(s.committed.get(3) == 0), assert(s.serviceRoots.size() == 0), assert(progressSafety), stutter })
    .then(withdrawOffer(3)).then(offerProgress(3, 1))
    .then(all { assert(not(canEnterOffer(3))), assert(s.receivedEpoch.get(3) != s.offerEpoch.get(3)), assert(progressSafety), stutter })
    .then(receiveOffer(3)).then(enterOffer(3)).then(observeTimeout(3)).then(close)
    .then(all { assert(ownerOf(3) == 1), assert(s.serviceRoots == Set(3)), assert(progressSafety), stutter })
    .then(readyService(3, true)).then(complete(3)).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(progressSafety), stutter })
  run progressOfferCancelTest = init.then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(3, 2)).then(receiveOffer(3)).then(cancelProducer(2))
    .then(all { assert(not(canEnterOffer(3))), assert(s.workHeld.size() == 0), assert(s.capacityHeld.size() == 0), assert(progressSafety), stutter })
    .then(close).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(progressSafety), stutter })
  run progressOfferCloseBeforeEntryTest = init.then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(3, 2)).then(receiveOffer(3)).then(close)
    .then(all { assert(not(canEnterOffer(3))), assert(progressSafety), stutter })
    .then(withdrawOffer(3)).then(cancelPool).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(s.workHeld.size() == 0), assert(progressSafety), stutter })
  run progressOfferDoesNotCompeteWithShortCreditTest = init.then(configureCapacity(1)).then(configureWork(8))
    .then(stagePool(1)).then(stagePool(3)).then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(1, 2)).then(receiveOffer(1))
    .then(all { assert(s.workHeld.size() == 0), assert(s.capacityHeld.size() == 0), assert(progressSafety), stutter })
    .then(claimPool(3, 2)).then(submitService(3))
    .then(all { assert(not(canEnterOffer(1))), assert(s.offerSlots.get(2) == 1), assert(progressSafety), stutter })
    .then(readyService(3, false)).then(complete(3)).then(enterOffer(1))
    .then(close).then(readyService(1, false)).then(complete(1)).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(progressSafety), stutter })
  run progressOfferSlotReuseTest = init.then(stagePool(1)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(3, 2)).then(receiveOffer(3)).then(withdrawOffer(3)).then(offerProgress(1, 2))
    .then(all { assert(s.offerSlots.get(2) == 1), assert(not(canEnterOffer(3))), assert(progressSafety), stutter })
    .then(receiveOffer(1)).then(enterOffer(1)).then(close).then(cancelPool)
    .then(readyService(1, false)).then(complete(1)).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(progressSafety), stutter })
  run progressSourcePriceNotificationTest = init.then(stagePool(2)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(2, 2)).then(receiveOffer(2)).then(notify(2)).then(enterOffer(2))
    .then(all { assert(s.wakeOwed.size() == 0), assert(progressSafety), stutter })
    .then(offerProgress(3, 1)).then(receiveOffer(3)).then(notify(1)).then(enterOffer(3))
    .then(all { assert(s.wakeOwed == Set(1, 2)), assert(progressSafety), stutter })
    .then(close).then(readyService(2, false)).then(readyService(3, false)).then(complete(2)).then(complete(3))
    .then(finishProducer(1)).then(finishProducer(2)).then(join).then(all { assert(progressSafety), stutter })
  // Resource return wakes the already registered owner acquisition, even if
  // no new source publication occurs while its Rust offer remains pending.
  run progressOfferCapacityReturnWakeTest = init.then(configureCapacity(1)).then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(3, 2)).then(receiveOffer(3)).then(notify(2)).then(clearWake(2))
    .then(selectOwner(2, 2)).then(reserve(2)).then(park(2)).then(releaseReservation(2))
    .then(all { assert(s.wakeOwed.contains(2)), assert(progressSafety), stutter })
    .then(enterOffer(3)).then(close).then(readyService(3, false)).then(complete(3))
    .then(finishProducer(1)).then(finishProducer(2)).then(join).then(all { assert(progressSafety), stutter })
  run progressOfferLostWakeMutationTest = init.then(stagePool(3)).then(registerProducer(1)).then(registerProducer(2))
    .then(enablePool).then(offerProgress(3, 2)).then(notify(2)).then(clearWake(2)).then(unsafePark(2))
    .then(all { assert(not(progressSafety)), stutter })
  run progressOfferCreditMutationTest = init.then(stagePool(3)).then(registerProducer(1)).then(registerProducer(2))
    .then(enablePool).then(offerProgress(3, 2)).then(unsafeOfferHoldCredit(3))
    .then(all { assert(not(progressSafety)), stutter })
  run progressOfferStaleSlotMutationTest = init.then(stagePool(3)).then(registerProducer(1)).then(registerProducer(2))
    .then(enablePool).then(offerProgress(3, 2)).then(unsafeRetainOfferSlot(3))
    .then(all { assert(not(progressSafety)), stutter })
  run progressStaleOfferMutationTest = init.then(stagePool(3))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool)
    .then(offerProgress(3, 2)).then(receiveOffer(3)).then(withdrawOffer(3)).then(unsafeStaleOfferEntry(3))
    .then(all { assert(not(progressSafety)), stutter })
  run progressShortPromotionTest = init.then(admit(1)).then(submitService(1))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool).then(offerShort(3))
    .then(armShort(1)).then(armShort(2)).then(progressShortWake)
    .then(all { assert(s.shortSignalled == Set(2)), assert(progressSafety), stutter })
    .then(resumeShort(2)).then(armShort(2)).then(progressShortWake)
    .then(resumeShort(2)).then(armShort(2)).then(progressShortWake)
    .then(all { assert(s.shortSignalled == Set(1)), assert(progressSafety), stutter })
    .then(claimPool(3, 2)).then(submitService(3)).then(close)
    .then(readyService(3, false)).then(complete(3)).then(readyService(1, false)).then(complete(1))
    .then(finishProducer(1)).then(finishProducer(2)).then(join).then(all { assert(progressSafety), stutter })
  run progressReleaseMutationTest = init.then(admit(1)).then(submitService(1)).then(reportProgress(1, 0))
    .then(unsafeProgressRelease(1)).then(all { assert(not(progressSafety)), stutter })
  action shortPoolStep = { nondet r = JOBS.oneOf(); any { poolStep, offerShort(r) } }
  run dynamicShortClaimAndCloseTest = init.then(configureCapacity(1)).then(admit(1))
    .then(registerProducer(1)).then(registerProducer(2)).then(enablePool).then(offerShort(3))
    .then(all { assert(not(s.workHeld.contains(3))), assert(s.committed.get(3) == 0), assert(safety), stutter })
    .then(claimPool(3, 2)).then(submitService(3)).then(observeTimeout(3)).then(close)
    .then(all { assert(ownerOf(3) == 2), assert(s.serviceRoots == Set(3)), assert(safety), stutter })
    .then(readyService(3, false)).then(complete(3)).then(complete(1)).then(finishProducer(1)).then(finishProducer(2)).then(join)
    .then(all { assert(s.serviceRoots.size() == 0), assert(safety), stutter })
  action streamStep = any { step, configureProducer(1), configureProducer(2) }
  action producerMutationInit = s' = { ...initial, producerLimit: 1,
    producers: Set(1), producerJobs: Set(1, 3), staged: Set(3), accepted: Set(1),
    committed: JOBS.mapBy(r => if (r == 1) ASSIGNED.get(r) else 0),
    capacityHeld: Set(1), workHeld: Set(1), inbox: Set(1), wakeOwed: Set(1) }
  action unsafeProducerActiveStep = {
    nondet r = JOBS.oneOf()
    any { streamStep, admitUnchecked(r) }
  }
  run serviceProducerWindowTest = init.then(configureProducer(1)).then(registerProducer(1))
    .then(stage(1)).then(stage(3)).then(admit(1)).then(submitService(1))
    .then(all { assert(not(canProducerEnter(3))), assert(s.workHeld.contains(1)), assert(safety), stutter })
    .then(readyService(1, false)).then(complete(1)).then(admit(3)).then(complete(3)).then(finishProducer(1))
    .then(all { assert(s.producers.size() == 0), assert(safety), stutter })
  run serviceProducerCancelDrainTest = init.then(registerProducer(1)).then(stage(1)).then(stage(3))
    .then(admit(1)).then(submitService(1)).then(reserveWork(3)).then(cancelProducer(1))
    .then(all { assert(s.producers.contains(1)), assert(s.staged.size() == 0), assert(s.serviceRoots.contains(1)), assert(s.workHeld == Set(1)), assert(safety), stutter })
    .then(close).then(readyService(1, true)).then(complete(1)).then(finishProducer(1)).then(join)
    .then(all { assert(safety), stutter })
  run serviceProducerWindowMutationTest = init.then(configureProducer(1)).then(registerProducer(1))
    .then(stage(1)).then(stage(3)).then(admit(1)).then(admitUnchecked(3))
    .then(all { assert(not(safety)), stutter })
  action reselectionStep = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    any { step, selectOwner(r, owner) }
  }
  action unsafeOwnerMoveStep = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    any { reselectionStep, unsafeOwnerMove(r, owner) }
  }
  run freeOwnerReselectionTest = init.then(configureCapacity(1)).then(admit(1))
    .then(all { assert(not(hasCapacity(3))), stutter })
    .then(selectOwner(3, 2)).then(reserve(3)).then(admitReserved(3)).then(complete(3))
    .then(all { assert(ownerOf(3) == 2), assert(s.capacityHeld == Set(1)), assert(safety), stutter })
  run reselectedReservationCancelTest = init.then(selectOwner(3, 2)).then(reserve(3))
    .then(releaseReservation(3)).then(selectOwner(3, 1))
    .then(all { assert(s.workHeld.size() == 0), assert(s.capacityHeld.size() == 0), assert(safety), stutter })
  run reselectedCloseDrainTest = init.then(selectOwner(3, 2)).then(reserve(3)).then(close)
    .then(releaseReservation(3)).then(join).then(all { assert(safety), stutter })
  run committedOwnerMutationTest = init.then(selectOwner(3, 2)).then(reserve(3)).then(admitReserved(3))
    .then(unsafeOwnerMove(3, 1)).then(all { assert(not(safety)), stutter })

}
module owner_lifecycle_small {
  import owner_lifecycle(JOBS = Set(1, 2, 3), ASSIGNED = Set(1, 2, 3).mapBy(r => if (r == 2) 2 else 1)).*
}
module owner_lifecycle_wide {
  import owner_lifecycle(JOBS = Set(1, 2, 3, 4, 5), ASSIGNED = Set(1, 2, 3, 4, 5).mapBy(r => if (Set(2, 4).contains(r)) 2 else 1)).*
}

// Refinement of the existing lifecycle, not another root/capacity model.
// Two single-processor VMs, two persistent Scheme workers each. Ticks are
// abstract delay observations, NOT milliseconds. OS/GC progress is external.
module service_sharing {
  const JOBS: Set[int]
  const ASSIGNED: int -> int
  import owner_lifecycle(JOBS = JOBS, ASSIGNED = ASSIGNED) as life
  val owners = Set(1, 2)
  val workers = Set(1, 2, 3, 4)
  val shortJobs = Set(3, 4)
  def workerOwner(w: int): int = if (w <= 2) 1 else 2
  // Invocation state refines the same CPU/root lifecycle. Metadata is immutable
  // logical (epoch,bound), not a claim of one physical allocation per cold race.
  val activationIds = Set(1,2,3)
  type ActivationState = {
    enabled: bool, cycle: bool, turn: bool, turnUsed: int, turnSpent: int, turnPermit: int, turnOpportunity: bool, turnSuspended: bool, accrued: int, settled: int, policy: int, maximum: int, metadata: int, metadataEpochs: Set[int],
    selected: Set[int], stack: List[int], published: Set[int], spans: int -> int, bounds: int -> int, epochs: int -> int,
    normal: Set[int], reported: Set[int], returns: int -> int, credits: int -> int, work: int, reports: int
  }
  val emptyActivations: ActivationState = {
    enabled: false, cycle: false, turn: false, turnUsed: 0, turnSpent: 0, turnPermit: 12, turnOpportunity: false, turnSuspended: false, accrued: 0, settled: 0, policy: 1, maximum: 12, metadata: 0, metadataEpochs: Set(),
    selected: Set(), stack: List(), published: Set(), spans: activationIds.mapBy(_ => 0), bounds: activationIds.mapBy(_ => 0), epochs: activationIds.mapBy(_ => 0),
    normal: Set(), reported: Set(), returns: activationIds.mapBy(_ => 0), credits: activationIds.mapBy(_ => 0), work: 0, reports: 0 }
  type Delivery = {
    enabled: bool, requests: Set[int], queues: int -> List[int], cursor: int,
    consumed: Set[int], armed: bool, wakeOwed: bool, dropped: bool,
    preparationRemaining: int, preparationCount: int, readyService: bool, coopAvailable: bool, turnMode: bool, turnChecked: bool, turnMissed: bool, expectedOwner: int, pickedOwner: int
  }
  val emptyDelivery: Delivery = { enabled: false, requests: Set(), queues: owners.mapBy(_ => List()),
    cursor: 1, consumed: Set(), armed: false, wakeOwed: false, dropped: false,
    preparationRemaining: 0, preparationCount: 0, readyService: false, coopAvailable: true, turnMode: false, turnChecked: false, turnMissed: false, expectedOwner: 0, pickedOwner: 0 }
  type ArrivalClock = {mode: Set[int], generation: int -> int, event: int -> int, observed: Set[int]}
  val emptyArrivalClock: ArrivalClock = {mode: Set(), generation: JOBS.mapBy(_ => 0), event: JOBS.mapBy(_ => 0), observed: Set()}
  type Shared = {
    clock: ArrivalClock, delivery: Delivery, activation: ActivationState, host: List[int], queued: List[int], jobs: int -> int, cpu: int -> int,
    left: int -> int, spent: int -> int, lastShort: int -> bool, debt: int -> bool,
    blocked: Set[int], gc: Set[int], paused: Set[int], age: int -> int, overtakes: int -> int,
    feedUsed: int -> int, feedMax: int,
    split: bool, eager: bool, reserved: bool, fairCpu: bool, bounded: bool,
    grantSlice: int -> bool, cacheMode: bool, cacheShort: int -> bool, cacheBulk: int -> bool, planMode: bool, selectedWork: int -> int, workMode: bool, workLeft: int -> int, workSpent: int -> int, workReady: int -> int, workOffered: int -> int,
    retirePublication: bool, publicationAfter: Set[int],
    completionQueued: Set[int], placementRequired: Set[int], placementObserved: Set[int],
    jointAwait: Set[int], receiveArmed: Set[int], clientWakeOwed: Set[int],
    clientProgress: bool, clientPlanned: Set[int], clientDue: Set[int], clientEligible: Set[int], clientTimers: Set[int], clientAfter: int -> int,
    cross: bool, tokioLimit: int, tokioPolling: Set[int], client: int -> str,
    projectedRust: Set[int], replies: Set[int], observedReplies: Set[int], droppedClients: Set[int],
    actorBatch: bool, actorPhase: int -> str, actorRemaining: int -> int, actorLease: int,
    nativeWaiting: Set[int], osSleeping: Set[int], crossViolation: bool,
    nativeScheduler: bool, nativeActorShort: bool, runtimeObserved: bool, runtimeJoint: bool, progressCycle: bool, executed: int -> int, observedExecution: int -> int, readyFirstCycle: bool, executionCheckpoint: bool, directed: bool, handoff: int -> int, signals: Set[int], registration: bool, waiters: Set[int], executionIssued: Set[int], executionPending: Set[int], executionRegistered: Set[int], executionAck: Set[int], executionGrants: Set[int], executionUngrant: Set[int], receiverMode: bool, receiving: Set[int], reclamation: bool, joint: bool, dispatchPending: Set[int], turnOffered: Set[int], runtimeGap: bool,
    completionFair: bool, coalesced: bool, nativeCycle: bool, readyFirst: bool, pipeline: bool, constructing: Set[int], payloads: Set[int], submitCapabilities: Set[int], fillBudget: int -> int, prepared: Set[int], harvested: Set[int], batchSlots: Set[int],
    completionEpoch: int -> int, tokenEpoch: int -> int, completionDue: Set[int],
    entryGap: bool, cpuGap: bool, resourceViolation: bool
  }
  def entries(values: List[int]): Set[int] = values.foldl(Set(), (seen, r) => seen.union(Set(r)))
  def keep(values: List[int], predicate: (int) => bool): List[int] = values.foldl(List(), (kept, r) => if (predicate(r)) kept.append(r) else kept)
  val inboxJobs = life::s.inbox
  val rootedJobs = life::s.serviceRoots
  val readyJobs = life::s.serviceReady
  val completedJobs = life::s.completed
  val capacityJobs = life::s.capacityHeld
  def canSleep(owner: int): bool = life::canPark(owner) and
    (not(q.joint) or not(dispatchDue(owner)) or q.turnOffered.contains(owner))
  def hostWakeLevel(owner: int): bool = life::s.notified.contains(owner)
  def nativeCanPark(owner: int): bool = life::canPark(owner)
  var q: Shared
  action resourceInitWith(split0: bool, eager0: bool, reserved0: bool, cpu0: bool, bounded0: bool, feed0: int): bool = all {
    life::init,
    q' = cacheEvent({ clock: emptyArrivalClock, delivery: emptyDelivery, activation: emptyActivations, host: List(), queued: List(), jobs: workers.mapBy(_ => 0), cpu: owners.mapBy(_ => 0),
      left: JOBS.mapBy(r => life::units(r)), spent: owners.mapBy(_ => 0),
      lastShort: workers.mapBy(_ => false), debt: owners.mapBy(_ => false),
      blocked: Set(), gc: Set(), paused: Set(), age: JOBS.mapBy(_ => 0), overtakes: JOBS.mapBy(_ => 0),
      feedUsed: owners.mapBy(_ => 0), feedMax: feed0,
      split: split0, eager: eager0, reserved: reserved0, fairCpu: cpu0, bounded: bounded0,
      grantSlice: workers.mapBy(_ => false), cacheMode: false, cacheShort: owners.mapBy(_ => false), cacheBulk: owners.mapBy(_ => false), planMode: false, selectedWork: workers.mapBy(_ => 0), workMode: false, workLeft: JOBS.mapBy(r => if (shortJobs.contains(r)) 1 else 12), workSpent: owners.mapBy(_ => 0), workReady: owners.mapBy(_ => 0), workOffered: owners.mapBy(_ => 0),
      retirePublication: false, publicationAfter: Set(),
      completionQueued: Set(), placementRequired: Set(), placementObserved: Set(),
      jointAwait: Set(), receiveArmed: Set(), clientWakeOwed: Set(),
      clientProgress: false, clientPlanned: Set(), clientDue: Set(), clientEligible: Set(), clientTimers: Set(), clientAfter: JOBS.mapBy(_ => 0),
      cross: false, tokioLimit: 1, tokioPolling: Set(), client: JOBS.mapBy(_ => "idle"),
      projectedRust: Set(), replies: Set(), observedReplies: Set(), droppedClients: Set(),
      actorBatch: false, actorPhase: Set(1,2).mapBy(_ => "absent"), actorRemaining: Set(1,2).mapBy(_ => 1), actorLease: 0,
      nativeWaiting: Set(), osSleeping: Set(), crossViolation: false,
      nativeScheduler: false, nativeActorShort: false, runtimeObserved: false, runtimeJoint: false, progressCycle: false, executed: owners.mapBy(_ => 0), observedExecution: owners.mapBy(_ => 0), readyFirstCycle: false, executionCheckpoint: false, directed: false, handoff: owners.mapBy(_ => 0), signals: Set(), registration: false, waiters: Set(), executionIssued: Set(), executionPending: Set(), executionRegistered: Set(), executionAck: Set(), executionGrants: Set(), executionUngrant: Set(), receiverMode: false, receiving: Set(), reclamation: false, joint: false, dispatchPending: Set(), turnOffered: Set(), runtimeGap: false,
      completionFair: false, coalesced: false, nativeCycle: false, readyFirst: false, pipeline: false, constructing: Set(), payloads: Set(), submitCapabilities: Set(), fillBudget: owners.mapBy(_ => 0), prepared: Set(), harvested: Set(), batchSlots: Set(),
      completionEpoch: owners.mapBy(_ => 0), tokenEpoch: owners.mapBy(_ => -1), completionDue: Set(),
      entryGap: false, cpuGap: false, resourceViolation: false })
  }
  action resourceInit = resourceInitWith(false, false, false, false, false, 1)
  action implementationInit = resourceInitWith(true, true, true, false, false, 2)
  action candidateInit = resourceInitWith(true, true, true, true, true, 2)
  action mutationInit = resourceInitWith(true, true, false, true, true, 1)
  action policy(split: bool, eager: bool, reserved: bool, fairCpu: bool, bounded: bool, feed: int): bool = all {
    life::s.accepted.size() == 0, feed > 0, feed <= JOBS.size(), life::stutter,
    q' = cacheEvent({ ...q, split: split, eager: eager, reserved: reserved, fairCpu: fairCpu, bounded: bounded, feedMax: feed })
  }
  def ownPending(owner: int): Set[int] = life::s.accepted.exclude(life::s.completed).filter(r => ASSIGNED.get(r) == owner)
  def classSpace(r: int): bool = not(q.reserved) or
    ownPending(ASSIGNED.get(r)).filter(j => shortJobs.contains(j) == shortJobs.contains(r)).size() < 1
  action offer(r: int): bool = all {
    clientCanSubmit(r), classSpace(r), life::admit(r), q' = cacheEvent({ ...q, host: q.host.append(r) })
  }
  def hostHead(owner: int): List[int] = keep(q.host, r => ASSIGNED.get(r) == owner)
  def ownerFree(owner: int): bool = (owner != ASSIGNED.get(1) or q.actorLease == 0) and q.cpu.get(owner) == 0 and not(q.gc.contains(owner)) and not(q.paused.contains(owner))
  def canFeed(r: int): bool = and {
    entries(q.host).contains(r),
    ownerFree(ASSIGNED.get(r)) or (q.planMode and shortJobs.contains(r) and runtimeAvailable(ASSIGNED.get(r))),
    hostHead(ASSIGNED.get(r)).head() == r,
    q.feedUsed.get(ASSIGNED.get(r)) < q.feedMax
  }
  action feed(r: int): bool = all {
    canFeed(r), not(q.readyFirst) or q.batchSlots.filter(j => ASSIGNED.get(j) == ASSIGNED.get(r)).size() == 0,
    life::submitService(r),
    q' = cacheEvent({ ...q, host: keep(q.host, j => j != r), queued: q.queued.append(r),
      executionIssued: if (q.registration and shortJobs.contains(r)) q.executionIssued.union(Set(r)) else q.executionIssued,
      executionPending: if (q.registration and shortJobs.contains(r)) q.executionPending.union(Set(r)) else q.executionPending,
      dispatchPending: if (q.joint and shortJobs.contains(r)) q.dispatchPending.union(Set(r)) else q.dispatchPending,
      turnOffered: if (q.joint and shortJobs.contains(r)) q.turnOffered.exclude(Set(ASSIGNED.get(r))) else q.turnOffered,
      feedUsed: q.feedUsed.set(ASSIGNED.get(r), q.feedUsed.get(ASSIGNED.get(r)) + 1) })
  }
  def feedShortFirst(owner: int): bool = {
    val pending = hostHead(owner)
    pending.length() > 0 and shortJobs.contains(pending.head()) and q.feedUsed.get(owner) < q.feedMax
  }
  action boundary(owner: int): bool = all {
    ownerFree(owner), not(q.eager and feedShortFirst(owner)), life::stutter,
    q' = cacheEvent({ ...q, feedUsed: q.feedUsed.set(owner, 0), entryGap: q.entryGap or feedShortFirst(owner) })
  }
  def eligible(w: int): List[int] = {
    val queue = keep(q.queued, r => ASSIGNED.get(r) == workerOwner(w))
    val shorts = keep(queue, r => shortJobs.contains(r))
    val bulk = keep(queue, r => not(shortJobs.contains(r)))
    if (not(q.split)) queue else
      if (q.lastShort.get(w) and bulk.length() > 0) bulk else
      if (shorts.length() > 0) shorts else bulk
  }
  action assignWorker(w: int, r: int): bool = all {
    (ownerFree(workerOwner(w)) or (q.planMode and runtimeAvailable(workerOwner(w)))), q.jobs.get(w) == 0,
    eligible(w).length() > 0, eligible(w).head() == r, life::stutter,
    q' = cacheEvent({ ...q, queued: keep(q.queued, j => j != r), jobs: q.jobs.set(w, r), lastShort: q.lastShort.set(w, shortJobs.contains(r)) })
  }
  def runnable(w: int): bool = q.jobs.get(w) != 0 and not(q.blocked.contains(w))
  def classRunnable(owner: int, short: bool): Set[int] = workers.filter(w =>
    workerOwner(w) == owner and runnable(w) and (not(q.registration) or q.waiters.contains(w)) and shortJobs.contains(q.jobs.get(w)) == short)
  def preferShortCpu(owner: int): bool = classRunnable(owner, true).size() > 0 and
    (not(q.debt.get(owner)) or classRunnable(owner, false).size() == 0)
  def reclamationGrantable(owner: int): bool = not(q.reclamation) or life::s.phase != "open" or
    (q.receiverMode and not(q.receiving.contains(owner))) or q.workReady.get(owner) == q.workOffered.get(owner)
  def allowedCpu(w: int): bool = reclamationGrantable(workerOwner(w)) and (not(q.fairCpu) or
    (if (preferShortCpu(workerOwner(w))) shortJobs.contains(q.jobs.get(w)) else
      (classRunnable(workerOwner(w), false).size() == 0 or not(shortJobs.contains(q.jobs.get(w))))))
  // Selection owns one wake token; receiving the token is a separate event.
  // A later class arrival cannot revoke an already selected native grant.
  action enableDirected = all {
    q.registration, life::s.accepted.size() == 0, life::stutter, q' = { ...q, directed: true }
  }
  action selectWaiter(w: int): bool = all {
    q.directed, ownerFree(workerOwner(w)), q.handoff.get(workerOwner(w)) == 0,
    q.waiters.contains(w), allowedCpu(w), life::stutter,
    q' = { ...q, handoff: q.handoff.set(workerOwner(w), w), signals: q.signals.union(Set(w)) }
  }
  action spuriousWake(w: int): bool = all {
    q.directed, q.waiters.contains(w), q.handoff.get(workerOwner(w)) != w,
    life::stutter, q' = q
  }
  action unsafeForeignWake(w: int): bool = all {
    q.directed, q.waiters.contains(w), q.handoff.get(workerOwner(w)) != w,
    life::stutter, q' = { ...q, signals: q.signals.union(Set(w)) }
  }
  action acquireCpu(w: int): bool = all {
    ownerFree(workerOwner(w)), runnable(w), (not(q.registration) or q.waiters.contains(w)),
    (if (q.directed) q.handoff.get(workerOwner(w)) == w and q.signals.contains(w) else allowedCpu(w)), life::stutter,
    q' = cacheEvent({ ...q, handoff: q.handoff.set(workerOwner(w), 0), signals: q.signals.exclude(Set(w)), waiters: q.waiters.exclude(Set(w)),
      executionPending: if (q.registration) q.executionPending.exclude(Set(q.jobs.get(w))) else q.executionPending,
      executionAck: if (q.registration and shortJobs.contains(q.jobs.get(w))) q.executionAck.union(Set(q.jobs.get(w))) else q.executionAck,
      executionGrants: if (q.registration and shortJobs.contains(q.jobs.get(w))) q.executionGrants.union(Set(q.jobs.get(w))) else q.executionGrants,
      grantSlice: q.grantSlice.set(w, q.cacheMode and cachedDemand(w)), workSpent: q.workSpent.set(workerOwner(w), 0), cpu: q.cpu.set(workerOwner(w), w), spent: q.spent.set(workerOwner(w), 0),
      debt: q.debt.set(workerOwner(w), shortJobs.contains(q.jobs.get(w))),
      overtakes: JOBS.mapBy(r => if (r == q.jobs.get(w)) 0 else
        if (not(shortJobs.contains(q.jobs.get(w))) and classRunnable(workerOwner(w), true).exists(sw => q.jobs.get(sw) == r))
          (if (q.overtakes.get(r) < 2) q.overtakes.get(r) + 1 else 2) else q.overtakes.get(r)),
      cpuGap: q.cpuGap or (not(q.directed) and preferShortCpu(workerOwner(w)) and not(shortJobs.contains(q.jobs.get(w)))) })
  }
  def newCompletionOpportunity(r: int): bool = q.coalesced and
    (not(q.completionDue.contains(ASSIGNED.get(r))) or
      (shortJobs.contains(r) and readyJobs.exclude(q.harvested).filter(j =>
        ASSIGNED.get(j) == ASSIGNED.get(r) and shortJobs.contains(j)).size() == 0))
  // One executed span is four records; admission/item units remain unchanged.
  action enableWork = all {
    life::s.accepted.size() == 0, q.joint, life::stutter,
    q' = cacheEvent({ ...q, workMode: true, bounded: false, fairCpu: true })
  }
  def workContended(w: int): bool = {
    val owner = workerOwner(w)
    val short = shortJobs.contains(q.jobs.get(w))
    classRunnable(owner, not(short)).size() > 0 or
      (not(short) and (if (q.registration) q.executionPending else q.dispatchPending).exists(r => ASSIGNED.get(r) == owner)) or
      ((not(q.receiverMode) or q.receiving.contains(owner)) and q.workReady.get(owner) > q.workOffered.get(owner))
  }
  // One selected fast span can cover 12 four-record units. Selection is a
  // Scheme action before computation; late input can arrive between the two.
  // Event-owned class permission, independent of any already-selected span.
  // This pure next-state function refines atomic queue/dispatch/ready changes;
  // it makes no statement about runtime instruction or allocation costs.
  def cacheDemand(state: Shared, owner: int, short: bool): bool =
    workers.exists(w => workerOwner(w) == owner and state.jobs.get(w) != 0 and
      not(state.blocked.contains(w)) and (not(state.registration) or state.waiters.contains(w)) and shortJobs.contains(state.jobs.get(w)) != short) or
    (not(short) and (if (state.registration) state.executionPending else state.dispatchPending).exists(r => ASSIGNED.get(r) == owner)) or
    ((not(state.receiverMode) or state.receiving.contains(owner)) and state.workReady.get(owner) > state.workOffered.get(owner))
  def cacheEvent(state: Shared): Shared = if (not(state.cacheMode)) state else {
    ...state, cacheShort: owners.mapBy(o => cacheDemand(state, o, true)),
    cacheBulk: owners.mapBy(o => cacheDemand(state, o, false)) }
  action enableCache = all {
    q.planMode, life::s.accepted.size() == 0, life::stutter,
    q' = cacheEvent({ ...q, cacheMode: true })
  }
  def cachedDemand(w: int): bool =
    if (shortJobs.contains(q.jobs.get(w))) q.cacheShort.get(workerOwner(w)) else q.cacheBulk.get(workerOwner(w))
  action enablePlans = all {
    q.workMode, life::s.accepted.size() == 0, life::stutter,
    q' = cacheEvent({ ...q, planMode: true })
  }
  def canSelectPlan(w: int): bool = and {
    q.planMode, q.cpu.get(workerOwner(w)) == w, runnable(w),
    runtimeAvailable(workerOwner(w)), q.selectedWork.get(w) == 0,
    q.workSpent.get(workerOwner(w)) == 0 or not(workContended(w))
  }
  action selectPlan(w: int): bool = all {
    canSelectPlan(w), life::stutter,
    q' = cacheEvent({ ...q, selectedWork: q.selectedWork.set(w,
        if (if (q.cacheMode) q.grantSlice.get(w) else workContended(w)) 1 else q.workLeft.get(q.jobs.get(w))),
      workSpent: q.workSpent.set(workerOwner(w), 0) })
  }
  def selectedCost(w: int): int = if (q.planMode) q.selectedWork.get(w) else 1
  def canCompute(w: int): bool = and {
    not(q.cross and q.actorBatch and q.jobs.get(w) == 1),
    q.cpu.get(workerOwner(w)) == w, runnable(w), not(q.dispatchPending.contains(q.jobs.get(w))),
    not(q.planMode) or q.selectedWork.get(w) > 0,
    not(q.gc.contains(workerOwner(w))), not(q.paused.contains(workerOwner(w))),
    not(q.bounded) or q.spent.get(workerOwner(w)) < 1,
    not(q.workMode) or q.workSpent.get(workerOwner(w)) < 1 or not(workContended(w)),
    not(q.coalesced and q.left.get(q.jobs.get(w)) == 1 and newCompletionOpportunity(q.jobs.get(w)))
      or q.completionEpoch.get(workerOwner(w)) < 2
  }
  action compute(w: int): bool = {
    val r = q.jobs.get(w)
    all {
      canCompute(w), q.left.get(r) > 0,
      if (q.workMode and q.workLeft.get(r) > selectedCost(w)) all {
        life::stutter,
        q' = cacheEvent({ ...q, executed: q.executed.set(workerOwner(w), q.executed.get(workerOwner(w)) + selectedCost(w)), workLeft: q.workLeft.set(r, q.workLeft.get(r) - selectedCost(w)),
          workSpent: q.workSpent.set(workerOwner(w), 1) })
      } else if (q.left.get(r) == 1) all {
        life::readyService(r, false),
        q' = cacheEvent({ ...q, executed: q.executed.set(workerOwner(w), q.executed.get(workerOwner(w)) + selectedCost(w)), selectedWork: q.selectedWork.set(w, 0), workSpent: q.workSpent.set(workerOwner(w), 0),
          workReady: if (q.workMode and shortJobs.contains(r)) q.workReady.set(workerOwner(w), q.workReady.get(workerOwner(w)) + 1) else q.workReady,
          left: q.left.set(r, 0), jobs: q.jobs.set(w, 0), cpu: q.cpu.set(workerOwner(w), 0), spent: q.spent.set(workerOwner(w), 0),
          completionEpoch: if (newCompletionOpportunity(r)) q.completionEpoch.set(workerOwner(w), q.completionEpoch.get(workerOwner(w)) + 1) else q.completionEpoch,
          completionDue: if (q.coalesced) q.completionDue.union(Set(workerOwner(w))) else q.completionDue })
      } else all {
        life::stutter,
        q' = cacheEvent({ ...q, executed: q.executed.set(workerOwner(w), q.executed.get(workerOwner(w)) + selectedCost(w)), selectedWork: q.selectedWork.set(w, 0), workLeft: if (q.workMode) q.workLeft.set(r, if (shortJobs.contains(r)) 1 else 12) else q.workLeft,
          workSpent: q.workSpent.set(workerOwner(w), 1), left: q.left.set(r, q.left.get(r) - 1), spent: q.spent.set(workerOwner(w), q.spent.get(workerOwner(w)) + 1) })
      }
    }
  }
  action yieldCpu(w: int): bool = all {
    q.cpu.get(workerOwner(w)) == w, not(q.paused.contains(workerOwner(w))), not(q.gc.contains(workerOwner(w))),
    not(q.workMode) or workContended(w), life::stutter,
    q' = cacheEvent({ ...q, workSpent: q.workSpent.set(workerOwner(w), 0), workOffered: if (q.reclamation) q.workOffered else q.workOffered.set(workerOwner(w), q.workReady.get(workerOwner(w))),
      cpu: q.cpu.set(workerOwner(w), 0), spent: q.spent.set(workerOwner(w), 0) })
  }
  action block(w: int): bool = all {
    q.cpu.get(workerOwner(w)) == w, not(q.gc.contains(workerOwner(w))), not(q.paused.contains(workerOwner(w))), life::stutter,
    q' = cacheEvent({ ...q, blocked: q.blocked.union(Set(w)), cpu: q.cpu.set(workerOwner(w), 0), spent: q.spent.set(workerOwner(w), 0) })
  }
  action wakeTask(w: int): bool = all { q.blocked.contains(w), life::stutter, q' = cacheEvent({ ...q, blocked: q.blocked.exclude(Set(w)), osSleeping: q.osSleeping.exclude(Set(workerOwner(w))), overtakes: q.overtakes.set(q.jobs.get(w), 0) }) }
  action gcStart(owner: int): bool = all { Set("open", "closing").contains(life::s.phase), not(q.gc.contains(owner)), life::stutter, q' = cacheEvent({ ...q, gc: q.gc.union(Set(owner)) }) }
  action gcFinish(owner: int): bool = all { q.gc.contains(owner), not(q.paused.contains(owner)), life::stutter, q' = cacheEvent({ ...q, gc: q.gc.exclude(Set(owner)) }) }
  action osPause(owner: int): bool = all { not(q.paused.contains(owner)), life::stutter, q' = cacheEvent({ ...q, paused: q.paused.union(Set(owner)) }) }
  action osResume(owner: int): bool = all { q.paused.contains(owner), life::stutter, q' = cacheEvent({ ...q, paused: q.paused.exclude(Set(owner)) }) }
  action delayTick = all {
    life::stutter,
    q' = cacheEvent({ ...q, age: JOBS.mapBy(r => if (shortJobs.contains(r) and life::s.accepted.contains(r) and not(life::s.serviceReady.contains(r)) and not(life::s.completed.contains(r)))
      (if (q.age.get(r) < 4) q.age.get(r) + 1 else 4) else q.age.get(r)) })
  }
  action completionPolicy = all {
    life::s.accepted.size() == 0, life::stutter,
    q' = cacheEvent({ ...q, completionFair: true })
  }
  action completionCoalescedPolicy = all {
    life::s.accepted.size() == 0, life::stutter,
    q' = cacheEvent({ ...q, completionFair: true, coalesced: true })
  }
  def harvestClass(ids: Set[int], short: bool): Set[int] = ids.filter(r => shortJobs.contains(r) == short)
  // The host offers a scheduler opportunity, but only a real Scheme worker
  // acknowledges dispatch. A wake or an empty checkpoint cannot settle debt.
  def dispatchDue(owner: int): bool = q.dispatchPending.exists(r => ASSIGNED.get(r) == owner)
  def checkpointDue(owner: int): bool = dispatchDue(owner) or
    (q.executionCheckpoint and q.executionPending.exists(r => ASSIGNED.get(r) == owner))
  action enableExecutionCheckpoint = all {
    q.registration, life::s.accepted.size() == 0, life::stutter,
    q' = { ...q, executionCheckpoint: true }
  }
  def shortReady(owner: int): bool = readyJobs.exists(r => ASSIGNED.get(r) == owner and shortJobs.contains(r))
  action enableJoint = all {
    q.pipeline, life::s.phase == "open", life::s.accepted.size() == 0,
    life::stutter, q' = cacheEvent({ ...q, joint: true })
  }
  def stalledReady(owner: int): bool = q.progressCycle and
    rootedJobs.exclude(readyJobs).exists(r => ASSIGNED.get(r) == owner) and
    q.executed.get(owner) == q.observedExecution.get(owner)
  action enableProgressCycle = all {
    q.readyFirstCycle, not(q.nativeScheduler), not(q.runtimeObserved), life::s.accepted.size() == 0, life::stutter, q' = { ...q, progressCycle: true }
  }
  // Reuse actual worker blocking and lease registration, not work-credit totals.
  def runtimeRunnable(owner: int, short: bool): bool = rootedJobs.exclude(readyJobs).exists(r =>
    ASSIGNED.get(r) == owner and shortJobs.contains(r) == short and
    not(workers.exists(w => q.jobs.get(w) == r and q.blocked.contains(w)))) or
    (q.cross and q.actorBatch and not(short) and owner == ASSIGNED.get(1) and actorRunnable)
  // Cold diagnostic projection: grants and waiters already have owners. No
  // mirrored per-acquisition ledger or new lease lifecycle is introduced.
  def runtimeBudgetPhase(r: int): str =
    if (readyJobs.contains(r)) "ready" else
    if (q.cross and q.actorBatch and r == 1) "actor-barrier" else
    if (workers.exists(w => q.jobs.get(w) == r and q.blocked.contains(w))) "external" else
    if (q.waiters.exists(w => q.jobs.get(w) == r)) "lease-wait" else
    if (owners.exists(o => q.cpu.get(o) != 0 and q.jobs.get(q.cpu.get(o)) == r)) "received" else "eligible"
  val runtimeBudgetProjection = and {
    q.waiters.forall(w => runtimeBudgetPhase(q.jobs.get(w)) == "lease-wait"),
    owners.forall(o => implies(q.cpu.get(o) != 0,
      runtimeBudgetPhase(q.jobs.get(q.cpu.get(o))) == "received")),
    workers.forall(w => implies(q.blocked.contains(w) and q.jobs.get(w) != 0 and not(q.cross and q.actorBatch and q.jobs.get(w) == 1),
      runtimeBudgetPhase(q.jobs.get(w)) == "external"))
  }
  // Direct observations of existing queues, worker blocking and Actor phases.
  // A worker not blocked by Scheme is running or waiting for a processor;
  // waiting for a processor must not be classified as external I/O.
  def nativeShortEligible(owner: int): bool =
    entries(q.queued).exists(r => ASSIGNED.get(r) == owner and shortJobs.contains(r)) or
    workers.exists(w => workerOwner(w) == owner and q.jobs.get(w) != 0 and
      shortJobs.contains(q.jobs.get(w)) and not(q.blocked.contains(w))) or
    (q.nativeActorShort and q.actorBatch and q.jobs.get(1) == 1 and owner == ASSIGNED.get(1) and
      (actorRunnable or actorAllDone))
  def nativeNeedsPass(owner: int): bool = not(shortReady(owner)) and
    (not(readyJobs.exists(r => ASSIGNED.get(r) == owner)) or nativeShortEligible(owner))
  action enableNativeScheduler = all {
    q.readyFirstCycle, not(q.runtimeObserved), not(q.progressCycle), life::s.accepted.size() == 0,
    life::stutter, q' = { ...q, nativeScheduler: true }
  }
  // Source class metadata is independent of root/capacity accounting. It
  // represents a producer-declared short Actor source within the existing job.
  action nativeActorClassShort = all {
    q.nativeScheduler, q.actorBatch, q.jobs.get(1) == 1, life::stutter,
    q' = { ...q, nativeActorShort: true }
  }
  action unsafeNativeHarvest(owner: int): bool = all {
    q.nativeScheduler, nativeNeedsPass(owner), not(q.turnOffered.contains(owner)),
    readyJobs.exists(r => ASSIGNED.get(r) == owner), life::stutter,
    q' = { ...q, runtimeGap: true }
  }
  def runtimeNeedsPass(owner: int): bool = not(shortReady(owner)) and
    (runtimeRunnable(owner,true) or (not(readyJobs.exists(r => ASSIGNED.get(r) == owner)) and runtimeRunnable(owner,false)))
  action enableRuntimeObservation = all {
    q.readyFirstCycle, not(q.nativeScheduler), not(q.progressCycle), life::s.accepted.size() == 0, life::stutter, q' = { ...q, runtimeObserved: true }
  }
  action enableRuntimeJoint = all {
    q.runtimeObserved, life::s.accepted.size() == 0, life::stutter, q' = { ...q, runtimeJoint: true }
  }
  def cycleCanPass(owner: int): bool = q.readyFirstCycle and runtimeAvailable(owner) and
    (if (q.nativeScheduler) nativeNeedsPass(owner) else if (q.runtimeJoint) runtimeNeedsPass(owner) else
      not(readyJobs.exists(r => ASSIGNED.get(r) == owner)) or stalledReady(owner)) and not(q.turnOffered.contains(owner))
  action enableReadyFirstCycle = all {
    q.executionCheckpoint, life::s.accepted.size() == 0, life::stutter, q' = { ...q, readyFirstCycle: true }
  }
  action cyclePass(owner: int): bool = all {
    cycleCanPass(owner), life::stutter, q' = { ...q, turnOffered: q.turnOffered.union(Set(owner)) }
  }
  action unsafeRuntimeHarvest(owner: int): bool = all {
    q.runtimeJoint, runtimeNeedsPass(owner), not(q.turnOffered.contains(owner)),
    readyJobs.exists(r => ASSIGNED.get(r) == owner), life::stutter, q' = { ...q, runtimeGap: true }
  }
  action unsafeProgressHarvest(owner: int): bool = all {
    stalledReady(owner), not(q.turnOffered.contains(owner)), life::stutter, q' = { ...q, runtimeGap: true }
  }
  action unsafeCyclePass(owner: int): bool = all {
    q.readyFirstCycle, runtimeAvailable(owner), not(cycleCanPass(owner)), life::stutter,
    q' = { ...q, runtimeGap: true }
  }
  action jointTurn(owner: int): bool = all {
    q.joint, checkpointDue(owner), not(shortReady(owner)), runtimeAvailable(owner),
    not(q.readyFirstCycle) or cycleCanPass(owner),
    life::stutter, q' = cacheEvent({ ...q, turnOffered: q.turnOffered.union(Set(owner)) })
  }
  action ackDispatch(w: int): bool = all {
    q.dispatchPending.contains(q.jobs.get(w)), runtimeAvailable(workerOwner(w)),
    life::ackServiceDispatch(q.jobs.get(w)), q' = cacheEvent({ ...q, dispatchPending: q.dispatchPending.exclude(Set(q.jobs.get(w))) })
  }
  action unsafeJointHarvest(owner: int): bool = all {
    q.joint, not(q.readyFirstCycle), checkpointDue(owner), not(shortReady(owner)), not(q.turnOffered.contains(owner)),
    readyJobs.exists(r => ASSIGNED.get(r) == owner), life::stutter,
    q' = cacheEvent({ ...q, runtimeGap: true })
  }
  action emptyHarvest(owner: int): bool = all {
    runtimeAvailable(owner), not(readyJobs.exists(r => ASSIGNED.get(r) == owner)),
    not(q.readyFirstCycle) or q.turnOffered.contains(owner) or (q.runtimeJoint and not(runtimeNeedsPass(owner))),
    life::stutter, q' = cacheEvent({ ...q, turnOffered: q.turnOffered.exclude(Set(owner)), observedExecution: q.observedExecution.set(owner,q.executed.get(owner)) })
  }
  action harvest(owner: int, ids: Set[int]): bool = all {
    not(q.joint) or not(shortReady(owner)) or ids.exists(r => shortJobs.contains(r)),
    q.readyFirstCycle or not(q.joint) or not(checkpointDue(owner)) or shortReady(owner) or q.turnOffered.contains(owner),
    not(stalledReady(owner)) or q.turnOffered.contains(owner),
    not(q.runtimeJoint) or not(runtimeNeedsPass(owner)) or q.turnOffered.contains(owner),
    not(q.nativeScheduler) or not(nativeNeedsPass(owner)) or q.turnOffered.contains(owner),
    runtimeAvailable(owner), q.completionFair, ids.size() > 0, ids.subseteq(readyJobs),
    ids.forall(r => ASSIGNED.get(r) == owner),
    q.harvested.filter(r => ASSIGNED.get(r) == owner).size() == 0,
    harvestClass(ids, true).size() <= 1, q.coalesced or harvestClass(ids, false).size() <= 1,
    not(q.coalesced) or (q.tokenEpoch.get(owner) == -1 and q.completionDue.contains(owner)),
    life::stutter, q' = cacheEvent({ ...q,
      publicationAfter: if (q.retirePublication) q.publicationAfter.union(ids) else q.publicationAfter,
      workOffered: if (readyJobs.exclude(q.harvested.union(ids)).filter(r => ASSIGNED.get(r) == owner and shortJobs.contains(r)).size() == 0)
        q.workOffered.set(owner, q.workReady.get(owner)) else q.workOffered,
      observedExecution: q.observedExecution.set(owner,q.executed.get(owner)), harvested: q.harvested.union(ids), turnOffered: q.turnOffered.exclude(Set(owner)),
      tokenEpoch: if (q.coalesced) q.tokenEpoch.set(owner, q.completionEpoch.get(owner)) else q.tokenEpoch })

  }
  action completionAck(owner: int, epoch: int): bool = all {
    q.coalesced, epoch >= 0, q.tokenEpoch.get(owner) == epoch,
    q.harvested.filter(r => ASSIGNED.get(r) == owner).size() == 0,
    not(q.completionEpoch.get(owner) == epoch and readyJobs.filter(r => ASSIGNED.get(r) == owner).size() > 0)
      or epoch < 2,
    life::stutter, q' = cacheEvent({ ...q, tokenEpoch: q.tokenEpoch.set(owner, -1),
      completionEpoch: if (q.completionEpoch.get(owner) == epoch and readyJobs.filter(r => ASSIGNED.get(r) == owner).size() > 0)
        q.completionEpoch.set(owner, epoch + 1) else q.completionEpoch,
      completionDue: if (q.completionEpoch.get(owner) == epoch and
        readyJobs.filter(r => ASSIGNED.get(r) == owner).size() == 0)
          q.completionDue.exclude(Set(owner)) else q.completionDue })
  }
  action completionIgnoreOldAck(owner: int, epoch: int): bool = all {
    q.coalesced, epoch >= 0, epoch != q.tokenEpoch.get(owner), life::stutter, q' = q
  }
  action unsafeStaleCompletionAck(owner: int): bool = all {
    q.coalesced, q.tokenEpoch.get(owner) >= 0,
    q.tokenEpoch.get(owner) < q.completionEpoch.get(owner), life::stutter,
    q' = cacheEvent({ ...q, tokenEpoch: q.tokenEpoch.set(owner, -1), completionDue: q.completionDue.exclude(Set(owner)) })
  }
  def canServiceControl(owner: int): bool = q.tokenEpoch.get(owner) == -1 and
    q.batchSlots.filter(r => ASSIGNED.get(r) == owner).size() == 0
  action enableNativeCycle = all { life::s.accepted.size() == 0, life::stutter, q' = cacheEvent({ ...q, nativeCycle: true }) }
  action enablePreparedCycle = all { q.nativeCycle, life::stutter, q' = cacheEvent({ ...q, readyFirst: true }) }
  action enablePipeline = all { q.readyFirst, life::stutter, q' = cacheEvent({ ...q, pipeline: true }) }
  action endFill(owner: int): bool = all { life::stutter, q' = cacheEvent({ ...q, fillBudget: q.fillBudget.set(owner, 0) }) }
  // A notification carries no request, reservation or root. It consumes one
  // bounded owner turn rather than discarding all remaining refill opportunity.
  // One owner-local native entry capability; application inputs never become
  // reusable buffers. Revocation does not refund or move request reservations.
  action acquireSubmitCapability(owner: int): bool = all {
    q.pipeline, life::s.phase == "open", not(q.submitCapabilities.contains(owner)),
    rootedJobs.filter(r => ASSIGNED.get(r) == owner).size() == 0,
    life::stutter, q' = cacheEvent({ ...q, submitCapabilities: q.submitCapabilities.union(Set(owner)) })
  }
  action revokeSubmitCapability(owner: int): bool = all {
    life::stutter, q' = cacheEvent({ ...q, submitCapabilities: q.submitCapabilities.exclude(Set(owner)) })
  }
  // A foreign call executes on the same VM processor, but the Scheme host
  // continuation may be suspended while other Scheme workers run. These stages
  // are NOT elapsed-time or GC-CPU counters. A prepared Rust input has no root;
  // construction introduces transient owner roots, before service acceptance.
  def runtimeAvailable(owner: int): bool = not(q.osSleeping.contains(owner)) and not(q.gc.contains(owner)) and not(q.paused.contains(owner))
  action beginConstruction(r: int): bool = all {
    life::s.phase == "open", q.prepared.contains(r), runtimeAvailable(ASSIGNED.get(r)),
    not(q.constructing.contains(r)), not(q.payloads.contains(r)),
    q.batchSlots.filter(j => ASSIGNED.get(j) == ASSIGNED.get(r)).size() == 0,
    life::stutter, q' = cacheEvent({ ...q, constructing: q.constructing.union(Set(r)) })
  }
  action finishConstruction(r: int): bool = all {
    q.constructing.contains(r), runtimeAvailable(ASSIGNED.get(r)),
    life::stutter, q' = cacheEvent({ ...q, constructing: q.constructing.exclude(Set(r)), payloads: q.payloads.union(Set(r)) })
  }
  action payloadEntry(r: int): bool = all {
    q.payloads.contains(r), q.submitCapabilities.contains(ASSIGNED.get(r)), runtimeAvailable(ASSIGNED.get(r)),
    q.batchSlots.filter(j => ASSIGNED.get(j) == ASSIGNED.get(r)).size() == 0,
    not(q.pipeline) or q.fillBudget.get(ASSIGNED.get(r)) > 0,
    life::admitReserved(r), q' = cacheEvent({ ...q, payloads: q.payloads.exclude(Set(r)),
      prepared: q.prepared.exclude(Set(r)), host: q.host.append(r),
      fillBudget: if (q.pipeline) q.fillBudget.set(ASSIGNED.get(r), q.fillBudget.get(ASSIGNED.get(r)) - 1) else q.fillBudget })
  }
  action unsafeGcConstruction(r: int): bool = all {
    q.constructing.contains(r), q.gc.contains(ASSIGNED.get(r)), life::stutter,
    q' = cacheEvent({ ...q, constructing: q.constructing.exclude(Set(r)), payloads: q.payloads.union(Set(r)), resourceViolation: true })
  }
  action nativeEntry(r: int): bool = all {
    q.submitCapabilities.contains(ASSIGNED.get(r)), enterPrepared(r)
  }
  action refillWake(owner: int): bool = all {
    q.pipeline, q.fillBudget.get(owner) > 0, life::stutter,
    q' = cacheEvent({ ...q, fillBudget: q.fillBudget.set(owner, q.fillBudget.get(owner) - 1) })
  }
  action unsafeFillWithoutRetirement(r: int): bool = all {
    q.pipeline, q.prepared.contains(r), q.fillBudget.get(ASSIGNED.get(r)) == 0,
    life::admitReserved(r), q' = cacheEvent({ ...q, prepared: q.prepared.exclude(Set(r)), host: q.host.append(r), resourceViolation: true })
  }
  action prepareInput(r: int): bool = all {
    q.readyFirst, q.prepared.filter(j => ASSIGNED.get(j) == ASSIGNED.get(r)).size() == 0,
    life::reserve(r), q' = cacheEvent({ ...q, prepared: q.prepared.union(Set(r)) })
  }
  action enterPrepared(r: int): bool = all {
    runtimeAvailable(ASSIGNED.get(r)), not(q.constructing.contains(r)), not(q.payloads.contains(r)),
    q.prepared.contains(r), q.batchSlots.filter(j => ASSIGNED.get(j) == ASSIGNED.get(r)).size() == 0,
    not(q.pipeline) or q.fillBudget.get(ASSIGNED.get(r)) > 0,
    life::admitReserved(r), q' = cacheEvent({ ...q, prepared: q.prepared.exclude(Set(r)), host: q.host.append(r),
      fillBudget: if (q.pipeline) q.fillBudget.set(ASSIGNED.get(r), q.fillBudget.get(ASSIGNED.get(r)) - 1) else q.fillBudget })
  }
  action discardPrepared(r: int): bool = all {
    q.prepared.contains(r), runtimeAvailable(ASSIGNED.get(r)), life::releaseReservation(r),
    q' = cacheEvent({ ...q, prepared: q.prepared.exclude(Set(r)), constructing: q.constructing.exclude(Set(r)), payloads: q.payloads.exclude(Set(r)) })
  }
  action unsafePreparedOvertake(r: int): bool = all {
    q.prepared.contains(r), q.batchSlots.filter(j => ASSIGNED.get(j) == ASSIGNED.get(r)).size() > 0,
    life::admitReserved(r), q' = cacheEvent({ ...q, prepared: q.prepared.exclude(Set(r)), host: q.host.append(r), resourceViolation: true })
  }
  action batchTransfer(owner: int): bool = all {
    runtimeAvailable(owner), q.nativeCycle, q.tokenEpoch.get(owner) >= 0,
    q.harvested.filter(r => ASSIGNED.get(r) == owner).size() > 0,
    q.batchSlots.filter(r => ASSIGNED.get(r) == owner).size() == 0,
    life::stutter, q' = cacheEvent({ ...q, batchSlots: q.batchSlots.union(q.harvested.filter(r => ASSIGNED.get(r) == owner)) })
  }
  action unsafeCreditBeforeSlotClear(r: int): bool = all {
    q.nativeCycle, q.batchSlots.contains(r), life::complete(r), q' = q
  }
  action project(r: int): bool = all {
    (not(q.cross) or q.projectedRust.contains(r)),
    runtimeAvailable(ASSIGNED.get(r)), life::s.serviceReady.contains(r), not(q.completionFair) or q.harvested.contains(r),
    not(q.nativeCycle) or q.batchSlots.contains(r),
    life::complete(r), q' = cacheEvent({ ...q, harvested: q.harvested.exclude(Set(r)), batchSlots: q.batchSlots.exclude(Set(r)),
      fillBudget: if (q.pipeline and life::s.phase == "open") q.fillBudget.set(ASSIGNED.get(r), q.fillBudget.get(ASSIGNED.get(r)) + 1) else q.fillBudget })
  }
  action abandon(r: int): bool = all { life::observeTimeout(r), q' = q }
  action resourceClose = all { life::close, q' = cacheEvent({ ...q, reclamation: false, receiverMode: false, receiving: Set(), fillBudget: owners.mapBy(_ => 0) }) }
  action resourceJoin = all {
    q.actorLease == 0, not(q.actorBatch) or Set(1,2).forall(c => q.actorPhase.get(c) == "done"),
    q.dispatchPending.size() == 0, q.executionPending.size() == 0, q.waiters.size() == 0, q.constructing.size() == 0, q.payloads.size() == 0,
    q.gc.size() == 0, q.paused.size() == 0, q.signals.size() == 0, workers.forall(w => q.jobs.get(w) == 0),
    not(q.coalesced) or (q.completionDue.size() == 0 and owners.forall(o => q.tokenEpoch.get(o) == -1)),
    life::join, q' = cacheEvent({ ...q, submitCapabilities: Set(), turnOffered: Set(), nativeWaiting: Set(), osSleeping: Set() })
  }
  action resourceStutter = all { life::stutter, q' = q }
  val resourceSafety = and {
    arrivalClockSafety, deliverySafety, runtimeBudgetProjection, jointReceiveSafety, completionQueueSafety, publicationSafety, clientProgressSafety, crossSafety, activationSafety, q.receiving.subseteq(owners),
    q.signals == owners.map(o => q.handoff.get(o)).exclude(Set(0)),
    q.signals.subseteq(q.waiters),
    owners.forall(o => implies(q.handoff.get(o) != 0,
      q.directed and q.cpu.get(o) == 0 and workerOwner(q.handoff.get(o)) == o)),
    q.waiters.subseteq(workers), q.waiters.forall(w => runnable(w) and q.cpu.get(workerOwner(w)) != w),
    q.executionIssued == q.executionPending.union(q.executionAck).union(q.executionUngrant),
    q.executionPending.intersect(q.executionAck.union(q.executionUngrant)).size() == 0,
    q.executionAck.intersect(q.executionUngrant).size() == 0,
    q.executionAck == q.executionGrants, q.executionGrants.subseteq(q.executionRegistered),
    q.executionRegistered.subseteq(q.executionIssued), q.executionRegistered.intersect(q.dispatchPending).size() == 0,
    q.executionUngrant.intersect(q.executionRegistered).size() == 0,
    implies(q.registration, owners.forall(o => implies(if (q.cpu.get(o) == 0) false else shortJobs.contains(q.jobs.get(q.cpu.get(o))),
      q.executionGrants.contains(q.jobs.get(q.cpu.get(o)))))),
    q.executionIssued.subseteq(shortJobs.intersect(life::s.accepted)),
    q.executionUngrant.subseteq(readyJobs.union(completedJobs)),
    implies(life::s.phase != "open", q.receiving.size() == 0),
    implies(q.reclamation, owners.forall(o =>
      implies(readyJobs.exclude(q.harvested).exists(r => ASSIGNED.get(r) == o and shortJobs.contains(r)),
        q.workReady.get(o) > q.workOffered.get(o)))),
    life::safety, not(q.runtimeGap), q.submitCapabilities.subseteq(owners),
    q.dispatchPending.subseteq(shortJobs.intersect(life::s.servicePending.exclude(readyJobs))),
    owners.forall(o => q.observedExecution.get(o) >= 0 and q.observedExecution.get(o) <= q.executed.get(o)),
    implies(q.nativeScheduler, q.readyFirstCycle and not(q.runtimeObserved) and not(q.progressCycle)),
    implies(q.runtimeJoint,q.runtimeObserved), implies(q.runtimeObserved,q.readyFirstCycle), implies(q.progressCycle,q.readyFirstCycle), q.turnOffered.subseteq(owners), implies(not(q.joint), q.dispatchPending.size() == 0),
    q.constructing.subseteq(q.prepared), q.payloads.subseteq(q.prepared),
    q.constructing.intersect(q.payloads).size() == 0,
    implies(life::s.phase == "joined", q.constructing.size() == 0 and q.payloads.size() == 0),
    implies(life::s.phase == "joined", q.submitCapabilities.size() == 0), entries(q.host) == life::s.inbox,
    q.host.length() == entries(q.host).size(), q.queued.length() == entries(q.queued).size(),
    entries(q.queued).union(workers.map(w => q.jobs.get(w)).exclude(Set(0))) == life::s.servicePending.exclude(life::s.serviceReady),
    workers.map(w => q.jobs.get(w)).filter(r => r != 0).size() == workers.filter(w => q.jobs.get(w) != 0).size(),
    workers.forall(w => implies(q.jobs.get(w) != 0, ASSIGNED.get(q.jobs.get(w)) == workerOwner(w))),
    owners.forall(o => implies(q.cpu.get(o) != 0, workerOwner(q.cpu.get(o)) == o and runnable(q.cpu.get(o)))),
    q.blocked.forall(w => q.jobs.get(w) != 0), q.gc.subseteq(owners), q.paused.subseteq(owners),
    owners.forall(o => q.fillBudget.get(o) >= 0 and
      q.fillBudget.get(o) <= life::s.completed.filter(r => ASSIGNED.get(r) == o).size()),
    q.prepared.subseteq(life::s.reserved), q.prepared.intersect(rootedJobs).size() == 0,
    owners.forall(o => q.prepared.filter(r => ASSIGNED.get(r) == o).size() <= 1),
    q.harvested.subseteq(readyJobs), q.batchSlots.subseteq(q.harvested), q.batchSlots.subseteq(rootedJobs),
    implies(not(q.nativeCycle), q.batchSlots.size() == 0),
    owners.forall(o => and {
      harvestClass(q.harvested.filter(r => ASSIGNED.get(r) == o), true).size() <= 1,
      q.coalesced or harvestClass(q.harvested.filter(r => ASSIGNED.get(r) == o), false).size() <= 1 }),
    implies(q.coalesced, and {
      owners.forall(o => q.completionEpoch.get(o) >= 0 and q.completionEpoch.get(o) <= 2),
      owners.forall(o => implies(readyJobs.filter(r => ASSIGNED.get(r) == o).size() > 0 or q.tokenEpoch.get(o) >= 0,
        q.completionDue.contains(o))),
      q.harvested.forall(r => q.tokenEpoch.get(ASSIGNED.get(r)) >= 0),
      owners.forall(o => q.tokenEpoch.get(o) <= q.completionEpoch.get(o)) }),
    implies(q.cacheMode, owners.forall(o =>
      q.cacheShort.get(o) == cacheDemand(q, o, true) and q.cacheBulk.get(o) == cacheDemand(q, o, false))),
    q.feedMax > 0, owners.forall(o => q.feedUsed.get(o) <= q.feedMax),
    owners.forall(o => q.workSpent.get(o) >= 0 and q.workSpent.get(o) <= 1 and q.workOffered.get(o) <= q.workReady.get(o)),
    workers.forall(w => q.selectedWork.get(w) >= 0 and q.selectedWork.get(w) <= 12 and
      (q.selectedWork.get(w) == 0 or (q.jobs.get(w) != 0 and q.selectedWork.get(w) <= q.workLeft.get(q.jobs.get(w))))),
    JOBS.forall(r => q.workLeft.get(r) >= 1 and q.workLeft.get(r) <= (if (shortJobs.contains(r)) 1 else 12)),
    implies(q.reserved and q.fairCpu, shortJobs.forall(r => q.overtakes.get(r) <= 1)),
    implies(q.eager, not(q.entryGap)), implies(q.fairCpu, not(q.cpuGap)), not(q.resourceViolation)
  }
  // Mutation controls validate the model's guards, not latency improvements.
  action unsafeBoundary(owner: int): bool = all {
    feedShortFirst(owner), life::stutter, q' = cacheEvent({ ...q, entryGap: true, feedUsed: q.feedUsed.set(owner, 0) })
  }
  action unsafeCpu(w: int): bool = all {
    runnable(w), life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(workerOwner(w), w),
      cpuGap: q.cpuGap or (preferShortCpu(workerOwner(w)) and not(shortJobs.contains(q.jobs.get(w)))),
      resourceViolation: q.resourceViolation or q.gc.contains(workerOwner(w)) or q.paused.contains(workerOwner(w))
        or (q.cpu.get(workerOwner(w)) != 0 and q.cpu.get(workerOwner(w)) != w) })
  }
  action unsafeFeed(r: int): bool = all {
    life::submitService(r), q' = cacheEvent({ ...q, host: keep(q.host, j => j != r), queued: q.queued.append(r),
      feedUsed: q.feedUsed.set(ASSIGNED.get(r), q.feedUsed.get(ASSIGNED.get(r)) + 1),
      resourceViolation: q.resourceViolation or not(canFeed(r)) })
  }
  action unsafeCompletionSnapshot(ids: Set[int]): bool = all {
    ids.subseteq(readyJobs), life::stutter, q' = cacheEvent({ ...q, harvested: ids })
  }
  action unsafeWorkSpan(w: int): bool = all {
    q.workMode, q.cpu.get(workerOwner(w)) == w, q.workSpent.get(workerOwner(w)) == 1,
    workContended(w), life::stutter, q' = cacheEvent({ ...q, resourceViolation: true })
  }
  action unsafePlanExtension(w: int): bool = all {
    q.planMode, q.cpu.get(workerOwner(w)) == w, q.selectedWork.get(w) == 0,
    q.workSpent.get(workerOwner(w)) == 1, workContended(w), life::stutter,
    q' = cacheEvent({ ...q, selectedWork: q.selectedWork.set(w, q.workLeft.get(q.jobs.get(w))), resourceViolation: true })
  }
  action unsafeStaleWorkCache = all {
    q.cacheMode, q.cacheBulk.get(1), life::stutter,
    q' = { ...q, cacheBulk: q.cacheBulk.set(1, false) }
  }
  def activeTurnSpan(a: ActivationState): int = entries(a.stack).fold(0, (n, id) => n + a.spans.get(id))
  val activationSafety = and {
    q.activation.published == entries(q.activation.stack),
    q.activation.turnUsed >= q.activation.turnSpent, q.activation.turnSpent >= 0,
    q.activation.turnPermit >= 1 and q.activation.turnPermit <= 12,
    not(q.activation.turnSuspended) or (q.activation.turn and q.cpu.get(1) != 1),
    q.activation.normal.subseteq(q.activation.selected), q.activation.reported.subseteq(q.activation.selected),
    entries(q.activation.stack).subseteq(q.activation.selected),
    q.activation.stack.length() == entries(q.activation.stack).size(),
    q.activation.stack.length() == 0 or q.cpu.get(1) == 1 or q.activation.cycle,
    not(q.activation.cycle) or (q.activation.work == q.activation.accrued + q.activation.settled and
      q.activation.accrued >= 0 and q.activation.settled >= 0 and
      (q.cpu.get(1) != 0 or q.activation.accrued == 0)),
    activationIds.forall(id => q.activation.credits.get(id) <= q.activation.returns.get(id)),
    q.activation.selected.forall(id => q.activation.spans.get(id) > 0 and
      q.activation.spans.get(id) <= q.activation.bounds.get(id) and q.activation.bounds.get(id) <= 12 and
      q.activation.epochs.get(id) >= 1 and q.activation.epochs.get(id) <= q.activation.policy),
    q.activation.reports <= q.activation.selected.filter(id => q.activation.epochs.get(id) == q.activation.policy).size(),
    q.activation.work == activationIds.fold(0, (total, id) => total +
      (if (q.activation.epochs.get(id) == q.activation.policy) q.activation.spans.get(id) * q.activation.credits.get(id) else 0))
  }
  action enableActivations = all {
    q.cacheMode, q.selectedWork.get(1) > 0, life::stutter,
    q' = cacheEvent({ ...q, activation: { ...q.activation, enabled: true } })
  }
  def canEnterActivation(id: int): bool = and {
    q.activation.enabled, q.cpu.get(1) == 1, runtimeAvailable(1),
    not(entries(q.activation.stack).contains(id))
  }
  action enterActivation(id: int): bool = {
    val a = q.activation
    val first = not(a.selected.contains(id))
    val span = if (a.turn) (if (a.turnPermit < a.maximum) a.turnPermit else a.maximum)
      else (if (q.selectedWork.get(1) < a.maximum) q.selectedWork.get(1) else a.maximum)
    val retained = if (first) span else a.spans.get(id)
    all { canEnterActivation(id), span > 0,
      not(a.turn) or (not(a.turnSuspended) and (a.turnUsed + retained <= a.turnPermit or a.turnOpportunity)),
      life::stutter,
      q' = cacheEvent({ ...q, activation: { ...a, selected: a.selected.union(Set(id)), stack: List(id).concat(a.stack), published: a.published.union(Set(id)),
        spans: if (first) a.spans.set(id, span) else a.spans,
        bounds: if (first) a.bounds.set(id, a.maximum) else a.bounds,
        epochs: if (first) a.epochs.set(id, a.policy) else a.epochs,
        metadata: if (first) a.policy else a.metadata,
        metadataEpochs: if (first) a.metadataEpochs.union(Set(a.policy)) else a.metadataEpochs,
        normal: a.normal.exclude(Set(id)),
        turnUsed: a.turnUsed + (if (a.turn) retained else 0),
        turnSpent: a.turnSpent + (if (a.turn) retained else 0), turnOpportunity: false,
        reported: if (a.cycle and first) a.reported.union(Set(id)) else a.reported,
        reports: a.reports + (if (a.cycle and first) 1 else 0) } }) }
  }
  action normalActivation(id: int): bool = {
    val a = q.activation
    val credit = a.cycle and a.epochs.get(id) == a.policy
    all {
      a.stack.length() > 0, a.stack.head() == id, q.cpu.get(1) == 1, runtimeAvailable(1),
      not(a.normal.contains(id)), a.returns.get(id) < 3, life::stutter,
      q' = cacheEvent({ ...q, activation: { ...a,
        normal: a.normal.union(Set(id)), returns: a.returns.set(id, a.returns.get(id) + 1),
        credits: if (credit) a.credits.set(id, a.credits.get(id) + 1) else a.credits,
        work: a.work + (if (credit) a.spans.get(id) else 0),
        accrued: a.accrued + (if (credit) a.spans.get(id) else 0) } })
    }
  }
  def exitedActivation(a: ActivationState, id: int, credit: bool): ActivationState = {
    ...a, stack: a.stack.tail(), published: a.published.exclude(Set(id)), reported: a.reported.union(Set(id)),
    credits: if (credit) a.credits.set(id, a.credits.get(id) + 1) else a.credits,
    work: a.work + (if (credit) a.spans.get(id) else 0),
    reports: a.reports + (if (not(a.reported.contains(id)) and a.epochs.get(id) == a.policy) 1 else 0)
  }
  action exitActivation(id: int): bool = all {
    q.activation.stack.length() > 0, q.activation.stack.head() == id, q.cpu.get(1) == 1, runtimeAvailable(1), life::stutter,
    q' = cacheEvent({ ...q,
      cpu: if (q.activation.stack.length() == 1 and not(q.activation.turn)) q.cpu.set(1, 0) else q.cpu,
      activation: if (q.activation.cycle) { ...q.activation, stack: q.activation.stack.tail(), published: q.activation.published.exclude(Set(id)),
        settled: q.activation.settled + (if (q.activation.stack.length() == 1 and not(q.activation.turn)) q.activation.accrued else 0),
        accrued: if (q.activation.stack.length() == 1 and not(q.activation.turn)) 0 else q.activation.accrued }
      else exitedActivation(q.activation, id, q.activation.normal.contains(id) and q.activation.epochs.get(id) == q.activation.policy) })
  }
  action replaceActivationPolicy = all {
    q.activation.enabled, q.activation.stack.length() == 0, q.cpu.get(1) == 0, q.activation.policy == 1, life::stutter,
    q' = cacheEvent({ ...q, activation: { ...q.activation, policy: 2, maximum: 1, metadata: 0, work: 0, reports: 0, accrued: 0, settled: 0 } })
  }
  // Failure control for the native split retirement/index publication race.
  // The index is observation ownership, never another lease or waiter queue.
  action unsafeSuspendedOwner(w: int): bool = all {
    q.blocked.contains(w), q.cpu.get(workerOwner(w)) == 0, life::stutter,
    q' = { ...q, cpu: q.cpu.set(workerOwner(w),w) }
  }
  action unsafeActivationIndexRetire = all {
    q.activation.stack == List(1), life::stutter,
    q' = { ...q, activation: { ...q.activation, stack: List() } }
  }
  action unsafeActivationAliasExit = all {
    q.activation.stack == List(1), not(q.activation.normal.contains(1)), q.activation.normal.contains(2), life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(1,0), activation: exitedActivation(q.activation, 1, true) })
  }
  action unsafeActivationEpochExit = all {
    q.activation.stack == List(1), q.activation.epochs.get(1) != q.activation.policy,
    q.activation.normal.contains(1), life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(1,0), activation: exitedActivation(q.activation,1,true) })
  }
  // Same invocation/epoch/CPU model, with successful-return accrual and
  // outer release/suspension settlement. No independent completion/root model.
  action enableCycleAccounting = all {
    life::stutter, q' = cacheEvent({ ...q, activation: { ...q.activation, cycle: true } })
  }
  action cycleSuspend = all {
    q.activation.cycle, q.activation.stack.length() > 0, q.cpu.get(1) == 1, life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(1,0), activation: { ...q.activation,
      settled: q.activation.settled + q.activation.accrued, accrued: 0 } })
  }
  action cycleResume = all {
    q.activation.cycle, q.activation.stack.length() > 0, q.cpu.get(1) == 0, runtimeAvailable(1), life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(1,1) })
  }
  action unsafeCycleDoubleSettle = all {
    q.activation.cycle, q.activation.accrued > 0, life::stutter,
    q' = cacheEvent({ ...q, activation: { ...q.activation,
      settled: q.activation.settled + q.activation.accrued } })
  }
  action enableTurn = all {
    q.activation.cycle, not(q.activation.turn), q.activation.stack.length() == 0, q.cpu.get(1) == 1, life::stutter,
    q' = cacheEvent({ ...q, activation: { ...q.activation, turn: true, turnUsed: 0, turnSpent: 0,
      turnPermit: if (q.cacheBulk.get(1)) 1 else 12, turnOpportunity: false, turnSuspended: false } })
  }
  action turnBoundary = all {
    q.activation.turn, q.cpu.get(1) == 1, life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(1,0), activation: { ...q.activation,
      settled: q.activation.settled + q.activation.accrued, accrued: 0,
      turnSuspended: true, turnOpportunity: false } })
  }
  def canTurnResume: bool = q.activation.turn and q.activation.turnSuspended and
    q.cpu.get(1) == 0 and runtimeAvailable(1) and allowedCpu(1)
  action turnResume = all {
    canTurnResume, life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(1,1), debt: q.debt.set(1,false), activation: { ...q.activation,
      turnUsed: activeTurnSpan(q.activation), turnSpent: activeTurnSpan(q.activation),
      turnPermit: if (q.cacheBulk.get(1)) 1 else 12, turnSuspended: false, turnOpportunity: true } })
  }
  action turnEnd = all {
    q.activation.turn, q.activation.stack.length() == 0, q.cpu.get(1) == 1, life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(1,0), activation: { ...q.activation,
      turn: false, turnSuspended: false, turnOpportunity: false,
      settled: q.activation.settled + q.activation.accrued, accrued: 0 } })
  }
  action unsafeTurnRefund = all {
    q.activation.turn, q.activation.stack.length() == 0, q.activation.turnSpent > 0, life::stutter,
    q' = cacheEvent({ ...q, activation: { ...q.activation, turnUsed: 0 } })
  }
  action unsafeTurnHeldYield = all {
    q.activation.turn, q.cpu.get(1) == 1, life::stutter,
    q' = cacheEvent({ ...q, activation: { ...q.activation, turnSuspended: true } })
  }
  action enableReclamation = all {
    q.cacheMode, life::s.accepted.size() == 0, life::stutter,
    q' = cacheEvent({ ...q, reclamation: true })
  }
  // Native budget queues contain registered waiters, not all runnable workers.
  // Execution debt survives the dispatch acknowledgement and settles at the
  // receiving thread's first grant or a distinct ungranted terminal outcome.
  action enableRegistration = all {
    q.cacheMode, q.joint, life::s.phase == "open", life::s.accepted.size() == 0, life::stutter,
    q' = cacheEvent({ ...q, registration: true })
  }
  action registerWaiter(w: int): bool = all {
    q.registration, runnable(w), runtimeAvailable(workerOwner(w)),
    q.cpu.get(workerOwner(w)) != w, not(q.waiters.contains(w)),
    not(q.dispatchPending.contains(q.jobs.get(w))), life::stutter,
    q' = cacheEvent({ ...q, waiters: q.waiters.union(Set(w)),
      executionRegistered: if (shortJobs.contains(q.jobs.get(w))) q.executionRegistered.union(Set(q.jobs.get(w))) else q.executionRegistered })
  }
  action finishUngrant(w: int): bool = {
    val r = q.jobs.get(w)
    all {
      q.registration, q.executionPending.contains(r), runnable(w),
      not(q.waiters.contains(w)), q.cpu.get(workerOwner(w)) != w,
      not(q.dispatchPending.contains(r)), runtimeAvailable(workerOwner(w)),
      not(newCompletionOpportunity(r)) or q.completionEpoch.get(workerOwner(w)) < 2,
      life::readyService(r, false),
      q' = cacheEvent({ ...q, jobs: q.jobs.set(w, 0), left: q.left.set(r, 0),
        executionPending: q.executionPending.exclude(Set(r)), executionUngrant: q.executionUngrant.union(Set(r)),
        workReady: q.workReady.set(workerOwner(w), q.workReady.get(workerOwner(w)) + 1),
        completionEpoch: if (newCompletionOpportunity(r)) q.completionEpoch.set(workerOwner(w), q.completionEpoch.get(workerOwner(w)) + 1) else q.completionEpoch,
        completionDue: if (q.coalesced) q.completionDue.union(Set(workerOwner(w))) else q.completionDue })
    }
  }
  action unsafeUnregisteredGrant(w: int): bool = all {
    q.registration, q.executionPending.contains(q.jobs.get(w)), not(q.waiters.contains(w)),
    ownerFree(workerOwner(w)), life::stutter,
    q' = cacheEvent({ ...q, cpu: q.cpu.set(workerOwner(w), w),
      executionPending: q.executionPending.exclude(Set(q.jobs.get(w))),
      executionAck: q.executionAck.union(Set(q.jobs.get(w))), executionGrants: q.executionGrants.union(Set(q.jobs.get(w))) })
  }
  action unsafeDispatchExecutionAck(w: int): bool = all {
    q.executionPending.contains(q.jobs.get(w)), life::stutter,
    q' = cacheEvent({ ...q, executionPending: q.executionPending.exclude(Set(q.jobs.get(w))),
      executionAck: q.executionAck.union(Set(q.jobs.get(w))) })
  }
  action enableReceiver = all {
    q.cacheMode, life::s.phase == "open", life::s.accepted.size() == 0, life::stutter,
    q' = cacheEvent({ ...q, reclamation: true, receiverMode: true })
  }
  action receiverEnter(owner: int): bool = all {
    q.receiverMode, q.reclamation, life::s.phase == "open", not(q.receiving.contains(owner)), life::stutter,
    q' = cacheEvent({ ...q, receiving: q.receiving.union(Set(owner)) })
  }
  action receiverLeave(owner: int): bool = all {
    q.receiving.contains(owner), life::stutter,
    q' = cacheEvent({ ...q, receiving: q.receiving.exclude(Set(owner)) })
  }
  action unsafeReclamationYield = all {
    life::stutter, q' = cacheEvent({ ...q, workOffered: q.workReady })
  }
  // Cross-layer refinement: Tokio futures dispatch to fixed native owners.
  // Actor children are distinct green threads of job1, not extra OS owners.
  val actorRunnable = Set(1,2).exists(c => Set("runnable","lease-wait","granted","running").contains(q.actorPhase.get(c)))
  val actorAllDone = Set(1,2).forall(c => q.actorPhase.get(c) == "done")
  // Per-result publication uses the harvested policy snapshot. A later mode
  // update cannot revoke that result's release-before-send obligation.
  val publicationSafety = and {
    q.publicationAfter.subseteq(life::s.accepted),
    q.publicationAfter.intersect(q.replies).intersect(capacityJobs).size() == 0,
    q.publicationAfter.intersect(q.replies).intersect(rootedJobs).size() == 0
  }
  action publicationMode(after: bool): bool = all {
    q.cross, life::s.phase == "open", life::stutter, q' = { ...q, retirePublication: after }
  }
  def publicationCanSend(r: int): bool = not(q.publicationAfter.contains(r)) or
    (not(capacityJobs.contains(r)) and not(rootedJobs.contains(r)))
  action unsafePublicationBeforeReturn(r: int): bool = all {
    q.publicationAfter.contains(r), q.projectedRust.contains(r), capacityJobs.contains(r),
    life::stutter, q' = { ...q, replies: q.replies.union(Set(r)) }
  }
  // Reuse life.poolStaged, native capacity, owner claim and the same final reply.
  // Placement acknowledgement is an optional client scheduling dependency only.
  val unassignedJobs = life::s.poolStaged
  val acceptedJobs = life::s.accepted
  action completionInstallOwner(owner: int): bool = all { life::registerProducer(owner), q' = q }
  action completionEnablePool = all { life::enablePool, q' = q }
  action completionFinishOwner(owner: int): bool = all { life::s.phase == "closing", life::finishProducer(owner), q' = q }
  val completionQueueSafety = and {
    q.completionQueued.subseteq(shortJobs), q.placementRequired.subseteq(q.completionQueued),
    q.placementObserved.subseteq(q.placementRequired.union(q.jointAwait).intersect(life::s.accepted)),
    JOBS.forall(r => implies(Set("await-placement","placement-poll").contains(q.client.get(r)),
      q.placementRequired.contains(r) and not(q.placementObserved.contains(r)))),
    q.completionQueued.intersect(life::s.poolStaged).intersect(capacityJobs.union(rootedJobs)).size() == 0
  }
  action completionQueue(r: int, direct: bool): bool = completionQueueWith(r, if (direct) "final" else "placement")
  action jointQueue(r: int): bool = completionQueueWith(r,"joint")
  action completionQueueWith(r: int, mode: str): bool = all {
    shortJobs.contains(r), q.client.get(r) == "submit-poll", q.tokioPolling.contains(r),
    life::offerShort(r),
    q' = { ...q, completionQueued: q.completionQueued.union(Set(r)),
      jointAwait: if (mode == "joint") q.jointAwait.union(Set(r)) else q.jointAwait,
      placementRequired: if (mode == "placement") q.placementRequired.union(Set(r)) else q.placementRequired,
      client: q.client.set(r, if (mode == "joint") "await-progress" else if (mode == "final") "await-reply" else "await-placement"),
      tokioPolling: q.tokioPolling.exclude(Set(r)) }
  }
  action completionClaim(r: int): bool = all {
    q.completionQueued.contains(r), runtimeAvailable(ASSIGNED.get(r)),
    life::claimPool(r, ASSIGNED.get(r)),
    q' = cacheEvent({ ...q, host: q.host.append(r),
      clientWakeOwed: if (q.jointAwait.contains(r) and q.receiveArmed.contains(r)) q.clientWakeOwed.union(Set(r)) else q.clientWakeOwed })
  }
  action completionCancel(r: int): bool = if (life::s.poolStaged.contains(r)) all {
    q.completionQueued.contains(r), life::cancelShort(r),
    q' = { ...q, client: q.client.set(r,"queue-cancelled"), tokioPolling: q.tokioPolling.exclude(Set(r)), receiveArmed: q.receiveArmed.exclude(Set(r)), clientWakeOwed: q.clientWakeOwed.exclude(Set(r)) }
  } else resourceStutter
  action completionPollPlacement(r: int): bool = all {
    q.client.get(r) == "await-placement", life::s.accepted.contains(r),
    q.tokioPolling.size() < q.tokioLimit, life::stutter,
    q' = { ...q, client: q.client.set(r,"placement-poll"), tokioPolling: q.tokioPolling.union(Set(r)) }
  }
  action completionObservePlacement(r: int): bool = all {
    q.client.get(r) == "placement-poll", life::stutter,
    q' = { ...q, client: q.client.set(r,"await-reply"),
      placementObserved: q.placementObserved.union(Set(r)), tokioPolling: q.tokioPolling.exclude(Set(r)) }
  }
  action completionQueueMutation(r: int): bool = all {
    q.completionQueued.contains(r), life::unsafeQueuedCredit(r), q' = q
  }
  // One registered observer for two events, not two polling slots or native leases.
  // Wake debt is a level/obligation, not a count of OS calls or actual resumptions.
  val jointReceiveSafety = and {
    q.jointAwait.subseteq(q.completionQueued), q.receiveArmed.subseteq(q.jointAwait),
    q.clientWakeOwed.subseteq(q.receiveArmed),
    q.receiveArmed.forall(r => Set("await-progress","progress-poll").contains(q.client.get(r))),
    q.receiveArmed.intersect(q.replies.union(life::s.accepted.exclude(q.placementObserved))).subseteq(q.clientWakeOwed.union(q.tokioPolling))
  }
  action jointBeginPoll(r: int): bool = all {
    q.jointAwait.contains(r), q.client.get(r) == "await-progress",
    not(q.receiveArmed.contains(r)) or q.clientWakeOwed.contains(r),
    q.tokioPolling.size() < q.tokioLimit, life::stutter,
    q' = { ...q, client: q.client.set(r,"progress-poll"), tokioPolling: q.tokioPolling.union(Set(r)),
      receiveArmed: q.receiveArmed.union(Set(r)), clientWakeOwed: q.clientWakeOwed.exclude(Set(r)) }
  }
  action jointPollPending(r: int): bool = all {
    q.client.get(r) == "progress-poll", not(q.replies.contains(r)), life::stutter,
    q' = { ...q, client: q.client.set(r,"await-progress"), tokioPolling: q.tokioPolling.exclude(Set(r)),
      placementObserved: if (life::s.accepted.contains(r)) q.placementObserved.union(Set(r)) else q.placementObserved,
      clientWakeOwed: q.clientWakeOwed.exclude(Set(r)) }
  }
  action jointPollReady(r: int): bool = all {
    q.client.get(r) == "progress-poll", q.replies.contains(r), life::stutter,
    q' = { ...q, client: q.client.set(r,"done"), observedReplies: q.observedReplies.union(Set(r)),
      tokioPolling: q.tokioPolling.exclude(Set(r)), receiveArmed: q.receiveArmed.exclude(Set(r)), clientWakeOwed: q.clientWakeOwed.exclude(Set(r)) }
  }
  action unsafeJointLoseWake(r: int): bool = all {
    q.client.get(r) == "await-progress", q.clientWakeOwed.contains(r), life::stutter,
    q' = { ...q, clientWakeOwed: q.clientWakeOwed.exclude(Set(r)) }
  }
  action unsafeJointClaimCompletes(r: int): bool = all {
    q.jointAwait.contains(r), life::s.accepted.contains(r), not(q.replies.contains(r)), life::stutter,
    q' = { ...q, observedReplies: q.observedReplies.union(Set(r)) }
  }
  // Client progress is not owner execution, admission or capacity release.
  // A native-ready reply can wait for a Tokio client to resume and consume it.
  val arrivalClockSafety = and {
    q.clock.observed.subseteq(q.clientDue.intersect(q.clientPlanned)),
    q.clock.observed.forall(r => q.clock.event.get(r) == q.clock.generation.get(r) and q.clock.generation.get(r) > 0),
    q.clientEligible.intersect(q.clock.mode).subseteq(q.clock.observed),
    q.clock.mode.subseteq(JOBS), JOBS.forall(r => q.clock.generation.get(r) >= 0 and q.clock.generation.get(r) <= 2)
  }
  action enableArrivalClock = all {
    q.clientProgress, life::s.accepted.size() == 0, life::stutter,
    q' = { ...q, clock: { ...q.clock, mode: shortJobs } }
  }
  action clockArm(r: int): bool = all {
    q.clock.mode.contains(r), q.clientPlanned.contains(r), not(q.clientEligible.contains(r)),
    not(q.clientTimers.contains(r)), not(q.clock.observed.contains(r)), q.clock.generation.get(r) < 2, life::stutter,
    q' = { ...q, clientTimers: q.clientTimers.union(Set(r)),
      clock: { ...q.clock, generation: q.clock.generation.set(r,q.clock.generation.get(r)+1), event: q.clock.event.set(r,0) } }
  }
  action clockFire(r: int, epoch: int): bool = all {
    q.clock.mode.contains(r), q.clientTimers.contains(r), q.clientDue.contains(r), epoch == q.clock.generation.get(r), epoch > 0,
    life::stutter, q' = { ...q, clock: { ...q.clock, event: q.clock.event.set(r,epoch) } }
  }
  action clockObserve(r: int): bool = all {
    q.clock.mode.contains(r), q.clientTimers.contains(r), q.clock.event.get(r) == q.clock.generation.get(r), q.clock.generation.get(r) > 0,
    life::stutter, q' = { ...q, clientTimers: q.clientTimers.exclude(Set(r)), clock: { ...q.clock, observed: q.clock.observed.union(Set(r)) } }
  }
  action clockCancel(r: int): bool = all {
    q.clock.mode.contains(r), not(q.clientEligible.contains(r)), life::stutter,
    q' = { ...q, clientTimers: q.clientTimers.exclude(Set(r)), clock: { ...q.clock, event: q.clock.event.set(r,0), observed: q.clock.observed.exclude(Set(r)) } }
  }
  action clockStale(r: int, epoch: int): bool = all {
    q.clock.mode.contains(r), epoch != q.clock.generation.get(r), life::stutter, q' = q
  }
  action unsafeClockEvent(r: int, epoch: int): bool = all {
    q.clock.mode.contains(r), q.clientTimers.contains(r), life::stutter,
    q' = { ...q, clock: { ...q.clock, event: q.clock.event.set(r,epoch), observed: q.clock.observed.union(Set(r)) } }
  }
  val clientProgressSafety = implies(q.clientProgress, and {
    q.clientDue.subseteq(q.clientPlanned), q.clientEligible.subseteq(q.clientDue),
    q.clientTimers.subseteq(q.clientPlanned),
    q.clientTimers.intersect(q.clientEligible).size() == 0,
    life::s.accepted.intersect(q.clientPlanned).subseteq(q.clientEligible),
    // A projected value can precede independent native root/credit retirement.
    q.projectedRust.subseteq(readyJobs.union(completedJobs)),
    q.clientEligible.forall(r => q.clientAfter.get(r) == 0 or q.observedReplies.contains(q.clientAfter.get(r)))
  })
  // Track the caller's issue-before-submit contract. Untracked bulk and older
  // APIs retain their existing routes; an observation token is not admission.
  def clientCanSubmit(r: int): bool = not(q.completionQueued.contains(r)) and (not(q.clientProgress) or
    not(q.clientPlanned.contains(r)) or q.clientEligible.contains(r))
  action enableClientProgress = all {
    q.cross, life::s.accepted.size() == 0, life::stutter, q' = { ...q, clientProgress: true }
  }
  action clientPlan(r: int, previous: int): bool = all {
    q.clientProgress, not(q.clientPlanned.contains(r)), not(life::s.accepted.contains(r)), q.client.get(r) == "idle",
    previous == 0 or (JOBS.contains(previous) and previous != r), life::stutter,
    q' = { ...q, clientPlanned: q.clientPlanned.union(Set(r)), clientAfter: q.clientAfter.set(r,previous) }
  }
  action clientDue(r: int): bool = all {
    q.clientPlanned.contains(r), life::stutter, q' = { ...q, clientDue: q.clientDue.union(Set(r)) }
  }
  // The legacy timer can also be armed for already-due work. The explicit
  // ready path reaches the same eligibility without another timer state.
  action clientTimer(r: int): bool = all {
    q.clientPlanned.contains(r), not(q.clientEligible.contains(r)), q.client.get(r) == "idle", life::stutter,
    q' = { ...q, clientTimers: q.clientTimers.union(Set(r)) }
  }
  def clientCanResume(r: int): bool = q.clientDue.contains(r) and
    (q.clientAfter.get(r) == 0 or q.observedReplies.contains(q.clientAfter.get(r)))
  action clientResume(r: int): bool = all {
    clientCanResume(r), q.client.get(r) == "idle",
    not(q.clock.mode.contains(r)) or q.clock.observed.contains(r), life::stutter,
    q' = { ...q, clientEligible: q.clientEligible.union(Set(r)), clientTimers: q.clientTimers.exclude(Set(r)) }
  }
  action clientForget(r: int): bool = all {
    q.clientProgress, q.client.get(r) == "idle", not(life::s.accepted.contains(r)), life::stutter,
    q' = { ...q, clientPlanned: q.clientPlanned.exclude(Set(r)), clientDue: q.clientDue.exclude(Set(r)),
      clientEligible: q.clientEligible.exclude(Set(r)), clientTimers: q.clientTimers.exclude(Set(r)),
      clock: { ...q.clock, event: q.clock.event.set(r,0), observed: q.clock.observed.exclude(Set(r)) } }
  }
  action unsafeClientDirectOffer(r: int): bool = all {
    q.clientProgress, q.clientPlanned.contains(r), not(q.clientEligible.contains(r)),
    classSpace(r), life::admit(r), q' = cacheEvent({ ...q, host: q.host.append(r) })
  }
  action unsafeClientEarlyIssue(r: int): bool = all {
    q.clientPlanned.contains(r), not(q.clientDue.contains(r)), life::stutter,
    q' = { ...q, clientEligible: q.clientEligible.union(Set(r)) }
  }
  action unsafeClientEarlyConsume(r: int): bool = all {
    life::s.accepted.contains(r), not(q.projectedRust.contains(r)), life::stutter,
    q' = { ...q, observedReplies: q.observedReplies.union(Set(r)) }
  }
  val crossSafety = and {
    q.tokioLimit >= 1, q.tokioLimit <= 2, q.tokioPolling.size() <= q.tokioLimit,
    q.tokioPolling.forall(r => Set("submit-poll","placement-poll","progress-poll","reply-poll").contains(q.client.get(r))),
    q.replies.subseteq(q.projectedRust), q.observedReplies.subseteq(q.replies),
    q.projectedRust.subseteq(life::s.accepted), q.droppedClients.subseteq(life::s.accepted),
    Set(1,2).contains(q.actorLease) or q.actorLease == 0,
    implies(q.actorLease != 0, q.cpu.get(ASSIGNED.get(1)) == 0 and
      Set("granted","running").contains(q.actorPhase.get(q.actorLease))),
    implies(q.actorBatch and not(actorAllDone), rootedJobs.contains(1) and not(readyJobs.contains(1))),
    q.osSleeping.subseteq(q.nativeWaiting), not(q.crossViolation)
  }
  action enableCross = all {
    q.runtimeJoint or q.nativeScheduler, life::s.accepted.size() == 0, life::stutter, q' = { ...q, cross: true }
  }
  action crossCapacityOne = all {
    q.cross, life::s.accepted.size() == 0, life::configureCapacity(1), q' = q
  }
  action crossCapacitySignal = all { q.cross, life::publishCapacity, q' = q }
  action crossTokioTwo = all {
    q.cross, life::s.accepted.size() == 0, q.tokioPolling.size() == 0,
    life::stutter, q' = { ...q, tokioLimit: 2 }
  }
  action tokioBegin(r: int): bool = all {
    q.cross, clientCanSubmit(r),
    q.client.get(r) == "idle", q.tokioPolling.size() < q.tokioLimit,
    life::stutter, q' = { ...q, client: q.client.set(r,"submit-poll"), tokioPolling: q.tokioPolling.union(Set(r)) }
  }
  action tokioPublish(r: int): bool = all {
    q.client.get(r) == "submit-poll", q.tokioPolling.contains(r), classSpace(r), life::admit(r),
    q' = cacheEvent({ ...q, host: q.host.append(r), client: q.client.set(r,"await-reply"),
      tokioPolling: q.tokioPolling.exclude(Set(r)) })
  }
  action tokioCapacityWait(r: int): bool = all {
    q.client.get(r) == "submit-poll", not(life::hasCapacity(r)), life::armCapacity(ASSIGNED.get(r)),
    q' = { ...q, client: q.client.set(r,"await-capacity"), tokioPolling: q.tokioPolling.exclude(Set(r)) }
  }
  action tokioCapacityReady(r: int): bool = all {
    q.client.get(r) == "await-capacity", life::hasCapacity(r),
    q.tokioPolling.size() < q.tokioLimit, life::disarmCapacity(ASSIGNED.get(r)),
    q' = { ...q, client: q.client.set(r,"submit-poll"), tokioPolling: q.tokioPolling.union(Set(r)) }
  }
  action nativeProjectRust(r: int): bool = all {
    q.cross, q.harvested.contains(r), q.batchSlots.contains(r), not(q.projectedRust.contains(r)),
    runtimeAvailable(ASSIGNED.get(r)), life::stutter,
    q' = { ...q, projectedRust: q.projectedRust.union(Set(r)) }
  }
  // Bounded indexed delivery refines the same reply/retirement state. Owners
  // publish once; ready receive rotates owners and never waits for an input index.
  val deliveryQueued = owners.fold(Set(), (seen, owner) => seen.union(entries(q.delivery.queues.get(owner))))
  val deliveryCount = owners.fold(0, (count, owner) => count + q.delivery.queues.get(owner).length())
  val deliveryFirstOwner = if (q.delivery.queues.get(q.delivery.cursor).length() > 0) q.delivery.cursor
    else if (deliveryQueued.size() > 0) (if (q.delivery.cursor == 1) 2 else 1) else 0
  val deliverySafety = and {
    deliveryQueued.subseteq(q.replies.intersect(q.delivery.requests)),
    deliveryQueued.intersect(q.delivery.consumed).size() == 0,
    deliveryCount == deliveryQueued.size(), deliveryCount <= q.delivery.requests.size(),
    q.delivery.consumed.subseteq(q.observedReplies),
    q.delivery.expectedOwner == q.delivery.pickedOwner, not(q.delivery.turnMissed),
    q.delivery.preparationRemaining >= 0, q.delivery.preparationCount >= 0,
    q.delivery.preparationRemaining + q.delivery.preparationCount <= 2,
    implies(q.delivery.armed and deliveryQueued.size() > 0, q.delivery.wakeOwed),
    implies(q.delivery.dropped, deliveryQueued.size() == 0 and not(q.delivery.armed))
  }
  action enableDelivery = all {
    q.cross, life::s.accepted.size() == 0, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, enabled: true, requests: JOBS.exclude(shortJobs) } }
  }
  action deliveryPending = all {
    q.delivery.enabled, not(q.delivery.dropped), deliveryQueued.size() == 0,
    life::stutter, q' = { ...q, delivery: { ...q.delivery, armed: true, wakeOwed: false } }
  }
  action deliveryTakeOwner(owner: int): bool = {
    val values = q.delivery.queues.get(owner)
    all {
      q.delivery.enabled, not(q.delivery.dropped), values.length() > 0,
      q.tokioPolling.size() < q.tokioLimit, life::stutter,
      q' = { ...q, observedReplies: q.observedReplies.union(Set(values.head())),
        delivery: { ...q.delivery, queues: q.delivery.queues.set(owner,values.tail()),
          cursor: if (owner == 1) 2 else 1, consumed: q.delivery.consumed.union(Set(values.head())),
          turnChecked: false, turnMissed: q.delivery.turnMode and not(q.delivery.turnChecked),
          expectedOwner: deliveryFirstOwner, pickedOwner: owner, armed: false, wakeOwed: false } }
    }
  }
  action deliveryTake = all { deliveryFirstOwner != 0, q.delivery.coopAvailable,
    not(q.delivery.turnMode) or q.delivery.turnChecked, deliveryTakeOwner(deliveryFirstOwner) }
  action enableClientTurn = all {
    q.delivery.enabled, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, turnMode: true } }
  }
  action clientTurnChecked = all { life::stutter, q' = { ...q, delivery: { ...q.delivery, turnChecked: true } } }
  val clientTurnCanCheck = q.tokioPolling.size() < q.tokioLimit and q.client.get(3) != "progress-poll" and
    (not(q.jointAwait.contains(3) and q.clientWakeOwed.contains(3)) or q.client.get(3) == "await-progress")
  action clientTurnCheck = if (clientTurnCanCheck)
    (if (q.jointAwait.contains(3) and q.clientWakeOwed.contains(3))
      jointBeginPoll(3).then(if (q.replies.contains(3)) jointPollReady(3) else jointPollPending(3)).then(clientTurnChecked)
      else clientTurnChecked)
    else all { false, life::stutter, q' = q }
  // Cooperation budget is a Rust task scheduling boundary, never native capacity.
  // One ready final transport can be consumed before deferral; bulk and pending
  // work still need normal cooperation. No new native lease or client ack.
  action enableCompletionService = all {
    q.delivery.turnMode, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, readyService: true } }
  }
  action exhaustClientBudget = all {
    q.delivery.turnMode, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, coopAvailable: false } }
  }
  action restoreClientBudget = all {
    q.delivery.turnMode, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, coopAvailable: true } }
  }
  action cooperativeClientCheck = if (q.delivery.coopAvailable) clientTurnCheck else clientTurnChecked
  action completionServiceCheck = if (q.delivery.readyService and q.replies.contains(3))
    clientTurnCheck else cooperativeClientCheck
  // Root-free Rust preparation reuses the client turn boundary, not native
  // constructed-input ownership, execution fuel or producer capacity.
  action preparationStart = all { q.delivery.turnMode, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, preparationRemaining: 2, preparationCount: 0, turnChecked: false } } }
  action preparationItem = all { q.delivery.preparationRemaining > 0, q.delivery.turnChecked, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, preparationRemaining: q.delivery.preparationRemaining - 1,
      preparationCount: q.delivery.preparationCount + 1, turnChecked: false } } }
  action preparationDiscard = all { life::stutter,
    q' = { ...q, delivery: { ...q.delivery, preparationRemaining: 0, preparationCount: 0 } } }
  action unsafePreparationSkip = all { q.delivery.preparationRemaining > 0, not(q.delivery.turnChecked), life::stutter,
    q' = { ...q, delivery: { ...q.delivery, turnMissed: true } } }
  action jointInputOpen = all { life::streamingInputOpen, q' = q }
  action jointInputSeal = all { life::streamingInputSeal, q' = q }
  action jointInputWindow(r: int): bool = all {
    q.delivery.turnMode, q.delivery.turnChecked, life::stagePool(r),
    q' = { ...q, delivery: { ...q.delivery, turnChecked: false } }
  }
  action unsafeClientTurnSkip = all {
    q.delivery.turnMode, not(q.delivery.turnChecked), deliveryFirstOwner != 0,
    deliveryTakeOwner(deliveryFirstOwner)
  }
  action deliveryDrop = all {
    q.delivery.enabled, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, queues: owners.mapBy(_ => List()), armed: false, wakeOwed: false, dropped: true } }
  }
  action unsafeDeliveryWake = all {
    q.delivery.armed, deliveryQueued.size() > 0, life::stutter,
    q' = { ...q, delivery: { ...q.delivery, wakeOwed: false } }
  }
  action unsafeDeliveryPick = {
    nondet owner = owners.oneOf()
    all { owner != deliveryFirstOwner, deliveryTakeOwner(owner) }
  }
  action ownerPublishReply(r: int): bool = all {
    q.projectedRust.contains(r), publicationCanSend(r), not(q.replies.contains(r)), life::stutter,
    q' = { ...q, replies: q.replies.union(Set(r)),
      delivery: if (q.delivery.enabled and q.delivery.requests.contains(r) and not(q.delivery.dropped))
        { ...q.delivery, queues: q.delivery.queues.set(ASSIGNED.get(r), q.delivery.queues.get(ASSIGNED.get(r)).append(r)),
          wakeOwed: q.delivery.armed or q.delivery.wakeOwed } else q.delivery,
      clientWakeOwed: if (q.jointAwait.contains(r) and q.receiveArmed.contains(r)) q.clientWakeOwed.union(Set(r)) else q.clientWakeOwed }
  }
  action tokioPollReply(r: int): bool = all {
    q.client.get(r) == "await-reply", q.replies.contains(r), not(q.droppedClients.contains(r)),
    not(q.delivery.enabled and q.delivery.requests.contains(r)),
    q.tokioPolling.size() < q.tokioLimit, life::stutter,
    q' = { ...q, client: q.client.set(r,"reply-poll"), tokioPolling: q.tokioPolling.union(Set(r)) }
  }
  action tokioObserve(r: int): bool = all {
    q.client.get(r) == "reply-poll", q.projectedRust.contains(r),
    not(q.delivery.enabled and q.delivery.requests.contains(r)), life::stutter,
    q' = { ...q, client: q.client.set(r,"done"), observedReplies: q.observedReplies.union(Set(r)), tokioPolling: q.tokioPolling.exclude(Set(r)) }
  }
  action tokioDrop(r: int): bool = all {
    q.cross, life::s.accepted.contains(r), not(life::s.completed.contains(r)), life::observeTimeout(r),
    q' = { ...q, droppedClients: q.droppedClients.union(Set(r)), client: q.client.set(r,"dropped"), tokioPolling: q.tokioPolling.exclude(Set(r)), receiveArmed: q.receiveArmed.exclude(Set(r)), clientWakeOwed: q.clientWakeOwed.exclude(Set(r)) }
  }
  // Scheme native wait still runs its green-thread queue and timers. OS sleep
  // is a separate event and must not conceal child runnable work or host input.
  action enterNativeWait(owner: int): bool = all {
    q.cross, not(readyJobs.exists(r => ASSIGNED.get(r) == owner)), life::stutter,
    q' = { ...q, nativeWaiting: q.nativeWaiting.union(Set(owner)) }
  }
  action enterOsSleep(owner: int): bool = all {
    q.nativeWaiting.contains(owner), life::canPark(owner),
    not(runtimeRunnable(owner,true)), not(runtimeRunnable(owner,false)),
    life::stutter, q' = { ...q, osSleeping: q.osSleeping.union(Set(owner)) }
  }
  action crossPublishWake(owner: int): bool = all {
    q.cross, life::notify(owner), q' = q
  }
  action crossWake(owner: int): bool = all {
    q.cross, life::s.notified.contains(owner), life::deliverWake(owner),
    q' = { ...q, osSleeping: q.osSleeping.exclude(Set(owner)), nativeWaiting: q.nativeWaiting.exclude(Set(owner)) }
  }
  action actorDispatch = all {
    q.cross, q.jobs.get(1) == 1, q.cpu.get(ASSIGNED.get(1)) == 1, not(q.actorBatch),
    life::stutter, q' = { ...q, actorBatch: true, blocked: q.blocked.union(Set(1)),
      cpu: q.cpu.set(ASSIGNED.get(1),0), actorPhase: Set(1,2).mapBy(_ => "runnable") }
  }
  action actorRegister(c: int): bool = all {
    q.actorBatch, q.actorPhase.get(c) == "runnable", life::stutter,
    q' = { ...q, actorPhase: q.actorPhase.set(c,"lease-wait") }
  }
  action actorSelect(c: int): bool = all {
    q.actorPhase.get(c) == "lease-wait", ownerFree(ASSIGNED.get(1)),
    not(preferShortCpu(ASSIGNED.get(1))), runtimeAvailable(ASSIGNED.get(1)), life::stutter,
    q' = { ...q, actorLease: c, actorPhase: q.actorPhase.set(c,"granted"), debt: q.debt.set(ASSIGNED.get(1),false) }
  }
  action actorReceive(c: int): bool = all {
    q.actorLease == c, q.actorPhase.get(c) == "granted", life::stutter,
    q' = { ...q, actorPhase: q.actorPhase.set(c,"running") }
  }
  action actorExecute(c: int): bool = all {
    q.actorLease == c, q.actorPhase.get(c) == "running", q.actorRemaining.get(c) > 0,
    runtimeAvailable(ASSIGNED.get(1)), life::stutter,
    q' = { ...q, actorRemaining: q.actorRemaining.set(c,q.actorRemaining.get(c)-1),
      executed: q.executed.set(ASSIGNED.get(1),q.executed.get(ASSIGNED.get(1))+1) }
  }
  action actorExternal(c: int): bool = all {
    q.actorLease == c, q.actorPhase.get(c) == "running", life::stutter,
    q' = { ...q, actorLease: 0, actorPhase: q.actorPhase.set(c,"external") }
  }
  action actorResume(c: int): bool = all {
    q.actorPhase.get(c) == "external", life::stutter,
    q' = { ...q, actorPhase: q.actorPhase.set(c,"runnable"),
      osSleeping: q.osSleeping.exclude(Set(ASSIGNED.get(1))) }
  }
  action actorFinish(c: int): bool = all {
    q.actorLease == c, q.actorPhase.get(c) == "running", q.actorRemaining.get(c) == 0,
    life::stutter, q' = { ...q, actorLease: 0, actorPhase: q.actorPhase.set(c,"done") }
  }
  action actorPublishBatch = all {
    q.actorBatch, actorAllDone, q.jobs.get(1) == 1, not(readyJobs.contains(1)),
    not(newCompletionOpportunity(1)) or q.completionEpoch.get(ASSIGNED.get(1)) < 2,
    life::readyService(1,false), q' = cacheEvent({ ...q, blocked: q.blocked.exclude(Set(1)),
      jobs: q.jobs.set(1,0), left: q.left.set(1,0),
      completionEpoch: if (newCompletionOpportunity(1)) q.completionEpoch.set(ASSIGNED.get(1),q.completionEpoch.get(ASSIGNED.get(1))+1) else q.completionEpoch,
      completionDue: q.completionDue.union(Set(ASSIGNED.get(1))) })
  }
  action unsafeTokioHeldAwait(r: int): bool = all {
    q.client.get(r) == "await-reply", life::stutter,
    q' = { ...q, tokioPolling: q.tokioPolling.union(Set(r)) }
  }
  action unsafeActorEarlyPublish = all {
    q.actorBatch, not(actorAllDone), life::stutter, q' = { ...q, crossViolation: true }
  }
  action unsafeCrossOsSleep = all {
    q.nativeWaiting.contains(1), actorRunnable, life::stutter, q' = { ...q, crossViolation: true }
  }
  action crossStep = {
    nondet r = JOBS.oneOf()
    nondet c = Set(1,2).oneOf()
    nondet owner = owners.oneOf()
    any { enableCompletionService, exhaustClientBudget, restoreClientBudget, completionServiceCheck, enableArrivalClock, clockArm(r), clockFire(r,q.clock.generation.get(r)), clockObserve(r), clockCancel(r), clockStale(r,q.clock.generation.get(r)-1), enableClientTurn, clientTurnCheck, enableDelivery, deliveryPending, deliveryTake, deliveryDrop, jointQueue(r), jointBeginPoll(r), jointPollPending(r), jointPollReady(r), completionQueue(r,true), completionQueue(r,false), completionClaim(r), completionCancel(r), completionPollPlacement(r), completionObservePlacement(r), completionFinishOwner(owner), resourceStep, publicationMode(true), publicationMode(false), enableClientProgress, clientPlan(r,0), clientDue(r), clientTimer(r), clientResume(r), clientForget(r), enableCross, crossCapacityOne, crossCapacitySignal, crossTokioTwo, tokioBegin(r), tokioPublish(r), tokioCapacityWait(r), tokioCapacityReady(r),
      nativeProjectRust(r), ownerPublishReply(r), tokioPollReply(r), tokioObserve(r), tokioDrop(r),
      enterNativeWait(owner), enterOsSleep(owner), crossPublishWake(owner), crossWake(owner), actorDispatch, actorRegister(c), actorSelect(c),
      actorReceive(c), actorExecute(c), actorExternal(c), actorResume(c), actorFinish(c), actorPublishBatch }
  }
  action resourceStep = {
    nondet r = JOBS.oneOf()
    nondet w = workers.oneOf()
    nondet owner = owners.oneOf()
    any { enableNativeScheduler, enableNativeCycle, nativeActorClassShort, enableRuntimeObservation, enableRuntimeJoint, enableProgressCycle, enableReadyFirstCycle, cyclePass(owner), enableExecutionCheckpoint, enableDirected, selectWaiter(w), spuriousWake(w), enableRegistration, registerWaiter(w), finishUngrant(w), enableReceiver, receiverEnter(owner), receiverLeave(owner), enableReclamation, enableCache, enablePlans, selectPlan(w), enableWork, enableJoint, jointTurn(owner), emptyHarvest(owner), ackDispatch(w), beginConstruction(r), finishConstruction(r), payloadEntry(r), acquireSubmitCapability(owner), revokeSubmitCapability(owner), nativeEntry(r), endFill(owner), refillWake(owner), prepareInput(r), enterPrepared(r), discardPrepared(r), offer(r), feed(r), boundary(owner), assignWorker(w, r), acquireCpu(w), compute(w), yieldCpu(w),
      block(w), wakeTask(w), gcStart(owner), gcFinish(owner), osPause(owner), osResume(owner), delayTick,
      harvest(owner, Set(r)), batchTransfer(owner), completionAck(owner, q.tokenEpoch.get(owner)), project(r), abandon(r), resourceClose, resourceJoin, resourceStutter }
  }
  action unsafeBoundaryStep = { nondet owner = owners.oneOf(); any { resourceStep, unsafeBoundary(owner) } }
  action unsafeCpuStep = { nondet w = workers.oneOf(); any { resourceStep, unsafeCpu(w) } }
  action unsafeFeedStep = { nondet r = JOBS.oneOf(); any { resourceStep, unsafeFeed(r) } }

}
module service_sharing_cases {
  const CASE_JOBS: Set[int]
  const CASE_ASSIGNED: int -> int
  import service_sharing(JOBS = CASE_JOBS, ASSIGNED = CASE_ASSIGNED).*
  // Architecture refinement exports projections of this SAME runtime state.
  action architectureBaseInit = deliveryStart
  action architectureBaseIdle = resourceStutter
  action architectureBaseComplete = deliveryOne
  action architectureBaseConsume = deliveryTake
  val architectureBaseSafe = resourceSafety
  val architectureBaseRoots = rootedJobs
  val architectureBaseCapacity = capacityJobs
  val architectureBaseConsumed = q.delivery.consumed
  action currentInit = implementationInit
  action sharedInit = candidateInit
  action sharedStep = resourceStep
  action crossSharedStep = crossStep
  val sharedSafety = resourceSafety
  action mutationStart = mutationInit
  action boundaryMutationStep = unsafeBoundaryStep
  action cpuMutationStep = unsafeCpuStep
  action feedMutationStep = unsafeFeedStep
  // Two ready short graphs share one owner; harvesting is distinct from
  // projection/root release and cannot turn a ready backlog into a sleep.
  action completionStart = resourceInit.then(completionPolicy)
  action completionReadyStart = completionStart
    .then(offer(3)).then(feed(3)).then(assignWorker(1, 3)).then(acquireCpu(1)).then(compute(1))
    .then(offer(4)).then(boundary(1)).then(feed(4)).then(assignWorker(1, 4)).then(acquireCpu(1)).then(compute(1))
  action completionMutationStep = any { sharedStep, unsafeCompletionSnapshot(readyJobs) }
  action completionCoalescedStart = resourceInit.then(completionCoalescedPolicy)
  action completionLateReadyStart = completionCoalescedStart
    .then(offer(3)).then(feed(3)).then(assignWorker(1, 3)).then(acquireCpu(1)).then(compute(1))
    .then(harvest(1, Set(3)))
    .then(offer(4)).then(boundary(1)).then(feed(4)).then(assignWorker(1, 4)).then(acquireCpu(1)).then(compute(1))
    .then(project(3))
  action completionStaleAckMutationStep = any { sharedStep, unsafeStaleCompletionAck(1) }
  action completionBulkReadyStart = completionCoalescedStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1, 1)).then(acquireCpu(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1))
    .then(offer(2)).then(boundary(1)).then(feed(2)).then(assignWorker(2, 2)).then(acquireCpu(2)).then(compute(2))
  action completionBulkSnapshotProof = completionBulkReadyStart
    .then(harvest(1, Set(1, 2))).then(project(1)).then(offer(3))
    .then(all { assert(q.harvested == Set(2)), assert(rootedJobs == Set(2)), assert(q.completionEpoch.get(1) == 1),
      assert(resourceSafety), resourceStutter })
    .then(project(2)).then(completionAck(1, 1)).then(boundary(1)).then(feed(3)).then(assignWorker(1, 3)).then(acquireCpu(1)).then(compute(1))
    .then(harvest(1, Set(3))).then(project(3)).then(completionAck(1, 2)).then(resourceClose).then(resourceJoin)
    .then(all { assert(resourceSafety), assert(rootedJobs.size() == 0), resourceStutter })
  action nativeCycleStart = completionCoalescedStart.then(enableNativeCycle)
  action nativeCycleBulkReadyStart = nativeCycleStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1, 1)).then(acquireCpu(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1))
    .then(offer(2)).then(boundary(1)).then(feed(2)).then(assignWorker(2, 2)).then(acquireCpu(2)).then(compute(2))
  action nativeCycleTransferredStart = nativeCycleBulkReadyStart.then(harvest(1, Set(1, 2))).then(batchTransfer(1))
  action preparedCycleStart = nativeCycleStart.then(enablePreparedCycle)
  action pipelineCycleStart = preparedCycleStart.then(enablePipeline)
  action pipelineHeldStart = nativeCycleTransferredStart.then(enablePreparedCycle).then(enablePipeline)
  action pipelineCreditMutationStart = pipelineCycleStart.then(prepareInput(3))
  action pipelineCreditMutationStep = { nondet r = CASE_JOBS.oneOf(); any { sharedStep, unsafeFillWithoutRetirement(r) } }
  action pipelineRefillProof = pipelineHeldStart.then(prepareInput(3)).then(project(1)).then(project(2))
    .then(completionAck(1,1)).then(enterPrepared(3)).then(prepareInput(4)).then(enterPrepared(4))
    .then(all { assert(q.fillBudget.get(1) == 0), assert(q.prepared.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action pipelineWakeRefillProof = pipelineHeldStart.then(prepareInput(3)).then(project(1)).then(project(2))
    .then(completionAck(1,1)).then(refillWake(1)).then(enterPrepared(3))
    .then(all { assert(q.fillBudget.get(1) == 0), assert(resourceSafety), resourceStutter })
  action pipelineWakeControlProof = pipelineHeldStart.then(prepareInput(3)).then(project(1)).then(project(2))
    .then(completionAck(1,1)).then(refillWake(1)).then(endFill(1)).then(resourceClose).then(discardPrepared(3))
    .then(all { assert(q.fillBudget.get(1) == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action turnStart = cycleAccountingStart.then(enableTurn)
  action turnSpentStart = turnStart.then(enterActivation(1)).then(exitActivation(1))
  action turnStep = any {
    all { not(q.activation.turn), activationSharedStep },
    all { q.activation.turn and q.activation.turnSuspended,
      any { offer(3), feed(3), boundary(1), assignWorker(2,3), ackDispatch(2), acquireCpu(2), selectPlan(2), compute(2), yieldCpu(2) } },
    all { q.activation.turn, any { enterActivation(1), enterActivation(2), enterActivation(3),
      normalActivation(1), normalActivation(2), normalActivation(3), exitActivation(1), exitActivation(2), exitActivation(3),
      turnBoundary, turnResume, turnEnd, resourceClose, resourceStutter } },
    enableTurn
  }
  action turnRefundMutationStep = any { turnStep, unsafeTurnRefund }
  action turnHeldYieldMutationStep = any { turnStep, unsafeTurnHeldYield }
  action turnExceptionProof = turnSpentStart
    .then(all { assert(q.activation.turnUsed == 12), assert(q.activation.work == 0), assert(resourceSafety), resourceStutter })
    .then(turnBoundary).then(turnResume).then(enterActivation(2)).then(normalActivation(2)).then(exitActivation(2)).then(turnEnd)
    .then(all { assert(q.activation.work == 12), assert(q.activation.settled == 12), assert(resourceSafety), resourceStutter })
  action turnSuspensionProof = turnStart.then(enterActivation(1)).then(turnBoundary).then(turnResume)
    .then(all { assert(q.activation.turnUsed == 12), assert(activeTurnSpan(q.activation) == 12), assert(resourceSafety), resourceStutter })
    .then(exitActivation(1)).then(turnBoundary).then(turnResume).then(enterActivation(2)).then(normalActivation(2)).then(exitActivation(2)).then(turnEnd)
    .then(acquireCpu(1)).then(enableTurn).then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1)).then(turnEnd)
    .then(acquireCpu(1)).then(enableTurn).then(enterActivation(1)).then(exitActivation(1)).then(turnEnd)
    .then(all { assert(q.activation.work == 24), assert(q.activation.reports == 2), assert(resourceSafety), resourceStutter })
  action turnArbitrationProof = turnSpentStart.then(offer(3)).then(feed(3)).then(turnBoundary)
    .then(assignWorker(2,3)).then(ackDispatch(2))
    .then(all { assert(not(canTurnResume)), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(2)).then(selectPlan(2)).then(compute(2)).then(turnResume)
    .then(all { assert(q.cpu.get(1) == 1), assert(resourceSafety), resourceStutter }).then(turnEnd)
  action turnShortProof = turnSpentStart.then(offer(3)).then(feed(3)).then(turnBoundary)
    .then(assignWorker(2,3)).then(ackDispatch(2)).then(acquireCpu(2))
    .then(all { assert(q.cpu.get(1) == 2), assert(resourceSafety), resourceStutter })
    .then(selectPlan(2)).then(compute(2))
    .then(all { assert(readyJobs.contains(3)), assert(q.cpu.get(1) == 0), assert(resourceSafety), resourceStutter })
    .then(turnResume).then(enterActivation(2)).then(normalActivation(2)).then(exitActivation(2)).then(turnEnd)
    .then(all { assert(resourceSafety), resourceStutter })
  action turnCloseProof = turnStart.then(enterActivation(1)).then(resourceClose)
    .then(normalActivation(1)).then(exitActivation(1)).then(turnEnd).then(acquireCpu(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action turnRefundMutationProof = turnSpentStart.then(unsafeTurnRefund)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action turnHeldYieldMutationProof = turnStart.then(unsafeTurnHeldYield)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action cycleAccountingStart = activationStart.then(enableCycleAccounting)
  action cyclePendingStart = cycleAccountingStart.then(enterActivation(1)).then(enterActivation(2)).then(normalActivation(2))
  action cycleAccountingStep = any { activationSharedStep, cycleSuspend, cycleResume }
  action cycleDoubleSettleMutationStep = any { cycleAccountingStep, unsafeCycleDoubleSettle }
  action cycleAccountingProof = cyclePendingStart.then(exitActivation(2))
    .then(all { assert(q.activation.work == 12), assert(q.activation.accrued == 12),
      assert(q.activation.settled == 0), assert(resourceSafety), resourceStutter })
    .then(cycleSuspend)
    .then(all { assert(q.activation.accrued == 0), assert(q.activation.settled == 12), assert(resourceSafety), resourceStutter })
    .then(cycleResume).then(exitActivation(1))
    .then(all { assert(q.activation.work == 12), assert(q.activation.credits.get(1) == 0), assert(resourceSafety), resourceStutter })
  action cycleReplayProof = cycleAccountingStart.then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(acquireCpu(1)).then(enterActivation(1)).then(exitActivation(1))
    .then(acquireCpu(1)).then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(all { assert(q.activation.work == 24), assert(q.activation.reports == 1),
      assert(q.activation.settled == 24), assert(q.activation.accrued == 0), assert(resourceSafety), resourceStutter })
    .then(replaceActivationPolicy).then(acquireCpu(1)).then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(all { assert(q.activation.work == 0), assert(q.activation.settled == 0), assert(resourceSafety), resourceStutter })
  action cycleDoubleSettleProof = cyclePendingStart.then(unsafeCycleDoubleSettle)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action activationStart = leasePlanSelectedStart.then(enableActivations)
  action activationAliasStart = activationStart.then(enterActivation(1)).then(enterActivation(2))
    .then(normalActivation(2)).then(exitActivation(2))
  action activationEpochStart = activationStart.then(enterActivation(1)).then(exitActivation(1))
    .then(replaceActivationPolicy).then(acquireCpu(1)).then(enterActivation(1)).then(normalActivation(1))
  action activationSharedStep = any {
    all { q.activation.stack.length() == 0, sharedStep },
    enterActivation(1), enterActivation(2), enterActivation(3),
    normalActivation(1), normalActivation(2), normalActivation(3),
    exitActivation(1), exitActivation(2), exitActivation(3), replaceActivationPolicy,
    resourceStutter
  }
  action activationAliasMutationStep = any { activationSharedStep, unsafeActivationAliasExit }
  action activationEpochMutationStep = any { activationSharedStep, unsafeActivationEpochExit }
  action activationIsolationProof = activationAliasStart
    .then(exitActivation(1))
    .then(all { assert(q.activation.work == 12), assert(q.activation.credits.get(1) == 0),
      assert(q.activation.reports == 2), assert(q.activation.metadataEpochs == Set(1)), assert(resourceSafety), resourceStutter })
  action activationReplayProof = activationStart.then(enterActivation(1)).then(exitActivation(1))
    .then(acquireCpu(1)).then(enterActivation(2)).then(normalActivation(2)).then(exitActivation(2))
    .then(acquireCpu(1)).then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(acquireCpu(1)).then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(all { assert(q.activation.work == 36), assert(q.activation.reports == 2), assert(resourceSafety), resourceStutter })
    .then(replaceActivationPolicy).then(acquireCpu(1)).then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(all { assert(q.activation.spans.get(1) == 12), assert(q.activation.work == 0), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(1)).then(enterActivation(3)).then(normalActivation(3)).then(exitActivation(3))
    .then(all { assert(q.activation.spans.get(3) == 1), assert(q.activation.work == 1),
      assert(q.activation.metadataEpochs == Set(1,2)), assert(resourceSafety), resourceStutter })
  action activationExceptionalReplayProof = activationStart.then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(acquireCpu(1)).then(enterActivation(1)).then(exitActivation(1))
    .then(all { assert(q.activation.work == 12), assert(q.activation.reports == 1),
      assert(q.cpu.get(1) == 0), assert(resourceSafety), resourceStutter })
  action activationCloseProof = activationStart.then(enterActivation(1)).then(resourceClose)
    .then(normalActivation(1)).then(exitActivation(1)).then(acquireCpu(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action cycleCloseProof = cycleAccountingStart.then(enterActivation(1)).then(resourceClose)
    .then(normalActivation(1)).then(exitActivation(1)).then(acquireCpu(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1)).then(selectPlan(1)).then(compute(1))
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action activationAliasMutationProof = activationAliasStart.then(unsafeActivationAliasExit)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action activationEpochMutationProof = activationEpochStart.then(unsafeActivationEpochExit)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action leasePlanStart = workPlanStart.then(enableCache)
  action leasePlanSelectedStart = leasePlanStart.then(offer(1)).then(feed(1)).then(assignWorker(1,1))
    .then(boundary(1)).then(acquireCpu(1)).then(selectPlan(1))
  action leasePlanLateStart = leasePlanSelectedStart.then(offer(3)).then(feed(3))
  // A copied grant permission is already selected before unlocked handler
  // setup. A later short arrival invalidates the cache but not that snapshot.
  action leasePlanGrantLateProof = leasePlanStart.then(offer(1)).then(feed(1)).then(assignWorker(1,1))
    .then(boundary(1)).then(acquireCpu(1)).then(offer(3)).then(feed(3))
    .then(all { assert(q.cacheBulk.get(1)), assert(not(q.grantSlice.get(1))), assert(resourceSafety), resourceStutter })
    .then(selectPlan(1))
    .then(all { assert(q.selectedWork.get(1) == 12), assert(resourceSafety), resourceStutter })
    .then(compute(1))
    .then(all { assert(not(canSelectPlan(1))), assert(resourceSafety), resourceStutter })
  action leasePlanLateProof = leasePlanLateStart
    .then(all { assert(q.selectedWork.get(1) == 12), assert(q.cacheBulk.get(1)), assert(resourceSafety), resourceStutter })
    .then(compute(1))
    .then(all { assert(not(canSelectPlan(1))), assert(resourceSafety), resourceStutter })
    .then(yieldCpu(1)).then(assignWorker(2,3)).then(ackDispatch(2)).then(acquireCpu(2)).then(selectPlan(2))
    .then(compute(2))
    .then(all { assert(q.cacheBulk.get(1)), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(all { assert(not(q.cacheBulk.get(1))), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(1)).then(selectPlan(1))
    .then(all { assert(q.selectedWork.get(1) == 12), assert(resourceSafety), resourceStutter })
  action leasePlanCancelProof = leasePlanStart.then(offer(3)).then(feed(3)).then(assignWorker(1,3))
    .then(ackDispatch(1)).then(acquireCpu(1)).then(selectPlan(1)).then(abandon(3)).then(resourceClose)
    .then(compute(1)).then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action leasePlanMutationProof = leasePlanLateStart.then(unsafeStaleWorkCache)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action leasePlanMutationStep = any { sharedStep, unsafeStaleWorkCache }
  action workPlanStart = workBudgetStart.then(enablePlans)
  action planSelectedStart = workPlanStart.then(offer(1)).then(feed(1)).then(assignWorker(1,1))
    .then(boundary(1)).then(acquireCpu(1)).then(selectPlan(1))
  action planLateArrivalStart = planSelectedStart.then(offer(3)).then(feed(3)).then(compute(1))
  action planUncontendedProof = planSelectedStart
    .then(all { assert(q.selectedWork.get(1) == 12), assert(resourceSafety), resourceStutter })
    .then(compute(1))
    .then(all { assert(q.left.get(1) == 7), assert(q.selectedWork.get(1) == 0), assert(resourceSafety), resourceStutter })
    .then(selectPlan(1)).then(compute(1))
    .then(all { assert(q.left.get(1) == 6), assert(resourceSafety), resourceStutter })
  action planLateArrivalProof = planLateArrivalStart
    .then(all { assert(q.left.get(1) == 7), assert(not(canSelectPlan(1))), assert(not(canCompute(1))), assert(resourceSafety), resourceStutter })
    .then(yieldCpu(1)).then(assignWorker(2,3)).then(ackDispatch(2)).then(acquireCpu(2)).then(selectPlan(2))
    .then(all { assert(q.selectedWork.get(2) == 1), assert(resourceSafety), resourceStutter })
    .then(compute(2)).then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(all { assert(not(rootedJobs.contains(3))), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
  action planCancelCloseProof = workPlanStart.then(offer(3)).then(feed(3)).then(assignWorker(1,3))
    .then(ackDispatch(1)).then(acquireCpu(1)).then(selectPlan(1)).then(abandon(3)).then(resourceClose)
    .then(compute(1)).then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action planMutationProof = planLateArrivalStart.then(unsafePlanExtension(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action planMutationStep = any { sharedStep, unsafePlanExtension(1) }
  // Completion custody participates in the existing Scheme grant protocol.
  // A scheduler yield never consumes a ready generation; harvest does, before
  // root/capacity retirement. Close revokes the grant fence, not the roots.
  action reclamationReadyStart = leasePlanStart.then(enableReclamation)
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
    .then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(offer(1)).then(boundary(1)).then(feed(1)).then(assignWorker(1,1))
  action registrationStart = leasePlanStart.then(enableRegistration)
  action crossStart = runtimeJointStart.then(enableCross)
  action crossActorStart = crossStart.then(crossActorOffer)
  action crossActorOffer = tokioBegin(1).then(tokioPublish(1))
    .then(feed(1)).then(assignWorker(1,1)).then(registerWaiter(1)).then(acquireCpu(1)).then(actorDispatch)
  action crossActorStep(c: int): bool = actorRegister(c).then(actorSelect(c)).then(actorReceive(c)).then(actorExecute(c)).then(actorFinish(c))
  action crossActorComplete = crossActorStart.then(crossActorStep(1)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(nativeProjectRust(1)).then(ownerPublishReply(1))
  action crossTwoTokioProof = crossStart.then(crossTokioTwo).then(tokioBegin(1)).then(tokioBegin(3))
    .then(all { assert(q.tokioPolling.size() == 2), assert(resourceSafety), resourceStutter })
    .then(tokioPublish(1)).then(tokioPublish(3))
    .then(all { assert(q.tokioPolling.size() == 0), assert(q.client.get(1) == "await-reply"), assert(q.client.get(3) == "await-reply"), assert(resourceSafety), resourceStutter })
  action crossCapacityProof = crossStart.then(crossCapacityOne).then(crossActorOffer)
    .then(crossActorStep(1)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(nativeProjectRust(1)).then(ownerPublishReply(1))
    .then(tokioPollReply(1)).then(tokioObserve(1)).then(tokioBegin(3)).then(tokioCapacityWait(3))
    .then(all { assert(q.tokioPolling.size() == 0), assert(capacityJobs.contains(1)), assert(not(capacityJobs.contains(3))), assert(resourceSafety), resourceStutter })
    .then(project(1)).then(completionAck(1,q.completionEpoch.get(1))).then(crossCapacitySignal).then(tokioCapacityReady(3))
    .then(all { assert(not(capacityJobs.contains(1))), assert(q.client.get(3) == "submit-poll"), assert(resourceSafety), resourceStutter })
  action crossReplyBeforePermitProof = crossActorComplete.then(tokioPollReply(1)).then(tokioObserve(1))
    .then(all { assert(capacityJobs.contains(1)), assert(q.observedReplies.contains(1)), assert(resourceSafety), resourceStutter })
    .then(project(1)).then(completionAck(1,q.completionEpoch.get(1))).then(resourceClose).then(resourceJoin)
    .then(all { assert(capacityJobs.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action crossDroppedDrainProof = crossActorStart.then(tokioDrop(1)).then(resourceClose)
    .then(crossActorStep(1)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(nativeProjectRust(1))
    .then(project(1)).then(completionAck(1,q.completionEpoch.get(1))).then(resourceJoin)
    .then(all { assert(q.observedReplies.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action crossExternalStart = crossActorStart
    .then(actorRegister(1)).then(actorSelect(1)).then(actorReceive(1)).then(actorExternal(1))
    .then(actorRegister(2)).then(actorSelect(2)).then(actorReceive(2)).then(actorExternal(2))
  action crossNativeWaitProof = crossExternalStart.then(enterNativeWait(1))
    .then(all { assert(not(runtimeNeedsPass(1))), assert(q.tokioPolling.size() == 0), assert(resourceSafety), resourceStutter })
    .then(actorResume(1)).then(crossActorStep(1)).then(actorResume(2)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(nativeProjectRust(1)).then(ownerPublishReply(1))
    .then(project(1)).then(completionAck(1,q.completionEpoch.get(1))).then(resourceClose).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action crossTokioMutationProof = crossActorStart.then(unsafeTokioHeldAwait(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action crossActorMutationProof = crossActorStart.then(unsafeActorEarlyPublish)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action crossOsMutationProof = crossActorStart.then(enterNativeWait(1)).then(unsafeCrossOsSleep)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action runtimeJointStart = readyCycleStart.then(enableRuntimeObservation).then(enableRuntimeJoint)
  action runtimeExternalStart = runtimeJointStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1,1)).then(registerWaiter(1)).then(acquireCpu(1)).then(block(1))
  action runtimeExternalProof = runtimeExternalStart
    .then(all { assert(not(cycleCanPass(1))), assert(not(runtimeRunnable(1,false))), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
    .then(emptyHarvest(1)).then(wakeTask(1))
    .then(all { assert(cycleCanPass(1)), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(cyclePass(1))
    .then(all { assert(not(cycleCanPass(1))), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
  action runtimeRunnableReadyStart = runtimeJointStart.then(readyCycleBulkWork)
  action runtimeRunnableProof = runtimeRunnableReadyStart
    .then(all { assert(cycleCanPass(1)), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(cyclePass(1))
    .then(all { assert(q.executionPending == Set(3)), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(registerWaiter(2)).then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(resourceClose).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action runtimeBlockedShortProof = runtimeRunnableReadyStart
    .then(registerWaiter(2)).then(acquireCpu(2)).then(block(2))
    .then(all { assert(not(cycleCanPass(1))), assert(not(runtimeRunnable(1,true))), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(wakeTask(2)).then(registerWaiter(2)).then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(resourceClose).then(resourceJoin)
    .then(all { assert(resourceSafety), resourceStutter })
  // These are refinements of existing budget transactions, not new leases.
  // Native fusion must preserve exactly these atomic publication boundaries.
  action arrivalClockStart = completionAwaitStart.then(enableArrivalClock)
  action arrivalClockArmed = arrivalClockStart.then(clientPlan(3,0)).then(clockArm(3))
  action arrivalClockObserved = arrivalClockArmed.then(clientDue(3)).then(clockFire(3,1)).then(clockObserve(3))
  action arrivalClockAdmissionProof = arrivalClockObserved
    .then(all { assert(not(q.clientEligible.contains(3))), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
    .then(clientResume(3)).then(tokioBegin(3)).then(jointQueue(3))
    .then(all { assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
    .then(completionClaim(3)).then(all { assert(capacityJobs == Set(3)), assert(resourceSafety), resourceStutter })
  action arrivalClockOverdueProof = arrivalClockStart.then(clientPlan(3,0)).then(clientDue(3)).then(clockArm(3))
    .then(all { assert(q.clock.generation.get(3) == 1), assert(q.clientTimers.contains(3)), assert(resourceSafety), resourceStutter })
    .then(clockFire(3,1)).then(clockObserve(3)).then(clientResume(3))
    .then(all { assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action arrivalClockCancelProof = arrivalClockArmed.then(clockCancel(3)).then(clockArm(3)).then(clientDue(3)).then(clockStale(3,1))
    .then(all { assert(not(q.clock.observed.contains(3))), assert(not(q.clientEligible.contains(3))), assert(resourceSafety), resourceStutter })
    .then(clockFire(3,2)).then(clockObserve(3)).then(clientResume(3))
    .then(all { assert(resourceSafety), resourceStutter })
  action arrivalClockCloseProof = arrivalClockArmed.then(resourceClose).then(clockCancel(3)).then(completionFinishOwner(1)).then(completionFinishOwner(2)).then(resourceJoin)
    .then(all { assert(capacityJobs.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action arrivalClockEarlyMutationProof = arrivalClockArmed.then(unsafeClockEvent(3,1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action arrivalClockStaleMutationProof = arrivalClockArmed.then(clockCancel(3)).then(clockArm(3)).then(clientDue(3)).then(unsafeClockEvent(3,1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action arrivalClockQuotaMutationProof = arrivalClockObserved.then(unsafeClientDirectOffer(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action completionServiceStart = clientTurnBulkReady.then(enableCompletionService)
  action completionServiceNative = completionServiceStart.then(claimProgressNative).then(exhaustClientBudget)
  action completionServiceReadyProof = completionServiceNative.then(completionServiceCheck)
    .then(all { assert(q.observedReplies == Set(3)), assert(not(q.delivery.coopAvailable)), assert(deliveryQueued == Set(1)), assert(resourceSafety), resourceStutter })
    .then(restoreClientBudget).then(deliveryTake)
    .then(all { assert(q.observedReplies == Set(1,3)), assert(resourceSafety), resourceStutter })
  action completionServiceLegacyProof = completionServiceNative.then(cooperativeClientCheck)
    .then(all { assert(not(q.observedReplies.contains(3))), assert(q.clientWakeOwed.contains(3)), assert(resourceSafety), resourceStutter })
    .then(completionServiceCheck).then(all { assert(q.observedReplies.contains(3)), assert(resourceSafety), resourceStutter })
  action completionServiceClaimProof = completionServiceStart.then(completionClaim(3)).then(exhaustClientBudget).then(completionServiceCheck)
    .then(all { assert(not(q.observedReplies.contains(3))), assert(capacityJobs == Set(3)), assert(resourceSafety), resourceStutter })
    .then(restoreClientBudget).then(clientTurnCheck)
  action completionServiceCancelProof = completionServiceStart.then(exhaustClientBudget).then(completionCancel(3)).then(resourceClose)
    .then(completionFinishOwner(1)).then(completionFinishOwner(2)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action completionServiceReadyMutationProof = completionServiceStart.then(completionClaim(3)).then(unsafeJointClaimCompletes(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action preparationBeforeNextProof = completionServiceStart.then(preparationStart)
    .then(completionServiceCheck).then(preparationItem).then(claimProgressNative)
    .then(completionServiceCheck).then(preparationItem).then(completionServiceCheck)
    .then(all { assert(q.delivery.preparationCount == 2), assert(q.observedReplies == Set(3)), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action preparationFinalProof = completionServiceStart.then(preparationStart)
    .then(completionServiceCheck).then(preparationItem).then(completionServiceCheck).then(preparationItem)
    .then(claimProgressNative).then(completionServiceCheck)
    .then(all { assert(q.observedReplies == Set(3)), assert(resourceSafety), resourceStutter })
  action preparationCancelProof = completionServiceStart.then(preparationStart)
    .then(completionServiceCheck).then(preparationItem).then(preparationDiscard).then(completionCancel(3))
    .then(resourceClose).then(completionFinishOwner(1)).then(completionFinishOwner(2)).then(resourceJoin)
    .then(all { assert(q.delivery.preparationCount == 0), assert(rootedJobs.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action preparationSkipProof = completionServiceStart.then(preparationStart).then(unsafePreparationSkip)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action streamingJointStart = claimProgressArmed.then(enableDelivery).then(enableClientTurn).then(enableCompletionService).then(jointInputOpen)
  action streamingJointClientProof = streamingJointStart.then(completionServiceCheck).then(jointInputWindow(1))
    .then(claimProgressNative).then(completionServiceCheck).then(jointInputWindow(2)).then(jointInputSeal)
    .then(all { assert(q.observedReplies == Set(3)), assert(unassignedJobs == Set(1,2)), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action streamingJointLastProof = streamingJointStart.then(completionServiceCheck).then(jointInputWindow(1))
    .then(claimProgressNative).then(completionServiceCheck).then(jointInputSeal)
    .then(all { assert(q.observedReplies == Set(3)), assert(resourceSafety), resourceStutter })
  action clientTurnStart = claimProgressArmed.then(enableDelivery).then(enableClientTurn)
  action clientTurnBulkReady = clientTurnStart.then(deliveryJob(1,1,1)).then(completionAck(1,q.completionEpoch.get(1))).then(boundary(1))
  action clientTurnFinalProof = clientTurnBulkReady.then(claimProgressNative).then(clientTurnCheck)
    .then(all { assert(q.observedReplies.contains(3)), assert(deliveryQueued == Set(1)), assert(resourceSafety), resourceStutter })
    .then(deliveryTake).then(all { assert(q.observedReplies == Set(1,3)), assert(resourceSafety), resourceStutter })
  action clientTurnEarlyProof = clientTurnBulkReady.then(completionClaim(3)).then(clientTurnCheck)
    .then(all { assert(q.placementObserved == Set(3)), assert(not(q.observedReplies.contains(3))), assert(capacityJobs == Set(3)), assert(resourceSafety), resourceStutter })
    .then(deliveryTake).then(all { assert(capacityJobs == Set(3)), assert(resourceSafety), resourceStutter })
  action clientTurnSkipProof = clientTurnBulkReady.then(unsafeClientTurnSkip)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action clientTurnBusyProof = clientTurnBulkReady.then(completionClaim(3)).then(jointBeginPoll(3))
    .then(all { assert(not(clientTurnCanCheck)), assert(resourceSafety), resourceStutter })
    .then(jointPollPending(3)).then(clientTurnCheck).then(deliveryTake)
    .then(all { assert(resourceSafety), resourceStutter })
  action clientTurnLateEventProof = clientTurnBulkReady.then(clientTurnCheck).then(completionClaim(3))
    .then(deliveryTake).then(clientTurnCheck)
    .then(all { assert(q.placementObserved == Set(3)), assert(resourceSafety), resourceStutter })
  action deliveryStart = publicationCrossStart.then(enableDelivery)
  action deliveryBegin(r: int, w: int): bool = clientPlan(r,0).then(clientDue(r)).then(clientResume(r)).then(tokioBegin(r)).then(tokioPublish(r))
    .then(feed(r)).then(assignWorker(w,r)).then(registerWaiter(w)).then(acquireCpu(w))
  action deliveryUnit(w: int): bool = selectPlan(w).then(compute(w))
  action deliveryCompute(r: int, w: int): bool = if (r == 1)
    deliveryUnit(w).then(deliveryUnit(w)).then(deliveryUnit(w)).then(deliveryUnit(w))
      .then(deliveryUnit(w)).then(deliveryUnit(w)).then(deliveryUnit(w)).then(deliveryUnit(w))
    else deliveryUnit(w)
  action deliveryJob(r: int, w: int, owner: int): bool = deliveryBegin(r,w).then(deliveryCompute(r,w))
    .then(harvest(owner,Set(r))).then(batchTransfer(owner)).then(nativeProjectRust(r)).then(project(r)).then(ownerPublishReply(r))
  action deliveryOne = deliveryStart.then(deliveryJob(1,1,1))
  action deliveryEarlyProof = deliveryStart.then(deliveryPending).then(deliveryJob(1,1,1))
    .then(all { assert(q.delivery.wakeOwed), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
    .then(deliveryTake).then(all { assert(q.delivery.consumed == Set(1)), assert(resourceSafety), resourceStutter })
  action deliveryRefillProof = deliveryOne.then(completionAck(1,q.completionEpoch.get(1)))
    .then(all { assert(q.delivery.consumed.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
    .then(deliveryTake).then(resourceClose).then(resourceJoin)
  action deliveryDropProof = deliveryStart.then(deliveryPending).then(deliveryDrop).then(deliveryJob(1,1,1))
    .then(resourceClose).then(completionAck(1,q.completionEpoch.get(1))).then(resourceJoin)
    .then(all { assert(deliveryQueued.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action deliveryWakeMutationProof = deliveryStart.then(deliveryPending).then(deliveryJob(1,1,1)).then(unsafeDeliveryWake)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action deliveryRotationProof = deliveryOne.then(completionAck(1,q.completionEpoch.get(1)))
    .then(deliveryJob(5,3,2)).then(deliveryTake)
    .then(all { assert(q.delivery.cursor == 2), assert(q.delivery.consumed == Set(1)), assert(resourceSafety), resourceStutter })
    .then(deliveryTake).then(all { assert(q.delivery.consumed == Set(1,5)), assert(resourceSafety), resourceStutter })
  action deliveryPickMutationProof = deliveryOne.then(deliveryJob(5,3,2)).then(unsafeDeliveryPick)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action deliveryBypassProof = deliveryStart.then(deliveryBegin(1,1)).then(block(1)).then(deliveryJob(5,3,2)).then(deliveryTake)
    .then(all { assert(q.delivery.consumed == Set(5)), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
  action claimProgressStart = completionAwaitStart
  action claimProgressQueued = claimProgressStart.then(clientPlan(3,0)).then(clientDue(3)).then(clientResume(3)).then(tokioBegin(3)).then(jointQueue(3))
  action claimProgressArmed = claimProgressQueued.then(jointBeginPoll(3)).then(jointPollPending(3))
  action claimProgressNative = completionClaim(3).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
    .then(registerWaiter(2)).then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(nativeProjectRust(3)).then(project(3)).then(ownerPublishReply(3))
  action claimProgressEarlyProof = claimProgressArmed.then(completionClaim(3))
    .then(all { assert(q.clientWakeOwed == Set(3)), assert(q.observedReplies.size() == 0), assert(q.tokioPolling.size() == 0), assert(resourceSafety), resourceStutter })
    .then(jointBeginPoll(3)).then(jointPollPending(3))
    .then(all { assert(q.placementObserved == Set(3)), assert(q.clientWakeOwed.size() == 0), assert(capacityJobs == Set(3)), assert(resourceSafety), resourceStutter })
  action claimProgressCoalescedProof = claimProgressArmed.then(claimProgressNative)
    .then(all { assert(q.clientWakeOwed == Set(3)), assert(q.placementObserved.size() == 0), assert(q.tokioPolling.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
    .then(jointBeginPoll(3)).then(jointPollReady(3))
    .then(all { assert(q.clientWakeOwed.size() == 0), assert(q.observedReplies == Set(3)), assert(q.placementObserved.size() == 0), assert(resourceSafety), resourceStutter })
  action claimProgressLateArmProof = claimProgressQueued.then(claimProgressNative)
    .then(all { assert(q.clientWakeOwed.size() == 0), assert(q.replies == Set(3)), assert(resourceSafety), resourceStutter })
    .then(jointBeginPoll(3)).then(jointPollReady(3))
    .then(all { assert(q.observedReplies == Set(3)), assert(resourceSafety), resourceStutter })
  action claimProgressFinalDuringPollProof = claimProgressQueued.then(jointBeginPoll(3)).then(claimProgressNative)
    .then(jointPollReady(3)).then(all { assert(q.observedReplies == Set(3)), assert(resourceSafety), resourceStutter })
  action claimProgressCancelProof = claimProgressArmed.then(completionCancel(3)).then(resourceClose)
    .then(completionFinishOwner(1)).then(completionFinishOwner(2)).then(resourceJoin)
    .then(all { assert(q.receiveArmed.size() == 0), assert(q.clientWakeOwed.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action claimProgressOwnerProof = claimProgressArmed.then(claimProgressNative).then(completionAck(1,q.completionEpoch.get(1)))
    .then(clientPlan(1,0)).then(clientDue(1)).then(clientResume(1)).then(tokioBegin(1)).then(tokioPublish(1))
    .then(all { assert(q.observedReplies.size() == 0), assert(capacityJobs == Set(1)), assert(q.clientWakeOwed == Set(3)), assert(resourceSafety), resourceStutter })
  action claimProgressDropProof = claimProgressArmed.then(completionClaim(3)).then(tokioDrop(3)).then(completionCancel(3))
    .then(all { assert(q.clientWakeOwed.size() == 0), assert(q.receiveArmed.size() == 0), assert(capacityJobs == Set(3)), assert(resourceSafety), resourceStutter })
  action claimProgressLostWakeProof = claimProgressArmed.then(completionClaim(3)).then(unsafeJointLoseWake(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action claimProgressEarlyValueProof = claimProgressArmed.then(completionClaim(3)).then(unsafeJointClaimCompletes(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action completionAwaitStart = publicationCrossStart
    .then(completionInstallOwner(1))
    .then(completionInstallOwner(2))
    .then(completionEnablePool)
  action completionQueueStart(direct: bool): bool = completionAwaitStart
    .then(clientPlan(3,0)).then(clientDue(3)).then(clientResume(3)).then(tokioBegin(3)).then(completionQueue(3,direct))
  action completionQueueBoundProof = completionQueueStart(true)
    .then(all { assert(unassignedJobs == Set(3)), assert(capacityJobs.size() == 0), assert(rootedJobs.size() == 0), assert(q.tokioPolling.size() == 0), assert(resourceSafety), resourceStutter })
  action completionQueueClaimProof = completionQueueStart(true).then(completionClaim(3))
    .then(all { assert(q.placementRequired.size() == 0), assert(q.client.get(3) == "await-reply"), assert(capacityJobs == Set(3)), assert(resourceSafety), resourceStutter })
    .then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
    .then(registerWaiter(2)).then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(nativeProjectRust(3)).then(project(3)).then(ownerPublishReply(3))
    .then(all { assert(q.placementObserved.size() == 0), assert(q.observedReplies.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
    .then(tokioPollReply(3)).then(tokioObserve(3))
    .then(all { assert(q.observedReplies == Set(3)), assert(resourceSafety), resourceStutter })
  action completionPlacementProof = completionQueueStart(false).then(completionClaim(3)).then(feed(3))
    .then(all { assert(q.client.get(3) == "await-placement"), assert(rootedJobs == Set(3)), assert(resourceSafety), resourceStutter })
    .then(completionPollPlacement(3)).then(completionObservePlacement(3))
    .then(all { assert(q.client.get(3) == "await-reply"), assert(q.placementObserved == Set(3)), assert(resourceSafety), resourceStutter })
  action completionQueueCancelProof = completionQueueStart(true).then(completionCancel(3)).then(completionCancel(3))
    .then(resourceClose).then(completionFinishOwner(1)).then(completionFinishOwner(2)).then(resourceJoin)
    .then(all { assert(unassignedJobs.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action completionClaimedCancelProof = completionQueueStart(true).then(completionClaim(3)).then(tokioDrop(3)).then(completionCancel(3))
    .then(all { assert(capacityJobs == Set(3)), assert(acceptedJobs == Set(3)), assert(resourceSafety), resourceStutter })
  action completionQueueCloseProof = completionQueueStart(true).then(resourceClose).then(completionCancel(3)).then(completionFinishOwner(1)).then(completionFinishOwner(2)).then(resourceJoin)
    .then(all { assert(unassignedJobs.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action completionQueueMutationProof = completionQueueStart(true).then(completionQueueMutation(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action publicationCrossStart = clientCrossStart.then(publicationMode(true))
  action publicationNativeReady = publicationCrossStart
    .then(clientPlan(1,0)).then(clientDue(1)).then(clientResume(1)).then(crossActorOffer)
    .then(crossActorStep(1)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(nativeProjectRust(1))
  action publicationOrderProof = publicationNativeReady
    .then(all { assert(not(publicationCanSend(1))), assert(capacityJobs.contains(1)), assert(q.observedReplies.size() == 0), assert(resourceSafety), resourceStutter })
    .then(project(1)).then(ownerPublishReply(1))
    .then(all { assert(q.replies == Set(1)), assert(not(capacityJobs.contains(1))), assert(q.observedReplies.size() == 0), assert(resourceSafety), resourceStutter })
    .then(tokioPollReply(1)).then(tokioObserve(1))
    .then(all { assert(resourceSafety), resourceStutter })
  action publicationSnapshotProof = publicationNativeReady.then(publicationMode(false))
    .then(all { assert(q.publicationAfter == Set(1)), assert(not(publicationCanSend(1))), assert(resourceSafety), resourceStutter })
    .then(project(1)).then(ownerPublishReply(1))
    .then(all { assert(resourceSafety), resourceStutter })
  action publicationDropCloseProof = publicationNativeReady.then(tokioDrop(1)).then(resourceClose)
    .then(all { assert(capacityJobs.contains(1)), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
    .then(project(1)).then(ownerPublishReply(1)).then(completionAck(1,q.completionEpoch.get(1))).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(q.observedReplies.size() == 0), assert(resourceSafety), resourceStutter })
  action publicationOwnerOpportunityProof = publicationNativeReady.then(project(1)).then(ownerPublishReply(1))
    .then(completionAck(1,q.completionEpoch.get(1)))
    .then(clientPlan(3,0)).then(clientDue(3)).then(clientResume(3)).then(tokioBegin(3)).then(tokioPublish(3))
    .then(all { assert(not(q.observedReplies.contains(1))), assert(capacityJobs.contains(3)), assert(resourceSafety), resourceStutter })
  action publicationEarlySendMutationProof = publicationNativeReady.then(unsafePublicationBeforeReturn(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action clientCrossStart = nativeCrossStart.then(enableClientProgress)
  action clientArrivalStart = clientCrossStart.then(clientPlan(1,0)).then(clientDue(1)).then(clientResume(1))
  action clientActorReady = clientArrivalStart.then(crossActorOffer)
    .then(crossActorStep(1)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(nativeProjectRust(1)).then(ownerPublishReply(1))
  action clientTimerProof = clientCrossStart.then(clientPlan(1,0)).then(clientTimer(1))
    .then(all { assert(q.tokioPolling.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
    .then(clientDue(1)).then(clientResume(1)).then(tokioBegin(1)).then(tokioPublish(1))
    .then(all { assert(q.projectedRust.size() == 0), assert(q.observedReplies.size() == 0), assert(resourceSafety), resourceStutter })
  action clientSerialProof = clientActorReady.then(clientPlan(3,1)).then(clientDue(3))
    .then(all { assert(not(clientCanResume(3))), assert(q.projectedRust == Set(1)), assert(q.observedReplies.size() == 0), assert(resourceSafety), resourceStutter })
    .then(tokioPollReply(1)).then(tokioObserve(1)).then(clientResume(3)).then(tokioBegin(3))
    .then(all { assert(capacityJobs.contains(1)), assert(q.clientEligible.contains(3)), assert(resourceSafety), resourceStutter })
  action clientAlreadyDueProof = clientCrossStart.then(clientPlan(3,0)).then(clientDue(3)).then(clientResume(3))
    .then(all { assert(q.clientTimers.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action clientLegacyDueProof = clientCrossStart.then(clientPlan(3,0)).then(clientDue(3)).then(clientTimer(3)).then(clientResume(3))
    .then(all { assert(q.clientTimers.size() == 0), assert(capacityJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action clientCancelTimerProof = clientCrossStart.then(clientPlan(1,0)).then(clientTimer(1)).then(clientForget(1))
    .then(resourceClose).then(resourceJoin)
    .then(all { assert(q.clientTimers.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action clientIndependentOwnerProof = clientActorReady.then(project(1)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(all { assert(q.observedReplies.size() == 0), assert(not(capacityJobs.contains(1))), assert(resourceSafety), resourceStutter })
    .then(clientPlan(3,0)).then(clientDue(3)).then(clientResume(3)).then(tokioBegin(3)).then(tokioPublish(3))
    .then(all { assert(q.observedReplies.size() == 0), assert(capacityJobs.contains(3)), assert(resourceSafety), resourceStutter })
  action clientDirectEntryMutationProof = clientCrossStart.then(clientPlan(1,0)).then(unsafeClientDirectOffer(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action clientEarlyIssueMutationProof = clientCrossStart.then(clientPlan(1,0)).then(unsafeClientEarlyIssue(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action clientEarlyConsumeMutationProof = clientArrivalStart.then(crossActorOffer).then(unsafeClientEarlyConsume(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action nativeSchedulerStart = readyCycleStart.then(enableNativeScheduler)
  action nativeCrossStart = nativeSchedulerStart.then(enableCross)
  action nativeRunnableStart = nativeSchedulerStart.then(readyCycleBulkWork)
  action nativeRunnableProof = nativeRunnableStart
    .then(all { assert(not(q.runtimeObserved)), assert(nativeShortEligible(1)), assert(q.cpu.get(1) == 0), assert(q.waiters.size() == 0), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(cyclePass(1))
    .then(all { assert(q.executionPending == Set(3)), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(all { assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
  action nativeExternalDrainProof = nativeRunnableStart
    .then(registerWaiter(2)).then(acquireCpu(2)).then(block(2)).then(abandon(3)).then(resourceClose)
    .then(all { assert(not(nativeShortEligible(1))), assert(not(cycleCanPass(1))), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(wakeTask(2)).then(registerWaiter(2)).then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,q.completionEpoch.get(1))).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action nativeEmptyPassProof = nativeCrossStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1,1)).then(registerWaiter(1)).then(acquireCpu(1)).then(block(1))
    .then(all { assert(cycleCanPass(1)), assert(not(q.runtimeObserved)), assert(resourceSafety), resourceStutter })
    .then(cyclePass(1)).then(emptyHarvest(1)).then(enterNativeWait(1)).then(enterOsSleep(1))
    .then(wakeTask(1))
    .then(all { assert(not(q.osSleeping.contains(1))), assert(cycleCanPass(1)), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
  action nativeActorStart = nativeCrossStart.then(crossActorOffer).then(nativeActorClassShort)
  action nativeActorSourceProof = nativeActorStart
    .then(all { assert(nativeShortEligible(1)), assert(q.tokioPolling.size() == 0), assert(not(q.runtimeObserved)), assert(resourceSafety), resourceStutter })
    .then(actorRegister(1)).then(actorSelect(1)).then(actorReceive(1)).then(actorExternal(1))
    .then(actorRegister(2)).then(actorSelect(2)).then(actorReceive(2)).then(actorExternal(2))
    .then(all { assert(not(nativeShortEligible(1))), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
    .then(actorResume(1))
    .then(all { assert(nativeShortEligible(1)), assert(resourceSafety), resourceStutter })
  action nativeSkipPassMutationProof = nativeRunnableStart.then(unsafeNativeHarvest(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action nativeOsSleepMutationProof = nativeActorStart.then(enterNativeWait(1)).then(unsafeCrossOsSleep)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action runtimeFusedSuspensionProof = runtimeExternalStart
    .then(all { assert(q.cpu.get(1) == 0), assert(runtimeBudgetPhase(1) == "external"), assert(resourceSafety), resourceStutter })
    .then(abandon(1)).then(resourceClose)
    .then(all { assert(rootedJobs.contains(1)), assert(runtimeBudgetPhase(1) == "external"), assert(resourceSafety), resourceStutter })
    .then(wakeTask(1)).then(registerWaiter(1))
    .then(all { assert(runtimeBudgetPhase(1) == "lease-wait"), assert(q.cpu.get(1) == 0), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(1))
    .then(all { assert(runtimeBudgetPhase(1) == "received"), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
  action runtimeFusedDispatchProof = runtimeJointStart.then(offer(3)).then(feed(3))
    .then(all { assert(runtimeRunnable(1,true)), assert(q.dispatchPending == Set(3)), assert(q.executionPending == Set(3)), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(assignWorker(2,3)).then(ackDispatch(2))
    .then(all { assert(q.dispatchPending.size() == 0), assert(q.executionPending == Set(3)), assert(runtimeRunnable(1,true)), assert(resourceSafety), resourceStutter })
  action runtimeFusedActorProof = crossActorStart
    .then(all { assert(q.actorPhase.get(1) == "runnable"), assert(q.actorPhase.get(2) == "runnable"), assert(runtimeBudgetPhase(1) == "actor-barrier"), assert(q.tokioPolling.size() == 0), assert(resourceSafety), resourceStutter })
    .then(crossActorStep(1)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(all { assert(resourceSafety), resourceStutter })
  action runtimeFusedSuspensionMutationProof = runtimeExternalStart.then(unsafeSuspendedOwner(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action runtimeIndexReplayProof = activationStart
    .then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(all { assert(q.activation.published.size() == 0), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(1)).then(enterActivation(1))
    .then(all { assert(q.activation.published == Set(1)), assert(resourceSafety), resourceStutter })
    .then(exitActivation(1))
    .then(all { assert(q.activation.published.size() == 0), assert(resourceSafety), resourceStutter })
  action runtimeIndexMutationProof = activationStart.then(enterActivation(1)).then(unsafeActivationIndexRetire)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action runtimeBudgetReceiptProof = directedQueuedStart
    .then(all { assert(runtimeBudgetPhase(3) == "lease-wait"), assert(resourceSafety), resourceStutter })
    .then(selectWaiter(2))
    .then(all { assert(q.handoff.get(1) == 2), assert(runtimeBudgetPhase(3) == "lease-wait"), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(2))
    .then(all { assert(runtimeBudgetPhase(3) == "received"), assert(q.executionAck == Set(3)), assert(resourceSafety), resourceStutter })
  action runtimeBudgetActorProof = crossActorStart
    .then(actorRegister(1)).then(actorSelect(1))
    .then(all { assert(q.actorPhase.get(1) == "granted"), assert(q.actorRemaining.get(1) == 1), assert(runtimeBudgetPhase(1) == "actor-barrier"), assert(resourceSafety), resourceStutter })
    .then(actorReceive(1)).then(actorExternal(1))
    .then(all { assert(q.actorLease == 0), assert(q.actorPhase.get(1) == "external"), assert(q.tokioPolling.size() == 0), assert(resourceSafety), resourceStutter })
    .then(actorResume(1)).then(crossActorStep(1)).then(crossActorStep(2)).then(actorPublishBatch)
    .then(all { assert(resourceSafety), resourceStutter })
  action runtimeExternalMutationProof = runtimeExternalStart.then(unsafeCyclePass(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action runtimeHarvestMutationProof = runtimeRunnableReadyStart.then(unsafeRuntimeHarvest(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action progressCycleStart = readyCycleStart.then(enableProgressCycle)
  // An exceptional result is ready without logical execution credit; another
  // native job is still unfinished. Retirement must not erase the opportunity.
  action progressCycleStalledStart = progressCycleStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1,1))
    .then(offer(3)).then(boundary(1)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2)).then(finishUngrant(2))
  action progressCycleStalledProof = progressCycleStalledStart
    .then(all { assert(stalledReady(1)), assert(cycleCanPass(1)), assert(resourceSafety), resourceStutter })
    .then(cyclePass(1))
    .then(all { assert(not(cycleCanPass(1))), assert(q.executed.get(1) == 0), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(all { assert(q.executed.get(1) == 0), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
  action progressCycleDrainProof = progressCycleStart
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2)).then(finishUngrant(2))
    .then(all { assert(not(stalledReady(1))), assert(not(cycleCanPass(1))), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(resourceClose).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action progressCycleMutationProof = progressCycleStalledStart
    .then(unsafeProgressHarvest(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action readyCycleStart = executionCheckpointStart.then(enableReadyFirstCycle)
  action readyCycleBulkStart = readyCycleStart.then(readyCycleBulkWork)
  action readyCycleBulkWork = offer(1).then(feed(1)).then(assignWorker(1,1)).then(registerWaiter(1)).then(acquireCpu(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(offer(3)).then(boundary(1)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
  action readyCycleCollectProof = readyCycleBulkStart
    .then(all { assert(not(cycleCanPass(1))), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(1))).then(batchTransfer(1))
    .then(all { assert(rootedJobs.contains(1)), assert(q.executionPending == Set(3)), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(project(1)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(all { assert(not(rootedJobs.contains(1))), assert(rootedJobs.contains(3)), assert(cycleCanPass(1)), assert(resourceSafety), resourceStutter })
    .then(cyclePass(1)).then(registerWaiter(2)).then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(resourceClose).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action readyCycleSinglePassProof = readyCycleStart
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
    .then(cyclePass(1))
    .then(all { assert(not(cycleCanPass(1))), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(emptyHarvest(1))
    .then(all { assert(cycleCanPass(1)), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
  action readyCycleCloseProof = readyCycleBulkStart.then(abandon(3)).then(resourceClose)
    .then(harvest(1,Set(1))).then(batchTransfer(1)).then(project(1)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(cyclePass(1)).then(finishUngrant(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,q.completionEpoch.get(1)))
    .then(resourceJoin).then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action readyCycleMutationProof = readyCycleBulkStart.then(unsafeCyclePass(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action executionCheckpointStart = registrationStart.then(enableExecutionCheckpoint)
  action executionCheckpointRunnableStart = executionCheckpointStart
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
  action executionCheckpointReceiptProof = executionCheckpointRunnableStart
    .then(all { assert(not(dispatchDue(1))), assert(checkpointDue(1)), assert(resourceSafety), resourceStutter })
    .then(jointTurn(1)).then(emptyHarvest(1)).then(jointTurn(1))
    .then(all { assert(q.executionPending == Set(3)), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(registerWaiter(2)).then(acquireCpu(2))
    .then(all { assert(not(checkpointDue(1))), assert(q.jobs.get(2) == 3), assert(not(readyJobs.contains(3))), assert(resourceSafety), resourceStutter })
  action executionCheckpointCloseProof = executionCheckpointRunnableStart
    .then(abandon(3)).then(resourceClose).then(jointTurn(1)).then(finishUngrant(2))
    .then(all { assert(not(checkpointDue(1))), assert(q.executionUngrant == Set(3)), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action executionCheckpointBulkReadyStart = executionCheckpointStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1,1)).then(registerWaiter(1)).then(acquireCpu(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(selectPlan(1)).then(compute(1))
    .then(offer(3)).then(boundary(1)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
  action executionCheckpointMutationProof = executionCheckpointBulkReadyStart
    .then(unsafeJointHarvest(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action directedStart = registrationStart.then(enableDirected)
  action directedQueuedStart = directedStart
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2)).then(registerWaiter(2))
    .then(offer(1)).then(boundary(1)).then(feed(1)).then(assignWorker(1,1)).then(registerWaiter(1))
  action directedReceiptProof = directedQueuedStart.then(selectWaiter(2))
    .then(all { assert(q.executionPending == Set(3)), assert(q.executionAck.size() == 0),
      assert(q.signals == Set(2)), assert(resourceSafety), resourceStutter })
    .then(spuriousWake(1))
    .then(all { assert(q.handoff.get(1) == 2), assert(q.executionAck.size() == 0), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(selectWaiter(1)).then(acquireCpu(1)).then(selectPlan(1))
    .then(all { assert(q.signals.size() == 0), assert(q.executionAck == Set(3)), assert(resourceSafety), resourceStutter })
  action directedCloseProof = directedStart
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2)).then(registerWaiter(2))
    .then(selectWaiter(2)).then(abandon(3)).then(resourceClose)
    .then(all { assert(rootedJobs.contains(3)), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action directedMutationProof = directedQueuedStart.then(selectWaiter(2)).then(unsafeForeignWake(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action registrationRunnableStart = registrationStart
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
  action registrationStagesProof = registrationRunnableStart
    .then(all { assert(q.dispatchPending.size() == 0), assert(q.executionPending == Set(3)),
      assert(q.waiters.size() == 0), assert(q.executionAck.size() == 0), assert(q.cacheBulk.get(1)), assert(resourceSafety), resourceStutter })
    .then(registerWaiter(2))
    .then(all { assert(q.waiters == Set(2)), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(2))
    .then(all { assert(q.waiters.size() == 0), assert(q.executionPending.size() == 0), assert(q.executionAck == Set(3)), assert(resourceSafety), resourceStutter })
    .then(selectPlan(2)).then(compute(2)).then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(resourceClose).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action registrationFairProof = registrationRunnableStart.then(offer(1)).then(boundary(1)).then(feed(1)).then(assignWorker(1,1))
    .then(registerWaiter(1))
    .then(all { assert(allowedCpu(1)), assert(q.executionPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(registerWaiter(2))
    .then(all { assert(not(allowedCpu(1))), assert(allowedCpu(2)), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(acquireCpu(1)).then(selectPlan(1))
    .then(all { assert(q.selectedWork.get(1) == 12), assert(resourceSafety), resourceStutter })
  action registrationUngrantProof = registrationRunnableStart.then(abandon(3)).then(resourceClose).then(finishUngrant(2))
    .then(all { assert(q.executionAck.size() == 0), assert(q.executionUngrant == Set(3)), assert(q.executionPending.size() == 0), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action registrationContinuationProof = registrationStart.then(offer(3)).then(feed(3)).then(assignWorker(1,3))
    .then(ackDispatch(1)).then(registerWaiter(1)).then(acquireCpu(1)).then(selectPlan(1)).then(enableActivations)
    .then(enterActivation(1)).then(exitActivation(1))
    .then(registerWaiter(1)).then(acquireCpu(1)).then(enterActivation(1)).then(normalActivation(1)).then(exitActivation(1))
    .then(registerWaiter(1)).then(acquireCpu(1)).then(enterActivation(1)).then(exitActivation(1))
    .then(all { assert(q.executionAck == Set(3)), assert(q.executionPending.size() == 0), assert(resourceSafety), resourceStutter })
  action registrationGrantMutationProof = registrationRunnableStart.then(unsafeUnregisteredGrant(2))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action registrationMutationProof = registrationRunnableStart.then(unsafeDispatchExecutionAck(2))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action receiverReadyStart = leasePlanStart.then(enableReceiver)
    .then(offer(3)).then(feed(3)).then(assignWorker(2,3)).then(ackDispatch(2))
    .then(acquireCpu(2)).then(selectPlan(2)).then(compute(2))
    .then(offer(1)).then(boundary(1)).then(feed(1)).then(assignWorker(1,1))
  action receiverOpportunityProof = receiverReadyStart
    .then(all { assert(allowedCpu(1)), assert(resourceSafety), resourceStutter })
    .then(receiverEnter(1))
    .then(all { assert(not(allowedCpu(1))), assert(resourceSafety), resourceStutter })
    .then(receiverLeave(1))
    .then(all { assert(allowedCpu(1)), assert(q.workOffered.get(1) == 0), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
    .then(receiverEnter(1)).then(harvest(1,Set(3)))
    .then(all { assert(allowedCpu(1)), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
    .then(batchTransfer(1)).then(project(3)).then(completionAck(1,1)).then(receiverLeave(1))
    .then(acquireCpu(1)).then(selectPlan(1))
    .then(all { assert(q.selectedWork.get(1) == 12), assert(resourceSafety), resourceStutter })
  action receiverCloseProof = receiverReadyStart.then(receiverEnter(1)).then(abandon(3)).then(resourceClose)
    .then(all { assert(q.receiving.size() == 0), assert(allowedCpu(1)), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(acquireCpu(1)).then(selectPlan(1))
    .then(all { assert(resourceSafety), resourceStutter })
  action receiverMutationProof = receiverReadyStart.then(receiverEnter(1)).then(receiverLeave(1)).then(unsafeReclamationYield)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action reclamationHarvestProof = reclamationReadyStart
    .then(all { assert(not(allowedCpu(1))), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3)))
    .then(all { assert(allowedCpu(1)), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
    .then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(acquireCpu(1)).then(selectPlan(1))
    .then(all { assert(q.selectedWork.get(1) == 12), assert(resourceSafety), resourceStutter })
  action reclamationCloseProof = reclamationReadyStart.then(abandon(3)).then(resourceClose)
    .then(all { assert(allowedCpu(1)), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(acquireCpu(1)).then(selectPlan(1))
    .then(all { assert(resourceSafety), resourceStutter })
  action reclamationMutationProof = reclamationReadyStart.then(unsafeReclamationYield)
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action workBudgetStart = pipelineCycleStart.then(enableJoint).then(enableWork)
  action workContendedStart = workBudgetStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1,1))
    .then(offer(3)).then(boundary(1)).then(feed(3))
    .then(acquireCpu(1)).then(compute(1))
  action workSpanProof = workContendedStart
    .then(all { assert(q.left.get(1) == 8), assert(q.workLeft.get(1) == 11), assert(not(canCompute(1))), assert(resourceSafety), resourceStutter })
    .then(yieldCpu(1)).then(assignWorker(2,3)).then(ackDispatch(2)).then(acquireCpu(2)).then(compute(2))
    .then(all { assert(readyJobs.contains(3)), assert(q.left.get(1) == 8), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1))
    .then(all { assert(not(rootedJobs.contains(3))), assert(rootedJobs.contains(1)), assert(q.workOffered.get(1) == q.workReady.get(1)), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(1)).then(compute(1)).then(compute(1))
    .then(all { assert(q.workLeft.get(1) == 9), assert(canCompute(1)), assert(resourceSafety), resourceStutter })
  action workUncontendedProof = workBudgetStart.then(offer(1)).then(feed(1)).then(assignWorker(1,1)).then(acquireCpu(1))
    .then(compute(1)).then(compute(1)).then(compute(1))
    .then(all { assert(q.cpu.get(1) == 1), assert(q.workLeft.get(1) == 9), assert(q.left.get(1) == 8), assert(canCompute(1)), assert(resourceSafety), resourceStutter })
  action workCancelCloseProof = workBudgetStart.then(offer(3)).then(feed(3)).then(assignWorker(1,3))
    .then(abandon(3)).then(resourceClose).then(ackDispatch(1)).then(acquireCpu(1)).then(compute(1))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action workMutationProof = workContendedStart.then(unsafeWorkSpan(1)).then(all { assert(not(resourceSafety)), resourceStutter })
  action workMutationStep = any { sharedStep, unsafeWorkSpan(1) }
  action jointStart = pipelineCycleStart.then(enableJoint)
  action jointBulkReadyStart = jointStart
    .then(offer(1)).then(feed(1)).then(assignWorker(1,1)).then(acquireCpu(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1))
    .then(offer(3)).then(boundary(1)).then(feed(3)).then(assignWorker(2,3))
  action jointObligationProof = jointBulkReadyStart.then(jointTurn(1))
    .then(all { assert(q.dispatchPending == Set(3)), assert(q.turnOffered == Set(1)), assert(resourceSafety), resourceStutter })
    .then(ackDispatch(2)).then(acquireCpu(2)).then(compute(2))
    .then(harvest(1,Set(1,3))).then(batchTransfer(1)).then(project(3)).then(project(1)).then(completionAck(1,2))
    .then(resourceClose).then(resourceJoin)
    .then(all { assert(q.dispatchPending.size() == 0), assert(resourceSafety), resourceStutter })
  action jointCloseProof = jointBulkReadyStart.then(abandon(3)).then(resourceClose).then(jointTurn(1))
    .then(ackDispatch(2)).then(acquireCpu(2)).then(compute(2))
    .then(harvest(1,Set(1,3))).then(batchTransfer(1)).then(project(1)).then(project(3)).then(completionAck(1,2))
    .then(resourceJoin).then(all { assert(resourceSafety), resourceStutter })
  action jointWaitProof = jointStart.then(offer(3)).then(feed(3)).then(assignWorker(1,3))
    .then(all { assert(nativeCanPark(1)), assert(not(canSleep(1))), assert(resourceSafety), resourceStutter })
    .then(jointTurn(1))
    .then(all { assert(canSleep(1)), assert(q.dispatchPending == Set(3)), assert(resourceSafety), resourceStutter })
    .then(ackDispatch(1))
    .then(all { assert(not(hostWakeLevel(1))), assert(canSleep(1)), assert(resourceSafety), resourceStutter })
    .then(acquireCpu(1)).then(compute(1))
    .then(all { assert(hostWakeLevel(1)), assert(not(canSleep(1))), assert(resourceSafety), resourceStutter })
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3))
    .then(completionAck(1,1)).then(resourceClose).then(resourceJoin)
    .then(all { assert(resourceSafety), resourceStutter })
  action jointMutationProof = jointBulkReadyStart.then(unsafeJointHarvest(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action jointMutationStep = any { sharedStep, unsafeJointHarvest(1) }
  action runtimePayloadStart = pipelineHeldStart.then(project(1)).then(project(2)).then(completionAck(1,1))
    .then(acquireSubmitCapability(1)).then(prepareInput(3)).then(beginConstruction(3))
  action runtimePayloadEntryProof = runtimePayloadStart.then(gcStart(1)).then(osPause(1))
    .then(all { assert(q.constructing == Set(3)), assert(not(runtimeAvailable(1))), assert(resourceSafety), resourceStutter })
    .then(osResume(1)).then(gcFinish(1)).then(finishConstruction(3)).then(payloadEntry(3))
    .then(all { assert(q.payloads.size() == 0), assert(q.constructing.size() == 0), assert(q.host == List(3)), assert(resourceSafety), resourceStutter })
  action runtimePayloadCancelProof = runtimePayloadStart.then(finishConstruction(3)).then(resourceClose)
    .then(discardPrepared(3)).then(resourceJoin)
    .then(all { assert(q.payloads.size() == 0), assert(q.constructing.size() == 0), assert(resourceSafety), resourceStutter })
  action runtimePayloadRevokeProof = runtimePayloadStart.then(finishConstruction(3)).then(revokeSubmitCapability(1))
    .then(all { assert(q.payloads == Set(3)), assert(q.host.length() == 0), assert(resourceSafety), resourceStutter })
    .then(resourceClose).then(discardPrepared(3)).then(resourceJoin)
    .then(all { assert(resourceSafety), resourceStutter })
  action runtimeGcMutationProof = runtimePayloadStart.then(gcStart(1)).then(unsafeGcConstruction(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action nativeEntryCapabilityProof = pipelineHeldStart.then(project(1)).then(project(2)).then(completionAck(1,1))
    .then(acquireSubmitCapability(1)).then(prepareInput(3)).then(nativeEntry(3))
    .then(all { assert(q.submitCapabilities == Set(1)), assert(q.fillBudget.get(1) == 1), assert(resourceSafety), resourceStutter })
  action nativeEntryCancelCloseProof = pipelineCycleStart.then(acquireSubmitCapability(1)).then(prepareInput(3))
    .then(resourceClose).then(discardPrepared(3)).then(resourceJoin)
    .then(all { assert(q.submitCapabilities.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  action nativeEntryRevokeProof = pipelineCycleStart.then(acquireSubmitCapability(1)).then(prepareInput(3))
    .then(revokeSubmitCapability(1)).then(all { assert(q.prepared == Set(3)), assert(q.submitCapabilities.size() == 0), assert(resourceSafety), resourceStutter })
    .then(resourceClose).then(discardPrepared(3)).then(resourceJoin)
  action pipelineCancelCloseProof = pipelineHeldStart.then(prepareInput(3)).then(project(1)).then(resourceClose)
    .then(discardPrepared(3)).then(project(2)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(q.fillBudget.get(1) == 0), assert(resourceSafety), resourceStutter })
  action pipelineCreditMutationProof = pipelineCreditMutationStart.then(unsafeFillWithoutRetirement(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action preparedCycleHeldStart = nativeCycleTransferredStart.then(enablePreparedCycle).then(prepareInput(3))
  action preparedCycleOvertakeMutationStep = { nondet r = CASE_JOBS.oneOf(); any { sharedStep, unsafePreparedOvertake(r) } }
  action nativeCycleEarlyCreditMutationStep = { nondet r = CASE_JOBS.oneOf(); any { sharedStep, unsafeCreditBeforeSlotClear(r) } }
  run completionNewEpochSurvivesOldAckTest = completionLateReadyStart.then(abandon(4)).then(resourceClose)
    .then(completionAck(1, 1))
    .then(all { assert(q.completionDue.contains(1)), assert(rootedJobs == Set(4)), assert(resourceSafety), resourceStutter })
    .then(harvest(1, Set(4))).then(completionIgnoreOldAck(1, 1))
    .then(all { assert(q.tokenEpoch.get(1) == 2), assert(rootedJobs == Set(4)), assert(resourceSafety), resourceStutter })
    .then(project(4)).then(completionAck(1, 2)).then(resourceJoin)
    .then(all { assert(q.completionDue.size() == 0), assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  run completionCarryReadyRejectsDuplicateAckTest = completionCoalescedStart
    .then(offer(3)).then(feed(3)).then(assignWorker(1, 3)).then(acquireCpu(1)).then(compute(1))
    .then(offer(4)).then(boundary(1)).then(feed(4)).then(assignWorker(1, 4)).then(acquireCpu(1)).then(compute(1))
    .then(harvest(1, Set(3))).then(project(3)).then(completionAck(1, 1))
    .then(all { assert(q.completionEpoch.get(1) == 2), assert(resourceSafety), resourceStutter })
    .then(harvest(1, Set(4))).then(project(4)).then(completionIgnoreOldAck(1, 1))
    .then(all { assert(q.tokenEpoch.get(1) == 2), assert(q.completionDue.contains(1)), assert(resourceSafety), resourceStutter })
    .then(completionAck(1, 2)).then(resourceClose).then(resourceJoin)
    .then(all { assert(resourceSafety), assert(rootedJobs.size() == 0), resourceStutter })
  run completionStaleAckMutationTest = completionLateReadyStart.then(unsafeStaleCompletionAck(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  run completionBoundedHandoffTest = completionStart
    .then(offer(3)).then(feed(3)).then(assignWorker(1, 3)).then(acquireCpu(1)).then(compute(1))
    .then(offer(4)).then(boundary(1)).then(feed(4)).then(assignWorker(1, 4)).then(acquireCpu(1)).then(compute(1))
    .then(harvest(1, Set(3))).then(abandon(4)).then(resourceClose).then(project(3))
    .then(all { assert(rootedJobs == Set(4)), assert(readyJobs == Set(4)), assert(not(canSleep(1))), assert(resourceSafety), resourceStutter })
    .then(harvest(1, Set(4))).then(project(4)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  run completionUnboundedSnapshotMutationTest = completionStart
    .then(offer(3)).then(feed(3)).then(assignWorker(1, 3)).then(acquireCpu(1)).then(compute(1))
    .then(offer(4)).then(boundary(1)).then(feed(4)).then(assignWorker(1, 4)).then(acquireCpu(1)).then(compute(1))
    .then(unsafeCompletionSnapshot(Set(3, 4))).then(all { assert(not(resourceSafety)), resourceStutter })
  run splitQueueStillBlockedAtEntryTest = resourceInit.then(policy(true, false, false, false, false, 1))
    .then(offer(1)).then(feed(1)).then(assignWorker(1, 1)).then(acquireCpu(1)).then(compute(1)).then(offer(3))
    .then(delayTick).then(delayTick).then(delayTick).then(delayTick)
    .then(all { assert(inboxJobs.contains(3)), assert(not(canFeed(3))), assert(q.age.get(3) == 4), assert(resourceSafety), resourceStutter })
  run reservedWorkerStillSharesCpuTest = resourceInit.then(policy(true, true, true, false, true, 2))
    .then(offer(1)).then(feed(1)).then(assignWorker(1, 1)).then(acquireCpu(1)).then(compute(1)).then(yieldCpu(1))
    .then(offer(3)).then(feed(3)).then(assignWorker(2, 3)).then(acquireCpu(1))
    .then(all { assert(q.jobs.get(2) == 3), assert(q.cpuGap), assert(resourceSafety), resourceStutter })
  run gcBlocksSameVmNotOtherVmTest = resourceInit.then(policy(true, true, false, true, true, 2))
    .then(offer(3)).then(feed(3)).then(assignWorker(2, 3)).then(gcStart(1))
    .then(offer(2)).then(feed(2)).then(assignWorker(3, 2)).then(acquireCpu(3)).then(compute(3))
    .then(all { assert(not(ownerFree(1))), assert(readyJobs.contains(2)), assert(rootedJobs.contains(3)), assert(resourceSafety), resourceStutter })
  run osPauseHasNoQueueOnlyDeadlineTest = resourceInit.then(offer(3)).then(osPause(1))
    .then(delayTick).then(delayTick).then(delayTick).then(delayTick)
    .then(all { assert(q.age.get(3) == 4), assert(not(canFeed(3))), assert(resourceSafety), resourceStutter })
  run eagerFeedBeforeAdvanceTest = resourceInit.then(policy(true, true, false, false, false, 2))
    .then(offer(1)).then(offer(3)).then(feed(1))
    .then(all { assert(feedShortFirst(1)), assert(resourceSafety), resourceStutter })
    .then(feed(3)).then(boundary(1))
    .then(all { assert(rootedJobs == Set(1, 3)), assert(not(q.entryGap)), assert(resourceSafety), resourceStutter })
  run cooperativeCpuAndClassDebtTest = resourceInit.then(policy(true, true, true, true, true, 2))
    .then(offer(1)).then(feed(1)).then(assignWorker(1, 1)).then(acquireCpu(1)).then(compute(1))
    .then(all { assert(not(canCompute(1))), assert(resourceSafety), resourceStutter }).then(yieldCpu(1))
    .then(offer(3)).then(feed(3)).then(assignWorker(2, 3))
    .then(all { assert(not(allowedCpu(1))), resourceStutter }).then(acquireCpu(2)).then(compute(2)).then(project(3))
    .then(offer(4)).then(boundary(1)).then(feed(4)).then(assignWorker(2, 4))
    .then(all { assert(not(allowedCpu(2))), assert(allowedCpu(1)), assert(rootedJobs.contains(1)), assert(resourceSafety), resourceStutter })
  run closeAndAbandonKeepSharedRootsTest = resourceInit.then(policy(true, true, false, true, false, 2))
    .then(offer(1)).then(feed(1)).then(assignWorker(1, 1)).then(acquireCpu(1)).then(block(1))
    .then(offer(3)).then(feed(3)).then(assignWorker(2, 3)).then(acquireCpu(2)).then(compute(2))
    .then(abandon(1)).then(resourceClose).then(project(3)).then(wakeTask(1)).then(acquireCpu(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1))
    .then(compute(1)).then(compute(1)).then(compute(1)).then(compute(1)).then(project(1)).then(resourceJoin)
    .then(all { assert(rootedJobs.size() == 0), assert(resourceSafety), resourceStutter })
  run prematureAdvanceMutationTest = resourceInit.then(policy(true, true, false, false, false, 2))
    .then(offer(3)).then(unsafeBoundary(1)).then(all { assert(not(resourceSafety)), resourceStutter })
  run cpuClassBypassMutationTest = resourceInit.then(policy(true, true, false, true, true, 2))
    .then(offer(1)).then(feed(1)).then(assignWorker(1, 1)).then(offer(3)).then(feed(3)).then(assignWorker(2, 3))
    .then(unsafeCpu(1)).then(all { assert(not(resourceSafety)), resourceStutter })
  run feedBudgetMutationTest = resourceInit.then(policy(true, true, false, false, false, 1))
    .then(offer(1)).then(offer(3)).then(feed(1)).then(unsafeFeed(3)).then(all { assert(not(resourceSafety)), resourceStutter })
  run gcExecutionMutationTest = resourceInit.then(offer(3)).then(feed(3)).then(assignWorker(2, 3)).then(gcStart(1))
    .then(unsafeCpu(2)).then(all { assert(not(resourceSafety)), resourceStutter })
  action preparedCycleReadyPriorityProof = preparedCycleHeldStart
    .then(all { assert(q.prepared == Set(3)), assert(not(rootedJobs.contains(3))), assert(resourceSafety), resourceStutter })
    .then(project(1)).then(project(2)).then(completionAck(1,1)).then(enterPrepared(3))
    .then(boundary(1)).then(feed(3)).then(assignWorker(1,3)).then(acquireCpu(1)).then(compute(1))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(project(3)).then(completionAck(1,2)).then(resourceClose).then(resourceJoin)
    .then(all { assert(resourceSafety), assert(rootedJobs.size()==0), resourceStutter })
  action preparedCycleCancelCloseProof = preparedCycleHeldStart.then(resourceClose).then(discardPrepared(3))
    .then(project(1)).then(project(2)).then(completionAck(1,1)).then(resourceJoin)
    .then(all { assert(q.prepared.size()==0), assert(resourceSafety), resourceStutter })
  action preparedCycleOvertakeMutationProof = preparedCycleHeldStart.then(unsafePreparedOvertake(3))
    .then(all { assert(not(resourceSafety)), resourceStutter })
  action nativeCycleIndependentRetirementProof = nativeCycleTransferredStart.then(abandon(2)).then(project(1))
    .then(offer(3)).then(boundary(1)).then(feed(3))
    .then(all { assert(q.batchSlots == Set(2)), assert(rootedJobs == Set(2,3)), assert(resourceSafety), resourceStutter })
    .then(project(2)).then(completionAck(1,1)).then(assignWorker(1,3)).then(acquireCpu(1)).then(compute(1))
    .then(harvest(1,Set(3))).then(batchTransfer(1)).then(resourceClose).then(project(3)).then(completionAck(1,2)).then(resourceJoin)
    .then(all { assert(q.batchSlots.size()==0), assert(rootedJobs.size()==0), assert(resourceSafety), resourceStutter })
  action nativeCycleControlBarrierProof = nativeCycleTransferredStart
    .then(all { assert(not(canServiceControl(1))), resourceStutter })
    .then(project(1)).then(all { assert(not(canServiceControl(1))), assert(resourceSafety), resourceStutter })
    .then(project(2)).then(all { assert(not(canServiceControl(1))), resourceStutter })
    .then(completionAck(1,1)).then(all { assert(canServiceControl(1)), assert(resourceSafety), resourceStutter })
    .then(resourceClose).then(resourceJoin)
  action nativeCyclePrematureCreditMutationProof = nativeCycleTransferredStart.then(unsafeCreditBeforeSlotClear(1))
    .then(all { assert(not(resourceSafety)), resourceStutter })

}
module service_completion_small {
  import service_sharing_cases(CASE_JOBS = Set(1, 2, 3, 4), CASE_ASSIGNED = Set(1, 2, 3, 4).mapBy(_ => 1)).*
  run coalescedBulkSnapshotReturnsCreditPerResultTest = completionBulkSnapshotProof
  // These witnesses require two bulk jobs assigned to one owner.
  run activationTurnExceptionTest = turnExceptionProof
  run activationTurnSuspensionTest = turnSuspensionProof
  run activationTurnArbitrationTest = turnArbitrationProof
  run activationTurnShortTest = turnShortProof
  run activationTurnCloseTest = turnCloseProof
  run activationTurnRefundMutationTest = turnRefundMutationProof
  run activationTurnHeldYieldMutationTest = turnHeldYieldMutationProof
  run activationCycleAccountingTest = cycleAccountingProof
  run activationCycleReplayTest = cycleReplayProof
  run activationCycleCloseTest = cycleCloseProof
  run activationCycleDoubleSettleTest = cycleDoubleSettleProof
  run activationIsolationTest = activationIsolationProof
  run activationReplayTest = activationReplayProof
  run activationExceptionalReplayTest = activationExceptionalReplayProof
  run activationCloseTest = activationCloseProof
  run activationAliasMutationTest = activationAliasMutationProof
  run activationEpochMutationTest = activationEpochMutationProof
  run leasePlanGrantSnapshotLateArrivalTest = leasePlanGrantLateProof
  run leasePlanCacheLateArrivalTest = leasePlanLateProof
  run leasePlanCacheCancelCloseTest = leasePlanCancelProof
  run leasePlanCacheMutationTest = leasePlanMutationProof
  run workPlanUncontendedTest = planUncontendedProof
  run workPlanLateArrivalTest = planLateArrivalProof
  run workPlanCancelCloseTest = planCancelCloseProof
  run workPlanExtensionMutationTest = planMutationProof
  run crossTwoTokioWorkersTest = crossTwoTokioProof
  run crossCapacityAwaitRetirementTest = crossCapacityProof
  run crossReplyBeforePermitTest = crossReplyBeforePermitProof
  run crossDroppedDrainTest = crossDroppedDrainProof
  run crossNativeWaitActorResumeTest = crossNativeWaitProof
  run crossTokioHeldAwaitMutationTest = crossTokioMutationProof
  run crossActorEarlyReadyMutationTest = crossActorMutationProof
  run crossOsParkRunnableMutationTest = crossOsMutationProof
  run streamingJointClientBetweenWindowsTest = streamingJointClientProof
  run streamingJointClientAtSealTest = streamingJointLastProof
  run preparationBeforeNextTest = preparationBeforeNextProof
  run preparationFinalHandoffTest = preparationFinalProof
  run preparationCancelCloseTest = preparationCancelProof
  run preparationSkipMutationTest = preparationSkipProof
  run completionServiceReadyBeforeCoopTest = completionServiceReadyProof
  run completionServiceLegacyDeferralTest = completionServiceLegacyProof
  run completionServiceClaimNotResultTest = completionServiceClaimProof
  run completionServiceCancelCloseTest = completionServiceCancelProof
  run completionServiceFabricatedResultMutationTest = completionServiceReadyMutationProof
  run arrivalClockObservationNotAdmissionTest = arrivalClockAdmissionProof
  run arrivalClockOverdueStillArmsTest = arrivalClockOverdueProof
  run arrivalClockCancelAndReuseTest = arrivalClockCancelProof
  run arrivalClockCloseBeforeAdmissionTest = arrivalClockCloseProof
  run arrivalClockEarlyMutationTest = arrivalClockEarlyMutationProof
  run arrivalClockStaleMutationTest = arrivalClockStaleMutationProof
  run arrivalClockQuotaMutationTest = arrivalClockQuotaMutationProof
  run clientTurnExistingPollOwnershipTest = clientTurnBusyProof
  run clientTurnFinalBeforeBulkTest = clientTurnFinalProof
  run clientTurnEarlyClaimNotCompletionTest = clientTurnEarlyProof
  run clientTurnSkipMutationTest = clientTurnSkipProof
  run clientTurnEventDuringBulkTest = clientTurnLateEventProof
  run deliveryEarlyWakeTest = deliveryEarlyProof
  run deliveryOwnerRefillBeforeConsumptionTest = deliveryRefillProof
  run deliveryDropNativeDrainTest = deliveryDropProof
  run deliveryLostWakeMutationTest = deliveryWakeMutationProof
  run claimProgressEarlyWakeTest = claimProgressEarlyProof
  run claimProgressCoalescedTest = claimProgressCoalescedProof
  run claimProgressLateArmTest = claimProgressLateArmProof
  run claimProgressFinalDuringPollTest = claimProgressFinalDuringPollProof
  run claimProgressCancelTest = claimProgressCancelProof
  run claimProgressIndependentOwnerTest = claimProgressOwnerProof
  run claimProgressClaimedDropTest = claimProgressDropProof
  run claimProgressLostWakeMutationTest = claimProgressLostWakeProof
  run claimProgressEarlyValueMutationTest = claimProgressEarlyValueProof
  run completionQueueBoundTest = completionQueueBoundProof
  run completionQueueClaimAndReplyTest = completionQueueClaimProof
  run completionQueueLegacyPlacementTest = completionPlacementProof
  run completionQueueCancelTest = completionQueueCancelProof
  run completionQueueClaimedCancelTest = completionClaimedCancelProof
  run completionQueueCloseTest = completionQueueCloseProof
  run completionQueueEarlyCreditMutationTest = completionQueueMutationProof
  run publicationRetireBeforeSendTest = publicationOrderProof
  run publicationCapturedPolicyTest = publicationSnapshotProof
  run publicationDropTimeoutCloseTest = publicationDropCloseProof
  run publicationOwnerOpportunityTest = publicationOwnerOpportunityProof
  run publicationEarlySendMutationTest = publicationEarlySendMutationProof
  run clientTimerNoOwnerCreditTest = clientTimerProof
  run clientNativeReadyBeforeConsumptionTest = clientSerialProof
  run clientAlreadyDueNoTimerTest = clientAlreadyDueProof
  run clientLegacyAlreadyDueTimerTest = clientLegacyDueProof
  run clientCancelTimerCloseTest = clientCancelTimerProof
  run clientIndependentOwnerProgressTest = clientIndependentOwnerProof
  run clientUnissuedDirectEntryMutationTest = clientDirectEntryMutationProof
  run clientEarlyIssueMutationTest = clientEarlyIssueMutationProof
  run clientEarlyConsumeMutationTest = clientEarlyConsumeMutationProof
  run nativeRunnableProcessorTest = nativeRunnableProof
  run nativeExternalDrainTest = nativeExternalDrainProof
  run nativeEmptyPassTest = nativeEmptyPassProof
  run nativeActorSourceTest = nativeActorSourceProof
  run nativeSkippedPassMutationTest = nativeSkipPassMutationProof
  run nativeOsSleepMutationTest = nativeOsSleepMutationProof
  run runtimeFusedSuspensionTest = runtimeFusedSuspensionProof
  run runtimeFusedDispatchTest = runtimeFusedDispatchProof
  run runtimeFusedActorTest = runtimeFusedActorProof
  run runtimeFusedSuspensionMutationTest = runtimeFusedSuspensionMutationProof
  run runtimeIndexReplayTest = runtimeIndexReplayProof
  run runtimeIndexRetirementMutationTest = runtimeIndexMutationProof
  run runtimeBudgetReceiptTest = runtimeBudgetReceiptProof
  run runtimeBudgetActorTest = runtimeBudgetActorProof
  run runtimeExternalWaitTest = runtimeExternalProof
  run runtimeRunnableCompletionTest = runtimeRunnableProof
  run runtimeBlockedShortDrainTest = runtimeBlockedShortProof
  run runtimeExternalPassMutationTest = runtimeExternalMutationProof
  run runtimePrematureHarvestMutationTest = runtimeHarvestMutationProof
  run progressCycleStalledReceiptTest = progressCycleStalledProof
  run progressCycleTerminalDrainTest = progressCycleDrainProof
  run progressCycleSkippedPassMutationTest = progressCycleMutationProof
  run readyCycleCollectProgressTest = readyCycleCollectProof
  run readyCycleSinglePassTest = readyCycleSinglePassProof
  run readyCycleCloseDrainTest = readyCycleCloseProof
  run readyCyclePrematurePassMutationTest = readyCycleMutationProof
  run executionCheckpointReceiptTest = executionCheckpointReceiptProof
  run executionCheckpointCloseDrainTest = executionCheckpointCloseProof
  run executionCheckpointDispatchOnlyMutationTest = executionCheckpointMutationProof
  run directedGrantReceiptTest = directedReceiptProof
  run directedCloseDrainTest = directedCloseProof
  run directedForeignWakeMutationTest = directedMutationProof
  run registrationThreeStagesTest = registrationStagesProof
  run registrationWaiterFairnessTest = registrationFairProof
  run registrationUngrantCloseTest = registrationUngrantProof
  run registrationContinuationReentryTest = registrationContinuationProof
  run registrationUnregisteredGrantMutationTest = registrationGrantMutationProof
  run registrationDispatchAckMutationTest = registrationMutationProof
  run reclamationReceiverOpportunityTest = receiverOpportunityProof
  run reclamationReceiverCloseTest = receiverCloseProof
  run reclamationReceiverDepartureMutationTest = receiverMutationProof
  run reclamationHarvestBeforeGrantTest = reclamationHarvestProof
  run reclamationCloseRevokesFenceTest = reclamationCloseProof
  run reclamationYieldCannotAcknowledgeTest = reclamationMutationProof
  run workBudgetSpanAndCompletionTest = workSpanProof
  run workBudgetUncontendedTest = workUncontendedProof
  run workBudgetCancelCloseTest = workCancelCloseProof
  run workBudgetMutationTest = workMutationProof
  run jointDispatchObligationTest = jointObligationProof
  run jointTimeoutCloseDrainTest = jointCloseProof
  run jointDispatchWakeAndWaitTest = jointWaitProof
  run jointBypassMutationTest = jointMutationProof
  run runtimePayloadEntryTest = runtimePayloadEntryProof
  run runtimePayloadCancelTest = runtimePayloadCancelProof
  run runtimePayloadRevokeTest = runtimePayloadRevokeProof
  run runtimeGcMutationTest = runtimeGcMutationProof
  run nativeEntryCapabilityTest = nativeEntryCapabilityProof
  run nativeEntryCancelCloseTest = nativeEntryCancelCloseProof
  run nativeEntryRevokeTest = nativeEntryRevokeProof
  run pipelineRefillTest = pipelineRefillProof
  run pipelineWakeRefillTest = pipelineWakeRefillProof
  run pipelineWakeControlTest = pipelineWakeControlProof
  run pipelineCancelCloseTest = pipelineCancelCloseProof
  run pipelineCreditMutationTest = pipelineCreditMutationProof
  run preparedCycleReadyPriorityTest = preparedCycleReadyPriorityProof
  run preparedCycleCancelCloseTest = preparedCycleCancelCloseProof
  run preparedCycleOvertakeMutationTest = preparedCycleOvertakeMutationProof
  run nativeCycleIndependentRetirementTest = nativeCycleIndependentRetirementProof
  run nativeCycleControlBarrierTest = nativeCycleControlBarrierProof
  run nativeCyclePrematureCreditMutationTest = nativeCyclePrematureCreditMutationProof

}
module service_completion_wide {
  import service_sharing_cases(CASE_JOBS = Set(1, 2, 3, 4, 5), CASE_ASSIGNED = Set(1, 2, 3, 4, 5).mapBy(r => if (r == 5) 2 else 1)).*
  run coalescedBulkSnapshotReturnsCreditPerResultTest = completionBulkSnapshotProof
  // These witnesses require two bulk jobs assigned to one owner.
  run activationTurnExceptionTest = turnExceptionProof
  run activationTurnSuspensionTest = turnSuspensionProof
  run activationTurnArbitrationTest = turnArbitrationProof
  run activationTurnShortTest = turnShortProof
  run activationTurnCloseTest = turnCloseProof
  run activationTurnRefundMutationTest = turnRefundMutationProof
  run activationTurnHeldYieldMutationTest = turnHeldYieldMutationProof
  run activationCycleAccountingTest = cycleAccountingProof
  run activationCycleReplayTest = cycleReplayProof
  run activationCycleCloseTest = cycleCloseProof
  run activationCycleDoubleSettleTest = cycleDoubleSettleProof
  run activationIsolationTest = activationIsolationProof
  run activationReplayTest = activationReplayProof
  run activationExceptionalReplayTest = activationExceptionalReplayProof
  run activationCloseTest = activationCloseProof
  run activationAliasMutationTest = activationAliasMutationProof
  run activationEpochMutationTest = activationEpochMutationProof
  run leasePlanGrantSnapshotLateArrivalTest = leasePlanGrantLateProof
  run leasePlanCacheLateArrivalTest = leasePlanLateProof
  run leasePlanCacheCancelCloseTest = leasePlanCancelProof
  run leasePlanCacheMutationTest = leasePlanMutationProof
  run workPlanUncontendedTest = planUncontendedProof
  run workPlanLateArrivalTest = planLateArrivalProof
  run workPlanCancelCloseTest = planCancelCloseProof
  run workPlanExtensionMutationTest = planMutationProof
  run crossTwoTokioWorkersTest = crossTwoTokioProof
  run crossCapacityAwaitRetirementTest = crossCapacityProof
  run crossReplyBeforePermitTest = crossReplyBeforePermitProof
  run crossDroppedDrainTest = crossDroppedDrainProof
  run crossNativeWaitActorResumeTest = crossNativeWaitProof
  run crossTokioHeldAwaitMutationTest = crossTokioMutationProof
  run crossActorEarlyReadyMutationTest = crossActorMutationProof
  run crossOsParkRunnableMutationTest = crossOsMutationProof
  run deliveryRotationTest = deliveryRotationProof
  run deliveryPickMutationTest = deliveryPickMutationProof
  run deliveryHeadBypassTest = deliveryBypassProof
  run streamingJointClientBetweenWindowsTest = streamingJointClientProof
  run streamingJointClientAtSealTest = streamingJointLastProof
  run preparationBeforeNextTest = preparationBeforeNextProof
  run preparationFinalHandoffTest = preparationFinalProof
  run preparationCancelCloseTest = preparationCancelProof
  run preparationSkipMutationTest = preparationSkipProof
  run completionServiceReadyBeforeCoopTest = completionServiceReadyProof
  run completionServiceLegacyDeferralTest = completionServiceLegacyProof
  run completionServiceClaimNotResultTest = completionServiceClaimProof
  run completionServiceCancelCloseTest = completionServiceCancelProof
  run completionServiceFabricatedResultMutationTest = completionServiceReadyMutationProof
  run arrivalClockObservationNotAdmissionTest = arrivalClockAdmissionProof
  run arrivalClockOverdueStillArmsTest = arrivalClockOverdueProof
  run arrivalClockCancelAndReuseTest = arrivalClockCancelProof
  run arrivalClockCloseBeforeAdmissionTest = arrivalClockCloseProof
  run arrivalClockEarlyMutationTest = arrivalClockEarlyMutationProof
  run arrivalClockStaleMutationTest = arrivalClockStaleMutationProof
  run arrivalClockQuotaMutationTest = arrivalClockQuotaMutationProof
  run clientTurnExistingPollOwnershipTest = clientTurnBusyProof
  run clientTurnFinalBeforeBulkTest = clientTurnFinalProof
  run clientTurnEarlyClaimNotCompletionTest = clientTurnEarlyProof
  run clientTurnSkipMutationTest = clientTurnSkipProof
  run clientTurnEventDuringBulkTest = clientTurnLateEventProof
  run deliveryEarlyWakeTest = deliveryEarlyProof
  run deliveryOwnerRefillBeforeConsumptionTest = deliveryRefillProof
  run deliveryDropNativeDrainTest = deliveryDropProof
  run deliveryLostWakeMutationTest = deliveryWakeMutationProof
  run claimProgressEarlyWakeTest = claimProgressEarlyProof
  run claimProgressCoalescedTest = claimProgressCoalescedProof
  run claimProgressLateArmTest = claimProgressLateArmProof
  run claimProgressFinalDuringPollTest = claimProgressFinalDuringPollProof
  run claimProgressCancelTest = claimProgressCancelProof
  run claimProgressIndependentOwnerTest = claimProgressOwnerProof
  run claimProgressClaimedDropTest = claimProgressDropProof
  run claimProgressLostWakeMutationTest = claimProgressLostWakeProof
  run claimProgressEarlyValueMutationTest = claimProgressEarlyValueProof
  run completionQueueBoundTest = completionQueueBoundProof
  run completionQueueClaimAndReplyTest = completionQueueClaimProof
  run completionQueueLegacyPlacementTest = completionPlacementProof
  run completionQueueCancelTest = completionQueueCancelProof
  run completionQueueClaimedCancelTest = completionClaimedCancelProof
  run completionQueueCloseTest = completionQueueCloseProof
  run completionQueueEarlyCreditMutationTest = completionQueueMutationProof
  run publicationRetireBeforeSendTest = publicationOrderProof
  run publicationCapturedPolicyTest = publicationSnapshotProof
  run publicationDropTimeoutCloseTest = publicationDropCloseProof
  run publicationOwnerOpportunityTest = publicationOwnerOpportunityProof
  run publicationEarlySendMutationTest = publicationEarlySendMutationProof
  run clientTimerNoOwnerCreditTest = clientTimerProof
  run clientNativeReadyBeforeConsumptionTest = clientSerialProof
  run clientAlreadyDueNoTimerTest = clientAlreadyDueProof
  run clientLegacyAlreadyDueTimerTest = clientLegacyDueProof
  run clientCancelTimerCloseTest = clientCancelTimerProof
  run clientIndependentOwnerProgressTest = clientIndependentOwnerProof
  run clientUnissuedDirectEntryMutationTest = clientDirectEntryMutationProof
  run clientEarlyIssueMutationTest = clientEarlyIssueMutationProof
  run clientEarlyConsumeMutationTest = clientEarlyConsumeMutationProof
  run nativeRunnableProcessorTest = nativeRunnableProof
  run nativeExternalDrainTest = nativeExternalDrainProof
  run nativeEmptyPassTest = nativeEmptyPassProof
  run nativeActorSourceTest = nativeActorSourceProof
  run nativeSkippedPassMutationTest = nativeSkipPassMutationProof
  run nativeOsSleepMutationTest = nativeOsSleepMutationProof
  run runtimeFusedSuspensionTest = runtimeFusedSuspensionProof
  run runtimeFusedDispatchTest = runtimeFusedDispatchProof
  run runtimeFusedActorTest = runtimeFusedActorProof
  run runtimeFusedSuspensionMutationTest = runtimeFusedSuspensionMutationProof
  run runtimeIndexReplayTest = runtimeIndexReplayProof
  run runtimeIndexRetirementMutationTest = runtimeIndexMutationProof
  run runtimeBudgetReceiptTest = runtimeBudgetReceiptProof
  run runtimeBudgetActorTest = runtimeBudgetActorProof
  run runtimeExternalWaitTest = runtimeExternalProof
  run runtimeRunnableCompletionTest = runtimeRunnableProof
  run runtimeBlockedShortDrainTest = runtimeBlockedShortProof
  run runtimeExternalPassMutationTest = runtimeExternalMutationProof
  run runtimePrematureHarvestMutationTest = runtimeHarvestMutationProof
  run progressCycleStalledReceiptTest = progressCycleStalledProof
  run progressCycleTerminalDrainTest = progressCycleDrainProof
  run progressCycleSkippedPassMutationTest = progressCycleMutationProof
  run readyCycleCollectProgressTest = readyCycleCollectProof
  run readyCycleSinglePassTest = readyCycleSinglePassProof
  run readyCycleCloseDrainTest = readyCycleCloseProof
  run readyCyclePrematurePassMutationTest = readyCycleMutationProof
  run executionCheckpointReceiptTest = executionCheckpointReceiptProof
  run executionCheckpointCloseDrainTest = executionCheckpointCloseProof
  run executionCheckpointDispatchOnlyMutationTest = executionCheckpointMutationProof
  run directedGrantReceiptTest = directedReceiptProof
  run directedCloseDrainTest = directedCloseProof
  run directedForeignWakeMutationTest = directedMutationProof
  run registrationThreeStagesTest = registrationStagesProof
  run registrationWaiterFairnessTest = registrationFairProof
  run registrationUngrantCloseTest = registrationUngrantProof
  run registrationContinuationReentryTest = registrationContinuationProof
  run registrationUnregisteredGrantMutationTest = registrationGrantMutationProof
  run registrationDispatchAckMutationTest = registrationMutationProof
  run reclamationReceiverOpportunityTest = receiverOpportunityProof
  run reclamationReceiverCloseTest = receiverCloseProof
  run reclamationReceiverDepartureMutationTest = receiverMutationProof
  run reclamationHarvestBeforeGrantTest = reclamationHarvestProof
  run reclamationCloseRevokesFenceTest = reclamationCloseProof
  run reclamationYieldCannotAcknowledgeTest = reclamationMutationProof
  run workBudgetSpanAndCompletionTest = workSpanProof
  run workBudgetUncontendedTest = workUncontendedProof
  run workBudgetCancelCloseTest = workCancelCloseProof
  run workBudgetMutationTest = workMutationProof
  run jointDispatchObligationTest = jointObligationProof
  run jointTimeoutCloseDrainTest = jointCloseProof
  run jointDispatchWakeAndWaitTest = jointWaitProof
  run jointBypassMutationTest = jointMutationProof
  run runtimePayloadEntryTest = runtimePayloadEntryProof
  run runtimePayloadCancelTest = runtimePayloadCancelProof
  run runtimePayloadRevokeTest = runtimePayloadRevokeProof
  run runtimeGcMutationTest = runtimeGcMutationProof
  run nativeEntryCapabilityTest = nativeEntryCapabilityProof
  run nativeEntryCancelCloseTest = nativeEntryCancelCloseProof
  run nativeEntryRevokeTest = nativeEntryRevokeProof
  run pipelineRefillTest = pipelineRefillProof
  run pipelineWakeRefillTest = pipelineWakeRefillProof
  run pipelineWakeControlTest = pipelineWakeControlProof
  run pipelineCancelCloseTest = pipelineCancelCloseProof
  run pipelineCreditMutationTest = pipelineCreditMutationProof
  run preparedCycleReadyPriorityTest = preparedCycleReadyPriorityProof
  run preparedCycleCancelCloseTest = preparedCycleCancelCloseProof
  run preparedCycleOvertakeMutationTest = preparedCycleOvertakeMutationProof
  run nativeCycleIndependentRetirementTest = nativeCycleIndependentRetirementProof
  run nativeCycleControlBarrierTest = nativeCycleControlBarrierProof
  run nativeCyclePrematureCreditMutationTest = nativeCyclePrematureCreditMutationProof

}
module service_sharing_small {
  import service_sharing_cases(CASE_JOBS = Set(1, 2, 3, 4), CASE_ASSIGNED = Set(1, 2, 3, 4).mapBy(r => if (r == 2) 2 else 1)).*
}
module service_sharing_wide {
  import service_sharing_cases(CASE_JOBS = Set(1, 2, 3, 4, 5), CASE_ASSIGNED = Set(1, 2, 3, 4, 5).mapBy(r => if (Set(2, 5).contains(r)) 2 else 1)).*
}

// Fixed stateless routing alternative. Stateful Actor ownership never migrates.
module service_routing_isolated {
  import service_sharing(JOBS = Set(1, 2, 3, 4), ASSIGNED = Set(1, 2, 3, 4).mapBy(r => if (Set(3, 4).contains(r)) 2 else 1)) as shared
  run statelessOtherVmSurvivesGcTest = shared::candidateInit.then(shared::gcStart(1))
    .then(shared::offer(3)).then(shared::feed(3)).then(shared::assignWorker(3, 3))
    .then(shared::acquireCpu(3)).then(shared::compute(3)).then(shared::project(3))
    .then(all { assert(shared::completedJobs.contains(3)), assert(shared::q.gc.contains(1)), assert(shared::resourceSafety), shared::resourceStutter })
}

// Architecture refinement of the existing Gerbil Actor / owner / Tokio model.
// CPU occupancy is a host scheduling abstraction, NOT a proposed runtime lease.
// Wait edges mean "cannot progress until target progresses"; they are not wakes.
module gerbil_service_architecture {
  const ARCH_JOBS: Set[int]
  const ARCH_ASSIGNED: int -> int
  import service_sharing_cases(CASE_JOBS = ARCH_JOBS, CASE_ASSIGNED = ARCH_ASSIGNED) as base
  // 1=fixed owner (including its Gerbil green threads), 2=Tokio, 3=Rayon.
  // Actor CPU is deliberately NOT counted again as a fourth native executor.
  val executors = Set(1,2,3)
  val nodes = Set(1,2,3,4,5,6,7)
  // Wait graph nodes: owner, Gerbil Actor, Tokio client, Rayon preparer,
  // native capacity, Rust result transport. A cycle is a structural warning,
  // not a proof that every concrete execution deadlocks.
  type Topology = {
    synchronous: bool, rayon: bool, cores: int, running: Set[int],
    waits: Set[(int,int)], nativeEntrants: Set[int],
    reserved: bool, prepared: bool, published: int, observed: int, armed: bool,
    wake: bool, wakes: int, ready: bool, retired: bool,
    producerClaimed: bool, producerByte: bool, producerDuty: bool,
    producerRetry: bool, producerParked: bool,
    supplyLocked: bool, supplyPhase: str, supplyRegistered: bool, supplyNotice: bool, supplyCredits: bool
  }
  var topology: Topology
  pure def reaches(edges: Set[(int,int)], start: int): Set[int] =
    nodes.fold(Set(start), (seen, _) => seen.union(nodes.filter(target =>
      seen.exists(previous => edges.contains((previous,target))))))
  pure def cyclic(edges: Set[(int,int)]): bool = nodes.exists(n =>
    edges.exists(edge => edge._1 == n and reaches(edges,edge._2).contains(n)))
  val architectureSafety = and {
    base::architectureBaseSafe,
    topology.running.subseteq(executors), topology.running.size() <= topology.cores,
    topology.nativeEntrants.subseteq(Set(1)),
    not(topology.running.contains(3)) or topology.rayon,
    not(topology.prepared) or topology.reserved,
    topology.observed <= topology.published,
    not(topology.armed and topology.published > topology.observed) or topology.wake,
    not(topology.retired) or topology.ready,
    // Native wait cannot bury a durable producer publication without transport.
    not(topology.producerParked and topology.producerDuty) or topology.producerByte,
    topology.supplyPhase != "waiting" or not(topology.supplyCredits),
    topology.supplyPhase != "waiting" or topology.supplyRegistered,
    not(topology.supplyPhase == "waiting" and not(topology.supplyLocked)) or topology.supplyNotice,
    topology.supplyPhase != "waiting" or not(topology.waits.contains((1,7))),
    not(cyclic(topology.waits))
  }
  pure def initialTopology(sync: bool, rayon: bool, cores: int): Topology = {
    synchronous: sync, rayon: rayon, cores: cores, running: Set(),
    waits: Set(), nativeEntrants: Set(), reserved: false, prepared: false,
    published: 0, observed: 0, armed: false, wake: false, wakes: 0,
    ready: false, retired: false, producerClaimed: false, producerByte: false,
    producerDuty: false, producerRetry: false, producerParked: false,
    supplyLocked: false, supplyPhase: "idle", supplyRegistered: false, supplyNotice: false, supplyCredits: false}
  action architectureInit(sync: bool, rayon: bool, cores: int): bool = all {
    base::architectureBaseInit, topology' = initialTopology(sync,rayon,cores)
  }
  // Node 7 is shared Rust supply data, not a Scheme execution resource.
  action architectureSupplyLock = all {
    not(topology.supplyLocked), base::architectureBaseIdle,
    topology' = {...topology, supplyLocked: true}
  }
  action architectureSupplyCredits = all {
    not(topology.supplyCredits), base::architectureBaseIdle,
    topology' = {...topology, supplyCredits: true}
  }
  action architectureSupplyBusy = all {
    topology.supplyLocked, base::architectureBaseIdle,
    topology' = {...topology, supplyPhase: "registering", supplyRegistered: false, supplyNotice: false}
  }
  action architectureSupplyRegister = all {
    topology.supplyPhase == "registering", base::architectureBaseIdle,
    topology' = {...topology, supplyRegistered: true}
  }
  action architectureSupplyRecheck = all {
    topology.supplyPhase == "registering", topology.supplyRegistered, base::architectureBaseIdle,
    topology' = if (topology.supplyLocked)
      {...topology, supplyPhase: "waiting", supplyCredits: false}
      else {...topology, supplyPhase: "ready", supplyRegistered: false, supplyNotice: false}
  }
  action architectureSupplyUnlock = all {
    topology.supplyLocked, base::architectureBaseIdle,
    topology' = {...topology, supplyLocked: false, supplyNotice: topology.supplyNotice or topology.supplyRegistered}
  }
  action architectureSupplyResume = all {
    topology.supplyPhase == "waiting", topology.supplyNotice, base::architectureBaseIdle,
    topology' = {...topology, supplyPhase: "retry", supplyRegistered: false, supplyNotice: false}
  }
  action architectureSupplyCancel = all {
    base::architectureBaseIdle,
    topology' = {...topology, supplyPhase: "idle", supplyRegistered: false, supplyNotice: false, supplyCredits: false}
  }
  action architectureSupplyUnsafeUnlock = all {
    topology.supplyLocked, base::architectureBaseIdle,
    topology' = {...topology, supplyLocked: false, supplyNotice: false}
  }
  action architectureSupplyUnsafeCredits = all {
    topology.supplyPhase == "waiting", base::architectureBaseIdle,
    topology' = {...topology, supplyCredits: true}
  }
  run architectureSupplyOwnerCanExecuteTest = architectureInit(false,false,2)
    .then(architectureSupplyLock).then(architectureSupplyCredits).then(architectureSupplyBusy)
    .then(architectureSupplyRegister).then(architectureSupplyRecheck).then(architectureStart(1))
    .then(all { assert(topology.running.contains(1)), assert(not(topology.supplyCredits)), assert(architectureSafety), architectureIdle })
    .then(architectureSupplyUnlock).then(architectureSupplyResume)
    .then(all { assert(architectureSafety), architectureIdle })
  run architectureSupplyUnlockBeforeRegistrationTest = architectureInit(false,false,2)
    .then(architectureSupplyLock).then(architectureSupplyBusy).then(architectureSupplyUnlock)
    .then(architectureSupplyRegister).then(architectureSupplyRecheck)
    .then(all { assert(topology.supplyPhase == "ready"), assert(architectureSafety), architectureIdle })
  run architectureSupplyUnlockBeforeParkTest = architectureInit(false,false,2)
    .then(architectureSupplyLock).then(architectureSupplyBusy).then(architectureSupplyRegister)
    .then(architectureSupplyUnlock).then(architectureSupplyRecheck)
    .then(all { assert(topology.supplyPhase == "ready"), assert(architectureSafety), architectureIdle })
  run architectureSupplyCancelTest = architectureInit(false,false,2)
    .then(architectureSupplyLock).then(architectureSupplyCredits).then(architectureSupplyBusy)
    .then(architectureSupplyRegister).then(architectureSupplyRecheck).then(architectureSupplyCancel)
    .then(architectureSupplyUnlock)
    .then(all { assert(not(topology.supplyRegistered)), assert(not(topology.supplyNotice)), assert(not(topology.supplyCredits)), assert(architectureSafety), architectureIdle })
  run architectureSupplyMissingUnlockMutationTest = architectureInit(false,false,2)
    .then(architectureSupplyLock).then(architectureSupplyBusy).then(architectureSupplyRegister)
    .then(architectureSupplyRecheck).then(architectureSupplyUnsafeUnlock)
    .then(all { assert(not(architectureSafety)), architectureIdle })
  run architectureSupplyHeldCreditMutationTest = architectureInit(false,false,2)
    .then(architectureSupplyLock).then(architectureSupplyBusy).then(architectureSupplyRegister)
    .then(architectureSupplyRecheck).then(architectureSupplyUnsafeCredits)
    .then(all { assert(not(architectureSafety)), architectureIdle })
  run architectureSupplyNativeBlockMutationTest = architectureInit(false,false,2)
    .then(architectureSupplyLock).then(architectureSupplyBusy).then(architectureSupplyRegister)
    .then(architectureSupplyRecheck).then(architectureWait(Set((1,7))))
    .then(all { assert(not(architectureSafety)), architectureIdle })
  action architectureIdle = all { base::architectureBaseIdle, topology' = topology }
  action architectureWait(edges: Set[(int,int)]): bool = all {
    base::architectureBaseIdle, topology' = {...topology, waits: edges}
  }
  action architectureStart(executor: int): bool = all {
    executors.contains(executor), not(topology.running.contains(executor)),
    topology.running.size() < topology.cores,
    executor != 3 or (topology.rayon and topology.reserved),
    base::architectureBaseIdle, topology' = {...topology, running: topology.running.union(Set(executor))}
  }
  action architectureStop(executor: int): bool = all {
    topology.running.contains(executor), base::architectureBaseIdle,
    topology' = {...topology, running: topology.running.exclude(Set(executor))}
  }
  action architectureReserve = all {
    not(topology.reserved), base::architectureBaseIdle,
    topology' = {...topology, reserved: true}
  }
  action architecturePrepare = all {
    topology.reserved, not(topology.prepared),
    not(topology.rayon) or topology.running.contains(3),
    base::architectureBaseIdle, topology' = {...topology, prepared: true}
  }
  action architectureNative(executor: int): bool = all {
    executor == 1, topology.prepared, topology.running.contains(1),
    base::architectureBaseIdle,
    topology' = {...topology, nativeEntrants: topology.nativeEntrants.union(Set(executor))}
  }
  // Publication increments a generation even when a notification is coalesced.
  // Registration plus recheck is atomic in this abstraction.
  action architecturePublish = all {
    base::architectureBaseIdle,
    topology' = {...topology, published: topology.published+1,
      wake: topology.wake or topology.armed,
      wakes: topology.wakes + (if (topology.armed and not(topology.wake)) 1 else 0)}
  }
  action architectureArm = all {
    base::architectureBaseIdle,
    topology' = {...topology, armed: true,
      wake: topology.wake or topology.published > topology.observed}
  }
  action architectureObserve = all {
    base::architectureBaseIdle,
    topology' = {...topology, observed: topology.published, armed: false, wake: false}
  }
  // Reuses actual rooted execution / projection / capacity retirement actions;
  // does not invent another native queue, root table, or execution budget.
  action architectureCompletedInit = all {
    base::architectureBaseComplete,
    topology' = {...initialTopology(false,false,2), ready: true, retired: true}
  }
  action architectureCancelPrepared = all {
    not(topology.ready), base::architectureBaseIdle,
    topology' = {...topology, reserved: false, prepared: false, running: Set()}
  }
  action architectureCpuMutation = all {
    base::architectureBaseIdle, topology' = {...topology, running: executors}
  }
  action architectureForeignMutation = all {
    base::architectureBaseIdle, topology' = {...topology, nativeEntrants: Set(3)}
  }
  action architectureLostWakeMutation = all {
    base::architectureBaseIdle, topology' = {...topology, wake: false}
  }
  // Resident producer notification refines Wake::notify_producer / arm. The
  // previous mailbox token was durable too; removing it without this duty is
  // the mutation, NOT a claimed lost-wake bug in the previous implementation.
  action residentProducerPublish = all {
    base::architectureBaseIdle,
    topology' = {...topology, producerDuty: true, producerClaimed: true,
      producerByte: topology.producerByte or not(topology.producerClaimed)}
  }
  action residentProducerRead = all {
    topology.producerByte, base::architectureBaseIdle,
    topology' = {...topology, producerByte: false, producerParked: false}
  }
  action residentProducerReopen = all {
    base::architectureBaseIdle,
    topology' = {...topology, producerClaimed: false}
  }
  action residentProducerCheck = all {
    base::architectureBaseIdle,
    topology' = {...topology, producerDuty: false,
      producerRetry: topology.producerDuty, producerParked: false}
  }
  action residentProducerArm = residentProducerReopen.then(residentProducerCheck)
  action residentProducerPark = all {
    not(topology.producerRetry), base::architectureBaseIdle,
    topology' = {...topology, producerParked: true}
  }
  action unsafeResidentProducerPark = all {
    base::architectureBaseIdle,
    topology' = {...topology, producerClaimed: false, producerParked: true}
  }
  run architectureResidentCoalescedDutyTest = architectureInit(false,false,2)
    .then(residentProducerPublish).then(residentProducerRead).then(residentProducerPublish)
    .then(all { assert(not(topology.producerByte)), assert(topology.producerDuty), architectureIdle })
    .then(residentProducerArm)
    .then(all { assert(topology.producerRetry), assert(architectureSafety), architectureIdle })
  run architectureResidentPublishAfterArmTest = architectureInit(false,false,2)
    .then(residentProducerArm).then(residentProducerPublish).then(residentProducerPark)
    .then(all { assert(topology.producerByte), assert(architectureSafety), architectureIdle })
    .then(residentProducerRead).then(residentProducerArm)
    .then(all { assert(topology.producerRetry), assert(architectureSafety), architectureIdle })
  run architectureResidentPublishDuringArmTest = architectureInit(false,false,2)
    .then(residentProducerReopen).then(residentProducerPublish).then(residentProducerCheck)
    .then(all { assert(topology.producerRetry), assert(topology.producerByte),
      assert(architectureSafety), architectureIdle })
  run architectureResidentEmptyParkTest = architectureInit(false,false,2)
    .then(residentProducerArm).then(residentProducerPark)
    .then(all { assert(architectureSafety), architectureIdle })
  run architectureResidentRemovedTokenMutationTest = architectureInit(false,false,2)
    .then(residentProducerPublish).then(residentProducerRead).then(residentProducerPublish)
    .then(unsafeResidentProducerPark)
    .then(all { assert(not(architectureSafety)), architectureIdle })

  // Counts only ADDITIONAL scheduler round trips at the Rust/Gerbil boundary.
  // Common construction, handler, GC and projection costs are not removed.
  pure def extraHandoffs(items: int, windows: int, perItem: bool, rayon: bool): int =
    (if (perItem) items else windows) * (if (rayon) 4 else 2)

  run architectureSyncDependencyTest = architectureInit(true,false,2)
    .then(architectureWait(Set((3,1),(1,2))))
    .then(all { assert(architectureSafety), architectureIdle })
  run architectureSyncCallbackCycleTest = architectureInit(true,false,2)
    .then(architectureWait(Set((3,1),(1,2),(2,3))))
    .then(all { assert(cyclic(topology.waits)), assert(not(architectureSafety)), architectureIdle })
  run architectureResidentDependencyTest = architectureInit(false,false,2)
    .then(architectureWait(Set((3,6),(6,1),(2,1))))
    .then(all { assert(architectureSafety), architectureIdle })
  run architectureCompletionCapacityCycleTest = architectureInit(false,false,2)
    .then(architectureWait(Set((1,6),(6,3),(3,5),(5,1))))
    .then(all { assert(not(architectureSafety)), architectureIdle })
  run architectureRayonCapacityCycleTest = architectureInit(false,true,2)
    .then(architectureWait(Set((1,4),(4,5),(5,1))))
    .then(all { assert(not(architectureSafety)), architectureIdle })
  run architectureBoundedRayonTest = architectureInit(false,true,2)
    .then(architectureReserve).then(architectureStart(3)).then(architecturePrepare)
    .then(architectureStop(3)).then(architectureStart(1)).then(architectureNative(1))
    .then(all { assert(architectureSafety), assert(base::architectureBaseRoots.size()==0), architectureIdle })
  run architectureSharedCpuTest = architectureInit(false,true,2)
    .then(architectureReserve).then(architectureStart(1)).then(architectureStart(3))
    .then(all { assert(not(topology.running.size() < topology.cores)), assert(architectureSafety), architectureIdle })
    .then(architectureStop(3)).then(architectureStart(2))
    .then(all { assert(architectureSafety), architectureIdle })
  run architectureOversubscriptionMutationTest = architectureInit(false,true,2)
    .then(architectureCpuMutation).then(all { assert(not(architectureSafety)), architectureIdle })
  run architectureForeignEntryMutationTest = architectureInit(false,true,2)
    .then(architectureForeignMutation).then(all { assert(not(architectureSafety)), architectureIdle })
  run architecturePublishBeforeArmTest = architectureInit(false,false,2)
    .then(architecturePublish).then(architectureArm)
    .then(all { assert(topology.wake), assert(architectureSafety), architectureIdle })
  run architectureCoalescedWakeTest = architectureInit(false,false,2)
    .then(architectureArm).then(architecturePublish).then(architecturePublish)
    .then(all { assert(topology.wakes==1), assert(topology.published==2), assert(architectureSafety), architectureIdle })
    .then(architectureObserve).then(architectureArm)
    .then(all { assert(not(topology.wake)), assert(architectureSafety), architectureIdle })
  run architectureLostWakeMutationTest = architectureInit(false,false,2)
    .then(architectureArm).then(architecturePublish).then(architectureLostWakeMutation)
    .then(all { assert(not(architectureSafety)), architectureIdle })
  run architectureRetireBeforeClientTest = architectureCompletedInit
    .then(all { assert(base::architectureBaseConsumed.size()==0), assert(base::architectureBaseCapacity.size()==0),
      assert(base::architectureBaseRoots.size()==0), assert(architectureSafety), architectureIdle })
    .then(all { base::architectureBaseConsume, topology'=topology })
    .then(all { assert(base::architectureBaseConsumed==Set(1)), assert(architectureSafety), architectureIdle })
  run architecturePreparedCancelTest = architectureInit(false,true,2)
    .then(architectureReserve).then(architectureStart(3)).then(architecturePrepare)
    .then(architectureCancelPrepared)
    .then(all { assert(not(topology.reserved)), assert(not(topology.prepared)),
      assert(base::architectureBaseRoots.size()==0), assert(architectureSafety), architectureIdle })
  run architectureBoundaryCostTest = architectureInit(false,false,2)
    .then(all { assert(extraHandoffs(48,6,true,false)==96),
      assert(extraHandoffs(48,6,false,false)==12), assert(extraHandoffs(48,6,false,true)==24),
      assert(architectureSafety), architectureIdle })
}
module gerbil_architecture_small {
  import gerbil_service_architecture(ARCH_JOBS=Set(1,2,3,4), ARCH_ASSIGNED=Set(1,2,3,4).mapBy(_ => 1)).*
}
module gerbil_architecture_wide {
  import gerbil_service_architecture(ARCH_JOBS=Set(1,2,3,4,5), ARCH_ASSIGNED=Set(1,2,3,4,5).mapBy(r => if (r==5) 2 else 1)).*
}
