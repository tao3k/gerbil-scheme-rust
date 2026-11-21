//! Own-process inspection rights, acquired after VM startup and released on drop.
use std::ptr;
unsafe extern "C" {
    static mach_task_self_: libc::mach_port_t;
    fn mach_port_deallocate(
        task: libc::mach_port_t,
        name: libc::mach_port_t,
    ) -> libc::kern_return_t;
}
struct Threads {
    ports: *mut libc::thread_t,
    count: libc::mach_msg_type_number_t,
}
impl Drop for Threads {
    fn drop(&mut self) {
        // SAFETY: owns the successful task_threads allocation and remaining rights.
        unsafe {
            for &port in std::slice::from_raw_parts(self.ports, self.count as usize) {
                if port != 0 {
                    mach_port_deallocate(mach_task_self_, port);
                }
            }
            libc::vm_deallocate(
                mach_task_self_,
                self.ports as libc::vm_address_t,
                self.count as libc::vm_size_t
                    * std::mem::size_of::<libc::thread_t>() as libc::vm_size_t,
            );
        }
    }
}
pub(super) struct Owners(pub [libc::thread_t; 4]);
impl Owners {
    pub(super) fn identities(&self) -> [u64; 4] {
        self.0.map(|port| {
            // SAFETY: this C output structure admits an all-zero initial value.
            let mut info: libc::thread_identifier_info = unsafe { std::mem::zeroed() };
            let mut count = libc::THREAD_IDENTIFIER_INFO_COUNT;
            // SAFETY: own-process inspection right and correctly sized output.
            let status = unsafe {
                libc::thread_info(
                    port,
                    libc::THREAD_IDENTIFIER_INFO.try_into().unwrap(),
                    (&raw mut info).cast(),
                    &raw mut count,
                )
            };
            assert_eq!(status, 0, "owner identity inspection");
            assert_eq!(count, libc::THREAD_IDENTIFIER_INFO_COUNT);
            info.thread_id
        })
    }
    pub(super) fn acquire() -> Self {
        let mut threads = Threads {
            ports: ptr::null_mut(),
            count: 0,
        };
        // SAFETY: task_threads initializes the allocation and its send rights.
        let status = unsafe {
            libc::task_threads(
                mach_task_self_,
                &raw mut threads.ports,
                &raw mut threads.count,
            )
        };
        if status != 0 {
            // A failed call transferred no array or rights.
            std::mem::forget(threads);
            panic!("task_threads failed: {status}");
        }
        let mut owners = Self([0; 4]);
        // SAFETY: the successful call returned count initialized entries.
        for port in unsafe { std::slice::from_raw_parts_mut(threads.ports, threads.count as usize) }
        {
            // SAFETY: this C output structure admits an all-zero initial value.
            let mut info: libc::thread_extended_info = unsafe { std::mem::zeroed() };
            let mut count = libc::THREAD_EXTENDED_INFO_COUNT;
            // SAFETY: own-process send right and correctly sized writable buffer.
            // Kernel name lookup avoids a potentially exiting pthread handle.
            let status = unsafe {
                libc::thread_info(
                    *port,
                    libc::THREAD_EXTENDED_INFO.try_into().unwrap(),
                    (&raw mut info).cast(),
                    &raw mut count,
                )
            };
            let name: Vec<u8> = info
                .pth_name
                .iter()
                .take_while(|c| **c != 0)
                .map(|c| c.to_ne_bytes()[0])
                .collect();
            let owner = (status == 0 && count == libc::THREAD_EXTENDED_INFO_COUNT)
                .then(|| std::str::from_utf8(&name).unwrap_or(""))
                .and_then(|name| {
                    name.strip_prefix("gerbil-owner-")
                        .and_then(|id| id.parse::<usize>().ok())
                });
            if let Some(owner) = owner.filter(|i| *i < owners.0.len()) {
                assert_eq!(owners.0[owner], 0, "duplicate owner thread");
                owners.0[owner] = *port;
                *port = 0; // transfer only this send right; Threads releases the rest
            }
        }
        assert!(
            owners.0.iter().all(|port| *port != 0),
            "missing live owner thread"
        );
        owners
    }
}
impl Drop for Owners {
    fn drop(&mut self) {
        for port in self.0 {
            if port != 0 {
                // SAFETY: our send right; never terminates or suspends the owner.
                unsafe { mach_port_deallocate(mach_task_self_, port) };
            }
        }
    }
}
