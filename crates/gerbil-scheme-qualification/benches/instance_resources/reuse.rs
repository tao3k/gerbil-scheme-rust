use super::{
    GATE, Progress, bounded_integer, checkpoint, command, config, finish, normal,
    observe::Observer, observed,
};
use gerbil_scheme::instances::{InstanceDispatcher, InstanceError, InstancePool, NativeSession};
use std::sync::atomic::{AtomicUsize, Ordering};
static PANICS: AtomicUsize = AtomicUsize::new(0);
fn live(values: &[[f64; 4]]) -> f64 {
    values.iter().map(|value| value[2]).sum()
}
pub(super) fn run(smoke: bool, observer: &Observer, progress: &Progress) {
    std::panic::set_hook(Box::new(|_| {
        PANICS.fetch_add(1, Ordering::Relaxed);
    }));
    let process_before = observer.emit("before-pool", 0, &[]);
    let mut pool = InstancePool::start(&normal(), config(4)).unwrap();
    observed(progress, "observed reusable owners initialized");
    super::contracts::types(&pool, progress);
    let dispatch = pool.dispatcher();
    continuations(&dispatch, progress);
    let epochs = if smoke { 6 } else { 32 };
    let jobs = if smoke { 128 } else { 2048 };
    let mut baseline = [0; 8];
    let mut base_live = 0.0;
    let mut successes = 0;
    let mut scheme_by_owner = [0; 4];
    let mut panic_by_owner = [0; 4];
    let mut scheme_errors = 0;
    let mut rust_errors = 0;
    let mut tail_live = Vec::new();
    let mut tail_rss = Vec::new();
    for epoch in 0..epochs {
        for start in (0..jobs).step_by(64) {
            let mut tickets = Vec::new();
            for index in start..(start + 64).min(jobs) {
                let owner = (index + index / 128) % 4;
                tickets.push((
                    index,
                    owner,
                    dispatch
                        .submit_native_on(owner, move |session| reuse_task(session, index, epoch))
                        .unwrap(),
                ));
            }
            for (index, owner, ticket) in tickets {
                match ticket.wait(GATE) {
                    Err(InstanceError::TaskPanicked) => {
                        assert_eq!(index % 128, 0);
                        rust_errors += 1;
                        panic_by_owner[owner] += 1;
                    }
                    Err(InstanceError::Native {
                        operation: "native root",
                        code: 0,
                    }) => {
                        assert_eq!(index % 16, 0);
                        scheme_errors += 1;
                        scheme_by_owner[owner] += 1;
                    }
                    Ok(value) => {
                        let size = if epoch < 2 { 8192 } else { 128 };
                        assert_eq!(value.len(), size - 1);
                        assert!(
                            value
                                .iter()
                                .all(|value| *value == bounded_integer(index) * 3 + 7)
                        );
                        successes += 1;
                    }
                    other => panic!("unexpected native task result: {other:?}"),
                }
                observed(progress, "observed completed native reuse task");
            }
        }
        let usage = checkpoint(&pool, progress);
        let resource = observer.emit("reuse", epoch, &usage);
        if epoch == 3 {
            baseline = resource;
            base_live = live(&usage);
        }
        if epoch >= 4 {
            plateau(resource, baseline, &usage, base_live);
            tail_live.push(live(&usage));
            tail_rss.push(resource[0]);
        }
    }
    assert_eq!(PANICS.load(Ordering::Relaxed), rust_errors);
    if !smoke {
        assert_eq!(scheme_by_owner, [896; 4]);
        assert_eq!(panic_by_owner, [128; 4]);
    }
    eprintln!(
        "RESOURCE-FAILURE-COVERAGE scheme-by-owner={scheme_by_owner:?} panic-by-owner={panic_by_owner:?}"
    );
    let (growth, released) = retention(&pool, observer, progress);
    finish(&mut pool, progress);
    let after = observer.settled("after-pool", 0, process_before[3]);
    assert_eq!(after[2], process_before[2]);
    assert_eq!(after[3], process_before[3]);
    assert_eq!(after[4], process_before[4] + 4);
    let relieved = observer.relieve();
    observer.emit("after-allocator-relief", 0, &[]);
    eprintln!("RESOURCE-ALLOCATOR released-by-pressure={relieved} default-policy-unchanged=true");
    eprintln!(
        "RESOURCE-REUSE epochs={epochs} jobs-per-epoch={jobs} success={successes} scheme-errors={scheme_errors} rust-panics={rust_errors} positive-live-growth={growth} cleared-live={released} tail-live={tail_live:?} tail-rss={tail_rss:?} retained-images=4 fd-thread-return=true json=false"
    );
}
pub(super) fn churn(observer: &Observer, progress: &Progress) {
    let baseline = observer.emit("churn-baseline", 0, &[]);
    let mut files = 0;
    let mut bytes = 0;
    for epoch in 0..8 {
        let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
        observed(progress, "observed replacement pool initialized");
        assert_eq!(
            command(&pool.dispatcher(), 0, vec![0, 3, 7], progress),
            [16, 28]
        );
        assert_eq!(
            command(&pool.dispatcher(), 0, vec![3, 32 * 1024 * 1024], progress),
            [1]
        );
        let usage = checkpoint(&pool, progress);
        observer.emit("churn-retained", epoch, &usage);
        assert_eq!(command(&pool.dispatcher(), 0, vec![4], progress), [0]);
        finish(&mut pool, progress);
        if let Ok(entries) = std::fs::read_dir(&pool.image_directory) {
            for entry in entries {
                files += 1;
                bytes += entry.unwrap().metadata().unwrap().len();
            }
        }
        assert!(!pool.image_directory.exists());
        let current = observer.settled("churn", epoch, baseline[3]);
        assert_eq!(current[2], baseline[2]);
        assert_eq!(current[3], baseline[3]);
        assert_eq!(current[4], baseline[4] + epoch as u64 + 1);
    }
    eprintln!(
        "RESOURCE-CHURN pools=8 native-owners-joined=8 retained-images=8 retained-files={files} retained-file-bytes={bytes} hot-unload=NOT-ADMITTED"
    );
}

