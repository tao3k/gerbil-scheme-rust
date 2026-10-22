//! Diagnostic point samples, not exact kernel runnable-to-dispatch intervals.
use super::{native_wait, os_threads::Owners};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
pub(super) struct Sampler {
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<Value>>,
}
impl Sampler {
    pub(super) fn start() -> Self {
        let owners = Owners::acquire();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let join = thread::spawn(move || {
            let mut rows = Vec::new();
            let mut overflow = 0;
            let mut errors = 0;
            while !stopping.load(Ordering::Acquire) {
                for (owner, &port) in owners.0.iter().enumerate() {
                    // SAFETY: this C output structure admits an all-zero initial value.
                    let mut info: libc::thread_basic_info = unsafe { std::mem::zeroed() };
                    let mut count = libc::THREAD_BASIC_INFO_COUNT;
                    let before = native_wait::clock();
                    // SAFETY: live own-process send right, correctly sized output.
                    let status = unsafe {
                        libc::thread_info(
                            port,
                            libc::THREAD_BASIC_INFO.try_into().unwrap(),
                            (&raw mut info).cast(),
                            &raw mut count,
                        )
                    };
                    let after = native_wait::clock();
                    if status != 0 || count != libc::THREAD_BASIC_INFO_COUNT {
                        errors += 1;
                        continue;
                    }
                    if rows.len() == 32768 {
                        overflow += 1;
                        continue;
                    }
                    let cpu_us = i64::from(info.user_time.seconds + info.system_time.seconds)
                        * 1_000_000
                        + i64::from(info.user_time.microseconds + info.system_time.microseconds);
                    rows.push(json!([
                        owner,
                        before,
                        after,
                        info.run_state,
                        cpu_us,
                        info.flags,
                        info.suspend_count
                    ]));
                }
                thread::sleep(Duration::from_millis(1));
            }
            // All inspection rights drop before native owner shutdown.
            drop(owners);
            json!({"row_width":7,"overflow":overflow,"errors":errors,"rows":rows})
        });
        Self {
            stop,
            join: Some(join),
        }
    }
    pub(super) fn finish(mut self) -> Value {
        self.stop.store(true, Ordering::Release);
        self.join.take().unwrap().join().unwrap()
    }
}
impl Drop for Sampler {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

pub(super) fn append(value: &mut Value, points: Option<Value>) {
    if let Some(points) = points {
        assert_eq!(points["errors"], 0);
        assert_eq!(points["overflow"], 0);
        value["os_thread_samples"] = points;
    }
}
