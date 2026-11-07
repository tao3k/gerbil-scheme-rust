//! Linked single-owner native execution and typed projection.
use super::{
    Answer, Arc, BTreeMap, BigInt, BinaryInteger, CollectionKind, Command, Connection,
    ExecutionKind, GerbilRuntime, Input, Mutex, ObjectKind, Output, PackageManifest, Path,
    Projection, PyResult, PyRuntimeError, Python, PythonLibrary, PythonProgram, Request, Root,
    RootedObjectBatch, Scalar, SchemeScalar, Service, Sign, TableEquality, mpsc, resources,
};
/// Open a verified, statically linked Scheme Library. Only maintainers construct
/// `program`; Python receives no SDK configuration or compiler capability.
/// # Errors
/// Preserves complete package admission, one-shot lifecycle and native failures.
pub fn open_library(
    py: Python<'_>,
    program: PythonProgram,
    manifest: &PackageManifest,
    package_root: &Path,
) -> PyResult<PythonLibrary> {
    let service = py
        .detach(|| {
            // One complete admission, before runtime entry; no duplicate scan
            // in the facade and no stat-only substitute for per-path SHA.
            let package = manifest.verify(package_root).map_err(|e| e.to_string())?;
            let (paths, namespace) = resources::linked(&package).map_err(|e| e.to_string())?;
            let (sender, requests) = mpsc::channel();
            let (ready, initialized) = mpsc::sync_channel(1);
            let worker = std::thread::Builder::new()
                .name("gerbil-python-owner".into())
                .spawn(move || owner(program, &paths, requests, &ready))
                .map_err(|e| e.to_string())?;
            match initialized.recv() {
                Ok(Ok(())) => Ok(Arc::new(Service {
                    kind: ExecutionKind::Linked,
                    connection: Mutex::new(Some(Connection { sender, worker })),
                    pid: std::process::id(),
                    package,
                    namespace,
                    #[cfg(feature = "instance-backend")]
                    instances: None,
                })),
                Ok(Err(error)) => {
                    let _ = worker.join();
                    if let Some(namespace) = &namespace {
                        let _ = resources::retire(namespace);
                    }
                    Err(error)
                }
                Err(error) => {
                    let _ = worker.join();
                    if let Some(namespace) = &namespace {
                        let _ = resources::retire(namespace);
                    }
                    Err(error.to_string())
                }
            }
        })
        .map_err(PyRuntimeError::new_err)?;
    Ok(PythonLibrary { service })
}

