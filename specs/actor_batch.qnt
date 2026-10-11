// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
// Bridge batch lifecycle abstraction, NOT a Gerbil/Tokio scheduler proof.
module actor_batch {
  const ACTORS: Set[int]
  const PROCESSORS: Set[int]
  var phase: str
  var status: int -> str
  var lease: int -> int
  var joined: Set[int]
  var cancelled: bool
  var published: bool
  var delivered: bool

  val ready = ACTORS.filter(a => status.get(a) == "ready")
  val running = ACTORS.filter(a => status.get(a) == "running")
  val terminal = ACTORS.filter(a => Set("done", "failed").contains(status.get(a)))
  val idle = PROCESSORS.filter(p => not(running.exists(a => lease.get(a) == p)))

  action init = all {
    phase' = "open", status' = ACTORS.mapBy(_ => "ready"),
    lease' = ACTORS.mapBy(_ => 0), joined' = Set(), cancelled' = false,
    published' = false, delivered' = false
  }
  action start(a: int, p: int): bool = all {
    Set("open", "draining").contains(phase), ready.contains(a), idle.contains(p),
    status' = status.set(a, "running"), lease' = lease.set(a, p),
    phase' = phase, joined' = joined, cancelled' = cancelled,
    published' = published, delivered' = delivered
  }
  action yieldActor(a: int): bool = all {
    running.contains(a), status' = status.set(a, "ready"), lease' = lease.set(a, 0),
    phase' = phase, joined' = joined, cancelled' = cancelled,
    published' = published, delivered' = delivered
  }
  action finish(a: int, failed: bool): bool = all {
    running.contains(a), status' = status.set(a, if (failed) "failed" else "done"),
    lease' = lease.set(a, 0), phase' = phase, joined' = joined,
    cancelled' = cancelled, published' = published, delivered' = delivered
  }
  action joinActor(a: int): bool = all {
    terminal.contains(a), not(joined.contains(a)), joined' = joined.union(Set(a)),
    phase' = phase, status' = status, lease' = lease, cancelled' = cancelled,
    published' = published, delivered' = delivered
  }
  action cancel = all {
    not(cancelled), not(published), cancelled' = true,
    phase' = phase, status' = status, lease' = lease, joined' = joined,
    published' = published, delivered' = delivered
  }
  action close = all {
    phase == "open", phase' = "draining", status' = status, lease' = lease,
    joined' = joined, cancelled' = cancelled, published' = published, delivered' = delivered
  }
  action publish = all {
    joined == ACTORS, not(published), published' = true, delivered' = not(cancelled),
    phase' = phase, status' = status, lease' = lease, joined' = joined, cancelled' = cancelled
  }
  action cleanup = all {
    phase == "draining", joined == ACTORS, published,
    phase' = "finalized", status' = status, lease' = lease, joined' = joined,
    cancelled' = cancelled, published' = published, delivered' = delivered
  }
  action stutter = all {
    phase' = phase, status' = status, lease' = lease, joined' = joined,
    cancelled' = cancelled, published' = published, delivered' = delivered
  }
  action step = {
    nondet a = ACTORS.oneOf()
    nondet p = PROCESSORS.oneOf()
    any { start(a, p), yieldActor(a), finish(a, false), finish(a, true),
      joinActor(a), cancel, close, publish, cleanup, stutter }
  }
  val safety = and {
    running.size() <= PROCESSORS.size(),
    running.forall(a => PROCESSORS.contains(lease.get(a))),
    PROCESSORS.forall(p => running.filter(a => lease.get(a) == p).size() <= 1),
    joined.subseteq(terminal),
    implies(published, joined == ACTORS),
    implies(delivered, published and not(cancelled)),
    implies(phase == "finalized", joined == ACTORS and published)
  }
  // Many runnable actors are legal; only simultaneous CPU leases are bounded.
  run actorsOutnumberProcessorsTest = init
    .then(all { assert(ready.size() > PROCESSORS.size()), assert(safety), stutter })
    .then(start(1, 1)).then(start(2, 2)).then(yieldActor(1)).then(start(3, 1))
    .then(all { assert(running.size() == 2), assert(safety), stutter })
  run cancellationDoesNotDestroyActorsTest = init.then(start(1, 1)).then(cancel)
    .then(all { assert(running.contains(1)), assert(not(published)), assert(safety), stutter })
  run failedActorStillRequiresJoinTest = init.then(start(1, 1)).then(finish(1, true))
    .then(all { assert(not(joined.contains(1))), assert(safety), stutter })
    .then(joinActor(1)).then(all { assert(joined.contains(1)), assert(safety), stutter })
  action unsafeCleanup = all {
    phase == "draining", phase' = "finalized", status' = status, lease' = lease,
    joined' = joined, cancelled' = cancelled, published' = published, delivered' = delivered
  }
  run earlyCleanupMutationTest = init.then(start(1, 1)).then(close).then(unsafeCleanup)
    .then(all { assert(not(safety)), stutter })
}

