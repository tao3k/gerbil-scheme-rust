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
