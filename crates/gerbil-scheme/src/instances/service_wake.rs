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
const DARWIN_F_SETNOSIGPIPE: libc::c_int = 73;
#[derive(Debug)]
pub(super) struct Wake {
    writer: Mutex<File>,
    #[cfg(feature = "tokio-dispatch")]
    pub write_fd: i32,
    #[cfg(feature = "tokio-dispatch")]
    pub lanes: AtomicUsize,
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
                pending: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                failed: AtomicBool::new(false),
            }),
        ))
    }
    pub fn notify(&self) {
        if self.pending.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut writer = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            match writer.write(&[1]) {
                Ok(1) => return,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                _ => {
                    self.failed.store(true, Ordering::Release);
                    return;
                }
            }
        }
    }
    #[cfg(feature = "tokio-dispatch")]
    pub fn arm(&self) {
        self.pending.store(false, Ordering::Release);
    }
    pub fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
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
