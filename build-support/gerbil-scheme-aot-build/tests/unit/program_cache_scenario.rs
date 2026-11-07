// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use super::{
    CFunctionLayout, ProgramArchiveObservation, ProgramArchiveObserver, compile_program_module,
};
use std::fs;
use std::sync::Mutex;

const INPUT: &str = include_str!("scenarios/program-content-cache/inputs/module.scm");

#[test]
fn module_function_layout_is_explicit_and_unknown_layouts_refuse() {
    let mut value =
        serde_json::json!({"module":"example/module","scm":"module.scm","system":false});
    let baseline: super::ProgramModule = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(baseline.host_layout, CFunctionLayout::CompilerDefault);
    value["host_layout"] = "separate-procedures".into();
    let separate: super::ProgramModule = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(separate.host_layout, CFunctionLayout::SeparateProcedures);
    value["host_layout"] = "unknown-layout".into();
    assert!(serde_json::from_value::<super::ProgramModule>(value).is_err());
}

#[test]
fn unchanged_compiler_path_does_not_cache_changed_sdk_abi_or_driver() {
    let root = std::env::temp_dir().join(format!(
        "gerbil-sdk-input-cache-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for directory in ["bin", "include", "lib"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    let compiler = root.join("bin/gsc");
    let header = root.join("include/gambit.h");
    let driver = root.join("bin/gambuild-C");
    let library = root.join("lib/libgambit.a");
    let source = root.join("module.c");
    fs::write(&compiler, "unchanged compiler").unwrap();
    fs::write(&header, "#define ___MAX_PROCESSORS 1").unwrap();
    fs::write(&driver, "configured compiler policy").unwrap();
    fs::write(&library, "runtime archive").unwrap();
    fs::write(&source, "unchanged module").unwrap();
    let fingerprint = || {
        super::native_input_fingerprint(&compiler, "sdk-regression", "-O2", &source)
            .unwrap()
            .unwrap()
    };
    let original = fingerprint();
    assert_eq!(original, fingerprint());
    fs::write(&header, "#define ___MAX_PROCESSORS 64").unwrap();
    let changed_abi = fingerprint();
    assert_ne!(original, changed_abi);
    fs::write(&driver, "different configured compiler policy").unwrap();
    assert_ne!(changed_abi, fingerprint());
    assert_eq!(
        crate::discovery::gambit_sdk_inputs(&compiler),
        vec![compiler, header, driver, library]
    );
    fs::remove_dir_all(root).unwrap();
}

struct Observations(Mutex<Vec<String>>);

impl ProgramArchiveObserver for Observations {
    fn observe(&self, observation: ProgramArchiveObservation<'_>) {
        self.0
            .lock()
            .expect("observation lock")
            .push(observation.to_string());
    }
}

#[test]
fn c_staging_keeps_manifest_order_and_rejects_shared_output_names() {
    let root = std::env::temp_dir().join(format!("gerbil-c-stage-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let compiler = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/unit/scenarios/program-content-cache/inputs/fake-gsc.sh");
    let stub = root.join("entry.scm");
    fs::write(&stub, INPUT).unwrap();
    let mut plan = super::ProgramManifest {
        schema: String::new(),
        modules: ["gamma", "beta", "delta", "alpha"]
            .into_iter()
            .map(|name| {
                let scm = root.join(format!("{name}.scm"));
                fs::write(&scm, INPUT).unwrap();
                super::ProgramModule {
                    module: name.into(),
                    scm,
                    system: false,
                    deferred: false,
                    host_layout: CFunctionLayout::CompilerDefault,
                }
            })
            .collect(),
        stub,
        library_dir: root.clone(),
        link_options: vec![],
    };
    let output = root.join("output");
    let request = super::ProgramArchiveRequest {
        manifest: &root.join("unused.json"),
        gsc: &compiler,
        archive_name: "test",
        linker_name: "test",
        out_dir: &output,
    };
    let observations = Observations(Mutex::new(Vec::new()));
    let staged = super::stage_program(&plan, request, &observations).unwrap();
    let ordered: Vec<_> = staged
        .compile_sources
        .iter()
        .map(|job| {
            assert!(job.source.is_file());
            job.module.as_str()
        })
        .collect();
    assert_eq!(ordered, ["gamma", "beta", "delta", "alpha"]);
    assert_eq!(staged.objects[0], output.join("module_0.o"));

    plan.modules[1].scm = plan.modules[0].scm.with_extension("ss");
    fs::write(&plan.modules[1].scm, INPUT).unwrap();
    let before = fs::read(output.join("compiler.log")).unwrap();
    assert!(matches!(
        super::stage_program(&plan, request, &observations),
        Err(error) if error.contains("conflicting staged C output")
    ));
    assert_eq!(fs::read(output.join("compiler.log")).unwrap(), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unchanged_native_input_reuses_content_addressed_output() {
    let root = std::env::temp_dir().join(format!(
        "gerbil-native-content-cache-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create cache scenario root");
    let log = root.join("compiler.log");
    // Linux CI rejected the newly written executable with ETXTBSY. Keep this
    // compiler fixture immutable during parallel test execution.
    let compiler = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/unit/scenarios/program-content-cache/inputs/fake-gsc.sh");
    let input = root.join("module.scm");
    let output = root.join("module.o");
    fs::write(&input, INPUT).expect("write first input");
    let header = root.join("bridge.h");
    fs::write(&header, "#define BRIDGE_VERSION 1\n").expect("write header input");
    let header_files = [header.clone()];
    let header_input = [super::NativeHeaderInput {
        include_directory: &root,
        header_files: &header_files,
    }];
    let observations = Observations(Mutex::new(Vec::new()));
    let options = super::native_compile_options(&header_input, &observations)
        .expect("validate header inputs");

    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &options,
        CFunctionLayout::CompilerDefault,
        &observations,
    )
    .expect("initial native compilation");
    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &options,
        CFunctionLayout::CompilerDefault,
        &observations,
    )
    .expect("cached native compilation");
    fs::write(&input, "changed input").expect("change native input");
    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &options,
        CFunctionLayout::CompilerDefault,
        &observations,
    )
    .expect("invalidated native compilation");

    fs::write(&header, "#define BRIDGE_VERSION 2\n").expect("change only header bytes");
    let changed = super::native_compile_options(&header_input, &observations)
        .expect("validate changed header inputs");
    compile_program_module(
        &compiler,
        &input,
        &output,
        "example/module",
        &changed,
        CFunctionLayout::CompilerDefault,
        &observations,
    )
    .expect("header-only change invalidates native object");
    for _ in 0..2 {
        compile_program_module(
            &compiler,
            &input,
            &output,
            "example/module",
            &changed,
            CFunctionLayout::SeparateProcedures,
            &observations,
        )
        .expect("layout change invalidates once, then reuses the matching object");
    }
    let outside = root.join("nested");
    fs::create_dir(&outside).expect("create separate include root");
    assert!(
        super::native_compile_options(
            &[super::NativeHeaderInput {
                include_directory: &outside,
                header_files: &header_files,
            }],
            &observations
        )
        .is_err()
    );
    fs::remove_file(&header).expect("remove declared header");
    assert!(super::native_compile_options(&header_input, &observations).is_err());

    assert_eq!(
        fs::read_to_string(&log)
            .expect("read compiler log")
            .lines()
            .count(),
        4,
        "unchanged inputs cache; source, header and function-layout changes recompile"
    );
    assert!(
        observations
            .0
            .into_inner()
            .expect("observation lock")
            .iter()
            .any(|row| row
                == "phase=native-object state=cached operation=compile program module subject=example/module")
    );
    fs::remove_dir_all(root).expect("remove cache scenario root");
}

#[test]
#[ignore = "requires owned generated module C and the configured GCC SDK"]
fn actual_gcc_module_completes_with_bounded_load_motion_policy() {
    let source = std::path::PathBuf::from(
        std::env::var_os("GERBIL_MODULE_DIAGNOSTIC_SOURCE").expect("owned module source"),
    );
    let gsc = std::path::PathBuf::from(
        std::env::var_os("GERBIL_GSC").expect("configured Gambit compiler"),
    );
    assert!(super::super::process::configured_gambit_gcc(&gsc));
    let root = std::env::temp_dir().join(format!("native-actual-module-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let object = root.join("module.o");
    compile_program_module(
        &gsc,
        &source,
        &object,
        "owned/diagnostic-module",
        "-O2 -Dmain=mrr_grammar_gambit_main",
        CFunctionLayout::CompilerDefault,
        &Observations(Mutex::new(Vec::new())),
    )
    .unwrap();
    assert!(object.is_file());
    assert!(!object.with_extension("i").exists());
    assert!(!object.with_extension("s").exists());
    std::fs::remove_dir_all(root).unwrap();
}
