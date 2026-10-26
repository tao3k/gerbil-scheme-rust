package SchemeRustScheduling {
    private import ScalarValues::*;

    // Architectural intent and API ownership; not an executable latency model.
    item def RustInput {
        attribute requestId : Integer;
        attribute stateful : Boolean;
        attribute ownerAffinity : Integer;
        attribute carriesSchemeRoot : Boolean = false;
    }
    item def RustResult {
        attribute requestId : Integer;
        attribute failed : Boolean;
        attribute ownedByRust : Boolean = true;
    }
    item def NativeCall;
    item def OwnerLocalSchemeValue;

    port def RequestPort {
        in item request : RustInput;
        out item result : RustResult;
    }
    port def NativePort {
        in item call : NativeCall;
        out item value : OwnerLocalSchemeValue;
    }
    part def BoundedQuota {
        attribute limit : Integer;
        attribute held : Integer;
        assert constraint bounded { held >= 0 and held <= limit }
    }
    part def PrivateHeap;
    part def GerbilActorScheduler {
        doc /* Gerbil schedules Actors and green threads inside the VM.
                Actors are not additional native execution threads. */
        attribute serviceWorkers : Integer = 4;
        attribute actorsPerBulkLane : Integer = 4;
        attribute additionalNativeExecutors : Integer = 0;
        // Declared record cost differs from native item-capacity credits.
        attribute shortRecordsPerItem : Integer = 4;
        attribute bulkRecordsPerItem : Integer = 48;
        attribute recordBudgetEnabled : Boolean;
    }
    part def GerbilVm {
        doc /* Scheme retains evaluation, continuations, exceptions and GC.
                Scheme references remain owned by this VM throughout execution. */
        attribute nativeExecutors : Integer = 1;
        part heap : PrivateHeap;
        part scheduler : GerbilActorScheduler;
        port native : NativePort;
        assert constraint singleExecutor { nativeExecutors == 1 }
    }
    part def OwnerBridge {
        doc /* NativeSession constructs and projects on the fixed owner.
                Rust-owned inputs precede native roots. Projection balances roots before publication. Native-credit retirement
                order is a captured policy; client quotas remain independent. */
        port rust : RequestPort;
        port scheme : ~NativePort;
        part requestSlots : BoundedQuota;
        part workUnits : BoundedQuota;
    }
    part def FixedOwner {
        part bridge : OwnerBridge;
        part vm : GerbilVm;
        connect bridge.scheme to vm.native;
    }
    part def TokioDispatcher {
        doc /* Routes stateless Rust inputs across owners. Stateful sessions
                retain affinity. It never schedules a Gerbil green thread. */
        attribute workers : Integer;
        part ingressSlots : BoundedQuota[0..1];
        part clientSlots : BoundedQuota[0..1];
        port route : ~RequestPort;
    }
    part def RayonPreparation {
        doc /* Optional bounded Rust-only preparation. Acquire preparation
                capacity before starting. Never enter Scheme or retain its roots. */
        attribute enabled : Boolean = false;
        part preparedInputs : BoundedQuota;
    }
    part def SingleVmBaseline {
        attribute messages : Integer = 1000000;
        attribute outstanding : Integer = 1;
        attribute retireBeforePublish : Boolean = false;
        part dispatcher : TokioDispatcher {
            attribute :>> workers = 1;
        }
        part owner : FixedOwner {
            part :>> vm {
                part :>> scheduler {
                    attribute :>> recordBudgetEnabled = false;
                }
            }
        }
        connect dispatcher.route to owner.bridge.rust;
        assert constraint workload { messages == 1000000 and outstanding == 1 }
    }
    part def TwelveVmTarget {
        doc /* Deployment target, not a twelve-owner model-checking claim.
                These are available routing links, not broadcast execution.
                Shared CPU occupancy and per-VM throughput need native evidence. */
        attribute physicalCores : Integer = 12;
        attribute measuredMessages : Integer = 1000000;
        attribute workloadProfile : String;
        attribute outstandingPerOwner : Integer;
        attribute fixedArrival : Boolean;
        part dispatcher : TokioDispatcher {
            attribute :>> workers = 12;
        }
        part owners : FixedOwner[12];
        part preparation : RayonPreparation;
        connect dispatcher.route to owners.bridge.rust;
    }

    state def RequestLifecycle {
        first start then prepared;
        state prepared;
        state admitted;
        state executing;
        state externallySuspended;
        state nativeReady;
        state projectedAndRetired;
        state consumed;
        state cancelledBeforeEntry;
        state drainedAfterCancellation;
        state drainingNative;
        state cancelledNativeReady;
        transition first prepared then admitted;
        transition first admitted then executing;
        transition first executing then externallySuspended;
        transition first externallySuspended then executing;
        transition first executing then nativeReady;
        transition first nativeReady then projectedAndRetired;
        transition first projectedAndRetired then consumed;
        transition first prepared then cancelledBeforeEntry;
        transition first executing then drainingNative;
        transition first externallySuspended then drainingNative;
        transition first drainingNative then cancelledNativeReady;
        transition first cancelledNativeReady then drainedAfterCancellation;
    }

    part def BaselineObservation {
        doc /* One measured cohort, not a predicted bound or acceptance threshold. */
        attribute completedMessages : Integer;
        attribute messagesPerSecond : Real;
        attribute wholeMillionP99Ms : Real;
        attribute wholeMillionP999Ms : Real;
        attribute ownerCpuSeconds : Real;
        attribute gcWallSeconds : Real;
        attribute gcCollections : Integer;
    }
    part observedBaseline : BaselineObservation {
        attribute :>> completedMessages = 1000000;
        attribute :>> messagesPerSecond = 28008.109206007673;
        attribute :>> wholeMillionP99Ms = 0.375917;
        attribute :>> wholeMillionP999Ms = 0.465083;
        attribute :>> ownerCpuSeconds = 33.915538917;
        attribute :>> gcWallSeconds = 6.453341722488403;
        attribute :>> gcCollections = 17923;
    }

    part baseline : SingleVmBaseline;
    part scalingTarget : TwelveVmTarget {
        attribute :>> workloadProfile = "short_cpu_same_corpus_as_single_VM";
        attribute :>> outstandingPerOwner = 1;
        attribute :>> fixedArrival = false;
    }
    part productionAcceptance : TwelveVmTarget {
        attribute :>> workloadProfile = "bulk8x48_short1x4_ratio16to1";
        attribute :>> fixedArrival = true;
    }
}
