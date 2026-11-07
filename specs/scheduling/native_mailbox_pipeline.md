# Gerbil-native mailbox pipeline experiment

This experiment concerns how an application uses Gerbil's existing mailbox and
green threads. It does not modify Gambit's scheduler/GC or integrate a new Rust
per-message path. The unfinished Rust exchange-cycle candidate remains isolated
in another worktree and is excluded from these sources and binaries.

The frozen native reference remains476,775.66 completed messages/second,
4.64288% rejection and0.418ms successful p99 at500,000 planned arrivals/second.
At1,000,000/s it recorded505,582.60/s,49.44061% rejection and0.644ms p99.
These figures must not be replaced by a slower Rust API control. A new protocol
is evaluated against its actual absolute outcomes and the original native path.

## Existing mechanism and application-level gap

The original raw gateway calls `thread-yield!` before every receive and takes
one response. It can hand execution away while its mailbox already holds verified
work opportunities. Supply is revisited only after generator catch-up.

The optional pipeline uses the same four resident workers, `thread-send`,
`thread-receive`, unchanged handler, fresh four-record inputs and21-field oracle.
It captures only currently available completions, bounded by the original three
active credits. It yields only when no response is visible. Each result is taken,
verified and retired independently; its active opportunity is immediately reused
for already-staged input before the generator's next catch-up turn. No handler
message is merged and no queue or credit limit is enlarged. Snapshot slots are
cleared independently, preserving GC reachability for the other responses. Three
fixed active-slot leases replace the per-message native/application ID hash map.
Each transport token contains a unique generation and slot; stale/duplicate
responses cannot return the slot of a later message. Worker assignment remains
round-robin over accepted messages.

Normal and exceptional shutdown join the same workers. The pipeline also clears
known unconsumed snapshot/mailbox responses after joining. Before the measured
clock starts, a native contract checks snapshot GC, rejected duplicate take and
close with unread/failed replies; no completion heartbeat is fabricated by it.

## Model and measurement boundaries

`native_mailbox_pipeline.qnt` refines the existing native Actor reference and
continuous/owner models. The negative forced-handoff scenario violates a work
conservation property while retaining resource safety. This does not prove a
numeric throughput target or justify changing a watchdog.

The existing `gerbil_actor_arrival` Rust executable only starts and checks one
native call per arm. All per-message construction, execution, verification and
reclamation run in Gerbil. It is not Rust scheduling or a production type bridge.

Use `--pipeline-compare --rate=1000000 --seconds=10` for the original mailbox then
pipeline on one live VM/image. Each arm plans ten million messages. The fixed
active3/window128, original absolute timestamps, per-message five-second bound
and verified-completion watchdog remain unchanged. Report actual completions,
rejections, successful p99 and full accounting; zero accepted errors alone does
not admit1M/s. Retain unsuccessful results. No default or PR update is authorized
by a component result.
