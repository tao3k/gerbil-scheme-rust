//! Real typed API, bounded in-flight work, competition and retained Scheme plans.
use super::*;
use gerbil_scheme::instances::SchemeWorkExecutionPolicy;
fn configure(d: &InstanceDispatcher, q: usize, m: usize) {
    let p = SchemeWorkExecutionPolicy {
        contended_work: q,
        maximum_uninterrupted_work: m,
    };
    assert_eq!(
        d.configure_scheme_service_actor_turn_on(0, p)
            .unwrap()
            .wait(GATE)
            .unwrap(),
        p
    );
}
pub(super) fn contract(
    rt: &tokio::runtime::Runtime,
    d: &InstanceDispatcher,
    progress: &mpsc::Sender<&'static str>,
) {
    for (q, m) in [(0, 48), (4, 3), (257, 257), (4, 65537)] {
        assert!(
            d.configure_scheme_service_work_plan_on(
                0,
                SchemeWorkExecutionPolicy {
                    contended_work: q,
                    maximum_uninterrupted_work: m
                }
            )
            .is_err()
        );
    }
    d.configure_scheme_service_budget_on(0, 1)
        .unwrap()
        .wait(GATE)
        .unwrap();
    specialized_api(rt, d);
    for mode in 0..12 {
        reclamation_api(rt, d, mode);
    }
    configure(d, 4, 48);
    rt.block_on(async {
        // A control reply can precede its capacity retirement. Compare two
        // actual idle boundaries, not a transient post-reply snapshot.
        idle(d).await;
        let cap = d.owner_capacity()[0].work_available;
        let out = d
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        assert_eq!(out.len(), 8);
        idle(d).await;
        assert_eq!(d.owner_capacity()[0].work_available, cap);
    });
    turn_receipt(d);
    let w = d.scheme_service_work_on(0).unwrap().wait(GATE).unwrap();
    assert_eq!((w.bulk_work, w.short_work, w.handoffs), (384, 0, 0));
    let p = d
        .scheme_service_work_plan_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(
        (
            p.contended_work,
            p.maximum_uninterrupted_work,
            p.whole_plans,
            p.sliced_plans,
            p.maximum_selected_work
        ),
        (4, 48, 8, 0, 48)
    );
    assert_eq!(
        d.scheme_service_lease_plan_on(0)
            .unwrap()
            .wait(GATE)
            .unwrap(),
        gerbil_scheme::instances::SchemeLeasePlanObservation {
            enabled: true,
            short_span: 48,
            bulk_span: 48
        }
    );
    assert_eq!(command(d, 0, 27), [48, 4, 196, 0]);
    let control = d
        .submit_native_on(0, |session| {
            let held = session.numeric_vector(&[41_i64])?;
            assert_eq!(observe_command(session, 31)?, [48, 4, 108, 1]);
            assert_eq!(observe_command(session, 30)?, [48, 4, 108, 1]);
            let result = observe_command(session, 26)?;
            session.collect()?;
            assert_eq!(session.copy_numeric::<i64>(&held)?, [41]);
            drop(held);
            assert_eq!(
                session.usage()?[3].to_bits(),
                f64::from(session.scheme_service_native_entry_enabled()).to_bits()
            );
            Ok(result)
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(&control[..3], &[4, 4, 196]);
    assert_eq!(control[3], 1);
    assert_eq!(&control[4..7], &[0, 1, 1]);
    assert_eq!(&control[7..9], &[4, 48]);
    assert_eq!(control[9] + control[10], 5);
    assert!(control[10] >= 1);
    assert_eq!(control[11], 48);
    configure(d, 0, 0);
    d.configure_scheme_service_budget_on(0, 0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    progress.send("Scheme lexical plans: bounded48 in-flight work, competitive4, independent quotas, reentry and stale-policy fencing passed").unwrap();
}

fn turn_receipt(d: &InstanceDispatcher) {
    let turns = d
        .scheme_service_actor_turn_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert!(turns.enabled);
    assert_eq!(
        (
            turns.outer_grants,
            turns.quantum_yields,
            turns.budget_yields
        ),
        (4, 4, 0)
    );
}

fn specialized_api(rt: &tokio::runtime::Runtime, d: &InstanceDispatcher) {
    let policy = SchemeWorkExecutionPolicy {
        contended_work: 4,
        maximum_uninterrupted_work: 48,
    };
    d.configure_scheme_service_specialized_plan_on(0, policy)
        .unwrap()
        .wait(GATE)
        .unwrap();
    rt.block_on(async {
        idle(d).await;
        let cap = d.owner_capacity()[0].work_available;
        let out = d
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        assert_eq!(out.len(), 8);
        idle(d).await;
        assert_eq!(d.owner_capacity()[0].work_available, cap);
    });
    let work = d.scheme_service_work_on(0).unwrap().wait(GATE).unwrap();
    assert_eq!(
        (work.bulk_work, work.short_work, work.handoffs),
        (384, 0, 0)
    );
    let turns = d
        .scheme_service_actor_turn_on(0)
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert!(!turns.enabled);
}

fn reclamation_api(rt: &tokio::runtime::Runtime, d: &InstanceDispatcher, mode: u8) {
    configure_fixture_policy(d, mode);
    rt.block_on(async {
        idle(d).await;
        let cap = d.owner_capacity()[0].work_available;
        let out = d
            .submit_scheme_service_backpressured_on(0, batch(false), GATE)
            .await
            .unwrap()
            .wait_completed(GATE)
            .await
            .unwrap()
            .0;
        assert_eq!(out.len(), 8);
        // Both success and Scheme exception publish short readiness. Dropping
        // a caller ticket must not keep the native grant fence closed.
        let short = d
            .submit_scheme_service_backpressured_on(0, short::Short(vec![500, 0, 1, 1, 1]), GATE)
            .await
            .unwrap();
        assert_eq!(short.wait_completed(GATE).await.unwrap().0.len(), 1);
        let failed = d
            .submit_scheme_service_backpressured_on(
                0,
                short::Short(corpus::order(501, false)),
                GATE,
            )
            .await
            .unwrap();
        assert!(matches!(
            failed.wait_completed(GATE).await,
            Err(InstanceError::Scheme(_))
        ));
        let abandoned = d
            .submit_scheme_service_backpressured_on(0, short::Short(vec![502, 0, 1, 1, 1]), GATE)
            .await
            .unwrap();
        drop(abandoned);
        idle(d).await;
        assert_eq!(d.owner_capacity()[0].work_available, cap);
    });
    let work = d.scheme_service_work_on(0).unwrap().wait(GATE).unwrap();
    assert_eq!(work.pending_start, 0);
    assert_eq!((work.ready_generation, work.offered_generation), (3, 3));
    if mode >= 2 {
        let e = d
            .scheme_service_execution_on(0)
            .unwrap()
            .wait(GATE)
            .unwrap();
        assert!(e.enabled);
        assert_eq!(
            (
                e.pending_execution,
                e.issued,
                e.runnable,
                e.registered,
                e.first_grants,
                e.finished_without_grant
            ),
            (0, 3, 3, 2, 2, 1)
        );
        assert_eq!(
            command(d, 0, registration_command(mode)),
            [1, 0, 4, 4, 3, 3, 1]
        );
        if mode >= 5 {
            let p = d
                .scheme_service_cycle_progress_on(0)
                .unwrap()
                .wait(GATE)
                .unwrap();
            assert!(p.enabled);
            assert_eq!(p.ready_first, mode >= 6);
            assert_eq!(p.progress_fair, mode == 7);
            assert_eq!((p.short_collected, p.bulk_collected), (3, 1));
            assert!(p.active_scheduler_passes <= p.scheduler_passes);
        }
        if mode == 7 {
            assert_eq!(command(d, 0, 41), [1, 1, 1]);
        }
        if mode == 11 {
            native_receipt(d);
        } else if mode >= 8 {
            runtime_receipt(d, mode >= 9, mode == 10);
        }
        if mode == 3 {
            assert_eq!(command(d, 0, 36), [1, 2, 3, 4]);
        }
    } else {
        assert_eq!(command(d, 0, if mode == 1 { 33 } else { 32 }), [1, 1, 1]);
    }
}

fn configure_fixture_policy(d: &InstanceDispatcher, mode: u8) {
    let policy = SchemeWorkExecutionPolicy {
        contended_work: 4,
        maximum_uninterrupted_work: 48,
    };
    let ticket = if mode >= 5 {
        d.configure_scheme_service_boundary_plan_on(
            0,
            policy,
            if mode >= 8 {
                if mode == 11 {
                    gerbil_scheme::instances::SchemeServiceCyclePolicy::NativeBudget
                } else if mode == 10 {
                    gerbil_scheme::instances::SchemeServiceCyclePolicy::RuntimeEvent
                } else if mode == 9 {
                    gerbil_scheme::instances::SchemeServiceCyclePolicy::RuntimeFair
                } else {
                    gerbil_scheme::instances::SchemeServiceCyclePolicy::RuntimeReadyFirst
                }
            } else if mode == 7 {
                gerbil_scheme::instances::SchemeServiceCyclePolicy::ProgressFair
            } else if mode == 6 {
                gerbil_scheme::instances::SchemeServiceCyclePolicy::ReadyFirst
            } else {
                gerbil_scheme::instances::SchemeServiceCyclePolicy::CheckpointFirst
            },
        )
    } else if mode == 4 {
        d.configure_scheme_service_execution_checkpoint_plan_on(0, policy)
    } else if mode == 3 {
        d.configure_scheme_service_directed_plan_on(0, policy)
    } else if mode == 2 {
        d.configure_scheme_service_registered_plan_on(0, policy)
    } else if mode == 1 {
        d.configure_scheme_service_receiver_plan_on(0, policy)
    } else {
        d.configure_scheme_service_reclamation_plan_on(0, policy)
    };
    ticket.unwrap().wait(GATE).unwrap();
}

fn runtime_receipt(d: &InstanceDispatcher, fair: bool, events: bool) {
    assert_eq!(command(d, 0, if events { 45 } else { 44 }), [1, 1, 1]);
    let p = d.scheme_runtime_progress_on(0).unwrap().wait(GATE).unwrap();
    assert!(p.enabled);
    assert_eq!(p.fair, fair);
    assert_eq!(
        [
            p.runnable,
            p.lease_wait,
            p.external_wait,
            p.actor_barrier,
            p.running
        ],
        [[0, 0]; 5]
    );
    assert_eq!(p.entered, p.retired);
    assert_eq!(p.ready_results, 0);
}

fn native_receipt(d: &InstanceDispatcher) {
    assert_eq!(command(d, 0, 48), [1, 1, 1]);
    let n = d.scheme_native_progress_on(0).unwrap().wait(GATE).unwrap();
    assert!(n.enabled);
    assert_eq!(n.actor_sources, 0);
    let p = d.scheme_runtime_progress_on(0).unwrap().wait(GATE).unwrap();
    assert!(!p.enabled && !p.fair);
    assert_eq!((p.entered, p.retired, p.ready_results), (0, 0, 0));
    assert_eq!(
        [
            p.runnable,
            p.lease_wait,
            p.external_wait,
            p.actor_barrier,
            p.running
        ],
        [[0, 0]; 5]
    );
}

fn registration_command(mode: u8) -> i64 {
    match mode {
        11 => 47,
        10 => 46,
        9 => 43,
        8 => 42,
        7 => 40,
        6 => 39,
        5 => 38,
        4 => 37,
        3 => 35,
        _ => 34,
    }
}
