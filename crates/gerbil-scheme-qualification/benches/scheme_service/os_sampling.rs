//! Diagnostic point samples, not exact kernel runnable-to-dispatch intervals.
use super::{native_wait, os_threads::Owners};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

/// Store fixed rows while requests run; JSON allocation happens after stopping.
struct Points {
    rows: Vec<[i64; 10]>,
    overflow: usize,
    errors: usize,
}
impl Points {
    fn sweep(&mut self, owners: &Owners) {
        for (owner, &port) in owners.0.iter().enumerate() {
            // SAFETY: this C output structure admits an all-zero initial value.
            let mut info: libc::thread_extended_info = unsafe { std::mem::zeroed() };
            let mut count = libc::THREAD_EXTENDED_INFO_COUNT;
            let before = native_wait::clock();
            // SAFETY: live own-process send right, correctly sized output.
            let status = unsafe {
                libc::thread_info(
                    port,
                    libc::THREAD_EXTENDED_INFO.try_into().unwrap(),
                    (&raw mut info).cast(),
                    &raw mut count,
                )
            };
            let after = native_wait::clock();
            if status != 0 || count != libc::THREAD_EXTENDED_INFO_COUNT {
                self.errors += 1;
                continue;
            }
            if self.rows.len() == 32768 {
                self.overflow += 1;
                continue;
            }
            self.rows.push([
                owner.try_into().unwrap(),
                before.try_into().unwrap(),
                after.try_into().unwrap(),
                i64::from(info.pth_run_state),
                (info.pth_user_time + info.pth_system_time)
                    .try_into()
                    .unwrap(),
                i64::from(info.pth_flags),
                i64::from(info.pth_policy),
                i64::from(info.pth_curpri),
                i64::from(info.pth_priority),
                i64::from(info.pth_maxpriority),
            ]);
        }
    }
    fn finish(self) -> Value {
        json!({
            "schema_version":2,"row_width":10,"cpu_resolution_ns":1000,
            "requested_period_ns":1_000_000,
            "columns":["owner","before_ns","after_ns","run_state","cpu_ns",
                "flags","policy","current_priority","base_priority","max_priority"],
            "overflow":self.overflow,"errors":self.errors,"rows":self.rows
        })
    }
}

pub(super) struct Sampler {
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<Value>>,
}
impl Sampler {
    pub(super) fn start() -> Self {
        let owners = Owners::acquire();
        let identities = owners.identities();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let (ready, started) = mpsc::sync_channel(1);
        let join = thread::spawn(move || {
            let mut points = Points {
                rows: Vec::with_capacity(512),
                overflow: 0,
                errors: 0,
            };
            points.sweep(&owners);
            ready.send(()).unwrap();
            loop {
                thread::sleep(Duration::from_millis(1));
                let finishing = stopping.load(Ordering::Acquire);
                points.sweep(&owners);
                if finishing {
                    break;
                }
            }
            // The final sweep brackets completion before inspection rights drop.
            drop(owners);
            let mut value = points.finish();
            value["owner_tids"] = json!(identities);
            value
        });
        let sampler = Self {
            stop,
            join: Some(join),
        };
        started
            .recv_timeout(super::GATE)
            .expect("initial owner state sweep");
        sampler
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
