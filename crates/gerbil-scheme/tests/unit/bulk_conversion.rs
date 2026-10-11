// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later

use syn::visit::{self, Visit};

const SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/runtime/rooted.rs"
));

#[derive(Default)]
struct LoopLogging {
    loop_depth: usize,
    logs_inside_loop: usize,
}

impl<'ast> Visit<'ast> for LoopLogging {
    fn visit_expr_for_loop(&mut self, expression: &'ast syn::ExprForLoop) {
        self.loop_depth += 1;
        visit::visit_expr_for_loop(self, expression);
        self.loop_depth -= 1;
    }

    fn visit_expr_while(&mut self, expression: &'ast syn::ExprWhile) {
        self.loop_depth += 1;
        visit::visit_expr_while(self, expression);
        self.loop_depth -= 1;
    }

    fn visit_expr_loop(&mut self, expression: &'ast syn::ExprLoop) {
        self.loop_depth += 1;
        visit::visit_expr_loop(self, expression);
        self.loop_depth -= 1;
    }

    fn visit_macro(&mut self, expression: &'ast syn::Macro) {
        if self.loop_depth > 0
            && expression.path.segments.last().is_some_and(|segment| {
                matches!(
                    segment.ident.to_string().as_str(),
                    "print" | "println" | "eprint" | "eprintln"
                )
            })
        {
            self.logs_inside_loop += 1;
        }
        visit::visit_macro(self, expression);
    }
}

#[test]
fn utf8_benchmarks_do_not_log_inside_transfer_loops() {
    for source in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/phases/buffers.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/benches/phases/utf8.rs"
        )),
    ] {
        let mut logging = LoopLogging::default();
        logging.visit_file(&syn::parse_file(source).unwrap());
        assert_eq!(
            logging.logs_inside_loop, 0,
            "sample completion logs must not become per-transfer I/O"
        );
    }
    let negative = "fn batch() { for _ in 0..1000 { if true { eprintln!(\"progress\"); } } while false { println!(\"progress\"); } loop { eprint!(\"progress\"); break; } eprintln!(\"sample done\"); }";
    let mut logging = LoopLogging::default();
    logging.visit_file(&syn::parse_file(negative).unwrap());
    assert_eq!(
        logging.logs_inside_loop, 3,
        "negative control must catch nested progress logging"
    );
}

fn validation_precedes_owned_string(source: &str) -> bool {
    let file = syn::parse_file(source).unwrap();
    let Some(function) = file.items.iter().find_map(|item| match item {
        syn::Item::Fn(function) if function.sig.ident == "validated_string" => Some(function),
        _ => None,
    }) else {
        return false;
    };
    let [
        syn::Stmt::Expr(syn::Expr::Try(check), Some(_)),
        syn::Stmt::Expr(syn::Expr::Call(result), None),
    ] = function.block.stmts.as_slice()
    else {
        return false;
    };
    let syn::Expr::MethodCall(mapped) = &*check.expr else {
        return false;
    };
    let syn::Expr::Call(validator) = &*mapped.receiver else {
        return false;
    };
    let syn::Expr::Path(path) = &*validator.func else {
        return false;
    };
    let names: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect();
    if mapped.method != "map_err" || names != ["simdutf8", "basic", "from_utf8"] {
        return false;
    }
    if validator.args.len() != 1 || result.args.len() != 1 {
        return false;
    }
    let syn::Expr::Reference(input) = &validator.args[0] else {
        return false;
    };
    if !matches!(&*input.expr, syn::Expr::Path(path) if path.path.is_ident("bytes")) {
        return false;
    }
    let syn::Expr::Unsafe(construction) = &result.args[0] else {
        return false;
    };
    let [syn::Stmt::Expr(syn::Expr::Call(constructor), None)] = construction.block.stmts.as_slice()
    else {
        return false;
    };
    let syn::Expr::Path(path) = &*constructor.func else {
        return false;
    };
    let names: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|part| part.ident.to_string())
        .collect();
    names == ["String", "from_utf8_unchecked"]
        && constructor.args.len() == 1
        && matches!(&constructor.args[0], syn::Expr::Path(path) if path.path.is_ident("bytes"))
}

