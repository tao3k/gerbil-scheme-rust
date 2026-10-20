//! Persistent native roots must remain owner and generation checked.
use super::{GATE, Progress, config, finish, normal, observed};
use gerbil_scheme::instances::{InstanceDispatcher, InstanceError, InstancePool};
use std::sync::mpsc;
pub(super) fn run(progress: &Progress) {
    let mut pool = InstancePool::start(&normal(), config(2)).unwrap();
    let dispatch = pool.dispatcher();
    let value = dispatch
        .submit_native_on(0, |session| session.retain(session.string("retained λ")?))
        .unwrap()
        .wait(GATE)
        .unwrap();
    let copied = dispatch
        .submit_native_on(0, move |session| {
            session.collect()?;
            session.copy_string(&session.retained(value)?)
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(copied, "retained λ");
    assert!(
        dispatch
            .submit_native_on(1, move |session| {
                session.retained(value)?;
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .is_err()
    );
    observed(
        progress,
        "observed retained root survives GC and rejects foreign owner",
    );
    dispatch
        .submit_native_on(0, move |session| session.release_retained(value))
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert!(
        dispatch
            .submit_native_on(0, move |session| {
                session.retained(value)?;
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .is_err()
    );
    abandoned(&dispatch, progress);
    let stale = dispatch
        .submit_native_on(0, |session| {
            session.retain(session.string("old generation")?)
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    finish(&mut pool, progress);
    let mut replacement = InstancePool::start(&normal(), config(1)).unwrap();
    assert!(
        replacement
            .dispatcher()
            .submit_native_on(0, move |session| {
                session.retained(stale)?;
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .is_err()
    );
    finish(&mut replacement, progress);
    observed(
        progress,
        "observed shutdown retires retained arena and rejects stale generation",
    );
    println!(
        "RETAINED-PASS owner-affinity=checked gc=checked failed-task=rolled-back abandoned-queued-reply=rolled-back stale-generation=rejected"
    );
}

fn abandoned(dispatch: &InstanceDispatcher, progress: &Progress) {
    let failed = dispatch
        .submit_native_on::<(), _>(0, |session| {
            session.retain(session.string("failed task")?)?;
            Err(InstanceError::TaskPanicked)
        })
        .unwrap()
        .wait(GATE);
    assert!(failed.is_err());
    let (entered, ready) = mpsc::channel();
    let (unblock, blocked) = mpsc::channel();
    let barrier = dispatch
        .submit_native_on(0, move |_| {
            entered.send(()).unwrap();
            blocked.recv_timeout(GATE).unwrap();
            Ok(())
        })
        .unwrap();
    ready.recv_timeout(GATE).unwrap();
    let abandoned = dispatch
        .submit_native_on(0, |session| {
            session.retain(session.string("abandoned reply")?)
        })
        .unwrap();
    drop(abandoned);
    unblock.send(()).unwrap();
    barrier.wait(GATE).unwrap();
    let roots = dispatch
        .submit_native_on(0, |session| session.usage().map(|usage| usage[3].to_bits()))
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(roots, 0.0_f64.to_bits());
    observed(
        progress,
        "observed failed and abandoned queued replies roll back new roots",
    );
}
