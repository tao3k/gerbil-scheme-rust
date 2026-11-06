//! Same Gerbil endpoint and order handler; one foreign call per measured arm.
use gerbil_scheme::instances::{
    InstanceConfig, InstanceError, InstancePool, IsolatedProgram, NativeRoot, NativeSession,
};
use gerbil_scheme::{ObjectKind, SchemeScalar};
use gerbil_scheme_qualification::mailbox_admission::MailboxSample;
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};
const GATE: Duration = Duration::from_secs(5);
const REQUEST_DEADLINE: Duration = GATE;
fn capacity_research() -> bool {
    std::env::args().any(|s| s == "--capacity-research")
}
#[path = "gerbil_actor_arrival/terminal_histogram.rs"]
mod terminal_histogram;
#[path = "gerbil_actor_arrival/wait_evidence.rs"]
mod wait_evidence;
const ARRIVAL_WINDOW: usize = 128;

// Reuse qualification's existing native QoS operation. This scope lives on the
// fixed owner and restores the saved class after Scheme failure or Rust panic.
fn with_owner_qos<T>(
    session: &NativeSession<'_>,
    enabled: bool,
    work: impl FnOnce() -> Result<T, InstanceError>,
) -> Result<T, InstanceError> {
    if !enabled {
        return work();
    }
    let request = session.numeric_vector::<i64>(&[12, 1])?;
    let set = session.copy_numeric::<i64>(&session.call(&request)?)?;
    println!(
        "{}",
        json!({"event":"native_qos_admission_receipt","receipt":set})
    );
    let outcome = if matches!(set.as_slice(), [0, _, _]) {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
    } else {
        Ok(Err(InstanceError::InvalidConfig("native QoS admission")))
    };
    let restore = (|| {
        let request = session.numeric_vector::<i64>(&[12, 0])?;
        let restored = session.copy_numeric::<i64>(&session.call(&request)?)?;
        println!(
            "{}",
            json!({"event":"native_qos_restore_receipt","receipt":restored})
        );
        match restored.as_slice() {
            [0, actual, saved] if actual == saved => Ok(()),
            _ => Err(InstanceError::InvalidConfig("native QoS restoration")),
        }
    })();
    match outcome {
        Ok(result) => {
            restore?;
            result
        }
        Err(panic) => {
            if let Err(error) = restore {
                eprintln!("NATIVE-QOS-RESTORE-FAILED {error} cleanup=UNVERIFIED");
            }
            std::panic::resume_unwind(panic)
        }
    }
}

