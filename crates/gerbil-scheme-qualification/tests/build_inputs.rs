use std::path::Path;

#[path = "../build/inputs.rs"]
mod inputs;

#[test]
fn original_sdk_and_producer_inputs_are_not_self_invalidating_outputs() {
    let out = Path::new("/build/cargo/out");
    for output in [
        "/build/cargo/out/stage/program.scm",
        "/build/cargo/out/gerbil-path/lib/static/module.scm",
    ] {
        assert!(!inputs::external_input(Path::new(output), out));
    }
    for source in [
        "/sdk/include/gambit.h",
        "/sdk/lib/static/runtime.scm",
        "/source/scheme/actors.ss",
        "/build/cargo/output/source.scm",
    ] {
        assert!(inputs::external_input(Path::new(source), out));
    }
}
