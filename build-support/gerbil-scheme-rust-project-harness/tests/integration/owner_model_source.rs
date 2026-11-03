//! Parser-owned module closures; canonical model declarations remain byte exact.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

thread_local! {
    static ACTIVE: RefCell<Option<Arc<ModelSource>>> = const { RefCell::new(None) };
}

struct Module {
    start: usize,
    text: String,
    imports: Vec<String>,
}

pub(super) struct ModelSource {
    _directory: tempfile::TempDir,
    canonical: PathBuf,
    digest: Vec<u8>,
    entries: BTreeMap<String, PathBuf>,
}

impl ModelSource {
    pub(super) fn prepare(root: &Path) -> Self {
        let canonical = root.join("specs/owner_lifecycle.qnt");
        let original = fs::read_to_string(&canonical).expect("canonical model source");
        let directory = tempfile::Builder::new()
            .prefix("gerbil-owner-model-source-")
            .tempdir()
            .expect("private parser output directory");
        let modules = parse_modules(&canonical, directory.path(), &original);
        let mut entries = BTreeMap::new();
        for main in modules.keys() {
            let text = projection(&modules, main);
            let path = directory.path().join(format!("{main}.qnt"));
            fs::write(&path, &text).expect("write immutable module closure");
            eprintln!(
                "QUINT-SOURCE main={main} bytes={} canonical={}",
                text.len(),
                original.len()
            );
            entries.insert(main.clone(), path);
        }
        let digest = Sha256::digest(original.as_bytes()).to_vec();
        assert_eq!(
            Sha256::digest(fs::read(&canonical).unwrap()).to_vec(),
            digest
        );
        Self {
            _directory: directory,
            canonical,
            digest,
            entries,
        }
    }

    fn entry(&self, main: &str) -> &Path {
        assert_eq!(
            Sha256::digest(fs::read(&self.canonical).unwrap()).to_vec(),
            self.digest,
            "canonical model changed after parser-owned projection"
        );
        self.entries.get(main).expect("parser-known main module")
    }
}

