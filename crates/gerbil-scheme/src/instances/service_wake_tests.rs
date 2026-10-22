//! Real descriptor controls for the publisher-claim gap; no Scheme replacement.
use super::{Reader, SchemeWakeDelivery, Wake};
use std::io::Read;
use std::{
    os::fd::AsRawFd,
    sync::{Arc, atomic::Ordering},
};
fn ready(reader: &Reader) -> bool {
    let mut fd = libc::pollfd {
        fd: reader.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one initialized descriptor record; zero timeout never parks a test.
    assert!(unsafe { libc::poll(&raw mut fd, 1, 0) } >= 0);
    fd.revents & libc::POLLIN != 0
}
#[test]
fn claimed_notification_does_not_suppress_independent_publish() {
    let (mut reader, wake) = Wake::create().unwrap();
    // The first publisher has claimed the flag but has not written its byte.
    wake.pending.store(true, Ordering::Release);
    assert!(matches!(wake.notify(), SchemeWakeDelivery::Coalesced));
    assert!(!ready(&reader));
    wake.independent_short.store(true, Ordering::Release);
    // Hold the legacy writer mutex as well: independent publication must not
    // wait for its owner. No timing threshold or artificially delayed producer.
    assert!(matches!(
        wake.notify_for(false),
        SchemeWakeDelivery::Coalesced
    ));
    assert!(!ready(&reader));
    let writer = wake.writer.lock().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let publisher = Arc::clone(&wake);
    let thread = std::thread::spawn(move || send.send(publisher.notify_for(true)).unwrap());
    let result = receive.recv_timeout(std::time::Duration::from_secs(5));
    drop(writer);
    thread.join().unwrap();
    assert!(matches!(result.unwrap(), SchemeWakeDelivery::Written));
    assert!(ready(&reader));
    let mut byte = [0];
    reader.0.read_exact(&mut byte).unwrap();
    assert_eq!(byte, [1]);
    assert!(!ready(&reader));
}
#[test]
fn concurrent_independent_publish_preserves_every_byte() {
    let (mut reader, wake) = Wake::create().unwrap();
    wake.pending.store(true, Ordering::Release);
    wake.independent_short.store(true, Ordering::Release);
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let wake = Arc::clone(&wake);
            std::thread::spawn(move || {
                for _ in 0..32 {
                    assert!(matches!(wake.notify_for(true), SchemeWakeDelivery::Written));
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert!(ready(&reader));
    let mut bytes = [0; 256];
    reader.0.read_exact(&mut bytes).unwrap();
    assert_eq!(bytes, [1; 256]);
    assert!(!ready(&reader));
}
#[test]
fn full_pipe_is_an_existing_wake_and_closed_reader_is_failure() {
    let (reader, wake) = Wake::create().unwrap();
    let (notifier, changes) = tokio::sync::watch::channel(());
    wake.capacity_changed
        .set(Arc::new(std::sync::OnceLock::from(notifier)))
        .unwrap();
    wake.independent_short.store(true, Ordering::Release);
    // Fill to the actual kernel capacity, including the byte after 64 KiB.
    // The finite safety guard is not a configured pipe size or timing budget.
    let full = (0..1_048_576).any(|_| match wake.notify_independent() {
        SchemeWakeDelivery::Written => false,
        SchemeWakeDelivery::Full => true,
        other => panic!("unexpected native write: {other:?}"),
    });
    assert!(
        full,
        "native pipe did not become full within the finite guard"
    );
    assert!(ready(&reader));
    assert!(!wake.failed.load(Ordering::Acquire));
    drop(reader);
    assert!(matches!(
        wake.notify_independent(),
        SchemeWakeDelivery::Failed
    ));
    assert!(wake.failed.load(Ordering::Acquire));
    assert!(
        changes.has_changed().unwrap(),
        "owner failure must wake capacity reselection"
    );
}

#[test]
fn owner_close_publishes_durable_capacity_change() {
    let (_reader, wake) = Wake::create().unwrap();
    let (notifier, changes) = tokio::sync::watch::channel(());
    wake.capacity_changed
        .set(Arc::new(std::sync::OnceLock::from(notifier)))
        .unwrap();
    wake.close();
    assert!(changes.has_changed().unwrap());
}
