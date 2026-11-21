//! One execution-host capacity for build workers and Scheme child admission.

pub(crate) fn host_build_parallelism() -> usize {
    std::thread::available_parallelism()
        .expect("read available compiler host parallelism")
        .get()
}
