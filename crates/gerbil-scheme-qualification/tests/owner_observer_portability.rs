//! Compile the actual embedded C without Darwin APIs; unsupported facts fail closed.
use std::{fs, path::PathBuf, process::Command, time::SystemTime};

#[test]
fn unavailable_kernel_observer_rejects_start_and_readings() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "gerbil-owner-observer-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&directory).unwrap();
    let source = directory.join("contract.c");
    let executable = directory.join("contract");
    // Include host headers before removing the platform selector. On Darwin,
    // this tests the unsupported branch without pretending to be a Linux SDK.
    let mut c = String::from(
        "#include <assert.h>\n#include <stdint.h>\n#include <errno.h>\n\
         #include <time.h>\n#undef __APPLE__\n",
    );
    for file in ["instance-mailbox-observation.ss", "instance-owner-watch.ss"] {
        let scheme = fs::read_to_string(root.join("scheme").join(file)).unwrap();
        c.push_str(
            scheme
                .split_once("(c-declare #<<c-end\n")
                .unwrap()
                .1
                .split_once("\nc-end")
                .unwrap()
                .0,
        );
        c.push('\n');
    }
    c.push_str(
        "int main(void) {\n\
         assert(mailbox_clock_now() != UINT64_MAX);\n\
         assert(mailbox_cpu_now() != UINT64_MAX);\n\
         assert(mailbox_owner_watch_start(0, 0) == ENOTSUP);\n\
         assert(mailbox_owner_watch_stop() == ENOTSUP);\n\
         assert(GERBIL_MAILBOX_OWNER_READING(mailbox_owner_watch.active) == UINT64_MAX);\n\
         gerbil_mailbox_native_progress(0, 1);\n\
         gerbil_mailbox_native_wait_enter();\n\
         gerbil_mailbox_native_wait_exit();\n\
         return 0;\n}\n",
    );
    fs::write(&source, c).unwrap();
    let compiled = Command::new("cc")
        .args([
            "-std=c11",
            "-D_POSIX_C_SOURCE=200809L",
            "-O2",
            "-Wall",
            "-Wextra",
            "-Werror",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let result = Command::new(&executable).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}
