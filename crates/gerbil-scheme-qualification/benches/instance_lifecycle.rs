//! Scheme semantics stay inside owner-affine private runtime images.
#[path = "instances/image.rs"]
mod image;
#[path = "instances/pool.rs"]
mod pool;

use image::Image;
use pool::Pool;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

static GENERATION: AtomicUsize = AtomicUsize::new(0);

fn paths(workers: usize) -> Vec<PathBuf> {
    let directory = PathBuf::from(env!("GERBIL_INSTANCE_PROBE_IMAGE"))
        .parent()
        .expect("image parent")
        .join(format!(
            "lifecycle-{}-{}",
            std::process::id(),
            GENERATION.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir(&directory).expect("new diagnostic directory");
    (0..workers)
        .map(|worker| {
            let path = directory.join(format!("owner-{worker}.dylib"));
            std::fs::copy(env!("GERBIL_INSTANCE_PROBE_IMAGE"), &path).expect("private image copy");
            path
        })
        .collect()
}

#[derive(Debug)]
struct Owner {
    commands: mpsc::Sender<u32>,
    replies: mpsc::Receiver<(i64, usize)>,
    thread: thread::JoinHandle<()>,
}

impl Owner {
    fn start(path: PathBuf, id: u32, progress: &mpsc::Sender<String>) -> Self {
        let (commands, input) = mpsc::channel();
        let (output, replies) = mpsc::channel();
        let thread = thread::spawn(move || {
            let image = Image::load(&path);
            output
                .send((image.contract(0, id), image.state))
                .expect("init receipt");
            for operation in input {
                if operation == 1000 {
                    drop(image);
                    output.send((0, 0)).expect("cleanup receipt");
                    return;
                }
                let result = if operation == 6 {
                    let value = image.run(id, 17, 32, std::ptr::null_mut());
                    assert_eq!(image.marker(), id);
                    value
                } else {
                    image.contract(operation, id)
                };
                output.send((result, image.state)).expect("foreign result");
            }
            panic!("owner must receive an explicit drain");
        });
        let owner = Self {
            commands,
            replies,
            thread,
        };
        let (processors, state) = owner.receive(progress);
        assert_eq!(
            processors, 1,
            "each independent instance must have one processor"
        );
        assert_ne!(state, 0);
        eprintln!(
            "LIFECYCLE owner={id} initialized=true processors={processors} gstate={state:#x}"
        );
        owner
    }

    fn receive(&self, progress: &mpsc::Sender<String>) -> (i64, usize) {
        let result = self
            .replies
            .recv_timeout(Duration::from_secs(5))
            .expect("five-second owner response gate");
        progress
            .send("observed owner response".into())
            .expect("watchdog");
        result
    }

    fn check(&self, op: u32, expected: i64, progress: &mpsc::Sender<String>) -> usize {
        self.commands.send(op).expect("owner admission");
        let (actual, state) = self.receive(progress);
        assert_eq!(actual, expected, "lifecycle operation {op}");
        state
    }

    fn stop(self, progress: &mpsc::Sender<String>) {
        self.check(1000, 0, progress);
        self.thread.join().expect("owner join");
        progress
            .send("observed owner joined".into())
            .expect("watchdog");
    }
}

fn lifecycle(progress: &mpsc::Sender<String>) {
    let mut paths = paths(2).into_iter();
    let first = Owner::start(paths.next().expect("first image"), 101, progress);
    let second = Owner::start(paths.next().expect("second image"), 202, progress);
    let a = first.check(98, -1, progress);
    let b = second.check(98, -1, progress);
    assert_ne!(a, b, "instances must have distinct GSTATEs");
    first.check(1, 23, progress);
    second.check(1, 23, progress);
    first.check(2, 108, progress);
    second.check(2, 209, progress);
    first.check(3, 110, progress);
    first.check(4, 31, progress);
    first.check(5, 101, progress);
    first.stop(progress);
    eprintln!("LIFECYCLE first-owner-cleanup=OK second-owner-holds-continuation=true");
    second.check(3, 211, progress);
    second.check(4, 31, progress);
    second.check(5, 202, progress);
    second.check(6, 8976, progress);
    second.stop(progress);
    eprintln!(
        "LIFECYCLE PASS gc-rooted-payload=true multi-shot=true dynamic-wind=true dynamic-binding=true scheme-thread-identity=true exception=true survivor-after-other-cleanup=true finalized-reuse-rejected=true images-unloaded=false"
    );
}

fn matched(progress: &mpsc::Sender<String>, smoke: bool) {
    let start = Instant::now();
    let one = Pool::new(paths(1));
    let one_init = start.elapsed();
    let start = Instant::now();
    let four = Pool::new(paths(4));
    let four_init = start.elapsed();
    eprintln!(
        "MATCHED setup-a-ns={} setup-b-ns={} a-owners=1 b-owners=4 setup-includes-image-copy=true",
        one_init.as_nanos(),
        four_init.as_nanos()
    );
    progress
        .send("observed both pools initialized".into())
        .expect("watchdog");
    for rounds in [32_u32, 4096] {
        for jobs in [1_000_u32, 10_000, 100_000] {
            let expected = i64::from(jobs) * i64::from(rounds) * (i64::from(rounds) + 1) / 2;
            assert_eq!(one.batch(jobs, rounds, true), expected);
            assert_eq!(four.batch(jobs, rounds, true), expected);
            progress
                .send("observed simultaneous Scheme entry".into())
                .expect("watchdog");
            let mut samples: [Vec<u128>; 2] = [Vec::new(), Vec::new()];
            for block in 0..if smoke { 1 } else { 10 } {
                for variant in [0, 1, 1, 0] {
                    let pool = if variant == 0 { &one } else { &four };
                    let start = Instant::now();
                    let actual = pool.batch(jobs, rounds, false);
                    let elapsed = start.elapsed().as_nanos();
                    assert_eq!(actual, expected);
                    samples[variant].push(elapsed);
                    eprintln!(
                        "MATCHED-SAMPLE jobs={jobs} rounds={rounds} block={block} variant={variant} ns={elapsed} checksum={actual}"
                    );
                    progress
                        .send("observed completed matched batch".into())
                        .expect("watchdog");
                }
            }
            for values in &mut samples {
                values.sort_unstable();
            }
            let a =
                samples[0][(samples[0].len() - 1) / 2].midpoint(samples[0][samples[0].len() / 2]);
            let b =
                samples[1][(samples[1].len() - 1) / 2].midpoint(samples[1][samples[1].len() / 2]);
            eprintln!(
                "MATCHED-RESULT jobs={jobs} rounds={rounds} samples-per-variant={} a-median-ns={a} b-median-ns={b} checksum={expected} workload=scheme-checksum-with-forced-gc",
                samples[0].len()
            );
        }
    }
    let start = Instant::now();
    drop(one);
    let a = start.elapsed().as_nanos();
    progress
        .send("observed baseline pool drained".into())
        .expect("watchdog");
    // Confirm a still-live pool after another runtime's process-hook cleanup.
    assert_eq!(four.batch(1000, 32, true), 528_000);
    let start = Instant::now();
    drop(four);
    eprintln!(
        "MATCHED teardown-a-ns={a} teardown-b-ns={} images-retained=true",
        start.elapsed().as_nanos()
    );
    progress
        .send("observed candidate pool drained".into())
        .expect("watchdog");
}

fn main() {
    let smoke = std::env::args().any(|arg| arg == "--test");
    let (progress, observations) = mpsc::channel::<String>();
    let watchdog = thread::spawn(move || {
        let mut last = "start".to_owned();
        loop {
            match observations.recv_timeout(Duration::from_secs(5)) {
                Ok(observed) => last = observed,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    eprintln!(
                        "LIFECYCLE STALL last={last} five-second-progress-gate=FAILED cleanup=UNVERIFIED"
                    );
                    std::process::exit(70);
                }
            }
        }
    });
    lifecycle(&progress);
    matched(&progress, smoke);
    drop(progress);
    watchdog.join().expect("watchdog join");
}
