// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
// Bridge switching protocol only. Qualification is an external assumption,
// not a proof of an SDK, loader, Tokio scheduler, or actual parser execution.
module vm_execution {
  var route: str
  var phase: str
  var outstanding: int
  var roots: int
  var qualified: Set[str]

  val routes = Set("instances", "scheme-smp")

  action init = all {
    route' = "none", phase' = "idle", outstanding' = 0, roots' = 0,
    qualified' = Set()
  }

  // Abstract completed external gates, not flags or queue worker counts.
  action qualify(candidate: str): bool = all {
    routes.contains(candidate), phase == "idle",
    qualified' = qualified.union(Set(candidate)),
    route' = route, phase' = phase, outstanding' = outstanding, roots' = roots
  }

  def canChoose(candidate: str): bool = and {
    qualified.contains(candidate), phase == "idle", outstanding == 0, roots == 0
  }
  val canRetire = phase == "draining" and outstanding == 0 and roots == 0

  action chooseRoute(candidate: str): bool = all {
    canChoose(candidate),
    route' = candidate, phase' = "open", qualified' = qualified,
    outstanding' = outstanding, roots' = roots
  }

  action accept = all {
    // Three requests bound this finite model, not the production queue.
    phase == "open", outstanding < 3,
    outstanding' = outstanding + 1, roots' = roots + 1,
    route' = route, phase' = phase, qualified' = qualified
  }

  action release = all {
    outstanding > 0, Set("open", "draining").contains(phase),
    outstanding' = outstanding - 1, roots' = roots - 1,
    route' = route, phase' = phase, qualified' = qualified
  }

  action close = all {
    phase == "open", phase' = "draining", route' = route,
    outstanding' = outstanding, roots' = roots, qualified' = qualified
  }

  // Includes externally completed owner joins and lifecycle teardown. The
  // retained-image prototype has NOT discharged these reusable-lifetime gates.
  action retire = all {
    canRetire,
    phase' = "idle", route' = "none", qualified' = qualified,
    outstanding' = outstanding, roots' = roots
  }

  action stutter = all {
    phase' = phase, route' = route, outstanding' = outstanding,
    roots' = roots, qualified' = qualified
  }

  action step = {
    nondet candidate = routes.oneOf()
    any { qualify(candidate), chooseRoute(candidate), accept, release, close, retire, stutter }
  }

  val safety = and {
    outstanding >= 0, outstanding <= 3, roots == outstanding,
    implies(phase == "idle", route == "none" and outstanding == 0 and roots == 0),
    implies(phase != "idle", qualified.contains(route))
  }

  run unqualifiedRouteCannotStartTest = init
    .then(all { assert(not(canChoose("instances"))), assert(not(canChoose("scheme-smp"))), stutter })

  run requestsAndRootsMustDrainBeforeSwitchTest = init
    .then(qualify("instances")).then(chooseRoute("instances")).then(accept).then(close)
    .then(all { assert(not(canRetire)), assert(not(canChoose("scheme-smp"))), stutter })
    .then(release).then(retire)
    .then(qualify("scheme-smp")).then(chooseRoute("scheme-smp"))
    .then(all { assert(safety), assert(route == "scheme-smp"), stutter })

  run openRouteCannotBeReplacedTest = init
    .then(qualify("instances")).then(qualify("scheme-smp")).then(chooseRoute("instances"))
    .then(all { assert(not(canChoose("scheme-smp"))), assert(safety), stutter })
}
