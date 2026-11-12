use super::{GATE, Progress, config, drained, observed, program};
use gerbil_scheme::{
    GerbilRuntime, NativeError,
    instances::{InstanceError, InstancePool, IsolatedProgram},
};
use serde_json::{Value, json};
use std::{
    thread,
    time::{Duration, Instant},
};

fn call(pool: &InstancePool, owner: usize, request: &Value, progress: &Progress) -> Value {
    let reply = pool
        .submit_on(owner, serde_json::to_vec(request).expect("request"))
        .expect("admission")
        .wait(GATE)
        .expect("Scheme response");
    assert_eq!(reply.owner, owner);
    observed(progress, "observed Scheme contract result");
    serde_json::from_slice(&reply.bytes).expect("Scheme JSON")
}
fn continuations(progress: &Progress) {
    let mut first = InstancePool::start(&program(), config(1)).expect("first pool");
    observed(progress, "observed first instance initialized");
    let mut second = InstancePool::start(&program(), config(1)).expect("second pool");
    observed(progress, "observed second instance initialized");
    assert_ne!(first.owners()[0].state, second.owners()[0].state);
    assert!(matches!(
        GerbilRuntime::initialize(),
        Err(NativeError::AlreadyInitialized)
    ));
    assert_eq!(call(&first, 0, &json!(["capture", 101]), progress), 23);
    assert_eq!(call(&second, 0, &json!(["capture", 202]), progress), 23);
    assert_eq!(call(&first, 0, &json!(["resume", 2, 101]), progress), 108);
    assert_eq!(call(&first, 0, &json!(["resume", 3, 101]), progress), 110);
    assert_eq!(call(&first, 0, &json!(["resume", 4, 101]), progress), 31);
    drained(&mut first, progress);
    assert!(matches!(first.submit(vec![]), Err(InstanceError::Closed)));
    assert_eq!(call(&second, 0, &json!(["resume", 2, 202]), progress), 209);
    assert_eq!(call(&second, 0, &json!(["resume", 3, 202]), progress), 211);
    assert_eq!(call(&second, 0, &json!(["resume", 4, 202]), progress), 31);
    let error = second
        .submit(b"not JSON".to_vec())
        .expect("admit rejection")
        .wait(GATE);
    assert!(matches!(
        error,
        Err(InstanceError::Native {
            operation: "Scheme export",
            code: 0
        })
    ));
    assert_eq!(
        call(&second, 0, &json!(["echo", "λ🧪\0"]), progress),
        "λ🧪\0"
    );
    drained(&mut second, progress);
    // Native cleanup must also reclaim globals holding an unreleased Scheme
    // continuation. This is distinct from a leaked external strong-root token.
    let mut retained =
        InstancePool::start(&program(), config(1)).expect("retained continuation pool");
    assert_eq!(call(&retained, 0, &json!(["capture", 303]), progress), 23);
    drained(&mut retained, progress);
    eprintln!(
        "LIBRARY-CONTRACT continuations=PASS gc=PASS dynamic-wind=PASS dynamic-binding=PASS thread-identity=PASS survivor=PASS cleanup-with-saved-continuation=PASS"
    );
}
fn deadlines(progress: &Progress) {
    let mut limits = config(1);
    limits.queue_capacity = 1;
    limits.maximum_request_bytes = 256;
    limits.maximum_reply_bytes = 16;
    let mut pool = InstancePool::start(&program(), limits).expect("bounded pool");
    observed(progress, "observed bounded instance initialized");
    assert!(matches!(
        pool.submit(vec![0; 257]),
        Err(InstanceError::InvalidConfig(_))
    ));
    assert!(matches!(
        pool.submit_on(1, vec![]),
        Err(InstanceError::Closed)
    ));
    assert!(matches!(
        InstancePool::start(&program(), config(0)),
        Err(InstanceError::InvalidConfig(_))
    ));
    let delayed = pool
        .submit(serde_json::to_vec(&json!(["delay", 0.15, 17])).unwrap())
        .expect("delayed request");
    let start = Instant::now();
    while !delayed.has_started() {
        assert!(start.elapsed() < GATE);
        thread::yield_now();
    }
    let queued = pool
        .submit(b"[\"echo\",18]".to_vec())
        .expect("one queued request");
    assert!(matches!(pool.submit(vec![]), Err(InstanceError::QueueFull)));
    assert_eq!(
        delayed.wait(Duration::from_millis(1)).unwrap_err(),
        InstanceError::Timeout
    );
    let pending = pool.shutdown(Duration::from_millis(1));
    assert_eq!(pending.pending, 1);
    assert_eq!(pending.joined, 0);
    assert!(pending.owners.is_empty());
    assert!(matches!(pool.submit(vec![]), Err(InstanceError::Closed)));
    assert_eq!(delayed.wait(GATE).unwrap().bytes, b"17");
    assert_eq!(queued.wait(GATE).unwrap().bytes, b"18");
    drained(&mut pool, progress);
    let mut pool = InstancePool::start(&program(), limits).expect("reply bound pool");
    let oversized = pool
        .submit(serde_json::to_vec(&json!(["echo", "x".repeat(128)])).unwrap())
        .unwrap()
        .wait(GATE);
    assert!(matches!(oversized, Err(InstanceError::InvalidConfig(_))));
    let abandoned = pool.submit(b"[\"delay\",0.02,19]".to_vec()).unwrap();
    let start = Instant::now();
    while !abandoned.has_started() {
        assert!(start.elapsed() < GATE);
        thread::yield_now();
    }
    drop(abandoned);
    assert_eq!(call(&pool, 0, &json!(["echo", 20]), progress), 20);
    drained(&mut pool, progress);
    // A missing export is rejected before setup. It never becomes a live VM.
    // SAFETY: the image is trusted; the absent application entry is never called.
    let missing = unsafe {
        IsolatedProgram::from_image(
            env!("GERBIL_OWNER_INSTANCE_IMAGE"),
            c"absent_application_export",
        )
    }
    .unwrap();
    assert!(matches!(
        InstancePool::start(&missing, config(1)),
        Err(InstanceError::Startup { cause, shutdown, .. })
            if matches!(*cause, InstanceError::Loader(_)) && shutdown.pending == 0
    ));
    observed(progress, "observed native admission negative controls");
    eprintln!(
        "LIBRARY-CONTRACT queue-full=PASS timeout-not-cancellation=PASS drain-pending=PASS rejected-call-root-balance=PASS oversized-reply=PASS abandoned-ticket=PASS missing-export-before-setup=PASS mixed-backend-rejected=PASS"
    );
}
fn concurrent_callers(progress: &Progress) {
    fn require_send_sync<T: Send + Sync>() {}
    require_send_sync::<gerbil_scheme::instances::InstanceDispatcher>();
    let mut pool = InstancePool::start(&program(), config(4)).expect("concurrent caller pool");
    observed(progress, "observed concurrent caller owners initialized");
    let dispatcher = pool.dispatcher();
    let mut callers = Vec::new();
    for caller in 0..4 {
        let dispatcher = dispatcher.clone();
        let progress = progress.clone();
        callers.push(thread::spawn(move || {
            for item in 0..8 {
                let value = json!([caller, item, "λ🧪"]);
                let reply = dispatcher
                    .submit(serde_json::to_vec(&json!(["echo", value])).unwrap())
                    .expect("concurrent admission")
                    .wait(GATE)
                    .expect("concurrent Scheme result");
                assert_eq!(
                    serde_json::from_slice::<Value>(&reply.bytes).unwrap(),
                    value
                );
                observed(&progress, "observed concurrent caller reply");
            }
        }));
    }
    for caller in callers {
        caller.join().expect("concurrent Rust caller joined");
    }
    drained(&mut pool, progress);
    assert!(matches!(
        dispatcher.submit(vec![]),
        Err(InstanceError::Closed)
    ));
    eprintln!("LIBRARY-CONTRACT rust-callers=4 dispatcher-send-sync=PASS close-all-clones=PASS");
}

pub(super) fn run(progress: &Progress) {
    continuations(progress);
    deadlines(progress);
    concurrent_callers(progress);
}
