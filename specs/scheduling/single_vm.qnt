// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
// Directed refinements of existing protocols; no replacement scheduler state.
// Million-message counts and wall-clock latency belong to Rust measurements.
module single_vm_host {
  import gerbil_service_architecture(
    ARCH_JOBS = Set(1,2,3,4),
    ARCH_ASSIGNED = Set(1,2,3,4).mapBy(_ => 1)
  ) as host from "../owner_lifecycle"

  run nativeOwnerThenTokioTest = host::architectureInit(false,false,1)
    .then(host::architectureReserve).then(host::architecturePrepare)
    .then(host::architectureStart(1)).then(host::architectureNative(1))
    .then(host::architectureStop(1)).then(host::architectureStart(2))
    .then(all { assert(host::topology.nativeEntrants == Set(1)),
      assert(host::topology.running == Set(2)), assert(host::architectureSafety),
      host::architectureIdle })

  run rayonPreparesWithoutSchemeEntryTest = host::architectureInit(false,true,1)
    .then(host::architectureReserve).then(host::architectureStart(3))
    .then(host::architecturePrepare).then(host::architectureStop(3))
    .then(host::architectureStart(1)).then(host::architectureNative(1))
    .then(all { assert(host::topology.nativeEntrants == Set(1)),
      assert(host::architectureSafety), host::architectureIdle })

  run foreignSchemeEntryMutationTest = host::architectureInit(false,true,1)
    .then(host::architectureForeignMutation)
    .then(all { assert(not(host::architectureSafety)), host::architectureIdle })

  run doubleCountedCpuMutationTest = host::architectureInit(false,true,1)
    .then(host::architectureCpuMutation)
    .then(all { assert(not(host::architectureSafety)), host::architectureIdle })

  run resultCapacityWaitCycleMutationTest = host::architectureInit(false,false,1)
    .then(host::architectureWait(Set((1,6),(6,3),(3,5),(5,1))))
    .then(all { assert(not(host::architectureSafety)), host::architectureIdle })

  run durableCoalescedWakeTest = host::architectureInit(false,false,1)
    .then(host::architectureArm).then(host::architecturePublish)
    .then(host::architecturePublish)
    .then(all { assert(host::topology.wakes == 1),
      assert(host::topology.published == 2), assert(host::architectureSafety),
      host::architectureIdle })
    .then(host::architectureObserve).then(host::architectureArm)
    .then(all { assert(not(host::topology.wake)), assert(host::architectureSafety),
      host::architectureIdle })

  run lostWakeMutationTest = host::architectureInit(false,false,1)
    .then(host::architectureArm).then(host::architecturePublish)
    .then(host::architectureLostWakeMutation)
    .then(all { assert(not(host::architectureSafety)), host::architectureIdle })
}

module single_vm_actors {
  import actor_batch(ACTORS = Set(1,2,3,4), PROCESSORS = Set(1))
    as actors from "../actor_batch"

  action init = actors::init
  action step = actors::step
  val safety = actors::safety

  run fourActorsOneProcessorTest = actors::init
    .then(actors::start(1,1)).then(actors::yieldActor(1))
    .then(actors::start(2,1)).then(actors::finish(2,false))
    .then(actors::start(3,1)).then(actors::finish(3,false))
    .then(actors::start(4,1)).then(actors::finish(4,false))
    .then(actors::start(1,1)).then(actors::finish(1,false))
    .then(all { assert(actors::terminal == Set(1,2,3,4)),
      assert(actors::running == Set()), assert(actors::safety), actors::stutter })

  run cancelledTurnStillDrainsTest = actors::init
    .then(actors::start(1,1)).then(actors::cancel).then(actors::close)
    .then(all { assert(actors::running == Set(1)), assert(actors::safety), actors::stutter })
    .then(actors::finish(1,true)).then(actors::joinActor(1))
    .then(actors::start(2,1)).then(actors::finish(2,false)).then(actors::joinActor(2))
    .then(actors::start(3,1)).then(actors::finish(3,false)).then(actors::joinActor(3))
    .then(actors::start(4,1)).then(actors::finish(4,false)).then(actors::joinActor(4))
    .then(actors::publish).then(actors::cleanup)
    .then(all { assert(not(actors::delivered)), assert(actors::safety), actors::stutter })

  run prematureActorCleanupMutationTest = actors::init
    .then(actors::start(1,1)).then(actors::close).then(actors::unsafeCleanup)
    .then(all { assert(not(actors::safety)), actors::stutter })
}
