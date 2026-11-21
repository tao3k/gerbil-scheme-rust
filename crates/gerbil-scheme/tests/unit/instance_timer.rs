//! Image-local SDK heartbeat ownership, independent of application evaluators.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn compile(arguments: &[&std::ffi::OsStr]) {
    let output = Command::new("/usr/bin/clang")
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn private_image_timers_preserve_host_masking_and_retire_before_unload() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let scratch = Scratch(
        std::env::temp_dir().join(format!("gsr-image-timer-{}-{nonce}", std::process::id())),
    );
    fs::create_dir(&scratch.0).unwrap();
    let package = Path::new(env!("CARGO_MANIFEST_DIR"));
    let image = scratch.0.join("a.dylib");
    let other = scratch.0.join("b.dylib");
    let main = scratch.0.join("check");
    compile(&[
        "-dynamiclib".as_ref(),
        "-fvisibility=hidden".as_ref(),
        "-Wall".as_ref(),
        "-Wextra".as_ref(),
        "-Werror".as_ref(),
        "-I".as_ref(),
        package.join("../../ffi").as_os_str(),
        package
            .join("tests/fixtures/instance_timer_image.c")
            .as_os_str(),
        "-o".as_ref(),
        image.as_os_str(),
    ]);
    fs::copy(&image, &other).unwrap();
    compile(&[
        "-Wall".as_ref(),
        "-Wextra".as_ref(),
        "-Werror".as_ref(),
        package
            .join("tests/fixtures/instance_timer_main.c")
            .as_os_str(),
        "-o".as_ref(),
        main.as_os_str(),
    ]);
    let mut child = Command::new(main)
        .args([image, other])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("image timer isolation child exceeded its fixed deadline");
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        result.stdout,
        b"INSTANCE-TIMER-ISOLATION-OK host-mask-retirement-unload\n"
    );
}
