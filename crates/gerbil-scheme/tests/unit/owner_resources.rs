//! Owner data retirement follows actual OS-thread joins, including return tails.
use super::{InstancePool, Mapping, OwnerThread};
use std::{
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
};
#[derive(Debug)]
struct Resource(Arc<AtomicUsize>, Arc<AtomicUsize>, usize);
impl Drop for Resource {
    fn drop(&mut self) {
        assert_eq!(
            self.1.load(Ordering::Acquire),
            self.2,
            "resource retired before owner return tail"
        );
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}
#[test]
fn one_shared_resource_waits_for_every_actual_owner_return() {
    let dropped = Arc::new(AtomicUsize::new(0));
    let returned = Arc::new(AtomicUsize::new(0));
    let resource = Arc::new(Resource(dropped.clone(), returned.clone(), 2));
    let first = Arc::new(Barrier::new(2));
    let last = Arc::new(Barrier::new(2));
    let (returned_sender, returned_receiver) = mpsc::channel();
    let (owners, incoming) = mpsc::channel();
    let (completed, reports) = mpsc::channel();
    let workers = [first.clone(), last.clone()]
        .into_iter()
        .enumerate()
        .map(|(index, tail)| {
            let returned = returned.clone();
            let returned_sender = returned_sender.clone();
            (
                index,
                OwnerThread {
                    worker: thread::spawn(move || {
                        tail.wait();
                        returned.fetch_add(1, Ordering::AcqRel);
                        returned_sender.send(index).unwrap();
                    }),
                    mapping: Mapping::new(index, u64::try_from(index).unwrap() + 1, false),
                    resources: vec![resource.clone()],
                },
            )
        })
        .collect();
    owners.send(workers).unwrap();
    drop(resource);
    let coordinator = thread::spawn(move || InstancePool::join_owners(&incoming, &completed));
    assert_eq!(dropped.load(Ordering::Acquire), 0);
    first.wait();
    assert_eq!(returned_receiver.recv().unwrap(), 0);
    assert_eq!(dropped.load(Ordering::Acquire), 0);
    last.wait();
    coordinator.join().unwrap();
    assert_eq!(reports.into_iter().collect::<Vec<_>>(), [false, false]);
    assert_eq!(dropped.load(Ordering::Acquire), 1);
}
#[test]
fn unverified_native_cleanup_retains_its_resource_after_wrapper_drop() {
    let dropped = Arc::new(AtomicUsize::new(0));
    let resource = Arc::new(Resource(dropped.clone(), Arc::new(AtomicUsize::new(0)), 0));
    let weak = Arc::downgrade(&resource);
    let mapping = Mapping::new(0, 101, false);
    mapping.initialized();
    let (owners, incoming) = mpsc::channel();
    let (completed, reports) = mpsc::channel();
    owners
        .send(vec![(
            0,
            OwnerThread {
                worker: thread::spawn(|| {}),
                mapping,
                resources: vec![resource.clone()],
            },
        )])
        .unwrap();
    drop(resource);
    InstancePool::join_owners(&incoming, &completed);
    assert!(!reports.recv().unwrap());
    assert_eq!(dropped.load(Ordering::Acquire), 0);
    assert!(
        weak.upgrade().is_some(),
        "quarantine must retain the data lease"
    );
}

#[test]
fn resource_destructor_panic_reports_retirement_failure_without_losing_join() {
    #[derive(Debug)]
    struct Failing;
    impl Drop for Failing {
        fn drop(&mut self) {
            panic!("qualification resource destructor failure");
        }
    }
    let (owners, incoming) = mpsc::channel();
    let (completed, reports) = mpsc::channel();
    let mapping = Mapping::new(0, 102, false);
    owners
        .send(vec![(
            0,
            OwnerThread {
                worker: thread::spawn(|| {}),
                mapping: mapping.clone(),
                resources: vec![Arc::new(Failing)],
            },
        )])
        .unwrap();
    InstancePool::join_owners(&incoming, &completed);
    assert!(reports.recv().unwrap());
    assert_eq!(
        mapping.snapshot().state,
        super::ImageMappingState::Quarantined
    );
}

fn directory_test_root(label: &str) -> std::path::PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "gsr-directory-test-{label}-{}-{nonce}",
        std::process::id()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    root
}

#[test]
fn stale_process_directory_does_not_block_or_modify_new_owner_resources() {
    use std::os::unix::fs::PermissionsExt;
    let root = directory_test_root("stale");
    let previous = root.join(format!("gerbil-owners-{}-0", std::process::id()));
    std::fs::create_dir(&previous).unwrap();
    let sentinel = previous.join("retained-image");
    std::fs::write(&sentinel, b"retained by another lifetime").unwrap();
    let created = super::create_owner_directory(&root).unwrap();
    assert_ne!(created, previous);
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"retained by another lifetime"
    );
    assert_eq!(
        std::fs::metadata(&created).unwrap().permissions().mode() & 0o777,
        0o700
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_owner_directories_are_exclusive_and_private() {
    use std::os::unix::fs::PermissionsExt;
    let root = directory_test_root("parallel");
    let directories = std::thread::scope(|scope| {
        let workers = (0..12)
            .map(|_| {
                let root = &root;
                scope.spawn(move || {
                    (0..16)
                        .map(|_| super::create_owner_directory(root).unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    let unique = directories
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), 192);
    for directory in directories {
        assert_eq!(directory.parent(), Some(root.as_path()));
        assert_eq!(
            std::fs::metadata(directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
