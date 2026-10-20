// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
// Fixed-owner lifetime refinement. bridge_handoff owns queue/receiver semantics;
// this module adds close-flag publication and lazy-executor retirement races.
// Native execution, GC, FFI ownership and successful joins remain assumptions.
module owner_lifecycle {
  const JOBS: Set[int]
  const ASSIGNED: int -> int
  type State = {
    phase: str,
    accepted: Set[int], completed: Set[int], ready: Set[int],
    roots: Set[int], registry: Set[int], timedOut: Set[int],
    executor: str, filesPresent: bool, imagesRetained: bool,
    codeLeases: Set[int], unloadAuthorized: bool, codeQuarantined: bool,
    unsafeCodeRelease: bool,
    latePublication: bool, lateExecutor: bool, foreignUse: bool
  }
  var s: State

  action init = s' = {
    phase: "open", accepted: Set(), completed: Set(), ready: Set(),
    roots: Set(), registry: Set(), timedOut: Set(), executor: "absent",
    filesPresent: true, imagesRetained: true,
    codeLeases: Set(), unloadAuthorized: false, codeQuarantined: false,
    unsafeCodeRelease: false,
    latePublication: false, lateExecutor: false, foreignUse: false
  }
  action admit(r: int): bool = all {
    s.phase == "open", JOBS.contains(r), not(s.accepted.contains(r)),
    s' = { ...s, accepted: s.accepted.union(Set(r)) }
  }
  // Completion is an externally validated owner action, not Rust computation.
  action complete(r: int): bool = all {
    Set("open", "closing").contains(s.phase),
    s.accepted.contains(r), not(s.completed.contains(r)),
    s' = { ...s, completed: s.completed.union(Set(r)),
      roots: s.roots.union(Set(r)), ready: s.ready.union(Set(r)) }
  }
  // Observation timeout leaves accepted native work and roots unchanged.
  action observeTimeout(r: int): bool = all {
    s.accepted.contains(r), not(s.timedOut.contains(r)),
    s' = { ...s, timedOut: s.timedOut.union(Set(r)) }
  }
  val canJoin = s.phase == "closing" and s.accepted.subseteq(s.completed)
  def canPublish(r: int): bool = s.phase == "open" and s.ready.contains(r)
  def canUse(r: int, owner: int): bool = and {
    s.phase == "open", s.registry.contains(r), s.roots.contains(r),
    ASSIGNED.get(r) == owner
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
    Set("open", "closing").contains(s.phase), s.roots.contains(r), ASSIGNED.get(r) == owner,
    s' = { ...s, roots: s.roots.exclude(Set(r)), registry: s.registry.exclude(Set(r)),
      ready: s.ready.exclude(Set(r)) }
  }
  action createExecutorUnchecked = all {
    s.executor != "alive",
    s' = { ...s, executor: "alive", lateExecutor: s.lateExecutor or s.phase != "open" }
  }
  action createExecutor = all { s.phase == "open", createExecutorUnchecked }
  action close = all { s.phase == "open", s' = { ...s, phase: "closing" } }
  action joinUnchecked = all {
    s.phase == "closing", s' = { ...s, phase: "joined", roots: Set() }
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
  action stutter = s' = s
  action step = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    any { admit(r), complete(r), observeTimeout(r), publish(r), use(r, owner),
      release(r, owner), createExecutor, close, join, clearRegistry, retire,
      codeLease(r), releaseCodeLease(r), authorizeCodeRelease, quarantineCode, releaseCode, stutter }
  }
  val safety = and {
    s.accepted.subseteq(JOBS), s.completed.subseteq(s.accepted),
    s.timedOut.subseteq(s.accepted), s.ready.subseteq(s.completed),
    s.roots.subseteq(s.completed), s.registry.subseteq(s.completed),
    not(s.latePublication), not(s.lateExecutor), not(s.foreignUse),
    implies(s.phase == "open", s.registry.subseteq(s.roots)),
    implies(Set("joined", "closed").contains(s.phase), and {
      s.accepted.subseteq(s.completed), s.roots.size() == 0 }),
    implies(s.phase == "closed", and { s.executor == "retired",
      not(s.filesPresent), s.registry.size() == 0 }),
    s.codeLeases.subseteq(JOBS), not(s.unsafeCodeRelease),
    implies(not(s.imagesRetained), and {
      Set("joined", "closed").contains(s.phase), s.unloadAuthorized,
      s.codeLeases.size() == 0, not(s.codeQuarantined) })
  }

  action unsafePublicationStep = { nondet r = JOBS.oneOf(); any { step, publishUnchecked(r) } }
  action unsafeExecutorStep = any { step, createExecutorUnchecked }
  action unsafeJoinStep = any { step, joinUnchecked }
  action unsafeCodeReleaseStep = any { step, releaseCodeUnchecked }
  action unsafeForeignUse = {
    nondet r = JOBS.oneOf()
    nondet owner = Set(1, 2).oneOf()
    all { s.phase == "open", s.registry.contains(r), ASSIGNED.get(r) != owner,
      s' = { ...s, foreignUse: true } }
  }
  action unsafeForeignStep = any { step, unsafeForeignUse }

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
}
module owner_lifecycle_small {
  import owner_lifecycle(JOBS = Set(1, 2, 3), ASSIGNED = Set(1, 2, 3).mapBy(r => if (r == 2) 2 else 1)).*
}
module owner_lifecycle_wide {
  import owner_lifecycle(JOBS = Set(1, 2, 3, 4, 5), ASSIGNED = Set(1, 2, 3, 4, 5).mapBy(r => if (Set(2, 4).contains(r)) 2 else 1)).*
}
