//! Darwin kqueue one-shot timer in nanoseconds. Never blocks in kevent64.
use std::{
    io,
    mem::MaybeUninit,
    os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
    time::Duration,
};
#[derive(Debug)]
pub(super) struct Timer(OwnedFd);
impl AsRawFd for Timer {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
impl Timer {
    pub(super) fn new() -> io::Result<Self> {
        // SAFETY: kqueue returns a new descriptor and does not retain Rust memory.
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: this is the sole owner of the newly returned descriptor.
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        // SAFETY: fcntl consumes no pointer; close-on-exec is set before exposure.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(owned))
    }
    fn change(&self, flags: u16, units: u32, data: i64, generation: u64) -> io::Result<()> {
        let event = libc::kevent64_s {
            ident: 1,
            filter: libc::EVFILT_TIMER,
            flags,
            fflags: units,
            data,
            udata: generation,
            ext: [0; 2],
        };
        let zero = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: one initialized change, no output buffer, zero timeout. The
        // kernel copies integer metadata only; it retains no Rust pointer.
        let result = unsafe {
            libc::kevent64(
                self.as_raw_fd(),
                &raw const event,
                1,
                std::ptr::null_mut(),
                0,
                0,
                &raw const zero,
            )
        };
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    pub(super) fn arm(&self, remaining: Duration, generation: u64) -> io::Result<()> {
        let ns = i64::try_from(remaining.as_nanos().max(1)).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "arrival duration overflow")
        })?;
        self.change(
            libc::EV_ADD | libc::EV_ONESHOT,
            libc::NOTE_NSECONDS,
            ns,
            generation,
        )
    }
    pub(super) fn disarm(&self) -> io::Result<()> {
        match self.change(libc::EV_DELETE, 0, 0, 0) {
            Err(error) if error.raw_os_error() == Some(libc::ENOENT) => Ok(()),
            result => result,
        }
    }
    pub(super) fn consume(&self, generation: u64) -> io::Result<()> {
        let mut event = MaybeUninit::<libc::kevent64_s>::uninit();
        let zero = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: one output slot, no changes and an explicit zero timeout, so
        // reactor reads cannot block a worker. Only a successful count reads it.
        let count = unsafe {
            libc::kevent64(
                self.as_raw_fd(),
                std::ptr::null(),
                0,
                event.as_mut_ptr(),
                1,
                0,
                &raw const zero,
            )
        };
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        // SAFETY: the kernel initialized the single returned output record.
        let event = unsafe { event.assume_init() };
        if event.flags & libc::EV_ERROR != 0 {
            return Err(io::Error::other(format!(
                "arrival kevent error {}",
                event.data
            )));
        }
        if event.ident != 1 || event.filter != libc::EVFILT_TIMER || event.udata != generation {
            return Err(io::Error::other(
                "stale or invalid arrival timer generation",
            ));
        }
        Ok(())
    }
}
pub(super) struct Armed<'a>(pub(super) &'a Timer);
impl Drop for Armed<'_> {
    fn drop(&mut self) {
        let _ = self.0.disarm();
    }
}
