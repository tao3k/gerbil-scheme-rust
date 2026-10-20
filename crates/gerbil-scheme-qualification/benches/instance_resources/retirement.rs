//! Image-resident pages, allocator reusable pages and actual copied file paths.
use super::{GATE, Progress, command, config, finish, normal, observe::Observer, observed};
use gerbil_scheme::instances::InstancePool;
use std::sync::{Arc, Barrier};
pub(super) fn run(observer: &Observer, progress: &Progress) {
    let baseline = observer.emit("retirement-baseline", 0, &[]);
    observer.regions("baseline");
    observer.tls("baseline");
    for epoch in 0..8 {
        let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
        observed(progress, "observed retirement native setup");
        assert_eq!(
            command(&pool.dispatcher(), 0, vec![3, 32 * 1024 * 1024], progress),
            [1]
        );
        observer.emit("retirement-live", epoch, &[]);
        observer.regions("live");
        assert_eq!(command(&pool.dispatcher(), 0, vec![4], progress), [0]);
        finish(&mut pool, progress);
        observer.settled("retirement-cleaned", epoch, baseline[3]);
        let regions = observer.regions("cleaned");
        observer.tls("cleaned");
        assert_eq!(regions[9], u64::try_from(epoch + 1).unwrap());
        let files = std::fs::read_dir(&pool.image_directory).map_or(0, Iterator::count);
        assert_eq!(files, 0);
        assert!(!pool.image_directory.exists());
        eprintln!(
            "RETIREMENT-FILES epoch={epoch} files={files} directory-exists={}",
            pool.image_directory.exists()
        );
    }
    let released = observer.relieve();
    observer.emit("retirement-relief", 8, &[]);
    observer.regions("relief");
    eprintln!("RETIREMENT-PRESSURE released={released}");
}
pub(super) fn allocator(observer: &Observer, progress: &Progress) {
    observer.emit("allocator-before", 0, &[]);
    observer.regions("allocator-before");
    let first = allocator_wave(observer, progress, 0);
    let second = allocator_wave(observer, progress, 1);
    let reused = first.intersection(&second).count();
    let released = observer.relieve();
    observer.emit("allocator-relief", 0, &[]);
    observer.regions("allocator-relief");
    eprintln!(
        "ALLOCATOR-CONTROL released-by-pressure={released} native-setups=0 touched-bytes-per-wave=134217728 reused-addresses={reused} blocks-per-wave=512"
    );
}
fn allocator_wave(
    observer: &Observer,
    progress: &Progress,
    wave: usize,
) -> std::collections::BTreeSet<usize> {
    let ready = Arc::new(Barrier::new(5));
    let release = Arc::new(Barrier::new(5));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let ready = Arc::clone(&ready);
            let release = Arc::clone(&release);
            std::thread::spawn(move || {
                let chunks: Vec<_> = (0..128).map(|_| vec![90_u8; 256 * 1024]).collect();
                std::hint::black_box(&chunks);
                let addresses: Vec<_> =
                    chunks.iter().map(|chunk| chunk.as_ptr() as usize).collect();
                ready.wait();
                release.wait();
                drop(chunks);
                addresses
            })
        })
        .collect();
    ready.wait();
    observed(progress, "observed allocator live control");
    observer.emit("allocator-live", wave, &[]);
    observer.regions("allocator-live");
    release.wait();
    let mut addresses = std::collections::BTreeSet::new();
    for worker in workers {
        addresses.extend(worker.join().unwrap());
    }
    observed(progress, "observed allocator free and joins");
    observer.emit("allocator-free", wave, &[]);
    observer.regions("allocator-free");
    addresses
}
pub(super) fn file_failure(observer: &Observer, progress: &Progress) {
    use std::os::unix::fs::PermissionsExt;
    let baseline = observer.emit("file-failure-baseline", 0, &[]);
    let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
    observed(progress, "observed cleanup failure control setup");
    std::fs::set_permissions(
        &pool.image_directory,
        std::fs::Permissions::from_mode(0o500),
    )
    .unwrap();
    let receipt = pool.shutdown(GATE);
    std::fs::set_permissions(
        &pool.image_directory,
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    assert!(!receipt.is_clean());
    assert_eq!(receipt.joined, 1);
    assert_eq!(receipt.pending, 0);
    let owner = &receipt.owners[0];
    assert_eq!(owner.status, 0);
    assert_eq!(owner.roots_before_cleanup, 0);
    assert!(!owner.image_file_removed);
    assert!(owner.image_file_error.is_some());
    eprintln!("FILE-FAILURE cleanup={receipt:?}");
    std::fs::remove_file(pool.image_directory.join("owner-0.dylib")).unwrap();
    std::fs::remove_dir(&pool.image_directory).unwrap();
    observed(
        progress,
        "observed surfaced file failure and manual control cleanup",
    );
    observer.settled("file-failure-drained", 0, baseline[3]);
    let mut next = InstancePool::start(&normal(), config(1)).unwrap();
    observed(
        progress,
        "observed native admission after filesystem-only failure",
    );
    assert_eq!(command(&next.dispatcher(), 0, vec![0, 1], progress), [10]);
    finish(&mut next, progress);
}

pub(super) fn mappings(observer: &Observer, progress: &Progress) {
    use std::os::unix::fs::DirBuilderExt;
    observer.emit("mapping-baseline", 0, &[]);
    observer.tls("mapping-baseline");
    for epoch in 0..8 {
        let directory = std::env::temp_dir().join(format!(
            "gerbil-owners-{}-mapping-{epoch}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .unwrap();
        let file = directory.join("owner-0.dylib");
        std::fs::copy(gerbil_scheme_qualification::owner_instance_image(), &file).unwrap();
        Observer::retain_uninitialized(&file);
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(directory).unwrap();
        observed(progress, "observed uninitialized retained mapping control");
        observer.emit("mapping-loaded", epoch, &[]);
        let regions = observer.regions("mapping-loaded");
        assert_eq!(regions[9], u64::try_from(epoch + 1).unwrap());
        observer.tls("mapping-loaded");
    }
}
