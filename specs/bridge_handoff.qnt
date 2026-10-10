// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
// Cross-runtime integration protocol; Tokio/Gerbil schedulers are assumptions.
module bridge_handoff {
  const REQUESTS: Set[int]
  const WORKERS: Set[int]
  const CAPACITY: int
  pure val requests = REQUESTS
  pure val workers = WORKERS
  pure val capacity = CAPACITY
  type State = {
    phase: str,
    status: int -> str,
    owner: int -> int,
    foreignHandoff: Set[int],
    roots: Set[int],
    cancelled: Set[int],
    completions: int -> int,
    receivers: Set[int],
    settled: Set[int],
    delivered: Set[int],
    enabled: Set[int],
    queueLimit: int
  }
  var s: State

  val queued = requests.filter(r => s.status.get(r) == "queued")
  val active = requests.filter(r => Set("running", "rooted").contains(s.status.get(r)))
  val terminal = requests.filter(r => Set("done", "cancelled").contains(s.status.get(r)))
  val idle = s.enabled.filter(w => not(active.exists(r => s.owner.get(r) == w)))
  // Scheme CPU workers may overlap; this protocol's foreign copy/root handoff
  // stays with one control owner. It does not limit Scheme processor count.
  val publishable = requests.filter(r => s.status.get(r) == "running" and s.foreignHandoff.size() == 0)

