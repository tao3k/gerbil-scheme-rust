// Process-wide backend choice only: not another owner/Actor scheduler.
// The old mutex is correct. This protects its linearizable semantics while
// evaluating a proposed mutex-free readiness read. The candidate was rejected.
module backend_choice {
  var mode: str
  var chosen: str
  var everPoisoned: bool
  var accepted: bool
  var saved: str

  val choices = Set("linked", "images")
  action init = all {
    mode' = "unclaimed", chosen' = "", everPoisoned' = false,
    accepted' = false, saved' = ""
  }
  action claim(kind: str): bool = all {
    choices.contains(kind),
    mode' = if (mode == "unclaimed") kind else mode,
    chosen' = if (mode == "unclaimed") kind else chosen,
    accepted' = (mode == "unclaimed" or mode == kind),
    everPoisoned' = everPoisoned, saved' = saved
  }
  action poison = all {
    mode' = "poisoned", everPoisoned' = true,
    chosen' = chosen, accepted' = accepted, saved' = saved
  }
  action idle = all {
    mode' = mode, chosen' = chosen, everPoisoned' = everPoisoned,
    accepted' = accepted, saved' = saved
  }
  val ready = mode == "images"
  val safety = and {
    Set("unclaimed","linked","images","poisoned").contains(mode),
    not(everPoisoned) or mode == "poisoned",
    not(choices.contains(mode)) or chosen == mode,
    mode != "unclaimed" or chosen == ""
  }
  action step = {
    nondet kind = choices.oneOf()
    any { claim(kind), poison, idle }
  }

  // Negative control for an unsafe load/check/store replacement, NOT a claim
  // that the original mutex implementation loses poison.
  action unsafeRead(kind: str): bool = all {
    (mode == "unclaimed" or mode == kind),
    saved' = kind, mode' = mode, chosen' = chosen,
    everPoisoned' = everPoisoned, accepted' = accepted
  }
  action unsafeStore = all {
    choices.contains(saved), mode' = saved, chosen' = saved,
    everPoisoned' = everPoisoned, accepted' = true, saved' = saved
  }
  run idempotentImageChoiceTest = init.then(claim("images")).then(claim("images"))
    .then(all { assert(ready), assert(accepted), assert(safety), idle })
  run conflictingChoiceRejectedTest = init.then(claim("linked")).then(claim("images"))
    .then(all { assert(not(accepted)), assert(mode == "linked"), assert(safety), idle })
  run poisonBeforeChoiceTest = init.then(poison).then(claim("images")).then(claim("linked"))
    .then(all { assert(not(accepted)), assert(not(ready)), assert(safety), idle })
  run poisonAfterChoiceTest = init.then(claim("images")).then(poison).then(claim("images"))
    .then(all { assert(not(accepted)), assert(not(ready)), assert(safety), idle })
  run staleWriteAfterPoisonMutationTest = init.then(claim("images"))
    .then(unsafeRead("images")).then(poison).then(unsafeStore)
    .then(all { assert(not(safety)), idle })
  run firstChoiceAfterPoisonMutationTest = init.then(unsafeRead("linked"))
    .then(poison).then(unsafeStore)
    .then(all { assert(not(safety)), idle })
}
