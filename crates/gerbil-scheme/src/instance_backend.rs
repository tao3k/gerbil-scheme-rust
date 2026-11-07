//! One startup choice for process-wide native hooks. Never silently mix routes.
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend {
    Unclaimed,
    Linked,
    Images,
    Poisoned,
}
static BACKEND: Mutex<Backend> = Mutex::new(Backend::Unclaimed);

pub(crate) fn claim_linked() -> bool {
    claim(Backend::Linked)
}
pub(crate) fn claim_images() -> bool {
    claim(Backend::Images)
}
fn claim(selected: Backend) -> bool {
    let mut backend = BACKEND
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if *backend != Backend::Unclaimed && *backend != selected {
        return false;
    }
    *backend = selected;
    true
}

pub(crate) fn images_ready() -> bool {
    *BACKEND
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        == Backend::Images
}
pub(crate) fn poison_images() {
    *BACKEND
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Backend::Poisoned;
}