fn configure_owner(
    runtime: &GerbilRuntime,
    program: PythonProgram,
    paths: &[String],
) -> Result<(), gerbil_scheme::NativeError> {
    (|| {
        let roots = paths
            .iter()
            .map(|p| {
                runtime
                    .string_from_utf8(p)
                    .map(gerbil_scheme::RootedSchemeString::into_object)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let input = runtime
            .collection_from_objects(CollectionKind::List, &roots.iter().collect::<Vec<_>>())?
            .into_object();
        // SAFETY: PythonProgram owns the library's checked linked configure export.
        unsafe { runtime.bind_object_export(program.configure) }?
            .call(&input)
            .into_result()
            .map(drop)
    })()
}

fn owner(
    program: PythonProgram,
    paths: &[String],
    requests: mpsc::Receiver<Request>,
    ready: &mpsc::SyncSender<Answer<()>>,
) {
    let runtime = match GerbilRuntime::initialize_program_single_processor(program.program) {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(error.to_string()));
            return;
        }
    };
    let configured = configure_owner(&runtime, program, paths);
    if let Err(error) = configured {
        let _ = ready.send(Err(error.to_string()));
        return;
    }
    if ready.send(Ok(())).is_err() {
        return;
    }
    // SAFETY: PythonProgram requires this process-lived initialized export.
    let Ok(export) = (unsafe { runtime.bind_object_export(program.dispatch) }) else {
        return;
    };
    // SAFETY: the descriptor names the upstream linked rooted apply export.
    let Ok(apply) = (unsafe { runtime.bind_object_export(program.apply) }) else {
        return;
    };
    let mut roots: BTreeMap<u64, Root<'_>> = BTreeMap::new();
    let mut next = 1_u64;
    for request in requests {
        let closing = matches!(request.command, Command::Close);
        let calling = matches!(request.command, Command::Call(_));
        let result: Answer<Output> = (|| match request.command {
            Command::ProcessorCount => runtime
                .active_processors()
                .map(Output::ProcessorCount)
                .map_err(|e| e.to_string()),
            Command::Close => Ok(Output::Unit),
            Command::Release(id) => {
                roots.remove(&id);
                Ok(Output::Unit)
            }
            Command::Identical(a, b) => {
                let a = roots.get(&a).ok_or("Scheme value is released")?;
                let b = roots.get(&b).ok_or("Scheme value is released")?;
                Ok(Output::Bool(a.as_ref().is_identical(b.as_ref())))
            }
            Command::Apply(id, arguments) => {
                let procedure = roots.get(&id).ok_or("Scheme value is released")?;
                let arguments = encode(&runtime, &roots, &arguments).map_err(|e| e.to_string())?;
                let request = runtime
                    .collection_from_objects(CollectionKind::Vector, &[procedure, &arguments])
                    .map_err(|e| e.to_string())?
                    .into_object();
                let value = apply
                    .call(&request)
                    .into_result()
                    .map_err(|e| e.to_string())?;
                insert(&mut roots, &mut next, value).map(Output::Token)
            }
            Command::Encode(input) | Command::Call(input) => {
                let value = encode(&runtime, &roots, &input).map_err(|e| e.to_string())?;
                let value = if calling {
                    export
                        .call(&value)
                        .into_result()
                        .map_err(|e| e.to_string())?
                } else {
                    value
                };
                insert(&mut roots, &mut next, value).map(Output::Token)
            }
            Command::Inspect(id, projection) => {
                let root = roots.get(&id).ok_or("Scheme value is released")?;
                match projection {
                    Projection::Kind => root
                        .as_ref()
                        .kind()
                        .map(|k| Output::Kind(format!("{k:?}")))
                        .map_err(|e| e.to_string()),
                    Projection::Scalar => scalar(&runtime, root).map(Output::Scalar),
                    Projection::Elements => {
                        let values = children(&runtime, root)?;
                        values
                            .into_iter()
                            .map(|v| insert(&mut roots, &mut next, v))
                            .collect::<Answer<_>>()
                            .map(Output::Tokens)
                    }
                }
            }
        })();
        deliver_reply(&request.reply, result, &mut roots);
        if closing {
            break;
        }
    }
    // The arena drops before the VM, always on this thread, even on unwind.
}

