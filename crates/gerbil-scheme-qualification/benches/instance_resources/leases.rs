//! Native qualification owns a real file until the library publishes clean join.
use std::{fs, path::PathBuf, sync::Arc};
#[derive(Debug)]
pub(super) struct FileLease(PathBuf);
impl Drop for FileLease {
    fn drop(&mut self) {
        fs::remove_file(&self.0).expect("retire this qualification-owned file");
    }
}
pub(super) fn owned_file(cycle: usize) -> (Arc<FileLease>, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "gerbil-owner-resource-{}-{cycle}",
        std::process::id()
    ));
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    (Arc::new(FileLease(path.clone())), path)
}
