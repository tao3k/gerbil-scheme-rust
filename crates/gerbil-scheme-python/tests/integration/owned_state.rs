// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use gerbil_scheme_python::{OwnedState, StateError};

#[test]
fn state_clones_share_updates_but_independent_owners_do_not() {
    let first = OwnedState::new(vec![1, 2]);
    let alias = first.clone();
    let other = OwnedState::new(vec![1, 2]);
    let captured = first.snapshot().unwrap();
    assert_eq!(first.replace(vec![3]).unwrap(), vec![1, 2]);
    assert_eq!(alias.snapshot().unwrap(), vec![3]);
    assert_eq!(other.snapshot().unwrap(), vec![1, 2]);
    assert_eq!(captured, vec![1, 2]);
    alias.close().unwrap();
    assert_eq!(first.snapshot(), Err(StateError::Closed));
    assert_eq!(first.replace(vec![7]), Err(StateError::Closed));
    assert_eq!(other.snapshot().unwrap(), vec![1, 2]);
    first.close().unwrap();
}

#[test]
fn concurrent_replacements_publish_only_complete_values() {
    let state = OwnedState::new(vec![0_u8; 256]);
    std::thread::scope(|scope| {
        for value in 1..=4 {
            let state = state.clone();
            scope.spawn(move || {
                for _ in 0..100 {
                    let previous = state.replace(vec![value; 256]).unwrap();
                    assert_eq!(previous.len(), 256);
                    assert!(previous.iter().all(|v| *v == previous[0]));
                }
            });
        }
    });
    state.close().unwrap();
    assert!(state.is_closed().unwrap());
}

#[test]
fn poisoned_state_is_not_reclassified_as_closed_or_success() {
    struct Poison;
    impl Drop for Poison { fn drop(&mut self) { panic!("controlled destructor failure"); } }
    let state = OwnedState::new(Poison);
    let worker = state.clone();
    assert!(std::thread::spawn(move || worker.close()).join().is_err());
    assert_eq!(state.is_closed(), Err(StateError::Poisoned));
    assert_eq!(state.close(), Err(StateError::Poisoned));
}

#[test]
fn compiled_callable_admission_and_close_have_explicit_lifetime() {
    use gerbil_scheme_python::OwnedCallable;
    let state = OwnedState::new(7);
    let reader = state.clone();
    let callable = OwnedCallable::new(move |()| reader.snapshot());
    assert_eq!(callable.call(()), Ok(7));
    state.replace(9).unwrap();
    assert_eq!(callable.call(()), Ok(9));
    let alias = callable.clone();
    callable.close().unwrap();
    assert_eq!(alias.call(()), Err(StateError::Closed));
    assert_eq!(state.snapshot(), Ok(9));
}
