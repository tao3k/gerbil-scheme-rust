//! Bounded OS wake transport; Scheme owns the port reader and task timers.
use super::InstanceError;
use std::{
    fs::File,
    io::{self, Write},
    os::fd::{AsRawFd, FromRawFd},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
// Darwin sys/fcntl.h; libc 0.2.186 does not export this command.
// This module is compiled only by the Darwin isolated-instances backend.
#[cfg(feature = "tokio-dispatch")]
use std::sync::atomic::AtomicUsize;
/// Native pipe notification result; coalescing is not evidence of a lost wake.
#[derive(Clone, Copy, Debug)]
pub enum SchemeWakeDelivery {
    Coalesced,
    Written,
    Full,
    Failed,
}
const DARWIN_F_SETNOSIGPIPE: libc::c_int = 73;
#[derive(Debug)]
pub(super) struct Wake {
    writer: Mutex<File>,
    #[cfg(feature = "tokio-dispatch")]
    pub write_fd: i32,
    #[cfg(feature = "tokio-dispatch")]
    pub lanes: AtomicUsize,
    #[cfg(feature = "tokio-dispatch")]
    pub service_version: AtomicUsize,
    #[cfg(feature = "tokio-dispatch")]
    pub native_cycle: AtomicBool,
    #[cfg(feature = "tokio-dispatch")]
    pub prepared_cycle: AtomicBool,
    #[cfg(feature = "tokio-dispatch")]
    pub pipeline_cycle: AtomicBool,
    #[cfg(feature = "tokio-dispatch")]
    pub coupled_refill: AtomicBool,
    #[cfg(feature = "tokio-dispatch")]
    pub feed_limit: AtomicUsize,
    #[cfg(feature = "tokio-dispatch")]
    pub progress: super::service_progress::Progress,
    #[cfg(feature = "tokio-dispatch")]
    pub direct: AtomicBool,
    #[cfg(feature = "tokio-dispatch")]
    pub independent_short: AtomicBool,
    #[cfg(feature = "tokio-dispatch")]
    pub observation: std::sync::OnceLock<Arc<super::service_metrics::Monitor>>,
    #[cfg(feature = "tokio-dispatch")]
    pub capacity_changed: std::sync::OnceLock<
        Arc<std::sync::OnceLock<crate::instances::capacity_event::CapacityEvents>>,
    >,
    #[cfg(feature = "tokio-dispatch")]
    pub short_pool: std::sync::OnceLock<super::short_pool::Port>,
    #[cfg(feature = "tokio-dispatch")]
    pub offers: super::OfferPort,
    pending: AtomicBool,
    pub closed: AtomicBool,
    pub failed: AtomicBool,
}
impl Wake {
    pub fn create() -> Result<(Reader, Arc<Self>), InstanceError> {
        let (mut reader, mut writer) = pipe()?;
        // A host may intentionally leave stdin/stdout/stderr closed. Never let
        // task-owned wake descriptors occupy those inherited standard slots.
        if reader.as_raw_fd() < 3 {
            let moved = duplicate_high(reader.as_raw_fd())?;
            // SAFETY: fcntl returned a new exclusively-owned descriptor.
            reader = unsafe { File::from_raw_fd(moved) };
        }
        if writer.as_raw_fd() < 3 {
            let moved = duplicate_high(writer.as_raw_fd())?;
            // SAFETY: same fresh-descriptor transfer; assignment closes old fd.
            writer = unsafe { File::from_raw_fd(moved) };
        }
        let fd = writer.as_raw_fd();
        // SAFETY: task-owned pipe descriptor; preserve flags and make host writes
        // nonblocking. A full pipe is already an outstanding wake source.
        // This task-owned fd must report EPIPE, never deliver a process-wide
        // SIGPIPE if startup/owner failure closed the reader first.
        if unsafe { libc::fcntl(fd, DARWIN_F_SETNOSIGPIPE, 1) } < 0 {
            return Err(InstanceError::Loader(
                io::Error::last_os_error().to_string(),
            ));
        }
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(InstanceError::Loader(
                io::Error::last_os_error().to_string(),
            ));
        }
        Ok((
            Reader(reader),
            Arc::new(Self {
                writer: Mutex::new(writer),
                #[cfg(feature = "tokio-dispatch")]
                write_fd: fd,
                #[cfg(feature = "tokio-dispatch")]
                lanes: AtomicUsize::new(0),
                #[cfg(feature = "tokio-dispatch")]
                service_version: AtomicUsize::new(0),
                #[cfg(feature = "tokio-dispatch")]
                feed_limit: AtomicUsize::new(1),
                #[cfg(feature = "tokio-dispatch")]
                native_cycle: AtomicBool::new(false),
                #[cfg(feature = "tokio-dispatch")]
                prepared_cycle: AtomicBool::new(false),
                #[cfg(feature = "tokio-dispatch")]
                pipeline_cycle: AtomicBool::new(false),
                #[cfg(feature = "tokio-dispatch")]
                coupled_refill: AtomicBool::new(false),
                #[cfg(feature = "tokio-dispatch")]
                progress: super::service_progress::Progress::default(),
                #[cfg(feature = "tokio-dispatch")]
                direct: AtomicBool::new(false),
                #[cfg(feature = "tokio-dispatch")]
                independent_short: AtomicBool::new(false),
                #[cfg(feature = "tokio-dispatch")]
                observation: std::sync::OnceLock::new(),
                #[cfg(feature = "tokio-dispatch")]
                capacity_changed: std::sync::OnceLock::new(),
                #[cfg(feature = "tokio-dispatch")]
                short_pool: std::sync::OnceLock::new(),
                #[cfg(feature = "tokio-dispatch")]
                offers: super::OfferPort::default(),
                pending: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                failed: AtomicBool::new(false),
            }),
        ))
    }
    pub fn notify(&self) -> SchemeWakeDelivery {
        if self.pending.swap(true, Ordering::AcqRel) {
            return SchemeWakeDelivery::Coalesced;
        }
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            match writer.write(&[1]) {
                Ok(1) => return SchemeWakeDelivery::Written,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    return SchemeWakeDelivery::Full;
                }
                _ => {
                    drop(writer);
                    self.fail();
                    return SchemeWakeDelivery::Failed;
                }
            }
        }
    }
    #[cfg(feature = "tokio-dispatch")]
    pub fn notify_for(&self, short: bool) -> SchemeWakeDelivery {
        if short && self.independent_short.load(Ordering::Acquire) {
            self.notify_independent()
        } else {
            self.notify()
        }
    }
    #[cfg(feature = "tokio-dispatch")]
    fn notify_independent(&self) -> SchemeWakeDelivery {
        loop {
            // SAFETY: this Wake owns the descriptor until all notifier Arcs drop.
            // One-byte writes are atomic; nonblocking and no-SIGPIPE flags were
            // established at creation. No other publisher owns our wake duty.
            match unsafe { libc::write(self.write_fd, [1_u8].as_ptr().cast(), 1) } {
                1 => return SchemeWakeDelivery::Written,
                -1 => match io::Error::last_os_error().kind() {
                    io::ErrorKind::Interrupted => continue,
                    io::ErrorKind::WouldBlock => return SchemeWakeDelivery::Full,
                    _ => {}
                },
                _ => {}
            }
            self.fail();
            return SchemeWakeDelivery::Failed;
        }
    }
    #[cfg(feature = "tokio-dispatch")]
    pub fn arm(&self) {
        self.pending.store(false, Ordering::Release);
    }
    #[cfg(feature = "tokio-dispatch")]
    fn signal_capacity_change(&self) {
        if let Some(changed) = self
            .capacity_changed
            .get()
            .and_then(|changed| changed.get())
        {
            changed.publish();
        }
    }
    fn fail(&self) {
        self.failed.store(true, Ordering::Release);
        #[cfg(feature = "tokio-dispatch")]
        self.offers.close();
        #[cfg(feature = "tokio-dispatch")]
        if let Some(port) = self.short_pool.get() {
            port.source.owner_closed();
        }
        #[cfg(feature = "tokio-dispatch")]
        self.signal_capacity_change();
    }
    pub fn close(&self) {
        #[cfg(feature = "tokio-dispatch")]
        self.progress
            .waiting_for_claim
            .store(false, Ordering::Release);
        if !self.closed.swap(true, Ordering::AcqRel) {
            #[cfg(feature = "tokio-dispatch")]
            self.offers.close();
            #[cfg(feature = "tokio-dispatch")]
            if let Some(port) = self.short_pool.get() {
                port.source.owner_closed();
            }
            #[cfg(feature = "tokio-dispatch")]
            self.signal_capacity_change();
            #[cfg(feature = "tokio-dispatch")]
            if self.independent_short.load(Ordering::Acquire) {
                self.notify_independent();
                return;
            }
            self.notify();
        }
    }
}

