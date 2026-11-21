// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
// Reusable protocol state; scenarios remain at the original entry.
module actor_batch_core {
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
  // A new physical Actor can occupy a previously joined logical slot. Existing
  // one-shot clients never call this action; lifetime epoch custody is external.
  action reopenActor(a:int):bool = all {
    phase=="open", not(published), joined.contains(a), terminal.contains(a),
    status'=status.set(a,"ready"), lease'=lease.set(a,0), joined'=joined.exclude(Set(a)),
    phase'=phase, cancelled'=cancelled, published'=published, delivered'=delivered
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

  action unsafeCleanup = all {
    phase == "draining", phase' = "finalized", status' = status, lease' = lease,
    joined' = joined, cancelled' = cancelled, published' = published, delivered' = delivered
  }

}