fn deliver_reply(
    reply: &mpsc::SyncSender<Answer<Output>>,
    result: Answer<Output>,
    roots: &mut BTreeMap<u64, Root<'_>>,
) {
    // An abandoned synchronous caller must not retain newly returned VM roots.
    if let Err(mpsc::SendError(Ok(output))) = reply.send(result) {
        let ids = match output {
            Output::Token(id) => vec![id],
            Output::Tokens(ids) => ids,
            _ => Vec::new(),
        };
        for id in ids {
            roots.remove(&id);
        }
    }
}
fn insert<'a>(roots: &mut BTreeMap<u64, Root<'a>>, next: &mut u64, value: Root<'a>) -> Answer<u64> {
    let id = *next;
    *next = next
        .checked_add(1)
        .ok_or("Scheme handle identity exhausted")?;
    roots.insert(id, value);
    Ok(id)
}
fn encode<'a>(
    runtime: &'a GerbilRuntime,
    roots: &BTreeMap<u64, Root<'a>>,
    input: &Input,
) -> Result<Root<'a>, gerbil_scheme::NativeError> {
    match input {
        #[cfg(feature = "instance-backend")]
        Input::Shared(input) => encode(runtime, roots, input),
        Input::Void => runtime.void_object(),
        Input::Bool(v) => runtime.scalar_object(SchemeScalar::Boolean(*v)),
        Input::Integer(v) => {
            let (sign, mut bytes) = v.to_bytes_le();
            if sign == Sign::NoSign {
                bytes.clear();
            }
            runtime.big_integer(&BinaryInteger::new(sign == Sign::Minus, bytes)?)
        }
        Input::Real(v) => runtime.scalar_object(SchemeScalar::Flonum(*v)),
        Input::Text(v) => runtime
            .string_from_utf8(v)
            .map(gerbil_scheme::RootedSchemeString::into_object),
        Input::Bytes(v) => runtime
            .bytevector_from_bytes(v)
            .map(gerbil_scheme::RootedSchemeBytevector::into_object),
        Input::List(values) | Input::Vector(values) => {
            let values = values
                .iter()
                .map(|v| encode(runtime, roots, v))
                .collect::<Result<Vec<_>, _>>()?;
            runtime
                .collection_from_objects(
                    if matches!(input, Input::List(_)) {
                        CollectionKind::List
                    } else {
                        CollectionKind::Vector
                    },
                    &values.iter().collect::<Vec<_>>(),
                )
                .map(gerbil_scheme::RootedSchemeCollection::into_object)
        }
        Input::Table(values) => {
            let values = values
                .iter()
                .map(|(k, v)| Ok((encode(runtime, roots, k)?, encode(runtime, roots, v)?)))
                .collect::<Result<Vec<_>, gerbil_scheme::NativeError>>()?;
            runtime.hash_table(
                TableEquality::Structural,
                &values.iter().map(|(k, v)| (k, v)).collect::<Vec<_>>(),
            )
        }
        Input::Reference(id) => {
            let root = roots.get(id).ok_or(gerbil_scheme::NativeError::Status {
                operation: "released Python handle",
                code: -1,
            })?;
            clone_root(runtime, root)
        }
    }
}
fn clone_root<'a>(
    runtime: &'a GerbilRuntime,
    root: &Root<'a>,
) -> Result<Root<'a>, gerbil_scheme::NativeError> {
    let vector = runtime.collection_from_objects(CollectionKind::Vector, &[root])?;
    let mut snapshot = RootedObjectBatch::default();
    vector.copy_into(&mut snapshot)?;
    snapshot.root_element(0)
}
fn scalar<'a>(runtime: &'a GerbilRuntime, root: &Root<'a>) -> Answer<Scalar> {
    let kind = root.as_ref().kind().map_err(|e| e.to_string())?;
    let result = (|| -> Result<Scalar, gerbil_scheme::NativeError> {
        match kind {
            ObjectKind::Void => Ok(Scalar::Null),
            ObjectKind::Boolean | ObjectKind::Flonum => match root.as_ref().scalar()? {
                SchemeScalar::Boolean(v) => Ok(Scalar::Boolean(v)),
                SchemeScalar::Flonum(v) => Ok(Scalar::Real(v)),
                _ => unreachable!(),
            },
            ObjectKind::ExactInteger => {
                let mut value = gerbil_scheme::BigIntegerBuffer::default();
                root.as_ref().copy_big_integer(&mut value)?;
                Ok(Scalar::Integer(BigInt::from_bytes_le(
                    if value.value().is_negative() {
                        Sign::Minus
                    } else {
                        Sign::Plus
                    },
                    value.value().magnitude(),
                )))
            }
            ObjectKind::String => Ok(Scalar::Text(
                clone_root(runtime, root)?
                    .into_string()?
                    .to_string()
                    .into_result()?,
            )),
            ObjectKind::Bytevector => {
                // Bytevector projection uses the upstream checked object conversion.
                let mut bytes = Vec::new();
                clone_root(runtime, root)?
                    .into_bytevector()?
                    .copy_to_vec(&mut bytes)
                    .into_result()?;
                Ok(Scalar::Bytes(bytes))
            }
            _ => Err(gerbil_scheme::NativeError::Status {
                operation: "no scalar projection; native value retained",
                code: -1,
            }),
        }
    })();
    result.map_err(|e| e.to_string())
}
fn children<'a>(runtime: &'a GerbilRuntime, root: &Root<'a>) -> Answer<Vec<Root<'a>>> {
    let mut snapshot = RootedObjectBatch::default();
    let kind = root.as_ref().kind().map_err(|e| e.to_string())?;
    (|| -> Result<_, gerbil_scheme::NativeError> {
        match kind {
            ObjectKind::GambitTable | ObjectKind::GerbilHashTable => {
                root.copy_table_entries(&mut snapshot)?;
            }
            ObjectKind::Pair | ObjectKind::Vector | ObjectKind::Nil => {
                let collection =
                    clone_root(runtime, root)?.into_collection(if kind == ObjectKind::Vector {
                        CollectionKind::Vector
                    } else {
                        CollectionKind::List
                    })?;
                collection.copy_into(&mut snapshot)?;
            }
            _ => {
                return Err(gerbil_scheme::NativeError::Status {
                    operation: "not a collection",
                    code: -1,
                });
            }
        }
        (0..snapshot.len())
            .map(|i| snapshot.root_element(i))
            .collect()
    })()
    .map_err(|e| e.to_string())
}