fn parse_modules(canonical: &Path, directory: &Path, original: &str) -> BTreeMap<String, Module> {
    let parsed = directory.join("parsed.json");
    let locations = directory.join("locations.json");
    let output = Command::new("quint")
        .arg("parse")
        .arg(canonical)
        .arg("--out")
        .arg(&parsed)
        .arg("--source-map")
        .arg(&locations)
        .output()
        .expect("installed Quint parser");
    assert!(
        output.status.success(),
        "canonical parser failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ast: Value = serde_json::from_slice(&fs::read(parsed).unwrap()).unwrap();
    let map: Value = serde_json::from_slice(&fs::read(locations).unwrap()).unwrap();
    assert!(ast["errors"].as_array().is_some_and(Vec::is_empty));
    modules(original, &ast, &map)
}

fn modules(source: &str, ast: &Value, map: &Value) -> BTreeMap<String, Module> {
    ast["modules"]
        .as_array()
        .expect("Quint module declarations")
        .iter()
        .map(|module| {
            let name = module["name"].as_str().unwrap();
            let id = module["id"].as_u64().unwrap().to_string();
            let span = &map["map"][&id];
            assert_eq!(
                span[0].as_u64(),
                Some(0),
                "external model source is not projected"
            );
            let start = byte_index(source, span[1]["index"].as_u64().unwrap());
            let end = byte_index(source, span[2]["index"].as_u64().unwrap() + 1);
            let imports = module["declarations"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|decl| matches!(decl["kind"].as_str(), Some("import" | "instance")))
                .map(|decl| {
                    assert!(
                        decl["fromSource"].is_null(),
                        "external import needs an explicit source closure"
                    );
                    decl["protoName"].as_str().unwrap().to_owned()
                })
                .collect();
            (
                name.to_owned(),
                Module {
                    start,
                    text: source[start..end].to_owned(),
                    imports,
                },
            )
        })
        .collect()
}

fn byte_index(source: &str, offset: u64) -> usize {
    let mut units = 0;
    for (index, character) in source.char_indices() {
        if units == offset {
            return index;
        }
        units += u64::try_from(character.len_utf16()).unwrap();
        assert!(units <= offset, "parser offset inside a Unicode scalar");
    }
    assert_eq!(units, offset, "parser offset outside canonical source");
    source.len()
}

fn projection(modules: &BTreeMap<String, Module>, main: &str) -> String {
    let mut closure = BTreeSet::new();
    let mut pending = vec![main.to_owned()];
    while let Some(name) = pending.pop() {
        if closure.insert(name.clone()) {
            let module = modules
                .get(&name)
                .expect("unknown parser-declared dependency");
            pending.extend(module.imports.iter().cloned());
        }
    }
    let mut included: Vec<_> = closure.iter().map(|name| &modules[name]).collect();
    included.sort_by_key(|module| module.start);
    included
        .iter()
        .map(|module| format!("{}\n", module.text))
        .collect()
}

struct Restore(Option<Arc<ModelSource>>);
impl Drop for Restore {
    fn drop(&mut self) {
        ACTIVE.with(|active| active.replace(self.0.take()));
    }
}

pub(super) fn scoped(source: &Arc<ModelSource>, work: impl FnOnce()) {
    let previous = ACTIVE.with(|active| active.replace(Some(Arc::clone(source))));
    let _restore = Restore(previous);
    work();
}

pub(super) fn arguments(arguments: &[&str]) -> Vec<OsString> {
    let mut output: Vec<_> = arguments.iter().map(OsString::from).collect();
    let main = arguments
        .windows(2)
        .find(|pair| pair[0] == "--main")
        .map(|pair| pair[1]);
    if let Some(main) = main {
        ACTIVE.with(|active| {
            if let Some(source) = active.borrow().as_ref() {
                assert_eq!(arguments.get(1), Some(&"specs/owner_lifecycle.qnt"));
                output[1] = source.entry(main).as_os_str().to_owned();
            }
        });
    }
    output
}

#[test]
fn closure_retains_exact_transitive_modules_and_excludes_unrelated_instances() {
    let modules = BTreeMap::from([
        (
            "Base".into(),
            Module {
                start: 0,
                text: "module Base { val retained = true }".into(),
                imports: vec![],
            },
        ),
        (
            "Other".into(),
            Module {
                start: 1,
                text: "module Other { val unrelated = false }".into(),
                imports: vec![],
            },
        ),
        (
            "Main".into(),
            Module {
                start: 2,
                text: "module Main { import Base.* }".into(),
                imports: vec!["Base".into()],
            },
        ),
    ]);
    assert_eq!(
        projection(&modules, "Main"),
        "module Base { val retained = true }\nmodule Main { import Base.* }\n"
    );
}

#[test]
#[should_panic(expected = "unknown parser-declared dependency")]
fn closure_does_not_silently_drop_unknown_dependencies() {
    let modules = BTreeMap::from([(
        "Main".into(),
        Module {
            start: 0,
            text: "module Main {}".into(),
            imports: vec!["Missing".into()],
        },
    )]);
    projection(&modules, "Main");
}

#[test]
fn parser_utf16_offsets_preserve_unicode_source_bytes() {
    let source = "// λ😀\nmodule Main {}";
    let prefix = "// λ😀\n";
    assert_eq!(
        byte_index(
            source,
            u64::try_from(prefix.encode_utf16().count()).unwrap()
        ),
        prefix.len()
    );
}

#[test]
#[should_panic(expected = "canonical model changed after parser-owned projection")]
fn cached_entry_rejects_changed_canonical_source() {
    let directory = tempfile::tempdir().unwrap();
    let canonical = directory.path().join("canonical.qnt");
    fs::write(&canonical, "original").unwrap();
    let source = ModelSource {
        canonical: canonical.clone(),
        digest: Sha256::digest(b"original").to_vec(),
        entries: BTreeMap::from([("Main".into(), directory.path().join("main.qnt"))]),
        _directory: directory,
    };
    fs::write(canonical, "modified").unwrap();
    source.entry("Main");
}