fn duplicate_high(fd: i32) -> Result<i32, InstanceError> {
    // SAFETY: duplicate only this task's owned pipe, preserving close-on-exec.
    let moved = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
    if moved < 0 {
        Err(InstanceError::Loader(
            io::Error::last_os_error().to_string(),
        ))
    } else {
        Ok(moved)
    }
}

/// Read-side ownership stays on the native owner until Scheme closes its duplicate.
#[derive(Debug)]
pub(super) struct Reader(File);
impl AsRawFd for Reader {
    fn as_raw_fd(&self) -> i32 {
        self.0.as_raw_fd()
    }
}
fn pipe() -> Result<(File, File), InstanceError> {
    let mut descriptors = [-1; 2];
    // SAFETY: two writable fd slots; successful pipe transfers both descriptors.
    if unsafe { libc::pipe(descriptors.as_mut_ptr()) } < 0 {
        return Err(InstanceError::Loader(
            io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: both newly returned descriptors have exclusive RAII ownership.
    let (read, write) = unsafe {
        (
            File::from_raw_fd(descriptors[0]),
            File::from_raw_fd(descriptors[1]),
        )
    };
    for file in [&read, &write] {
        // SAFETY: only these task-owned descriptors are modified.
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
        if flags < 0
            || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0
        {
            return Err(InstanceError::Loader(
                io::Error::last_os_error().to_string(),
            ));
        }
    }
    Ok((read, write))
}
pub(super) fn notify_enqueued(wake: Option<Arc<Wake>>, enqueued: bool) {
    if enqueued {
        if let Some(wake) = wake {
            wake.notify();
        }
    }
}

#[cfg(all(test, feature = "tokio-dispatch"))]
#[path = "service_wake_tests.rs"]
mod tests;
