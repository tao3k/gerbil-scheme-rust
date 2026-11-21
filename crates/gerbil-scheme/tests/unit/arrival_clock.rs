use super::InstanceArrivalClock;
use crate::instances::InstanceArrival;
use std::{
    future::Future,
    os::fd::AsRawFd,
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};
fn run(f: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f);
}
#[test]
fn original_plan_event_issue_and_repeated_generations_remain_distinct() {
    run(async {
        let mut clock = InstanceArrivalClock::new().unwrap();
        let base = Instant::now() + Duration::from_micros(500);
        for index in 0..8 {
            let planned = base + Duration::from_micros(index * 500);
            let arrival = InstanceArrival::new(planned, None).unwrap();
            let (issue, wake) = arrival.wait_with_clock(&mut clock).await.unwrap();
            assert_eq!(wake.planned, planned);
            assert_eq!(wake.generation, index + 1);
            assert!(wake.arm_started <= wake.armed && wake.armed <= wake.observed);
            assert!(planned <= wake.observed && wake.observed <= issue.issued_at());
        }
    });
}
#[test]
fn cancellation_disarms_and_stale_readiness_cannot_release_a_new_plan() {
    run(async {
        let mut clock = InstanceArrivalClock::new().unwrap();
        let mut wait = Box::pin(clock.wait_until(Instant::now() + Duration::from_secs(1)));
        assert!(matches!(
            wait.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(wait);
        let (_, wake) = InstanceArrival::new(Instant::now() + Duration::from_micros(500), None)
            .unwrap()
            .wait_with_clock(&mut clock)
            .await
            .unwrap();
        assert_eq!(wake.generation, 2);
        let (_, overdue) = InstanceArrival::new(Instant::now() - Duration::from_millis(1), None)
            .unwrap()
            .wait_with_clock(&mut clock)
            .await
            .unwrap();
        assert_eq!(overdue.generation, 3);
        assert!(overdue.observed >= overdue.armed);
    });
}
#[test]
fn drop_closes_descriptor_and_overflow_keeps_clock_reusable() {
    run(async {
        let mut clock = InstanceArrivalClock::new().unwrap();
        assert!(clock.timer.get_ref().arm(Duration::MAX, 1).is_err());
        clock.wait_until(Instant::now()).await.unwrap();
        let fd = clock.timer.get_ref().as_raw_fd();
        drop(clock);
        // SAFETY: query only; no operation consumes or reuses the descriptor.
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
    });
}
