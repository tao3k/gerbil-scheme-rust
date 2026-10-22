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
    producerLimit: int, producers: Set[int], staged: Set[int], producerJobs: Set[int],
    servicePending: Set[int], serviceReady: Set[int], serviceRoots: Set[int], serviceFailed: Set[int],
    inbox: Set[int], wakeOwed: Set[int], notified: Set[int], parked: Set[int],
    latePublication: bool, lateExecutor: bool, foreignUse: bool
  }
  var s: State

  val initial = {
    selected: ASSIGNED, committed: JOBS.mapBy(_ => 0),
    phase: "open", accepted: Set(), completed: Set(), ready: Set(),
    roots: Set(), registry: Set(), timedOut: Set(), executor: "absent",
    filesPresent: true, imagesRetained: true,
    codeLeases: Set(), unloadAuthorized: false, codeQuarantined: false,
    unsafeCodeRelease: false,
    capacityLimit: JOBS.size(), capacityHeld: Set(), reserved: Set(),
    workLimit: JOBS.size() + 7, workHeld: Set(),
    producerLimit: JOBS.size(), producers: Set(), staged: Set(), producerJobs: Set(),
    servicePending: Set(), serviceReady: Set(), serviceRoots: Set(), serviceFailed: Set(),
    inbox: Set(), wakeOwed: Set(), notified: Set(), parked: Set(),
    latePublication: false, lateExecutor: false, foreignUse: false
  }
  action init = s' = initial
  def ownerOf(r: int): int = s.selected.get(r)
  // Only unreserved, unpublished stateless requests can select another owner.
  // Affine requests use the existing fixed-owner path and never this action.
  action selectOwner(r: int, owner: int): bool = all {
    s.phase == "open", JOBS.contains(r), Set(1, 2).contains(owner),
    not(s.workHeld.contains(r)), not(s.accepted.contains(r)), not(s.producerJobs.contains(r)),
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
    s.phase == "open", JOBS.contains(r), not(s.workHeld.contains(r)), not(s.accepted.contains(r)), hasWork(r),
    s' = { ...s, workHeld: s.workHeld.union(Set(r)) }
  }
  action reserveSlot(r: int): bool = all {
    s.phase == "open", s.workHeld.contains(r), not(s.capacityHeld.contains(r)), not(s.accepted.contains(r)), hasSlot(r),
    s' = { ...s, reserved: s.reserved.union(Set(r)), capacityHeld: s.capacityHeld.union(Set(r)) }
  }
  action cancelWork(r: int): bool = all {
    s.workHeld.contains(r), not(s.capacityHeld.contains(r)), not(s.accepted.contains(r)),
    s' = { ...s, workHeld: s.workHeld.exclude(Set(r)) }
  }
  action configureWork(limit: int): bool = all {
    s.phase == "open", s.capacityHeld.size() == 0, s.workHeld.size() == 0, s.producers.size() == 0, limit > 0,
    s' = { ...s, workLimit: limit }
  }
  action reserve(r: int): bool = all {
    s.phase == "open", JOBS.contains(r), not(s.accepted.contains(r)),
    not(s.reserved.contains(r)), not(s.workHeld.contains(r)), hasCapacity(r),
    s' = { ...s, reserved: s.reserved.union(Set(r)), capacityHeld: s.capacityHeld.union(Set(r)), workHeld: s.workHeld.union(Set(r)) }
  }
  action admitReserved(r: int): bool = all {
    s.phase == "open", s.reserved.contains(r), canProducerEnter(r),
    s' = { ...s, accepted: s.accepted.union(Set(r)), committed: s.committed.set(r, ownerOf(r)), inbox: s.inbox.union(Set(r)), wakeOwed: s.wakeOwed.union(Set(ownerOf(r))), staged: s.staged.exclude(Set(r)), reserved: s.reserved.exclude(Set(r)) }
  }
  action releaseReservation(r: int): bool = all {
    s.reserved.contains(r),
    s' = { ...s, reserved: s.reserved.exclude(Set(r)), capacityHeld: s.capacityHeld.exclude(Set(r)), workHeld: s.workHeld.exclude(Set(r)) }
  }
  action configureCapacity(limit: int): bool = all {
    s.phase == "open", s.capacityHeld.size() == 0, s.workHeld.size() == 0, s.producers.size() == 0, limit > 0,
    s' = { ...s, capacityLimit: limit }
  }
  action admitUnchecked(r: int): bool = all {
    s.phase == "open", JOBS.contains(r), not(s.accepted.contains(r)),
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
    s.phase == "open", JOBS.contains(r), s.producers.contains(ownerOf(r)),
    not(s.accepted.contains(r)), not(s.staged.contains(r)), not(s.workHeld.contains(r)),
    s' = { ...s, staged: s.staged.union(Set(r)), producerJobs: s.producerJobs.union(Set(r)) }
  }
  action cancelProducer(owner: int): bool = all {
    Set("open", "closing").contains(s.phase), s.producers.contains(owner),
    all {
      val pending = s.staged.filter(job => ownerOf(job) == owner)
      s' = { ...s, staged: s.staged.exclude(pending), producerJobs: s.producerJobs.exclude(pending),
        reserved: s.reserved.exclude(pending), capacityHeld: s.capacityHeld.exclude(pending), workHeld: s.workHeld.exclude(pending) }
    }
  }
  action finishProducer(owner: int): bool = all {
    Set("open", "closing").contains(s.phase), s.producers.contains(owner),
    s.producerJobs.filter(job => ownerOf(job) == owner).subseteq(s.completed),
    s' = { ...s, producers: s.producers.exclude(Set(owner)),
      producerJobs: s.producerJobs.filter(job => ownerOf(job) != owner) }
  }
  // Completion is an externally validated owner action, not Rust computation.
  action complete(r: int): bool = all {
    Set("open", "closing").contains(s.phase),
    s.accepted.contains(r), not(s.completed.contains(r)),
    not(s.servicePending.contains(r)) or s.serviceReady.contains(r),
    s' = { ...s, servicePending: s.servicePending.exclude(Set(r)), serviceReady: s.serviceReady.exclude(Set(r)), serviceRoots: s.serviceRoots.exclude(Set(r)), inbox: s.inbox.exclude(Set(r)), completed: s.completed.union(Set(r)), capacityHeld: s.capacityHeld.exclude(Set(r)), workHeld: s.workHeld.exclude(Set(r)),
      roots: s.roots.union(Set(r)), ready: s.ready.union(Set(r)) }
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
  action close = all { s.phase == "open", s' = { ...s, phase: "closing", notified: Set(1, 2) } }
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
    not(s.serviceReady.exists(r => ownerOf(r) == owner))
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
    JOBS.forall(r => Set(1, 2).contains(ownerOf(r))),
    s.accepted.forall(r => ownerOf(r) == s.committed.get(r)),
    s.servicePending.subseteq(s.accepted.exclude(s.completed)), s.serviceRoots == s.servicePending,
    s.serviceReady.subseteq(s.servicePending), s.serviceFailed.subseteq(s.accepted),
    s.inbox.subseteq(s.accepted.exclude(s.completed)), s.inbox.intersect(s.servicePending).size() == 0,
    s.parked.subseteq(Set(1, 2)), s.notified.subseteq(Set(1, 2)), s.wakeOwed.subseteq(Set(1, 2)),
    s.parked.forall(owner => implies(
      s.inbox.exists(r => ownerOf(r) == owner) or s.serviceReady.exists(r => ownerOf(r) == owner),
      s.notified.contains(owner) or s.wakeOwed.contains(owner))),
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
  type Shared = {
    host: List[int], queued: List[int], jobs: int -> int, cpu: int -> int,
    left: int -> int, spent: int -> int, lastShort: int -> bool, debt: int -> bool,
    blocked: Set[int], gc: Set[int], paused: Set[int], age: int -> int, overtakes: int -> int,
    feedUsed: int -> int, feedMax: int,
    split: bool, eager: bool, reserved: bool, fairCpu: bool, bounded: bool,
    entryGap: bool, cpuGap: bool, resourceViolation: bool
  }
  def entries(values: List[int]): Set[int] = values.foldl(Set(), (seen, r) => seen.union(Set(r)))
  def keep(values: List[int], predicate: (int) => bool): List[int] = values.foldl(List(), (kept, r) => if (predicate(r)) kept.append(r) else kept)
  val inboxJobs = life::s.inbox
  val rootedJobs = life::s.serviceRoots
  val readyJobs = life::s.serviceReady
  val completedJobs = life::s.completed
  var q: Shared
  action resourceInitWith(split0: bool, eager0: bool, reserved0: bool, cpu0: bool, bounded0: bool, feed0: int): bool = all {
    life::init,
    q' = { host: List(), queued: List(), jobs: workers.mapBy(_ => 0), cpu: owners.mapBy(_ => 0),
      left: JOBS.mapBy(r => life::units(r)), spent: owners.mapBy(_ => 0),
      lastShort: workers.mapBy(_ => false), debt: owners.mapBy(_ => false),
      blocked: Set(), gc: Set(), paused: Set(), age: JOBS.mapBy(_ => 0), overtakes: JOBS.mapBy(_ => 0),
      feedUsed: owners.mapBy(_ => 0), feedMax: feed0,
      split: split0, eager: eager0, reserved: reserved0, fairCpu: cpu0, bounded: bounded0,
      entryGap: false, cpuGap: false, resourceViolation: false }
  }
  action resourceInit = resourceInitWith(false, false, false, false, false, 1)
  action implementationInit = resourceInitWith(true, true, true, false, false, 2)
  action candidateInit = resourceInitWith(true, true, true, true, true, 2)
  action mutationInit = resourceInitWith(true, true, false, true, true, 1)
  action policy(split: bool, eager: bool, reserved: bool, fairCpu: bool, bounded: bool, feed: int): bool = all {
    life::s.accepted.size() == 0, feed > 0, feed <= JOBS.size(), life::stutter,
    q' = { ...q, split: split, eager: eager, reserved: reserved, fairCpu: fairCpu, bounded: bounded, feedMax: feed }
  }
  def ownPending(owner: int): Set[int] = life::s.accepted.exclude(life::s.completed).filter(r => ASSIGNED.get(r) == owner)
  def classSpace(r: int): bool = not(q.reserved) or
    ownPending(ASSIGNED.get(r)).filter(j => shortJobs.contains(j) == shortJobs.contains(r)).size() < 1
  action offer(r: int): bool = all {
    classSpace(r), life::admit(r), q' = { ...q, host: q.host.append(r) }
  }
  def hostHead(owner: int): List[int] = keep(q.host, r => ASSIGNED.get(r) == owner)
  def ownerFree(owner: int): bool = q.cpu.get(owner) == 0 and not(q.gc.contains(owner)) and not(q.paused.contains(owner))
  def canFeed(r: int): bool = and {
    entries(q.host).contains(r), ownerFree(ASSIGNED.get(r)),
    hostHead(ASSIGNED.get(r)).head() == r,
    q.feedUsed.get(ASSIGNED.get(r)) < q.feedMax
  }
  action feed(r: int): bool = all {
    canFeed(r), life::submitService(r),
    q' = { ...q, host: keep(q.host, j => j != r), queued: q.queued.append(r),
      feedUsed: q.feedUsed.set(ASSIGNED.get(r), q.feedUsed.get(ASSIGNED.get(r)) + 1) }
  }
  def feedShortFirst(owner: int): bool = {
    val pending = hostHead(owner)
    pending.length() > 0 and shortJobs.contains(pending.head()) and q.feedUsed.get(owner) < q.feedMax
  }
  action boundary(owner: int): bool = all {
    ownerFree(owner), not(q.eager and feedShortFirst(owner)), life::stutter,
    q' = { ...q, feedUsed: q.feedUsed.set(owner, 0), entryGap: q.entryGap or feedShortFirst(owner) }
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
    ownerFree(workerOwner(w)), q.jobs.get(w) == 0,
    eligible(w).length() > 0, eligible(w).head() == r, life::stutter,
    q' = { ...q, queued: keep(q.queued, j => j != r), jobs: q.jobs.set(w, r), lastShort: q.lastShort.set(w, shortJobs.contains(r)) }
  }
  def runnable(w: int): bool = q.jobs.get(w) != 0 and not(q.blocked.contains(w))
  def classRunnable(owner: int, short: bool): Set[int] = workers.filter(w =>
    workerOwner(w) == owner and runnable(w) and shortJobs.contains(q.jobs.get(w)) == short)
  def preferShortCpu(owner: int): bool = classRunnable(owner, true).size() > 0 and
    (not(q.debt.get(owner)) or classRunnable(owner, false).size() == 0)
  def allowedCpu(w: int): bool = not(q.fairCpu) or
    (if (preferShortCpu(workerOwner(w))) shortJobs.contains(q.jobs.get(w)) else
      (classRunnable(workerOwner(w), false).size() == 0 or not(shortJobs.contains(q.jobs.get(w)))))
  action acquireCpu(w: int): bool = all {
    ownerFree(workerOwner(w)), runnable(w), allowedCpu(w), life::stutter,
    q' = { ...q, cpu: q.cpu.set(workerOwner(w), w), spent: q.spent.set(workerOwner(w), 0),
      debt: q.debt.set(workerOwner(w), shortJobs.contains(q.jobs.get(w))),
      overtakes: JOBS.mapBy(r => if (r == q.jobs.get(w)) 0 else
        if (not(shortJobs.contains(q.jobs.get(w))) and classRunnable(workerOwner(w), true).exists(sw => q.jobs.get(sw) == r))
          (if (q.overtakes.get(r) < 2) q.overtakes.get(r) + 1 else 2) else q.overtakes.get(r)),
      cpuGap: q.cpuGap or (preferShortCpu(workerOwner(w)) and not(shortJobs.contains(q.jobs.get(w)))) }
  }
  def canCompute(w: int): bool = and {
    q.cpu.get(workerOwner(w)) == w, runnable(w),
    not(q.gc.contains(workerOwner(w))), not(q.paused.contains(workerOwner(w))),
    not(q.bounded) or q.spent.get(workerOwner(w)) < 1
  }
  action compute(w: int): bool = {
    val r = q.jobs.get(w)
    all {
      canCompute(w), q.left.get(r) > 0,
      if (q.left.get(r) == 1) all {
        life::readyService(r, false),
        q' = { ...q, left: q.left.set(r, 0), jobs: q.jobs.set(w, 0), cpu: q.cpu.set(workerOwner(w), 0), spent: q.spent.set(workerOwner(w), 0) }
      } else all {
        life::stutter,
        q' = { ...q, left: q.left.set(r, q.left.get(r) - 1), spent: q.spent.set(workerOwner(w), q.spent.get(workerOwner(w)) + 1) }
      }
    }
  }
  action yieldCpu(w: int): bool = all {
    q.cpu.get(workerOwner(w)) == w, not(q.paused.contains(workerOwner(w))), not(q.gc.contains(workerOwner(w))), life::stutter,
    q' = { ...q, cpu: q.cpu.set(workerOwner(w), 0), spent: q.spent.set(workerOwner(w), 0) }
  }
  action block(w: int): bool = all {
    q.cpu.get(workerOwner(w)) == w, not(q.gc.contains(workerOwner(w))), not(q.paused.contains(workerOwner(w))), life::stutter,
    q' = { ...q, blocked: q.blocked.union(Set(w)), cpu: q.cpu.set(workerOwner(w), 0), spent: q.spent.set(workerOwner(w), 0) }
  }
  action wakeTask(w: int): bool = all { q.blocked.contains(w), life::stutter, q' = { ...q, blocked: q.blocked.exclude(Set(w)), overtakes: q.overtakes.set(q.jobs.get(w), 0) } }
  action gcStart(owner: int): bool = all { Set("open", "closing").contains(life::s.phase), not(q.gc.contains(owner)), life::stutter, q' = { ...q, gc: q.gc.union(Set(owner)) } }
  action gcFinish(owner: int): bool = all { q.gc.contains(owner), not(q.paused.contains(owner)), life::stutter, q' = { ...q, gc: q.gc.exclude(Set(owner)) } }
  action osPause(owner: int): bool = all { not(q.paused.contains(owner)), life::stutter, q' = { ...q, paused: q.paused.union(Set(owner)) } }
  action osResume(owner: int): bool = all { q.paused.contains(owner), life::stutter, q' = { ...q, paused: q.paused.exclude(Set(owner)) } }
  action delayTick = all {
    life::stutter,
    q' = { ...q, age: JOBS.mapBy(r => if (shortJobs.contains(r) and life::s.accepted.contains(r) and not(life::s.serviceReady.contains(r)) and not(life::s.completed.contains(r)))
      (if (q.age.get(r) < 4) q.age.get(r) + 1 else 4) else q.age.get(r)) }
  }
  action project(r: int): bool = all { life::s.serviceReady.contains(r), life::complete(r), q' = q }
  action abandon(r: int): bool = all { life::observeTimeout(r), q' = q }
  action resourceClose = all { life::close, q' = q }
  action resourceJoin = all {
    q.gc.size() == 0, q.paused.size() == 0, workers.forall(w => q.jobs.get(w) == 0),
    life::join, q' = q
  }
  action resourceStutter = all { life::stutter, q' = q }
  val resourceSafety = and {
    life::safety, entries(q.host) == life::s.inbox,
    q.host.length() == entries(q.host).size(), q.queued.length() == entries(q.queued).size(),
    entries(q.queued).union(workers.map(w => q.jobs.get(w)).exclude(Set(0))) == life::s.servicePending.exclude(life::s.serviceReady),
    workers.map(w => q.jobs.get(w)).filter(r => r != 0).size() == workers.filter(w => q.jobs.get(w) != 0).size(),
    workers.forall(w => implies(q.jobs.get(w) != 0, ASSIGNED.get(q.jobs.get(w)) == workerOwner(w))),
    owners.forall(o => implies(q.cpu.get(o) != 0, workerOwner(q.cpu.get(o)) == o and runnable(q.cpu.get(o)))),
    q.blocked.forall(w => q.jobs.get(w) != 0), q.gc.subseteq(owners), q.paused.subseteq(owners),
    q.feedMax > 0, owners.forall(o => q.feedUsed.get(o) <= q.feedMax),
    implies(q.reserved and q.fairCpu, shortJobs.forall(r => q.overtakes.get(r) <= 1)),
    implies(q.eager, not(q.entryGap)), implies(q.fairCpu, not(q.cpuGap)), not(q.resourceViolation)
  }
  // Mutation controls validate the model's guards, not latency improvements.
  action unsafeBoundary(owner: int): bool = all {
    feedShortFirst(owner), life::stutter, q' = { ...q, entryGap: true, feedUsed: q.feedUsed.set(owner, 0) }
  }
  action unsafeCpu(w: int): bool = all {
    runnable(w), life::stutter,
    q' = { ...q, cpu: q.cpu.set(workerOwner(w), w),
      cpuGap: q.cpuGap or (preferShortCpu(workerOwner(w)) and not(shortJobs.contains(q.jobs.get(w)))),
      resourceViolation: q.resourceViolation or q.gc.contains(workerOwner(w)) or q.paused.contains(workerOwner(w))
        or (q.cpu.get(workerOwner(w)) != 0 and q.cpu.get(workerOwner(w)) != w) }
  }
  action unsafeFeed(r: int): bool = all {
    life::submitService(r), q' = { ...q, host: keep(q.host, j => j != r), queued: q.queued.append(r),
      feedUsed: q.feedUsed.set(ASSIGNED.get(r), q.feedUsed.get(ASSIGNED.get(r)) + 1),
      resourceViolation: q.resourceViolation or not(canFeed(r)) }
  }
  action resourceStep = {
    nondet r = JOBS.oneOf()
    nondet w = workers.oneOf()
    nondet owner = owners.oneOf()
    any { offer(r), feed(r), boundary(owner), assignWorker(w, r), acquireCpu(w), compute(w), yieldCpu(w),
      block(w), wakeTask(w), gcStart(owner), gcFinish(owner), osPause(owner), osResume(owner), delayTick,
      project(r), abandon(r), resourceClose, resourceJoin, resourceStutter }
  }
  action unsafeBoundaryStep = { nondet owner = owners.oneOf(); any { resourceStep, unsafeBoundary(owner) } }
  action unsafeCpuStep = { nondet w = workers.oneOf(); any { resourceStep, unsafeCpu(w) } }
  action unsafeFeedStep = { nondet r = JOBS.oneOf(); any { resourceStep, unsafeFeed(r) } }

}
module service_sharing_cases {
  const CASE_JOBS: Set[int]
  const CASE_ASSIGNED: int -> int
  import service_sharing(JOBS = CASE_JOBS, ASSIGNED = CASE_ASSIGNED).*
  action currentInit = implementationInit
  action sharedInit = candidateInit
  action sharedStep = resourceStep
  val sharedSafety = resourceSafety
  action mutationStart = mutationInit
  action boundaryMutationStep = unsafeBoundaryStep
  action cpuMutationStep = unsafeCpuStep
  action feedMutationStep = unsafeFeedStep
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