  action init = s' = {
    phase: "running",
    status: requests.mapBy(_ => "new"),
    owner: requests.mapBy(_ => 0),
    foreignHandoff: Set(),
    roots: Set(), cancelled: Set(),
    completions: requests.mapBy(_ => 0), receivers: Set(), settled: Set(),
    delivered: Set(), enabled: workers, queueLimit: capacity
  }

  action admit(r: int): bool = all {
    s.phase == "running", s.status.get(r) == "new", queued.size() < s.queueLimit,
    s' = { ...s, status: s.status.set(r, "queued"), receivers: s.receivers.union(Set(r)) }
  }
  action dispatch(r: int, w: int): bool = all {
    Set("running", "draining").contains(s.phase),
    queued.contains(r), idle.contains(w),
    s' = { ...s, status: s.status.set(r, "running"), owner: s.owner.set(r, w) }
  }
  action publish(r: int): bool = all {
    publishable.contains(r),
    s' = { ...s, status: s.status.set(r, "rooted"), roots: s.roots.union(Set(r)), foreignHandoff: Set(r) }
  }
  // Owner-side root release precedes terminal host completion, even on cancel.
  action finish(r: int): bool = all {
    s.status.get(r) == "rooted", s.roots.contains(r), s.foreignHandoff.contains(r),
    s' = { ...s,
      status: s.status.set(r, if (s.cancelled.contains(r)) "cancelled" else "done"),
      owner: s.owner.set(r, 0), roots: s.roots.exclude(Set(r)), foreignHandoff: Set(),
      completions: s.completions.set(r, s.completions.get(r) + 1)
    }
  }
  action cancel(r: int): bool = all {
    Set("queued", "running", "rooted", "done").contains(s.status.get(r)),
    not(s.cancelled.contains(r)), not(s.settled.contains(r)),
    // Dropping/aborting a Rust awaiter does not abort already admitted FFI work.
    s' = { ...s, cancelled: s.cancelled.union(Set(r)), receivers: s.receivers.exclude(Set(r)) }
  }
  action notify(r: int): bool = all {
    terminal.contains(r), not(s.settled.contains(r)),
    s' = { ...s, settled: s.settled.union(Set(r)), receivers: s.receivers.exclude(Set(r)),
      delivered: if (s.receivers.contains(r)) s.delivered.union(Set(r)) else s.delivered }
  }
  action close = all { s.phase == "running", s' = { ...s, phase: "draining" } }
  action cleanup = all {
    s.phase == "draining", queued.size() == 0, active.size() == 0, s.roots.size() == 0,
    terminal.subseteq(s.settled),
    s' = { ...s, phase: "finalized" }
  }
  action stutter = s' = s
  // Adapt within a resource budget; never retire a worker holding a request/root.
  action park(w: int): bool = all {
    s.phase == "running", idle.contains(w), s.enabled.size() > 1,
    s' = { ...s, enabled: s.enabled.exclude(Set(w)) }
  }
  action enable(w: int): bool = all {
    s.phase == "running", workers.contains(w), not(s.enabled.contains(w)),
    s' = { ...s, enabled: s.enabled.union(Set(w)) }
  }
  action rebudget(n: int): bool = all {
    s.phase == "running", n >= 1, n <= capacity, queued.size() <= n,
    s' = { ...s, queueLimit: n }
  }
  action step = {
    nondet r = requests.oneOf()
    nondet w = workers.oneOf()
    nondet n = 1.to(capacity).oneOf()
    any { admit(r), dispatch(r, w), publish(r), finish(r), cancel(r), notify(r),
      park(w), enable(w), rebudget(n), close, cleanup, stutter }
  }

  val safety = and {
    s.queueLimit >= 1, s.queueLimit <= capacity, queued.size() <= s.queueLimit,
    s.enabled.subseteq(workers), s.enabled.size() >= 1,
    active.forall(r => s.enabled.contains(s.owner.get(r))),
    workers.forall(w => active.filter(r => s.owner.get(r) == w).size() <= 1),
    requests.forall(r => s.roots.contains(r) == (s.status.get(r) == "rooted")),
    s.roots == s.foreignHandoff, s.foreignHandoff.size() <= 1,
    terminal.forall(r => not(s.roots.contains(r))),
    s.settled.subseteq(terminal), s.delivered.subseteq(s.settled),
    s.delivered.intersect(s.cancelled).size() == 0,
    s.receivers.intersect(s.settled).size() == 0,
    requests.forall(r => s.completions.get(r) == (if (terminal.contains(r)) 1 else 0)),
    implies(s.phase == "finalized", and { queued.size() == 0, active.size() == 0,
      s.roots.size() == 0, terminal.subseteq(s.settled) })
  }

  // Mutation: missing shutdown barrier must yield a safety counterexample.
  action unsafeCleanup = all {
    s.phase == "draining", s' = { ...s, phase: "finalized" }
  }
  action unsafeStep = any { step, unsafeCleanup }

  run parallelWorkersTest = init.then(admit(1)).then(admit(2))
    .then(dispatch(1, 1)).then(dispatch(2, 2))
    .then(all { assert(active.size() == 2), assert(safety), s' = s })
  run parallelWorkersSerializeForeignHandoffTest = init.then(admit(1)).then(admit(2))
    .then(dispatch(1, 1)).then(dispatch(2, 2)).then(publish(1))
    .then(all { assert(active.size() == 2), assert(not(publishable.contains(2))), assert(safety), s' = s })
    .then(finish(1)).then(publish(2)).then(finish(2))
    .then(all { assert(s.foreignHandoff.size() == 0), assert(safety), s' = s })
  run cancellationRetainsRootTest = init.then(admit(1)).then(dispatch(1, 1))
    .then(publish(1)).then(cancel(1))
    .then(all { assert(s.roots.contains(1)), assert(safety), s' = s })
    .then(finish(1)).then(all { assert(s.completions.get(1) == 1), assert(safety), s' = s })
  run shutdownDrainsTest = init.then(admit(1)).then(dispatch(1, 1)).then(close)
    .then(publish(1)).then(finish(1)).then(notify(1)).then(cleanup)
    .then(all { assert(s.phase == "finalized"), assert(safety), s' = s })
  run earlyCleanupMutationTest = init.then(admit(1)).then(dispatch(1, 1))
    .then(publish(1)).then(close).then(unsafeCleanup)
    .then(all { assert(not(safety)), s' = s })
  run adaptiveWorkerTest = init.then(park(2)).then(rebudget(1))
    .then(admit(1)).then(dispatch(1, 1)).then(enable(2)).then(rebudget(2))
    .then(admit(2)).then(dispatch(2, 2))
    .then(all { assert(active.size() == 2), assert(safety), s' = s })
  run droppedRustAwaiterTest = init.then(admit(1)).then(dispatch(1, 1))
    .then(publish(1)).then(cancel(1)).then(finish(1)).then(notify(1))
    .then(all { assert(s.settled.contains(1)), assert(not(s.delivered.contains(1))),
      assert(s.roots.size() == 0), assert(safety), s' = s })
  run queuedCancellationStillDrainsTest = init.then(admit(1)).then(cancel(1))
    .then(close).then(dispatch(1, 1)).then(publish(1)).then(finish(1)).then(notify(1)).then(cleanup)
    .then(all { assert(safety), assert(s.completions.get(1) == 1), s' = s })
  run completedReceiverDroppedBeforeNotificationTest = init.then(admit(1))
    .then(dispatch(1, 1)).then(publish(1)).then(finish(1)).then(cancel(1))
    .then(notify(1)).then(close).then(cleanup)
    .then(all { assert(safety), assert(s.settled.contains(1)),
      assert(not(s.delivered.contains(1))), assert(s.completions.get(1) == 1), s' = s })
  action unsafeNotify(r: int): bool = all {
    s.status.get(r) == "rooted", s' = { ...s, settled: s.settled.union(Set(r)) }
  }
  run notifyBeforeRootReleaseMutationTest = init.then(admit(1)).then(dispatch(1, 1))
    .then(publish(1)).then(unsafeNotify(1))
    .then(all { assert(not(safety)), s' = s })
  action unsafeForeignPublish(r: int): bool = all {
    s.status.get(r) == "running",
    s' = { ...s, status: s.status.set(r, "rooted"), roots: s.roots.union(Set(r)), foreignHandoff: Set(r) }
  }
  run overlappingForeignHandoffMutationTest = init.then(admit(1)).then(admit(2))
    .then(dispatch(1, 1)).then(dispatch(2, 2)).then(publish(1)).then(unsafeForeignPublish(2))
    .then(all { assert(not(safety)), s' = s })
}

// Finite checker instances, not production worker/request limits.
module bridge_handoff_small {
  import bridge_handoff(REQUESTS = Set(1, 2, 3), WORKERS = Set(1, 2), CAPACITY = 2).*
}
module bridge_handoff_wide {
  import bridge_handoff(REQUESTS = Set(1, 2, 3, 4, 5), WORKERS = Set(1, 2, 3, 4), CAPACITY = 4).*
}
