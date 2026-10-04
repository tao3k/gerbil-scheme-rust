//! Actual native process forwarding and failure propagation.
use super::run_process;
use std::process::Command;

#[test]
fn quiet_failure_retains_both_diagnostic_streams() {
    let error = run_process(
        Command::new("sh").args(["-c", "echo output; echo diagnostic >&2; exit 7"]),
        "compile fixture",
        false,
    )
    .unwrap_err();
    assert!(error.contains("compile fixture"));
    assert!(error.contains("output"));
    assert!(error.contains("diagnostic"));
    assert!(error.contains('7'));
}

#[test]
fn live_failure_preserves_nonzero_status() {
    let error = run_process(
        Command::new("sh").args(["-c", "echo diagnostic >&2; exit 7"]),
        "compile fixture",
        true,
    )
    .unwrap_err();
    assert!(error.contains("compile fixture"));
    assert!(error.contains('7'));
    assert!(error.contains("streamed"));
}

#[test]
#[ignore = "six-second real child-output regression; run under five-second idle supervision"]
fn live_child_forwards_real_events_before_exit() {
    run_process(
        Command::new("sh").args([
            "-c",
            "echo fixture-start; sleep 3; echo fixture-middle; sleep 3; echo fixture-end",
        ]),
        "compile fixture",
        true,
    )
    .unwrap();
}

#[test]
fn verbose_gambit_runtime_options_precede_compiler_options_and_preserve_heap_defaults() {
    let quiet = super::gambit_progress_command_with_mode(std::path::Path::new("gsc"), false);
    assert_eq!(quiet.get_args().count(), 0);
    let mut live = super::gambit_progress_command_with_mode(std::path::Path::new("gsc"), true);
    live.args(["-c", "input.scm"]);
    let args = live
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(args, ["-:1n,2n,d5qQ", "-verbose", "-c", "input.scm"]);
}

