//! Actual native process forwarding and failure propagation.
use super::run_process;
use std::process::Command;

struct ReadyWriter<'a, W>(&'a mut W);
impl<W: std::io::Write + Unpin> tokio::io::AsyncWrite for ReadyWriter<'_, W> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::task::Poll::Ready(self.0.write(bytes))
    }
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(self.0.flush())
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

fn emit_compiler_artifact_event(
    output: &mut (impl std::io::Write + Unpin),
    path: &std::path::Path,
    size: u64,
) -> Result<(), String> {
    super::process_runtime::acquire()?.block_on(super::emit_compiler_artifact_event(
        &mut ReadyWriter(output),
        path,
        size,
    ))
}

fn write_compiler_chunk(
    output: &mut (impl std::io::Write + Unpin),
    bytes: &[u8],
) -> std::io::Result<()> {
    super::process_runtime::acquire()
        .map_err(std::io::Error::other)?
        .block_on(super::write_compiler_chunk(&mut ReadyWriter(output), bytes))
}

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
fn tokio_runtime_releases_threads_after_last_native_child() {
    const CHILD: &str = "GERBIL_PROCESS_THREAD_RECEIPT";
    if std::env::var_os(CHILD).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "program::process::process_tests::tokio_runtime_releases_threads_after_last_native_child", "--nocapture"])
            .env(CHILD, "1").output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let threads = || {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("ps")
                .args(["-M", "-p", &std::process::id().to_string(), "-o", "pid="])
                .output()
                .unwrap();
            assert!(output.status.success());
            String::from_utf8(output.stdout)
                .unwrap()
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        }
        #[cfg(not(target_os = "macos"))]
        {
            std::fs::read_dir("/proc/self/task").unwrap().count()
        }
    };
    let before = threads();
    for _ in 0..4 {
        let runtime = super::process_runtime::acquire().unwrap();
        assert_eq!(
            runtime.worker_count(),
            std::thread::available_parallelism().unwrap().get()
        );
        assert!(threads() >= before + runtime.worker_count());
        let weak = std::sync::Arc::downgrade(&runtime);
        drop(runtime);
        assert!(weak.upgrade().is_none());
        run_process(
            Command::new("printf").arg("actual-native-output\n"),
            "thread receipt",
            true,
        )
        .unwrap();
        // /proc task-directory removal can lag pthread completion. Observe
        // actual kernel quiescence within the existing five-second gate;
        // a persistent extra thread still fails with the original exact count.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while threads() != before && std::time::Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(
            threads(),
            before,
            "Tokio threads must be joined at the last build lease"
        );
    }
    eprintln!(
        "TOKIO-PROCESS-RECEIPT cpu={} threads-before={before} threads-after={}",
        std::thread::available_parallelism().unwrap(),
        threads()
    );
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
    let error = emit_compiler_artifact_event(
        &mut ClosedDiagnosticPipe,
        std::path::Path::new("module.i"),
        262_144,
    )
    .unwrap_err();
    assert!(error.contains("write compiler artifact progress"));
    assert!(error.to_lowercase().contains("broken pipe"));
}

