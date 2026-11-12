use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn unique_temp_dir(prefix: &str) -> PathBuf {
    static NEXT_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after epoch")
        .as_nanos();
    let id = NEXT_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}-{id}", std::process::id()))
}

#[test]
fn temporary_paths_stay_distinct_across_concurrent_test_workers() {
    let workers = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4)
        .max(4);
    let paths = std::thread::scope(|scope| {
        let jobs: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    (0..4096)
                        .map(|_| unique_temp_dir("parallel-program-control"))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        jobs.into_iter()
            .flat_map(|job| job.join().expect("temporary path worker"))
            .collect::<std::collections::HashSet<_>>()
    });
    assert_eq!(paths.len(), workers * 4096);
}