// Finite checking domains, never production actor/processor limits.
module actor_batch_small {
  import actor_batch(ACTORS = Set(1, 2, 3, 4), PROCESSORS = Set(1, 2)).*
  run cancelledBatchDrainsBeforeCleanupTest = init.then(cancel).then(close)
    .then(start(1, 1)).then(finish(1, false)).then(joinActor(1))
    .then(start(2, 2)).then(finish(2, true)).then(joinActor(2))
    .then(start(3, 1)).then(finish(3, false)).then(joinActor(3))
    .then(start(4, 2)).then(finish(4, false)).then(joinActor(4))
    .then(publish).then(cleanup)
    .then(all { assert(safety), assert(not(delivered)), assert(phase == "finalized"), stutter })
}
module actor_batch_wide {
  import actor_batch(ACTORS = Set(1, 2, 3, 4, 5, 6), PROCESSORS = Set(1, 2, 3)).*
}

// Configuration is not processor activation. This bridge admission model
// complements actor_batch; it does not prove Gambit's resize implementation.
module processor_admission {
  var configured: int
  var active: int
  var admitted: bool
  action init = all { configured' = 1, active' = 1, admitted' = false }
  action configure(n: int): bool = all {
    n > 0, configured' = n, active' = active, admitted' = false
  }
  // Abstract successful completion of the official VM resize operation.
  // Failure leaves admission false; no worker may rely on the requested count.
  action activate = all {
    not(admitted), active' = configured, configured' = configured, admitted' = false
  }
  // Official resize clamps to the SDK's compiled processor capacity. An SMP
  // scheduler feature alone must never be treated as successful activation.
  action resizeClamped(capacity: int): bool = all {
    not(admitted), capacity > 0,
    active' = if (configured > capacity) capacity else configured,
    configured' = configured, admitted' = false
  }
  action admit = all {
    active == configured, admitted' = true, active' = active, configured' = configured
  }
  action unchanged = all {
    configured' = configured, active' = active, admitted' = admitted
  }
  action step = any { configure(1), configure(2), activate, resizeClamped(1), admit, unchanged }
  val safety = configured > 0 and active > 0 and implies(admitted, active == configured)
  run configurationDoesNotActivateTest = init.then(configure(2))
    .then(all { assert(active == 1), assert(not(admitted)), assert(safety), unchanged })
  run resizeBeforeAdmissionTest = init.then(configure(2)).then(activate).then(admit)
    .then(all { assert(active == 2), assert(admitted), assert(safety), unchanged })
  run oneProcessorControlTest = init.then(admit)
    .then(all { assert(active == 1), assert(admitted), assert(safety), unchanged })
  run sdkCapacityClampRejectsAdmissionTest = init.then(configure(12)).then(resizeClamped(1))
    .then(all { assert(configured == 12), assert(active == 1), assert(not(admitted)),
      assert(not(active == configured)), assert(safety), unchanged })
  action unsafeAdmitConfiguration = all {
    configured' = configured, active' = active, admitted' = true
  }
  run configurationOnlyAdmissionMutationTest = init.then(configure(2))
    .then(unsafeAdmitConfiguration)
    .then(all { assert(not(safety)), unchanged })
}