#[test]
fn nested_tokio_runtime_is_refused_without_detached_work() {
    let owner = super::process_runtime::acquire().unwrap();
    owner.block_on(async {
        let result = super::process_runtime::acquire();
        assert!(matches!(result, Err(error) if error.contains("cannot nest")));
    });
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
    for path in super::compiler_artifact_paths(&object) {
        std::fs::write(path, b"stale").unwrap();
    }
    let error = super::run_with_compiler_artifacts(
        Command::new("sh")
            .args([
                "-c",
                "test ! -e \"$1\" || exit 8; printf new > \"$1\"; exit 7",
                "fixture",
            ])
            .arg(object.with_extension("reload")),
        "compile artifact fixture",
        &object,
    )
    .unwrap_err();
    assert!(error.contains('7'));
    assert!(!object.with_extension("i").exists());
    assert!(!object.with_extension("s").exists());
    assert!(!object.with_extension("ira").exists());
    assert!(!object.with_extension("reload").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "six-second compiler-written artifact regression under five-second idle supervision"]
fn compiler_artifact_events_forward_actual_growth_and_cleanup() {
    let root = artifact_fixture("live");
    let object = root.join("program_link.o");
    super::run_with_compiler_artifacts(
        Command::new("sh").args(["-c", "dd if=/dev/zero of=\"$1\" bs=262144 count=1 2>/dev/null; sleep 3; dd if=/dev/zero of=\"$1\" bs=262144 count=2 2>/dev/null; sleep 3", "fixture"]).arg(object.with_extension("reload")),
        "compile artifact fixture",
        &object,
    ).unwrap();
    for path in super::compiler_artifact_paths(&object) {
        assert!(!path.exists());
    }
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
    assert!(!object.with_extension("ira").exists());
    assert!(!object.with_extension("reload").exists());
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
    emit_compiler_artifact_event(&mut output, std::path::Path::new("module.i"), 262_144).unwrap();
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
    emit_compiler_artifact_event(&mut output, std::path::Path::new("module.i"), 262_144).unwrap();
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

#[test]
fn gcc_allocation_dump_options_preserve_exact_shell_paths() {
    let object = std::path::Path::new("directory with spaces/owner's.o");
    let options = super::gcc_allocation_diagnostics(object).unwrap();
    let output = Command::new("sh")
        .args(["-c", &format!("printf '%s\\n' {options}")])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "-fdump-rtl-ira-details=directory with spaces/owner's.ira\n-fdump-rtl-reload-details=directory with spaces/owner's.reload\n"
    );
}

#[test]
#[ignore = "requires an owned generated C input and explicit GCC; native build supervision"]
fn actual_gcc_allocation_dump_progress_completes() {
    let source =
        std::env::var_os("GERBIL_ALLOCATION_DIAGNOSTIC_SOURCE").expect("owned generated C input");
    let compiler =
        std::env::var_os("GERBIL_ALLOCATION_DIAGNOSTIC_CC").expect("explicit diagnostic GCC");
    let root = artifact_fixture("actual-allocation");
    let object = root.join("module.o");
    let mut command = Command::new(compiler);
    command.args([
        "-O2",
        "-fno-gcse-lm",
        "-save-temps=obj",
        "-Q",
        "-fopt-info-all",
    ]);
    for extension in ["ira", "reload"] {
        command.arg(format!(
            "-fdump-rtl-{extension}-details={}",
            object.with_extension(extension).display()
        ));
    }
    command.arg("-c").arg(source).arg("-o").arg(&object);
    super::run_with_compiler_artifacts(
        &mut command,
        "compile actual allocation diagnostic",
        &object,
    )
    .unwrap();
    assert!(object.is_file());
    for path in super::compiler_artifact_paths(&object) {
        assert!(!path.exists());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicitly selected native GCC"]
fn allocation_diagnostics_preserve_optimized_object_bytes() {
    let compiler =
        std::env::var_os("GERBIL_ALLOCATION_DIAGNOSTIC_CC").expect("explicit diagnostic GCC");
    let root = artifact_fixture("diagnostic-object-parity");
    let source = root.join("fixture.c");
    std::fs::write(&source, "unsigned long fold(const unsigned long *p, unsigned long n) { unsigned long s=0; for (unsigned long i=0;i<n;++i) s=(s*33)^p[i]; return s; }\n").unwrap();
    let baseline = root.join("baseline.o");
    let object = root.join("observed.o");
    let status = Command::new(&compiler)
        .args(["-O2", "-fno-gcse-lm", "-c"])
        .arg(&source)
        .arg("-o")
        .arg(&baseline)
        .status()
        .unwrap();
    assert!(status.success());
    let mut command = Command::new(&compiler);
    command.args(["-O2", "-fno-gcse-lm", "-save-temps=obj"]);
    for extension in ["ira", "reload"] {
        command.arg(format!(
            "-fdump-rtl-{extension}-details={}",
            object.with_extension(extension).display()
        ));
    }
    command.arg("-c").arg(&source).arg("-o").arg(&object);
    super::run_with_compiler_artifacts(&mut command, "observe optimized object", &object).unwrap();
    assert_eq!(
        std::fs::read(&baseline).unwrap(),
        std::fs::read(&object).unwrap()
    );
    for path in super::compiler_artifact_paths(&object) {
        assert!(!path.exists());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn compiler_fragment_is_framed_without_waiting_for_the_next_record() {
    let mut output = Vec::new();
    write_compiler_chunk(&mut output, b"partial compiler diagnostic").unwrap();
    assert_eq!(output, b"partial compiler diagnostic\n");
    let before = output.len();
    write_compiler_chunk(&mut output, b"").unwrap();
    assert_eq!(output.len(), before, "silence cannot fabricate progress");
    write_compiler_chunk(&mut output, b"complete\n").unwrap();
    assert_eq!(&output[before..], b"complete\n");
}

#[test]
fn cargo_observes_fragment_before_compiler_child_can_finish() {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;
    use std::sync::atomic::{AtomicBool, Ordering};

    let root = artifact_fixture("cargo-framing");
    let release = root.join("release");
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "").unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"build-progress-framing-fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n[build-dependencies]\ntokio = { version = \"=1.53.1\", features = [\"full\"] }\n").unwrap();
    let process = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/program/process.rs");
    let parallelism = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/parallelism.rs");
    std::fs::write(root.join("build.rs"), format!(r#"
#[path = {parallelism:?}]
mod parallelism;
fn gerbil_command(program: &std::path::Path) -> std::process::Command {{
    std::process::Command::new(program)
}}
#[allow(dead_code)]
#[path = {process:?}]
mod compiler_process;
fn main() {{
    use std::io::Write;
    let release = std::path::PathBuf::from(std::env::var_os("FIXTURE_RELEASE").unwrap());
    if std::env::args().any(|arg| arg == "--compiler-fixture") {{
        print!("compiler-fragment-before-exit");
        std::io::stdout().flush().unwrap();
        let started = std::time::Instant::now();
        while !release.exists() {{
            assert!(started.elapsed() < std::time::Duration::from_secs(5), "Cargo withheld compiler fragment");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }}
        eprintln!("compiler-finished-after-receipt");
        return;
    }}
    compiler_process::run_native_process(
        std::process::Command::new(std::env::current_exe().unwrap()).arg("--compiler-fixture"),
        "compiler fragment fixture", true).unwrap();
}}
"#)).unwrap();
    let mut child = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["build", "--offline", "--verbose", "--verbose"])
        .current_dir(&root)
        .env("FIXTURE_RELEASE", &release)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let observed = AtomicBool::new(false);
    let completed = AtomicBool::new(false);
    let status = std::thread::scope(|scope| {
        let read = |input: Box<dyn std::io::Read + Send>| {
            let mut transcript = String::new();
            for line in BufReader::new(input).lines() {
                let line = line.unwrap();
                if line.contains("] compiler-fragment-before-exit") {
                    observed.store(true, Ordering::SeqCst);
                    std::fs::write(&release, b"observed actual Cargo output").unwrap();
                }
                if line.contains("] compiler-finished-after-receipt") {
                    completed.store(true, Ordering::SeqCst);
                }
                transcript.push_str(&line);
                transcript.push('\n');
            }
            transcript
        };
        let out = scope.spawn(move || read(Box::new(stdout)));
        let err = scope.spawn(move || read(Box::new(stderr)));
        let status = child.wait().unwrap();
        (status, out.join().unwrap(), err.join().unwrap())
    });
    assert!(
        status.0.success(),
        "Cargo fixture failed; retained at {}\n{}{}",
        root.display(),
        status.1,
        status.2
    );
    assert!(observed.load(Ordering::SeqCst));
    assert!(completed.load(Ordering::SeqCst));
    // The child itself enforces the unchanged five-second observation bound.
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires GERBIL_GSC selecting the actual paired Gambit compiler"]
fn actual_gambit_diagnostic_modes_preserve_object_bytes() {
    let gsc = std::path::PathBuf::from(std::env::var_os("GERBIL_GSC").expect("paired compiler"));
    assert!(super::configured_gambit_gcc(&gsc));
    let root = artifact_fixture("paired-gambit-diagnostics");
    let scheme = root.join("module.scm");
    let source = root.join("module.c");
    std::fs::write(&scheme, "(define (squared x) (* x x))\n").unwrap();
    run_process(
        super::gambit_progress_command_with_mode(&gsc, true)
            .args(["-c", "-o"])
            .arg(&source)
            .arg(&scheme),
        "generate actual Scheme fixture",
        true,
    )
    .unwrap();
    assert!(
        source.is_file(),
        "GSC status alone cannot establish generated C"
    );
    let mut original = None;
    for (name, progress, detailed) in [
        ("baseline", false, false),
        ("progress", true, false),
        ("detailed", true, true),
    ] {
        let object = root.join(format!("{name}.o"));
        let options = super::super::archive::compiler_diagnostic_options(
            "-O2 -fno-gcse-lm",
            progress,
            detailed,
        );
        run_process(
            super::gambit_progress_command_with_mode(&gsc, progress)
                .args(["-obj", "-cc-options", &options, "-o"])
                .arg(&object)
                .arg(&source),
            "compile actual Scheme fixture",
            true,
        )
        .unwrap();
        let bytes = std::fs::read(&object).expect("GSC success requires an actual object");
        assert!(!bytes.is_empty());
        if let Some(expected) = &original {
            assert_eq!(
                &bytes,
                expected,
                "diagnostics changed object identity; retained at {}",
                root.display()
            );
        } else {
            original = Some(bytes);
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