#[test]
fn owned_string_never_bypasses_the_required_utf8_validation() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/utf8.rs"));
    assert!(validation_precedes_owned_string(source));
    let negative = source.replace(
        "simdutf8::basic::from_utf8(&bytes)",
        "Ok::<&str, simdutf8::basic::Utf8Error>(\"\")",
    );
    assert_ne!(source, negative);
    // Parse the invalid implementation, never execute an unchecked String on
    // malformed input merely to prove the regression detector works.
    assert!(!validation_precedes_owned_string(&negative));
}

#[derive(Default)]
struct CopyShape {
    bulk_calls: usize,
    scalar_calls: usize,
    loops: usize,
}

impl<'ast> Visit<'ast> for CopyShape {
    fn visit_expr_call(&mut self, expression: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*expression.func {
            let name = path.path.segments.last().unwrap().ident.to_string();
            self.bulk_calls += usize::from(
                name.ends_with("bytevector_copy")
                    || name.ends_with("root_string_to_utf8")
                    || name.ends_with("root_string_encode_into")
                    || name.ends_with("scalar_batch")
                    || name.ends_with("collection_copy_raw")
                    || name.ends_with("collection_snapshot_raw")
                    || name.ends_with("bigint_copy_raw")
                    || name.ends_with("bigint_copy_indexed")
                    || name.ends_with("numeric_copy"),
            );
            self.scalar_calls += usize::from(
                name.ends_with("bytevector_u8_ref")
                    || name.ends_with("string_char_ref")
                    || name.contains("scheme_object_as_")
                    || name.ends_with("collection_element_raw")
                    || name.ends_with("object_scalar_raw")
                    || name.ends_with("object_scalar_indexed"),
            );
        }
        visit::visit_expr_call(self, expression);
    }

    fn visit_expr_method_call(&mut self, expression: &'ast syn::ExprMethodCall) {
        self.bulk_calls += usize::from(expression.method == "to_utf8_bytes");
        self.scalar_calls += usize::from(
            expression.method == "u8_at"
                || expression.method == "char_at"
                || matches!(
                    expression.method.to_string().as_str(),
                    "as_boolean" | "as_fixnum" | "as_char" | "as_flonum"
                ),
        );
        visit::visit_expr_method_call(self, expression);
    }

    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        self.loops += usize::from(matches!(
            expression,
            syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_)
        ));
        visit::visit_expr(self, expression);
    }
}

fn bulk_conversions_are_bounded(source: &str) -> bool {
    let file = syn::parse_file(source).expect("parse Rust conversion source");
    let mut checked = 0;
    for item in file.items {
        let syn::Item::Impl(implementation) = item else {
            continue;
        };
        let syn::Type::Path(path) = &*implementation.self_ty else {
            continue;
        };
        let name = &path.path.segments.last().unwrap().ident;
        if name != "SchemeBytevector"
            && name != "RootedSchemeBytevector"
            && name != "RootedSchemeString"
        {
            continue;
        }
        for item in implementation.items {
            let syn::ImplItem::Fn(method) = item else {
                continue;
            };
            if method.sig.ident != "to_vec"
                && method.sig.ident != "to_string"
                && method.sig.ident != "copy_to_vec"
                && method.sig.ident != "copy_into"
                && method.sig.ident != "to_utf8_bytes"
            {
                continue;
            }
            checked += 1;
            let mut shape = CopyShape::default();
            shape.visit_block(&method.block);
            if shape.bulk_calls != 1 || shape.scalar_calls != 0 || shape.loops != 0 {
                return false;
            }
        }
    }
    checked == 7
}

fn rooted_copy_is_bounded(source: &str, method_name: &str) -> bool {
    let file = syn::parse_file(source).expect("parse rooted copy source");
    let mut checked = 0;
    for item in file.items {
        let syn::Item::Impl(implementation) = item else {
            continue;
        };
        for item in implementation.items {
            let syn::ImplItem::Fn(method) = item else {
                continue;
            };
            if method.sig.ident != method_name {
                continue;
            }
            checked += 1;
            let mut shape = CopyShape::default();
            shape.visit_block(&method.block);
            if shape.bulk_calls != 1 || shape.scalar_calls != 0 || shape.loops != 0 {
                return false;
            }
        }
    }
    checked == 1
}