fn artifact_fixture(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "native-artifact-{name}-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn compiler_artifact_diagnostic_write_failure_is_typed_without_panicking() {
    struct ClosedDiagnosticPipe;
    impl std::io::Write for ClosedDiagnosticPipe {
        fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = super::emit_compiler_artifact_event(
        &mut ClosedDiagnosticPipe,
        std::path::Path::new("module.i"),
        262_144,
    )
    .unwrap_err();
    assert!(error.contains("write compiler artifact progress"));
    assert!(error.to_lowercase().contains("broken pipe"));
}

#[test]
fn compiler_monitor_retains_actual_thread_panic_cause() {
    let payload = std::thread::spawn(|| panic!("fixture monitor cause"))
        .join()
        .unwrap_err();
    assert_eq!(
        super::compiler_monitor_panic_message(payload),
        "compiler artifact monitor panicked: fixture monitor cause"
    );
}

#[test]
fn artifact_events_require_new_bytes_and_do_not_repeat_for_stalled_files() {
    let root = artifact_fixture("growth");
    let path = root.join("program_link.i");
    let mut observed = 0;
    assert_eq!(
        super::compiler_artifact_growth(&path, &mut observed).unwrap(),
        None
    );
    std::fs::write(
        &path,
        vec![0; super::COMPILER_ARTIFACT_PROGRESS_BYTES as usize],
    )
    .unwrap();
    assert_eq!(
        super::compiler_artifact_growth(&path, &mut observed).unwrap(),
        Some(super::COMPILER_ARTIFACT_PROGRESS_BYTES)
    );
    assert_eq!(
        super::compiler_artifact_growth(&path, &mut observed).unwrap(),
        None
    );
    std::fs::write(&path, b"short").unwrap();
    assert_eq!(
        super::compiler_artifact_growth(&path, &mut observed).unwrap(),
        None
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_compiler_artifact_run_cleans_stale_and_new_intermediates() {
    let root = artifact_fixture("failure");
    let object = root.join("program_link.o");
    std::fs::write(object.with_extension("i"), b"stale").unwrap();
    let error = super::run_with_compiler_artifacts(
        Command::new("sh")
            .args([
                "-c",
                "test ! -e \"$1\" || exit 8; printf new > \"$1\"; exit 7",
                "fixture",
            ])
            .arg(object.with_extension("i")),
        "compile artifact fixture",
        &object,
    )
    .unwrap_err();
    assert!(error.contains('7'));
    assert!(!object.with_extension("i").exists());
    assert!(!object.with_extension("s").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "six-second compiler-written artifact regression under five-second idle supervision"]
fn compiler_artifact_events_forward_actual_growth_and_cleanup() {
    let root = artifact_fixture("live");
    let object = root.join("program_link.o");
    super::run_with_compiler_artifacts(
        Command::new("sh").args(["-c", "dd if=/dev/zero of=\"$1\" bs=262144 count=1 2>/dev/null; sleep 3; dd if=/dev/zero of=\"$1\" bs=262144 count=2 2>/dev/null; sleep 3", "fixture"]).arg(object.with_extension("i")),
        "compile artifact fixture",
        &object,
    ).unwrap();
    assert!(!object.with_extension("i").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires the exact owned generated linker C; supervise as a native build"]
fn actual_gcc_linker_emits_artifacts_and_completes() {
    let source = std::path::PathBuf::from(
        std::env::var_os("GERBIL_LINKER_DIAGNOSTIC_SOURCE").expect("owned linker source"),
    );
    let gsc = std::path::PathBuf::from(
        std::env::var_os("GERBIL_GSC").expect("configured Gambit compiler"),
    );
    assert!(super::configured_gambit_gcc(&gsc));
    let root = artifact_fixture("actual");
    let object = root.join("program_link.o");
    let mut command = super::gambit_progress_command_with_mode(&gsc, true);
    command
        .args([
            "-obj",
            "-cc-options",
            "-O2 -Dmain=mrr_grammar_gambit_main -save-temps=obj -Q",
            "-o",
        ])
        .arg(&object)
        .arg(&source);
    super::run_with_compiler_artifacts(&mut command, "compile actual linker", &object).unwrap();
    assert!(object.is_file());
    assert!(!object.with_extension("i").exists());
    assert!(!object.with_extension("s").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn progress_retains_partial_writes_and_retries_interrupted_write_and_flush() {
    use std::io::{ErrorKind, Write};
    #[derive(Default)]
    struct Backpressure {
        bytes: Vec<u8>,
        writes: usize,
        flushes: usize,
    }
    impl Write for Backpressure {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.writes += 1;
            match self.writes {
                2 => return Err(ErrorKind::Interrupted.into()),
                3 => return Err(ErrorKind::WouldBlock.into()),
                _ => {}
            }
            let count = bytes.len().min(3);
            self.bytes.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.flushes += 1;
            match self.flushes {
                1 => Err(ErrorKind::WouldBlock.into()),
                2 => Err(ErrorKind::Interrupted.into()),
                _ => Ok(()),
            }
        }
    }
    let mut output = Backpressure::default();
    super::emit_compiler_artifact_event(&mut output, std::path::Path::new("module.i"), 262_144)
        .unwrap();
    assert_eq!(
        output.bytes,
        b"native compiler artifact=module.i bytes=262144\n"
    );
    assert_eq!(output.flushes, 3);
}

#[test]
fn actual_nonblocking_full_pipe_delivers_event_without_loss_after_consumer_drains() {
    use std::io::{ErrorKind, Write};
    use std::os::fd::OwnedFd;
    use std::process::Stdio;
    let root = artifact_fixture("backpressure");
    let ready = root.join("drain");
    let receipt = root.join("received");
    let mut child = Command::new("python3")
        .args(["-c", "import pathlib,sys,time; flag=pathlib.Path(sys.argv[1]); receipt=pathlib.Path(sys.argv[2]);\nwhile not flag.exists(): time.sleep(.001)\nreceipt.write_bytes(sys.stdin.buffer.read())"])
        .arg(&ready).arg(&receipt).stdin(Stdio::piped()).spawn().unwrap();
    let mut pipe = std::fs::File::from(OwnedFd::from(child.stdin.take().unwrap()));
    // fcntl changes the shared open-file description through a duplicate;
    // neither the parent nor the reader process needs unsafe Rust code.
    assert!(Command::new("python3")
        .args(["-c", "import fcntl,os; fcntl.fcntl(0,fcntl.F_SETFL,fcntl.fcntl(0,fcntl.F_GETFL)|os.O_NONBLOCK)"])
        .stdin(Stdio::from(pipe.try_clone().unwrap())).status().unwrap().success());
    let filler = [b'x'; 4096];
    let mut filled = 0;
    loop {
        match pipe.write(&filler) {
            Ok(count) => {
                filled += count;
                assert!(filled < 16 * 1024 * 1024);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => break,
            other => panic!("fill nonblocking pipe: {other:?}"),
        }
    }
    struct ReleaseConsumer {
        pipe: std::fs::File,
        ready: std::path::PathBuf,
        blocked: bool,
    }
    impl Write for ReleaseConsumer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let result = self.pipe.write(bytes);
            if result
                .as_ref()
                .is_err_and(|e| e.kind() == ErrorKind::WouldBlock)
            {
                self.blocked = true;
                std::fs::write(&self.ready, b"drain")?;
            }
            result
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.pipe.flush()
        }
    }
    let mut output = ReleaseConsumer {
        pipe,
        ready,
        blocked: false,
    };
    super::emit_compiler_artifact_event(&mut output, std::path::Path::new("module.i"), 262_144)
        .unwrap();
    assert!(
        output.blocked,
        "the event must encounter real pipe backpressure"
    );
    drop(output);
    assert!(child.wait().unwrap().success());
    let received = std::fs::read(&receipt).unwrap();
    assert_eq!(&received[..filled], vec![b'x'; filled]);
    assert_eq!(
        &received[filled..],
        b"native compiler artifact=module.i bytes=262144\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn streaming_native_child_cannot_change_parent_stdio_file_status_flags() {
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;
    use std::process::Stdio;
    let (parent, _reader) = UnixStream::pair().unwrap();
    let check = || {
        Command::new("python3")
            .args([
                "-c",
                "import os,sys; sys.exit(0 if os.get_blocking(0) else 1)",
            ])
            .stdin(Stdio::from(OwnedFd::from(parent.try_clone().unwrap())))
            .status()
            .unwrap()
            .success()
    };
    assert!(check());
    super::run_process(Command::new("python3")
        .args(["-c", "import os; os.set_blocking(1,False); os.set_blocking(2,False); os.write(1,b'isolated-native-output\\n'); os.write(2,b'isolated-native-diagnostic\\n')"])
        .stdout(Stdio::from(OwnedFd::from(parent.try_clone().unwrap())))
        .stderr(Stdio::from(OwnedFd::from(parent.try_clone().unwrap()))),
        "native file-status fixture", true).unwrap();
    assert!(
        check(),
        "child flag mutation must not reach the parent's file description"
    );
}