fn continuations(dispatch: &InstanceDispatcher, progress: &Progress) {
    // All continuation steps use native s64vectors, no application codec.
    for owner in 0..4 {
        assert_eq!(
            command(
                dispatch,
                owner,
                vec![2, 1, 100 + bounded_integer(owner)],
                progress
            ),
            [23]
        );
    }
    for operation in [2, 3, 4] {
        for owner in 0..4 {
            let expected = match operation {
                2 => 107 + bounded_integer(owner),
                3 => 109 + bounded_integer(owner),
                _ => 31,
            };
            assert_eq!(
                command(
                    dispatch,
                    owner,
                    vec![2, operation, 100 + bounded_integer(owner)],
                    progress
                ),
                [expected]
            );
        }
    }
}
fn retention(pool: &InstancePool, observer: &Observer, progress: &Progress) -> (f64, f64) {
    let dispatch = pool.dispatcher();
    // A deliberate retained-object control must defeat zero-root-only admission.
    let before = checkpoint(pool, progress);
    let retained_bytes = 8 * 1024 * 1024_u32;
    for epoch in 0..4 {
        for owner in 0..4 {
            assert_eq!(
                command(
                    &dispatch,
                    owner,
                    vec![3, i64::from(retained_bytes)],
                    progress
                ),
                [epoch + 1]
            );
        }
        let usage = checkpoint(pool, progress);
        observer.emit(
            "retention-positive",
            usize::try_from(epoch).unwrap(),
            &usage,
        );
    }
    let retained = checkpoint(pool, progress);
    let growth = live(&retained) - live(&before);
    assert!(
        growth >= f64::from(retained_bytes) * 16.0 * 0.95,
        "resource metric must detect retained Scheme heap"
    );
    for owner in 0..4 {
        assert_eq!(command(&dispatch, owner, vec![4], progress), [0]);
    }
    let cleared = checkpoint(pool, progress);
    let released = live(&retained) - live(&cleared);
    assert!(released >= f64::from(retained_bytes) * 16.0 * 0.95);
    observer.emit("retention-cleared", 0, &cleared);
    for owner in 0..4 {
        let counts = command(&dispatch, owner, vec![5], progress);
        assert_eq!(counts[0], counts[1]);
    }
    (growth, released)
}

fn reuse_task(
    session: &NativeSession<'_>,
    index: usize,
    epoch: usize,
) -> Result<Vec<i64>, InstanceError> {
    if index.is_multiple_of(128) {
        let _root = session.numeric_vector(&[-7_i64, 1, 2])?;
        panic!("injected Rust projection panic");
    }
    if index.is_multiple_of(16) {
        let root = session.numeric_vector(&[1_i64])?;
        let _ = session.call(&root)?;
        return Ok(Vec::new());
    }
    let size = if epoch < 2 { 8192 } else { 128 };
    let mut values = vec![bounded_integer(index); size];
    values[0] = 0;
    let input = session.numeric_vector(&values)?;
    let result = session.call(&input)?;
    session.copy_numeric::<i64>(&result)
}

fn plateau(resource: [u64; 8], baseline: [u64; 8], usage: &[[f64; 4]], base_live: f64) {
    assert_eq!(
        resource[2..5],
        baseline[2..5],
        "fixed pool must retain fixed fd/thread/image counts"
    );
    assert!(
        live(usage) <= base_live + 1024.0 * 1024.0,
        "post-GC live growth"
    );
    assert!(
        resource[0] <= baseline[0] + 32 * 1024 * 1024,
        "resident growth above warm plateau"
    );
}
