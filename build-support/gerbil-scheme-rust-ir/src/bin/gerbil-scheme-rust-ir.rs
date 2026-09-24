//! Build-time command for lowering a versioned Scheme function IR file.

use gerbil_scheme_rust_ir::compile_function_json;
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let (Some(input), Some(output), None) = (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: gerbil-scheme-rust-ir INPUT.json OUTPUT.rs");
        return ExitCode::FAILURE;
    };
    let result = (|| {
        let input = fs::read_to_string(input)?;
        let source = compile_function_json(&input)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        fs::write(output, source)?;
        Ok::<_, std::io::Error>(())
    })();
    if let Err(error) = result {
        eprintln!("Scheme Rust IR compilation failed: {error}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
