//! Full Gerbil graph coverage, separate from the minimal Gambit scalar archive.
#![cfg(feature = "actor-aot")]
use gerbil_scheme::{GerbilRuntime, ObjectKind, RootedObjectBatch, SchemeScalar, TableEquality};
unsafe extern "C" {
    fn gerbil_objects_echo(root: i64) -> i64;
    fn gerbil_objects_reject(root: i64) -> i64;
}
#[test]
fn full_graph_root_exports_preserve_types_and_contain_failures() {
    #[cfg(unix)]
    let flags = stdio_flags();
    if std::env::var_os("GERBIL_SCHEME_RUST_STDIO_CHILD").is_none() {
        let executable = std::env::current_exe().unwrap();
        let mut children = (0..4)
            .map(|_| {
                std::process::Command::new(&executable)
                    .args([
                        "--exact",
                        "full_graph_root_exports_preserve_types_and_contain_failures",
                        "--nocapture",
                    ])
                    .env("GERBIL_SCHEME_RUST_STDIO_CHILD", "1")
                    .spawn()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        for child in &mut children {
            assert!(child.wait().unwrap().success());
        }
        #[cfg(unix)]
        assert_eq!(flags, stdio_flags());
        return;
    }
    let runtime =
        GerbilRuntime::initialize_program(gerbil_scheme_qualification::linked_program()).unwrap();
    #[cfg(unix)]
    {
        assert_eq!(flags, stdio_flags());
        assert_eq!(
            std::process::Command::new("/bin/sh")
                .args(["-c", "exit 7"])
                .status()
                .unwrap()
                .code(),
            Some(7)
        );
    }
    // SAFETY: build.rs links these process-lived exports into this native graph;
    // each borrows its argument and uses the upstream fresh-root/catch helper.
    let echo = unsafe { runtime.bind_object_export(gerbil_objects_echo) }.unwrap();
    let reject = unsafe { runtime.bind_object_export(gerbil_objects_reject) }.unwrap();
    let value = runtime.scalar_object(SchemeScalar::Boolean(false)).unwrap();
    assert!(reject.call(&value).into_result().is_err());
    assert_eq!(
        echo.call(&value)
            .into_result()
            .unwrap()
            .as_ref()
            .scalar()
            .unwrap(),
        SchemeScalar::Boolean(false)
    );
    let key = runtime.string_from_utf8("原生\0🦀").unwrap().into_object();
    let table = runtime
        .gerbil_hash_table(TableEquality::Structural, &[(&key, &value)])
        .unwrap();
    assert_eq!(table.as_ref().kind().unwrap(), ObjectKind::GerbilHashTable);
    let output = echo.call(&table).into_result().unwrap();
    #[cfg(unix)]
    assert_eq!(flags, stdio_flags());
    drop(table);
    drop(key);
    drop(value);
    let mut entries = RootedObjectBatch::default();
    output.copy_table_entries(&mut entries).unwrap();
    drop(output);
    assert_eq!(entries.len(), 2);
    assert_eq!(
        entries
            .root_element(0)
            .unwrap()
            .into_string()
            .unwrap()
            .to_string()
            .into_result()
            .unwrap(),
        "原生\0🦀"
    );
    assert_eq!(
        entries.get(1).unwrap().scalar().unwrap(),
        SchemeScalar::Boolean(false)
    );
    assert!(
        reject
            .call(&entries.root_element(1).unwrap())
            .into_result()
            .is_err()
    );
    assert_eq!(
        echo.call(&entries.root_element(1).unwrap())
            .into_result()
            .unwrap()
            .as_ref()
            .scalar()
            .unwrap(),
        SchemeScalar::Boolean(false)
    );
}

#[cfg(unix)]
fn stdio_flags() -> [(i32, i32); 3] {
    [0, 1, 2].map(|fd| {
        // SAFETY: query-only calls on the host standard descriptors.
        unsafe {
            (
                libc::fcntl(fd, libc::F_GETFL),
                libc::fcntl(fd, libc::F_GETFD),
            )
        }
    })
}
