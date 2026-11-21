//! Retire the copied path after known cleanup, without unloading native code.
use std::{fs, io::ErrorKind, path::Path};
pub(super) fn file(path: &Path, status: i32) -> (bool, Option<String>) {
    if status != 0 {
        return (false, None);
    }
    if let Err(error) = fs::remove_file(path) {
        if error.kind() != ErrorKind::NotFound {
            return (false, Some(error.to_string()));
        }
    }
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::remove_dir(parent) {
            if !matches!(
                error.kind(),
                ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty
            ) {
                return (true, Some(error.to_string()));
            }
        }
    }
    (true, None)
}
