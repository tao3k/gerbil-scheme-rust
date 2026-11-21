//! Explicit kernel arrival clock. No Scheme state, permit, thread or polling timer.
#[cfg(target_os = "macos")]
use super::darwin as platform;
use std::{io, time::Instant};
/// One observed timer event, distinct from a later issue/admission boundary.
#[derive(Clone, Copy, Debug)]
pub struct InstanceArrivalWake {
    pub planned: Instant,
    pub arm_started: Instant,
    pub armed: Instant,
    pub observed: Instant,
    pub generation: u64,
}
/// One exclusively borrowed single-shot clock reused across a request sequence.
/// Nanosecond input units do not promise nanosecond OS scheduling. This opt-in
/// backend currently supports macOS; other hosts return Unsupported explicitly.
#[derive(Debug)]
pub struct InstanceArrivalClock {
    #[cfg(target_os = "macos")]
    timer: tokio::io::unix::AsyncFd<platform::Timer>,
    generation: u64,
}
impl InstanceArrivalClock {
    /// Create inside an I/O-enabled Tokio runtime. No new thread is started.
    /// # Errors
    /// Missing runtime, unsupported host, fd creation or reactor registration.
    /// # Panics
    /// Like Tokio `AsyncFd`, an entered runtime with its I/O driver disabled panics.
    pub fn new() -> io::Result<Self> {
        #[cfg(target_os = "macos")]
        {
            tokio::runtime::Handle::try_current().map_err(io::Error::other)?;
            Ok(Self {
                timer: tokio::io::unix::AsyncFd::with_interest(
                    platform::Timer::new()?,
                    tokio::io::Interest::READABLE,
                )?,
                generation: 0,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "kernel arrival clock requires macOS",
            ))
        }
    }
    /// Arm the original plan, including overdue plans, and observe its event.
    /// Cancellation disarms this generation; stale reactor readiness cannot make
    /// the next generation eligible. The mutable borrow forbids overlapping arms.
    /// # Errors
    /// Clock I/O, generation/duration overflow or observed eligibility before plan.
    pub async fn wait_until(&mut self, planned: Instant) -> io::Result<InstanceArrivalWake> {
        #[cfg(target_os = "macos")]
        {
            self.generation = self
                .generation
                .checked_add(1)
                .ok_or_else(|| io::Error::other("arrival clock generation overflow"))?;
            let generation = self.generation;
            let arm_started = Instant::now();
            let timer = self.timer.get_ref();
            timer.disarm()?;
            timer.arm(
                planned.saturating_duration_since(Instant::now()),
                generation,
            )?;
            let guard = platform::Armed(timer);
            let armed = Instant::now();
            loop {
                let mut ready = self.timer.readable().await?;
                // A stale readiness is cleared by try_io; await the real event.
                if let Ok(result) = ready.try_io(|fd| fd.get_ref().consume(generation)) {
                    result?;
                    let observed = Instant::now();
                    if observed < planned {
                        return Err(io::Error::other("arrival timer observed before plan"));
                    }
                    drop(guard);
                    return Ok(InstanceArrivalWake {
                        planned,
                        arm_started,
                        armed,
                        observed,
                        generation,
                    });
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (planned, self.generation);
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "kernel arrival clock requires macOS",
            ))
        }
    }
}
#[cfg(all(test, target_os = "macos"))]
#[path = "../../../tests/unit/arrival_clock.rs"]
mod tests;