#[test]
fn rooted_collection_and_integer_copies_reject_scalar_ffi_loops() {
    let sources = [
        (
            include_str!("../../src/runtime/objects/values.rs"),
            "copy_into",
        ),
        (
            include_str!("../../src/runtime/objects/integer.rs"),
            "copy_big_integer",
        ),
    ];
    for (source, method) in sources {
        assert!(rooted_copy_is_bounded(source, method));
        let scalar = source
            .replace("collection_copy_raw", "collection_element_raw")
            .replace("collection_snapshot_raw", "collection_element_raw")
            .replace("bigint_copy_raw", "object_scalar_raw")
            .replace("bigint_copy_indexed", "object_scalar_indexed");
        assert!(!rooted_copy_is_bounded(&scalar, method));
        let looped = source.replacen(
            "let len = self.length;",
            "let len = self.length; for _ in 0..2 {}",
            1,
        );
        if method == "copy_into" {
            assert_ne!(looped, source);
            assert!(!rooted_copy_is_bounded(&looped, method));
        }
    }
}

#[test]
fn bulk_conversion_rejects_per_element_ffi_regressions() {
    assert!(
        bulk_conversions_are_bounded(SOURCE),
        "whole-value conversions must use one bulk entry, not per-element FFI calls"
    );
    // Negative controls prove that the gate catches both the old method-based
    // loop and replacement of the bulk ABI with a scalar ABI.
    let loop_regression = SOURCE.replacen(
        "pub fn to_vec(&self) -> NativeResult<Vec<u8>> {",
        "pub fn to_vec(&self) -> NativeResult<Vec<u8>> { for i in 0..5 { let _ = self.u8_at(i); }",
        1,
    );
    assert!(!bulk_conversions_are_bounded(&loop_regression));
    let scalar_regression = SOURCE.replacen(
        "gerbil_scheme_rust_scheme_object_bytevector_copy(",
        "gerbil_scheme_rust_scheme_object_bytevector_u8_ref(",
        1,
    );
    assert_ne!(
        scalar_regression, SOURCE,
        "negative control must mutate a call"
    );
    assert!(!bulk_conversions_are_bounded(&scalar_regression));
    let string_regression = SOURCE.replacen(
        "pub fn to_string(&self) -> NativeResult<String> {",
        "pub fn to_string(&self) -> NativeResult<String> { for i in 0..5 { let _ = self.char_at(i); }",
        1,
    );
    assert_ne!(string_regression, SOURCE);
    assert!(!bulk_conversions_are_bounded(&string_regression));
    let reuse_regression = SOURCE.replacen(
        "pub fn copy_to_vec(&self, output: &mut Vec<u8>) -> NativeResult<()> {",
        "pub fn copy_to_vec(&self, output: &mut Vec<u8>) -> NativeResult<()> { for i in 0..5 { let _ = self.u8_at(i); }",
        1,
    );
    assert_ne!(reuse_regression, SOURCE);
    assert!(!bulk_conversions_are_bounded(&reuse_regression));
}

#[test]
fn scalar_batch_rejects_per_item_foreign_projections() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/runtime/scalar_batch.rs"
    ));
    let bounded = |source: &str| {
        let mut shape = CopyShape::default();
        shape.visit_file(&syn::parse_file(source).unwrap());
        shape.bulk_calls == 1 && shape.scalar_calls == 0
    };
    assert!(bounded(source));
    let negative = source.replace(
        "self.handles.clear();",
        "for v in values { let _ = v.as_flonum(); } self.handles.clear();",
    );
    assert_ne!(source, negative);
    assert!(!bounded(&negative));
    let negative = source.replace(
        "gerbil_scheme_rust_scalar_batch(",
        "gerbil_scheme_rust_scheme_object_as_flonum(",
    );
    assert_ne!(source, negative);
    assert!(!bounded(&negative));
}

#[test]
fn numeric_transport_rejects_element_foreign_loops() {
    let source = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/runtime/objects/numeric/transfer.rs"
    ));
    let bounded = |source: &str| {
        let mut shape = CopyShape::default();
        shape.visit_file(&syn::parse_file(source).unwrap());
        shape.bulk_calls == 1 && shape.scalar_calls == 0 && shape.loops == 0
    };
    assert!(bounded(source));
    let negative = source.replace("let status = unsafe {", "for _index in 0..len { let _ = sys::gerbil_scheme_rust_numeric_copy(self.root.0, self.index, T::KIND, output.staged.as_mut_ptr().cast(), 0, 1); } let status = unsafe {");
    assert_ne!(source, negative);
    assert!(!bounded(&negative));
    let negative = source.replace(
        "gerbil_scheme_rust_numeric_copy(",
        "gerbil_scheme_rust_object_scalar_raw(",
    );
    assert_ne!(source, negative);
    assert!(!bounded(&negative));
}