// The native report owns its names and shape. This projection only understands
// existing Scheme collection/scalar types; no stage count or field order.
fn native_report(
    session: &NativeSession<'_>,
    root: &NativeRoot<'_>,
) -> Result<serde_json::Value, InstanceError> {
    match session.kind(root)? {
        ObjectKind::GambitTable | ObjectKind::GerbilHashTable => {
            let entries = session.elements(root)?;
            let mut result = serde_json::Map::new();
            let mut pairs = entries.chunks_exact(2);
            for pair in &mut pairs {
                result.insert(
                    session.copy_string(&pair[0])?,
                    native_report(session, &pair[1])?,
                );
            }
            assert!(pairs.remainder().is_empty(), "native table key/value pairs");
            Ok(result.into())
        }
        ObjectKind::Vector => session
            .elements(root)?
            .iter()
            .map(|value| native_report(session, value))
            .collect::<Result<Vec<_>, _>>()
            .map(Into::into),
        ObjectKind::String => session.copy_string(root).map(Into::into),
        ObjectKind::ExactInteger => {
            let integer = session.copy_big_integer(root)?;
            if !integer.is_negative() && integer.magnitude().len() <= 8 {
                let mut bytes = [0_u8; 8];
                bytes[..integer.magnitude().len()].copy_from_slice(integer.magnitude());
                Ok(u64::from_le_bytes(bytes).into())
            } else {
                Ok(json!({"negative":integer.is_negative(),"magnitude_le":integer.magnitude()}))
            }
        }
        ObjectKind::Boolean => match session.copy_scalar(root)? {
            SchemeScalar::Fixnum(value) => Ok(value.into()),
            SchemeScalar::Boolean(value) => Ok(value.into()),
            _ => Err(InstanceError::InvalidConfig("native observation scalar")),
        },
        _ => Err(InstanceError::InvalidConfig(
            "native observation report type",
        )),
    }
}
fn clock() -> u64 {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    #[cfg(target_os = "macos")]
    let clock_id = libc::CLOCK_UPTIME_RAW;
    #[cfg(not(target_os = "macos"))]
    let clock_id = libc::CLOCK_MONOTONIC;
    // SAFETY: writable timespec and the matching platform clock.
    assert_eq!(unsafe { libc::clock_gettime(clock_id, &raw mut now) }, 0);
    u64::try_from(now.tv_sec).unwrap() * 1_000_000_000 + u64::try_from(now.tv_nsec).unwrap()
}
struct Watchdog {
    last: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    task: Option<thread::JoinHandle<()>>,
}
impl Watchdog {
    fn start() -> Self {
        let last = Arc::new(AtomicU64::new(clock()));
        let stop = Arc::new(AtomicBool::new(false));
        let (progress, done) = (Arc::clone(&last), Arc::clone(&stop));
        let task = thread::spawn(move || {
            while !done.load(Ordering::Acquire) {
                if clock().saturating_sub(progress.load(Ordering::Acquire)) >= 10_000_000_000 {
                    // Reload after the clock in case a real completion raced it.
                    if clock().saturating_sub(progress.load(Ordering::Acquire)) >= 10_000_000_000 {
                        eprintln!("GERBIL-ACTOR-STALL ten_second_gate cleanup=UNVERIFIED");
                        std::process::exit(70);
                    }
                }
                thread::sleep(Duration::from_millis(100));
            }
        });
        Self {
            last,
            stop,
            task: Some(task),
        }
    }
}
impl Drop for Watchdog {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.task.take().unwrap().join().unwrap();
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum NativePath {
    BackpressureSchedule,
    BackpressureCoalesced,
    BackpressurePark,
    BackpressureUrgent,
    BackpressureOwnerUrgent,
    BackpressureOwnerWatch,
    BackpressureObserved,
    Backpressure,
    BackpressureAccess,
    Access,
    Direct,
    Service,
    Mailbox,
    Pipeline,
}
impl NativePath {
    fn command(self) -> i64 {
        match self {
            Self::BackpressurePark => 59,
            Self::BackpressureCoalesced => 60,
            Self::BackpressureSchedule => {
                if scope_diagnostic(self) {
                    64
                } else {
                    62
                }
            }
            Self::BackpressureUrgent | Self::BackpressureAccess => 56,
            Self::BackpressureOwnerUrgent | Self::BackpressureOwnerWatch => 58,
            Self::BackpressureObserved => 57,
            Self::Backpressure => 55,
            Self::Direct => 53,
            Self::Access => 54,
            Self::Service => 50,
            Self::Mailbox => 51,
            Self::Pipeline => 52,
        }
    }
}
impl NativePath {
    fn assert_rate(self, rate: u64, backpressure_compare: bool) {
        if capacity_research() {
            assert!(rate == 1_000_000);
            return;
        }
        if matches!(
            self,
            NativePath::Backpressure
                | NativePath::BackpressureAccess
                | NativePath::BackpressureObserved
                | NativePath::BackpressureOwnerWatch
                | NativePath::BackpressureOwnerUrgent
                | NativePath::BackpressureUrgent
                | NativePath::BackpressurePark
                | NativePath::BackpressureCoalesced
                | NativePath::BackpressureSchedule
        ) || backpressure_compare
        {
            assert!(
                rate <= 500_000,
                "500k zero-rejection guard precedes 1M research"
            );
        }
    }
}

fn scope_diagnostic(path: NativePath) -> bool {
    path == NativePath::BackpressureSchedule
        && std::env::args().any(|arg| arg == "--owner-wait-scope")
}

fn copy_arrival_receipt(
    session: &NativeSession<'_>,
    result: &NativeRoot<'_>,
    path: NativePath,
    warmup: bool,
) -> Result<Vec<i64>, InstanceError> {
    if scope_diagnostic(path)
        || matches!(
            path,
            NativePath::BackpressureObserved
                | NativePath::BackpressureOwnerWatch
                | NativePath::BackpressureOwnerUrgent
        )
    {
        let report = session.vector_element(result, 1)?;
        let stages = native_report(session, &report)?;
        let wait_evidence = wait_evidence::report(&stages);
        println!(
            "{}",
            json!({"event":if scope_diagnostic(path) {"owner_wait_scope"}else if matches!(path,NativePath::BackpressureOwnerWatch|NativePath::BackpressureOwnerUrgent) {"native_mailbox_owner_watch"}else{"native_mailbox_observation"},"diagnostic_only":true,
                    "scoped_user_initiated_qos":matches!(path,NativePath::BackpressureOwnerUrgent|NativePath::BackpressureUrgent),"native_source_park":path==NativePath::BackpressurePark,
                    "warmup":warmup,"stages":stages,"wait_evidence":wait_evidence})
        );
        session.copy_numeric::<i64>(&session.vector_element(result, 0)?)
    } else if path == NativePath::BackpressureCoalesced {
        let wake = session.copy_numeric::<i64>(&session.vector_element(result, 1)?)?;
        let [issued, taken, unread] = wake.as_slice() else {
            panic!("native coalesced wake receipt")
        };
        assert!(*issued >= *taken && issued - taken == *unread && (0..=1).contains(unread));
        println!(
            "{}",
            json!({"event":"native_ready_wake_receipt","warmup":warmup,
                    "issued":issued,"notifications_taken":taken,"retired_without_receiver_take":unread,"payload_frames":3,"wake_queue_bound":1})
        );
        session.copy_numeric::<i64>(&session.vector_element(result, 0)?)
    } else {
        session.copy_numeric::<i64>(result)
    }
}
#[expect(clippy::too_many_lines, reason = "Frozen benchmark")]
fn run(pool: &InstancePool, rate: u64, seconds: u64, warmup: bool, path: NativePath) -> bool {
    let watchdog = Watchdog::start();
    let anchored = std::env::args().any(|arg| arg == "--native-clock-anchor");
    let progress = Arc::clone(&watchdog.last);
    // AtomicU64 has the aligned C atomic-word representation used by this
    // Darwin qualification bridge. The owner closure keeps the allocation live
    // until the synchronous Scheme call has returned or unwound.
    let (receipt, whole_call_ns) = pool
        .dispatcher()
        .submit_native_on(0, move |session| {
            with_owner_qos(
                session,
                matches!(
                    path,
                    NativePath::BackpressureOwnerUrgent | NativePath::BackpressureUrgent
                ),
                || {
                    if anchored {
                        native_clock_anchor(session, warmup, "before")?;
                    }
                    let address = Arc::as_ptr(&progress) as usize;
                    let mut fields = vec![
                        path.command(),
                        rate.try_into().unwrap(),
                        seconds.try_into().unwrap(),
                        address.try_into().unwrap(),
                    ];
                    if scope_diagnostic(path)
                        || matches!(
                            path,
                            NativePath::BackpressureObserved
                                | NativePath::BackpressureOwnerWatch
                                | NativePath::BackpressureOwnerUrgent
                        )
                    {
                        fields.push(ARRIVAL_WINDOW.try_into().unwrap());
                    }
                    let header = session.numeric_vector(&fields)?;
                    let command = session.vector(&[&header])?;
                    let cpu_gc_before = session.diagnostics();
                    let call_started = clock();
                    let call_result = session.call_export(c"gerbil_owner_service_reference", &command);
                    let native_call_wall_ns = clock().saturating_sub(call_started);
                    let cpu_gc_after = session.diagnostics();
                    let cpu_gc_envelope = match (cpu_gc_before, cpu_gc_after) {
                        (Some(before),Some(after)) if (0..4).all(|i| after[i]>=before[i]) =>
                            json!({"owner_cpu_upper_ns":(after[3]-before[3])*1e9,"gc_wall_enclosing_ns":(after[2]-before[2])*1e9,"gc_cpu_enclosing_ns":(after[0]+after[1]-before[0]-before[1])*1e9,"gc_count_enclosing":after[5]-before[5],"scope":"encloses call, startup and terminal capture; not phase-exclusive"}),
                        _=>serde_json::Value::Null,
                    };
                    let result = match call_result {
                        Ok(result) => result,
                        Err(original) => {
                            println!("{}",json!({"event":"native_call_failure_before_report","warmup":warmup,"original_error":original.to_string(),"whole_native_call_ns":native_call_wall_ns,"cpu_gc_envelope":cpu_gc_envelope}));
                            let terminal = (|| -> Result<serde_json::Value, InstanceError> {
                                let report = session.call_export(c"gerbil_owner_mailbox_terminal_take", &command)?;
                                let phase = session.vector_element(&report, 0)?;
                                let core = session.vector_element(&report, 1)?;
                                let phase = session.copy_string(&phase)?;
                                let core = session.copy_numeric::<i64>(&core)?;
                                assert_eq!(core.len(), 17);
                                let elapsed = core[2];
                                let remaining = core[0]-core[7];
                                let minimum_pending_age = (elapsed-(core[0]-1)*1_000_000_000/core[1]).max(0);
                                let p99_lower = (core[3]==core[0] && remaining>core[0]-(core[0]*99+99)/100).then_some(minimum_pending_age);
                                let p999_lower = (core[3]==core[0] && remaining>core[0]-(core[0]*999+999)/1000).then_some(minimum_pending_age);
                                Ok(json!({"event":"gerbil_actor_main_failure", "warmup":warmup,
                                    "phase":phase,"original_error":original.to_string(),"cpu_gc_envelope":cpu_gc_envelope,"whole_native_call_ns":native_call_wall_ns,
                                    "planned":core[0],"rate":core[1],"elapsed_ns_at_failure":elapsed,
                                    "due_at_failure":core[3],"published":core[4],"completed":core[5],
                                    "active":core[6],"timed_samples":core[7],
                                    "completed_prefix_per_second":if elapsed>0 {Some(core[5] as f64*1e9/elapsed as f64)}else{None},
                                    "conditional_sample_p99_ns":(core[7]>0).then_some(core[8]),"conditional_sample_p999_ns":(core[7]>0).then_some(core[9]),
                                    "peak_source_debt":core[10],"peak_due_unconsumed":core[11],
                                    "source_debt_at_failure":core[12],"native_outstanding_at_failure":core[13],
                                    "max_source_lag_ns":core[14],"last_publication_elapsed_ns":core[15],
                                    "samples_over_window":core[16],"all_offered_quantiles_available":false,"planned_corpus_latency_p99_lower_bound_ns":p99_lower,"planned_corpus_latency_p999_lower_bound_ns":p999_lower,
                                    "unoffered_future":core[0]-core[3],"due_unconsumed":core[3]-core[5],
                                    "rejected":0,"terminal_errors":1,"request_deadline_seconds":5,"whole_stage_target_seconds":3}))
                            })();
                            match terminal {
                                Ok(value) => println!("{value}"),
                                Err(capture) => println!("{}",json!({"event":"gerbil_actor_failure_capture_error",
                                    "original_error":original.to_string(),"capture_error":capture.to_string()})),
                            }
                            return Err(original);
                        }
                    };

                    if anchored {
                        native_clock_anchor(session, warmup, "after")?;
                    }
                    println!("{}", json!({"event":"native_call_cpu_gc_envelope","warmup":warmup,"values":cpu_gc_envelope}));
                    let receipt = copy_arrival_receipt(session, &result, path, warmup)?;
                    Ok((receipt, clock().saturating_sub(call_started)))
                },
            )
        })
        .unwrap()
        .wait(Duration::from_secs(seconds) + GATE)
        .unwrap();
    drop(watchdog);
    let (core, debt) = receipt.split_at(8);
    let [
        planned,
        completed,
        rejected,
        elapsed,
        offered_elapsed,
        p99,
        p999,
        max_lag,
    ] = core
    else {
        panic!("native arrival receipt")
    };
    assert_eq!(*planned, i64::try_from(rate * seconds).unwrap());
    assert_eq!(completed + rejected, *planned);
    let source = if matches!(
        path,
        NativePath::Backpressure
            | NativePath::BackpressureAccess
            | NativePath::BackpressureObserved
            | NativePath::BackpressureOwnerWatch
            | NativePath::BackpressureOwnerUrgent
            | NativePath::BackpressureUrgent
            | NativePath::BackpressurePark
            | NativePath::BackpressureCoalesced
            | NativePath::BackpressureSchedule
    ) {
        let [source_debt, outstanding, last_publication, over_window] = debt else {
            panic!("native source-debt receipt")
        };
        assert_eq!(*rejected, 0);
        json!({"maximum_pending_source_messages":source_debt,
            "maximum_due_unconsumed_messages":outstanding,
            "last_publication_elapsed_ns":last_publication,
            "samples_over_original_window":over_window,
            "original_window_guard_passed":*outstanding<=128,
            "drain_after_schedule_ns":elapsed-offered_elapsed})
    } else {
        assert!(debt.is_empty());
        serde_json::Value::Null
    };
    let sample = MailboxSample {
        rate,
        seconds,
        warmup,
        observed: scope_diagnostic(path)
            || matches!(
                path,
                NativePath::BackpressureObserved
                    | NativePath::BackpressureOwnerWatch
                    | NativePath::BackpressureOwnerUrgent
            )
            || std::env::args()
                .any(|arg| matches!(arg.as_str(), "--trace-gate" | "--native-clock-anchor")),
        planned: *planned,
        completed: *completed,
        rejected: *rejected,
        p99_ns: *p99,
        maximum_due_unconsumed: source
            .get("maximum_due_unconsumed_messages")
            .and_then(serde_json::Value::as_i64),
    };
    let admission = if capacity_research() && !warmup {
        sample.capacity_research_failures(whole_call_ns)
    } else {
        sample.failures()
    };
    let passed = admission.is_empty();
    let mut value = json!({"event":"gerbil_actor_arrival","mailbox":path!=NativePath::Service,"ready_drain_pipeline":matches!(path,NativePath::Pipeline|NativePath::Direct),"diagnostic_no_actor_handoff":path==NativePath::Direct,"native_data_access":matches!(path,NativePath::Access|NativePath::BackpressureAccess|NativePath::BackpressureObserved|NativePath::BackpressureOwnerWatch|NativePath::BackpressureOwnerUrgent|NativePath::BackpressureUrgent|NativePath::BackpressurePark|NativePath::BackpressureCoalesced|NativePath::BackpressureSchedule),"data_access_edge_contract":matches!(path,NativePath::Access|NativePath::BackpressureAccess|NativePath::BackpressureObserved|NativePath::BackpressureOwnerWatch|NativePath::BackpressureOwnerUrgent|NativePath::BackpressureUrgent|NativePath::BackpressurePark|NativePath::BackpressureCoalesced|NativePath::BackpressureSchedule),"diagnostic_native_observation":path==NativePath::BackpressureObserved,"diagnostic_owner_watch":matches!(path,NativePath::BackpressureOwnerWatch|NativePath::BackpressureOwnerUrgent),"scoped_user_initiated_qos":matches!(path,NativePath::BackpressureOwnerUrgent|NativePath::BackpressureUrgent),"native_source_park":path==NativePath::BackpressurePark,"warmup":warmup,"rate":rate,"seconds":seconds,
        "owned_source_schedule":path==NativePath::BackpressureSchedule,"native_coalesced_wake":path==NativePath::BackpressureCoalesced,"performance_admission":{"passed":passed,"failures":admission,"scope":"native_500k_original_guard"},
        "planned":planned,"completed":completed,"accepted":completed,"rejected":rejected,"failed":0,"accepted_timeouts":0,"remaining_staged":0,"remaining_native":0,"source_backpressure":source,"snapshot_gc_duplicate_and_unread_close_contract":matches!(path,NativePath::Pipeline|NativePath::Direct),
        "completed_per_second":f64::from(u32::try_from(*completed).unwrap())/Duration::from_nanos(u64::try_from(*elapsed).unwrap()).as_secs_f64(),"rejection_percent":100.0*f64::from(u32::try_from(*rejected).unwrap())/f64::from(u32::try_from(*planned).unwrap()),
        "elapsed_ns":elapsed,"offered_elapsed_ns":offered_elapsed,"successful_planned_to_consumed_p99_ns":p99,
        "successful_planned_to_consumed_p999_ns":p999,"latency_bucket_upper_bound_ns":1000,"all_arrival_max_generator_lag_ns":max_lag,
        "owners":1,"scheme_workers":if path==NativePath::Direct {0}else{4},"native_active":3,"window":128,"rust_per_message_entries":0,"watchdog_seconds":5,"defaults_changed":false});
    value["whole_native_call_ns"] = json!(whole_call_ns);
    if capacity_research() {
        value["performance_admission"]["scope"] = json!("capacity_1m_three_second_goal");
    }
    println!("{value}");
    if capacity_research() {
        println!(
            "{}",
            json!({"event":"capacity_research_acceptance","warmup":warmup,"rate":rate,"planned":planned,"whole_stage_target_seconds":3,"passed":passed})
        );
    }
    passed
}
fn option(name: &str, default: u64) -> u64 {
    std::env::args()
        .find_map(|s| s.strip_prefix(name).map(|v| v.parse().unwrap()))
        .unwrap_or(default)
}

fn trace_gate() {
    if std::env::args().any(|arg| arg == "--trace-gate") {
        println!(
            "{}",
            json!({"event":"native_trace_ready","pid":std::process::id()})
        );
        let mut ready = String::new();
        std::io::stdin().read_line(&mut ready).unwrap();
    }
}

fn native_clock_anchor(
    session: &NativeSession<'_>,
    warmup: bool,
    edge: &str,
) -> Result<(), InstanceError> {
    let mark = session.numeric_vector::<i64>(&[14, 0])?;
    session.call(&mark)?;
    let read = session.numeric_vector::<i64>(&[13, 0])?;
    let anchor = session.copy_numeric::<i64>(&session.call(&read)?)?;
    let [before, after, owner] = anchor.as_slice() else {
        panic!("native clock anchor type");
    };
    assert!(before <= after);
    println!(
        "{}",
        json!({"event":"native_clock_anchor","warmup":warmup,"edge":edge,
        "before_wall_ns":before,"after_wall_ns":after,"owner_thread_id":owner})
    );
    Ok(())
}
fn start_pool() -> InstancePool {
    let image = std::env::var("GERBIL_SERVICE_BENCH_IMAGE")
        .expect("GERBIL_SERVICE_BENCH_IMAGE must name the prebuilt audited image");
    // SAFETY: qualification image exports the audited service reference ABI.
    let program = unsafe {
        IsolatedProgram::from_image(image, c"gerbil_owner_orders_call")
            .unwrap()
            .with_exports(&[
                c"gerbil_owner_service_reference",
                c"gerbil_owner_mailbox_terminal_take",
                c"gerbil_owner_mailbox_terminal_contract",
                c"gerbil_owner_native_call",
            ])
            .with_scheme_service(c"gerbil_owner_service_call")
    };
    let pool = InstancePool::start(
        &program,
        InstanceConfig {
            owners: 1,
            queue_capacity: ARRIVAL_WINDOW,
            maximum_request_bytes: 16 * 1024 * 1024,
            maximum_reply_bytes: 16 * 1024 * 1024,
            startup_timeout: GATE,
        },
    )
    .unwrap();
    pool.dispatcher()
        .configure_owner_budgets(4, Some(32))
        .unwrap();
    pool
}

fn qos_scope_contract(pool: &InstancePool) {
    // A failing work result must still restore the actual owner class.
    pool.dispatcher()
        .submit_native_on(0, |session| {
            let result: Result<(), _> = with_owner_qos(session, true, || {
                Err(InstanceError::InvalidConfig(
                    "intentional QoS scope failure",
                ))
            });
            assert!(
                matches!(
                    result,
                    Err(InstanceError::InvalidConfig(
                        "intentional QoS scope failure"
                    ))
                ),
                "QoS contract outcome: {result:?}"
            );
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    println!(
        "{}",
        json!({"event":"native_owner_qos_contract","error_restore_contract":true})
    );
}

fn qos_candidate(pool: &InstancePool, rate: u64, seconds: u64, native: bool) {
    let candidate = if native {
        NativePath::BackpressureUrgent
    } else {
        NativePath::BackpressureOwnerUrgent
    };
    run(pool, 1000, 1, true, candidate);
    run(pool, rate, seconds, false, candidate);
    println!(
        "{}",
        json!({"event":"native_owner_qos_scope","error_restore_contract":true,"restored_after_run":true,"defaults_changed":false})
    );
}

fn check_strict_scope(path: NativePath, rate: u64, seconds: u64) {
    if capacity_research() {
        assert!(
            path == NativePath::BackpressureSchedule
                && !scope_diagnostic(path)
                && rate == 1_000_000
                && seconds == 1
        );
        return;
    }
    if std::env::args().any(|arg| arg == "--require-native-acceptance") {
        assert!(
            !scope_diagnostic(path)
                && matches!(
                    path,
                    NativePath::BackpressureAccess
                        | NativePath::BackpressureCoalesced
                        | NativePath::BackpressureSchedule
                )
                && rate == 500_000
                && seconds == 10
                && !std::env::args().any(|arg| arg.ends_with("-compare")
                    || matches!(arg.as_str(), "--trace-gate" | "--native-clock-anchor")),
            "strict admission requires one unobserved native mailbox arm at 500k/s for ten seconds"
        );
    }
}

fn finish(pool: &mut InstancePool, admitted: bool) {
    let report = pool.shutdown(GATE);
    assert!(report.is_clean());
    println!("{}", json!({"event":"gerbil_actor_cleanup","clean":true}));
    if std::env::args().any(|arg| arg == "--require-native-acceptance") {
        assert!(
            admitted,
            "native 500k performance admission failed; cleanup success is independent"
        );
    }
}

fn coalesced_pair(pool: &mut InstancePool, rate: u64, seconds: u64) -> bool {
    if !std::env::args().any(|arg| arg == "--backpressure-coalesced-compare") {
        return false;
    }
    run(pool, 1000, 1, true, NativePath::BackpressureAccess);
    run(pool, rate, seconds, false, NativePath::BackpressureAccess);
    run(pool, 1000, 1, true, NativePath::BackpressureCoalesced);
    let admitted = run(
        pool,
        rate,
        seconds,
        false,
        NativePath::BackpressureCoalesced,
    );
    finish(pool, admitted);
    true
}

fn ready_contract(pool: &InstancePool, path: NativePath) {
    if path != NativePath::BackpressureCoalesced
        && !std::env::args().any(|arg| arg == "--backpressure-coalesced-compare")
    {
        return;
    }
    pool.dispatcher()
        .submit_native_on(0, |session| {
            let header = session.numeric_vector::<i64>(&[61])?;
            let command = session.vector(&[&header])?;
            let result = session.call_export(c"gerbil_owner_service_reference", &command)?;
            assert_eq!(session.copy_numeric::<i64>(&result)?, [0]);
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    println!(
        "{}",
        json!({"event":"native_ready_contract","passed":true,"outside_measured_arms":true})
    );
}

fn source_plan_contract(pool: &InstancePool, path: NativePath) {
    if path != NativePath::BackpressureSchedule
        && !std::env::args().any(|arg| arg == "--backpressure-schedule-compare")
    {
        return;
    }
    pool.dispatcher()
        .submit_native_on(0, |session| {
            let header = session.numeric_vector::<i64>(&[63])?;
            let command = session.vector(&[&header])?;
            let result = session.call_export(c"gerbil_owner_service_reference", &command)?;
            assert_eq!(session.copy_numeric::<i64>(&result)?, [0]);
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    println!(
        "{}",
        json!({"event":"native_source_plan_contract","passed":true,"outside_measured_arms":true})
    );
}

fn source_plan_pair(pool: &mut InstancePool, rate: u64, seconds: u64) -> bool {
    if !std::env::args().any(|arg| arg == "--backpressure-schedule-compare") {
        return false;
    }
    run(pool, 1000, 1, true, NativePath::BackpressureAccess);
    run(pool, rate, seconds, false, NativePath::BackpressureAccess);
    run(pool, 1000, 1, true, NativePath::BackpressureSchedule);
    let admitted = run(pool, rate, seconds, false, NativePath::BackpressureSchedule);
    finish(pool, admitted);
    true
}

fn select_path() -> NativePath {
    let has = |flag| std::env::args().any(|arg| arg == flag);
    let qos_native_compare = has("--backpressure-native-qos-compare");
    let park_compare = has("--backpressure-park-compare");
    let qos_compare = qos_native_compare
        || has("--owner-qos-contract-only")
        || has("--backpressure-owner-qos-compare");
    let access_compare = has("--access-compare");
    let backpressure_compare = has("--backpressure-compare");
    let pipeline_compare = has("--pipeline-compare");
    if has("--owner-wait-scope") || has("--backpressure-owned-schedule") {
        NativePath::BackpressureSchedule
    } else if std::env::args().any(|s| s == "--backpressure-coalesced") {
        NativePath::BackpressureCoalesced
    } else if qos_native_compare
        || park_compare
        || has("--backpressure-coalesced-compare")
        || has("--backpressure-schedule-compare")
    {
        NativePath::BackpressureAccess
    } else if qos_compare || std::env::args().any(|s| s == "--backpressure-owner-watch") {
        NativePath::BackpressureOwnerWatch
    } else if std::env::args().any(|s| s == "--backpressure-observed") {
        NativePath::BackpressureObserved
    } else if std::env::args().any(|s| s == "--backpressure-native-access") {
        NativePath::BackpressureAccess
    } else if std::env::args().any(|s| s == "--backpressure") {
        NativePath::Backpressure
    } else if std::env::args().any(|s| s == "--native-access") {
        NativePath::Access
    } else if std::env::args().any(|s| s == "--direct-diagnostic") {
        NativePath::Direct
    } else if std::env::args().any(|s| s == "--pipeline") {
        NativePath::Pipeline
    } else if access_compare
        || backpressure_compare
        || pipeline_compare
        || std::env::args().any(|s| s == "--mailbox")
    {
        NativePath::Mailbox
    } else {
        NativePath::Service
    }
}

// Functional admission only: no offered-rate or latency result is produced.
fn native_host_contract(pool: &mut InstancePool) {
    fn call(pool: &InstancePool, words: &[i64]) -> Result<Vec<i64>, InstanceError> {
        let words = words.to_vec();
        pool.dispatcher()
            .submit_native_on(0, move |session| {
                let input = session.numeric_vector::<i64>(&words)?;
                let output = session.call_export(c"gerbil_owner_native_call", &input)?;
                session.copy_numeric::<i64>(&output)
            })
            .unwrap()
            .wait(GATE)
    }
    assert_eq!(call(pool, &[0, 3, 7]).unwrap(), vec![16, 28]);
    assert!(matches!(call(pool, &[1]), Err(InstanceError::Scheme(_))));
    assert_eq!(call(pool, &[5]).unwrap(), vec![1, 1]);
    for (operation, expected) in [(1, 23), (2, 20), (3, 22), (4, 31), (5, 13)] {
        assert_eq!(call(pool, &[2, operation, 13]).unwrap(), vec![expected]);
    }
    assert_eq!(call(pool, &[6, 0, 4]).unwrap(), vec![4]);
    let check_batch = |values: Vec<i64>| {
        assert_eq!(values.len(), 17);
        assert_eq!(&values[..16], &(16..32).collect::<Vec<i64>>());
        assert!((0..=4).contains(&values[16]));
    };
    check_batch(call(pool, &[6, 1, 10, 16, 3, 1, 4, -1]).unwrap());
    assert!(matches!(
        call(pool, &[6, 1, 10, 16, 3, 1, 4, 1]),
        Err(InstanceError::Scheme(_))
    ));
    check_batch(call(pool, &[6, 1, 10, 16, 3, 1, 4, -1]).unwrap());
    assert_eq!(call(pool, &[6, 2]).unwrap(), vec![4]);
    let shutdown = pool.shutdown(GATE);
    assert!(
        shutdown.is_clean(),
        "native host contract did not cleanly shut down: {shutdown:?}"
    );
    println!(
        "{}",
        json!({"event":"native_host_boundary_contract","passed":true,
        "gc_and_native_types":true,"exception_unwind":true,"multi_shot_continuation_across_ffi":true,
        "actor_failure_drain_and_reuse":true,"clean_shutdown":true,"performance_result":false})
    );
}

fn main() {
    trace_gate();
    let rate = option(
        "--rate=",
        if capacity_research() {
            1_000_000
        } else {
            500_000
        },
    );
    let seconds = option("--seconds=", if capacity_research() { 1 } else { 10 });
    assert!((1..=5_000_000).contains(&rate) && (1..=10).contains(&seconds));
    if capacity_research() {
        assert_eq!(seconds, 1);
    }
    let access_compare = std::env::args().any(|s| s == "--access-compare");
    let backpressure_compare = std::env::args().any(|s| s == "--backpressure-compare");
    let qos_contract_only = std::env::args().any(|s| s == "--owner-qos-contract-only");
    let qos_native_compare = std::env::args().any(|s| s == "--backpressure-native-qos-compare");
    let park_compare = std::env::args().any(|s| s == "--backpressure-park-compare");
    let qos_compare = qos_native_compare
        || qos_contract_only
        || std::env::args().any(|s| s == "--backpressure-owner-qos-compare");
    let pipeline_compare = std::env::args().any(|s| s == "--pipeline-compare");
    let service_compare = std::env::args().any(|s| s == "--compare");
    assert!(!(pipeline_compare && service_compare));
    let path = select_path();
    path.assert_rate(rate, backpressure_compare);
    check_strict_scope(path, rate, seconds);
    let mut pool = start_pool();
    pool.dispatcher()
        .submit_native_on(0, |session| {
            let command = session.vector(&[])?;
            let result =
                session.call_export(c"gerbil_owner_mailbox_terminal_contract", &command)?;
            assert_eq!(session.copy_numeric::<i64>(&result)?, vec![1]);
            assert_eq!(
                terminal_histogram::summarize(&[99, 0, 1]),
                Ok([100, 1000, 3000])
            );
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    println!(
        "{}",
        json!({"event":"native_terminal_receipt_contract","passed":true})
    );

    if std::env::args().any(|arg| arg == "--host-boundary-contract-only") {
        native_host_contract(&mut pool);
        return;
    }
    ready_contract(&pool, path);
    source_plan_contract(&pool, path);
    if source_plan_pair(&mut pool, rate, seconds) {
        return;
    }
    if coalesced_pair(&mut pool, rate, seconds) {
        return;
    }
    if qos_compare {
        qos_scope_contract(&pool);
    }
    if qos_contract_only {
        assert!(pool.shutdown(GATE).is_clean());
        println!(
            "{}",
            json!({"event":"gerbil_actor_cleanup","clean":true,"contract_only":true})
        );
        return;
    }
    run(&pool, 1000, 1, true, path);
    let admitted = run(&pool, rate, seconds, false, path);
    if park_compare {
        run(&pool, 1000, 1, true, NativePath::BackpressurePark);
        run(&pool, rate, seconds, false, NativePath::BackpressurePark);
    }
    if qos_compare {
        qos_candidate(&pool, rate, seconds, qos_native_compare);
    }
    if service_compare {
        assert!(
            path == NativePath::Service,
            "comparison starts with service control"
        );
        run(&pool, 1000, 1, true, NativePath::Mailbox);
        run(&pool, rate, seconds, false, NativePath::Mailbox);
    }
    if pipeline_compare {
        assert!(
            path == NativePath::Mailbox,
            "pipeline comparison starts with original mailbox"
        );
        run(&pool, 1000, 1, true, NativePath::Pipeline);
        run(&pool, rate, seconds, false, NativePath::Pipeline);
    }
    if access_compare {
        assert!(
            path == NativePath::Mailbox,
            "data access comparison starts with original mailbox"
        );
        run(&pool, 1000, 1, true, NativePath::Access);
        run(&pool, rate, seconds, false, NativePath::Access);
    }
    if backpressure_compare {
        assert!(
            path == NativePath::Mailbox,
            "backpressure comparison starts with original mailbox"
        );
        run(&pool, 1000, 1, true, NativePath::Backpressure);
        run(&pool, rate, seconds, false, NativePath::Backpressure);
    }
    finish(&mut pool, admitted);
}
