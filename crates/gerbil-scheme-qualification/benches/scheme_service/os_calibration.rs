//! Known blocking/CPU controls validate the observer, never Scheme performance.
use super::{GATE, native_wait, os_sampling::Sampler};
use gerbil_scheme::instances::InstancePool;
use serde_json::json;
use std::time::{Duration, Instant};
pub(super) fn run(pool: &InstancePool) {
    let sampling = Sampler::start();
    let mut rows = Vec::new();
    for busy in [false, true] {
        // All four existing owners participate concurrently, with no Scheme call.
        let tickets: Vec<_> = (0..4)
            .map(|owner| {
                pool.dispatcher()
                    .submit_native_on(owner, move |session| {
                        let before = session
                            .diagnostics()
                            .expect("native CPU diagnostics required");
                        let begin = native_wait::clock();
                        let guard = Instant::now();
                        if busy {
                            while session.diagnostics().unwrap()[3] - before[3] < 0.020 {
                                assert!(guard.elapsed() < GATE, "CPU calibration stalled");
                                for _ in 0..1024 {
                                    std::hint::spin_loop();
                                }
                            }
                        } else {
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        let end = native_wait::clock();
                        let after = session.diagnostics().unwrap();
                        let cpu = u64::try_from(
                            Duration::try_from_secs_f64(after[3] - before[3])
                                .unwrap()
                                .as_nanos(),
                        )
                        .unwrap();
                        assert_eq!(
                            after[2].to_bits(),
                            before[2].to_bits(),
                            "calibration must not enter Scheme GC"
                        );
                        assert_eq!(
                            after[5].to_bits(),
                            before[5].to_bits(),
                            "calibration GC count changed"
                        );
                        Ok((owner, begin, end, cpu))
                    })
                    .unwrap()
            })
            .collect();
        for ticket in tickets {
            let (owner, begin, end, cpu) = ticket.wait(GATE).unwrap();
            rows.push(
                json!({"owner":owner,"kind":if busy {"cpu-20ms"} else {"sleep-20ms"},
                "start_ns":begin,"end_ns":end,"wall_ns":end-begin,"cpu_ns":cpu}),
            );
        }
    }
    let points = sampling.finish();
    assert_eq!(points["errors"], 0);
    assert_eq!(points["overflow"], 0);
    println!(
        "{}",
        json!({"event":"os_calibration","controls":rows,"os_thread_samples":points})
    );
}
