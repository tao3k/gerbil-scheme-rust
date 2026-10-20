//! Native conversion and projection; no Scheme heap pointer crosses an owner.
use super::super::{Input, Projection, Scalar};
use super::{Command, NativeOutput, Output, Registry};
use gerbil_scheme::{
    BinaryInteger, ObjectKind, SchemeScalar,
    instances::{InstanceError, NativeRoot, NativeSession},
};
use num_bigint::{BigInt, Sign};
use std::{collections::BTreeMap, ffi::CStr};
pub(super) fn execute(
    session: &NativeSession<'_>,
    command: Command,
    registry: &Registry,
    apply: &CStr,
) -> Result<NativeOutput, InstanceError> {
    execute_staged(session, command, registry, apply, &mut None)
}
pub(super) fn execute_staged(
    session: &NativeSession<'_>,
    command: Command,
    registry: &Registry,
    apply: &CStr,
    stages: &mut Option<(std::time::Instant, [u64; 2])>,
) -> Result<NativeOutput, InstanceError> {
    let output = match command {
        Command::ProcessorCount => NativeOutput::Plain(Output::ProcessorCount(1)),
        Command::Close => NativeOutput::Plain(Output::Unit),
        Command::Release(id) => {
            if let Ok(value) = registry.get(id) {
                session.release_retained(value)?;
                registry
                    .values
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&id);
            }
            NativeOutput::Plain(Output::Unit)
        }
        Command::Encode(input) => {
            NativeOutput::Root(session.retain(encode(session, registry, &input)?)?)
        }
        Command::Call(input) => {
            let input = encode(session, registry, &input)?;
            if let Some((started, values)) = stages {
                values[0] = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
            }
            let result = session.call(&input)?;
            if let Some((started, values)) = stages {
                values[1] = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
            }
            NativeOutput::Root(session.retain(result)?)
        }
        Command::Apply(id, input) => {
            let procedure = session.retained(registry.get(id)?)?;
            let args = encode(session, registry, &input)?;
            let input = session.vector(&[&procedure, &args])?;
            NativeOutput::Root(session.retain(session.call_export(apply, &input)?)?)
        }
        Command::Identical(left, right) => {
            let left = session.retained(registry.get(left)?)?;
            let right = session.retained(registry.get(right)?)?;
            NativeOutput::Plain(Output::Bool(session.identical(&left, &right)?))
        }
        Command::Inspect(id, projection) => {
            let root = session.retained(registry.get(id)?)?;
            match projection {
                Projection::Kind => {
                    NativeOutput::Plain(Output::Kind(format!("{:?}", session.kind(&root)?)))
                }
                Projection::Elements => NativeOutput::Children(
                    session
                        .elements(&root)?
                        .into_iter()
                        .map(|root| session.retain(root))
                        .collect::<Result<_, _>>()?,
                ),
                Projection::Scalar => NativeOutput::Plain(Output::Scalar(project(
                    session,
                    &root,
                    &[],
                    &mut (16 * 1024 * 1024),
                )?)),
            }
        }
    };
    Ok(output)
}
fn encode<'vm>(
    session: &NativeSession<'vm>,
    registry: &Registry,
    input: &Input,
) -> Result<NativeRoot<'vm>, InstanceError> {
    match input {
        Input::Void => session.void_object(),
        Input::Bool(value) => session.scalar(SchemeScalar::Boolean(*value)),
        Input::Real(value) => session.scalar(SchemeScalar::Flonum(*value)),
        Input::Text(value) => session.string(value),
        Input::Bytes(value) => session.bytevector(value),
        Input::Integer(value) => {
            let (sign, mut bytes) = value.to_bytes_le();
            if sign == Sign::NoSign {
                bytes.clear();
            }
            let value = BinaryInteger::new(sign == Sign::Minus, bytes)
                .map_err(|e| InstanceError::Loader(e.to_string()))?;
            session.big_integer(&value)
        }
        Input::Reference(id) => session.retained(registry.get(*id)?),
        Input::List(values) | Input::Vector(values) => {
            let roots = values
                .iter()
                .map(|v| encode(session, registry, v))
                .collect::<Result<Vec<_>, _>>()?;
            let refs = roots.iter().collect::<Vec<_>>();
            if matches!(input, Input::List(_)) {
                session.list(&refs)
            } else {
                session.vector(&refs)
            }
        }
        Input::Table(values) => {
            let roots = values
                .iter()
                .map(|(k, v)| Ok((encode(session, registry, k)?, encode(session, registry, v)?)))
                .collect::<Result<Vec<_>, InstanceError>>()?;
            session.table(&roots.iter().map(|(k, v)| (k, v)).collect::<Vec<_>>())
        }
    }
}
fn scalar(session: &NativeSession<'_>, value: &NativeRoot<'_>) -> Result<Scalar, InstanceError> {
    match session.kind(value)? {
        ObjectKind::Void => Ok(Scalar::Null),
        ObjectKind::String => session.copy_string(value).map(Scalar::Text),
        ObjectKind::Bytevector => session.copy_bytes(value).map(Scalar::Bytes),
        ObjectKind::ExactInteger => {
            let value = session.copy_big_integer(value)?;
            Ok(Scalar::Integer(BigInt::from_bytes_le(
                if value.is_negative() {
                    Sign::Minus
                } else {
                    Sign::Plus
                },
                value.magnitude(),
            )))
        }
        ObjectKind::Boolean | ObjectKind::Flonum => match session.copy_scalar(value)? {
            SchemeScalar::Boolean(v) => Ok(Scalar::Boolean(v)),
            SchemeScalar::Flonum(v) => Ok(Scalar::Real(v)),
            _ => Err(InstanceError::InvalidConfig("native scalar")),
        },
        _ => Err(InstanceError::InvalidConfig(
            "no scalar projection; native object retained",
        )),
    }
}

fn project<'vm>(
    session: &NativeSession<'vm>,
    value: &NativeRoot<'vm>,
    ancestors: &[&NativeRoot<'vm>],
    remaining: &mut usize,
) -> Result<Scalar, InstanceError> {
    *remaining = remaining
        .checked_sub(std::mem::size_of::<Scalar>())
        .ok_or(InstanceError::InvalidConfig("projection byte bound"))?;
    if ancestors.len() > 128 {
        return Err(InstanceError::InvalidConfig("projection depth bound"));
    }
    let kind = session.kind(value)?;
    if matches!(
        kind,
        ObjectKind::Pair
            | ObjectKind::Vector
            | ObjectKind::Nil
            | ObjectKind::GambitTable
            | ObjectKind::GerbilHashTable
    ) {
        for parent in ancestors {
            if session.identical(parent, value)? {
                return Err(InstanceError::InvalidConfig(
                    "cyclic projection; native graph retained",
                ));
            }
        }
        let mut stack = ancestors.to_vec();
        stack.push(value);
        let children = session.elements(value)?;
        if matches!(kind, ObjectKind::GambitTable | ObjectKind::GerbilHashTable) {
            let mut map = BTreeMap::new();
            for pair in children.chunks_exact(2) {
                let key = session.copy_string(&pair[0])?;
                *remaining = remaining
                    .checked_sub(key.len())
                    .ok_or(InstanceError::InvalidConfig("projection byte bound"))?;
                map.insert(key, project(session, &pair[1], &stack, remaining)?);
            }
            Ok(Scalar::Map(map))
        } else {
            children
                .iter()
                .map(|value| project(session, value, &stack, remaining))
                .collect::<Result<Vec<_>, _>>()
                .map(Scalar::List)
        }
    } else {
        let value = scalar(session, value)?;
        let bytes = match &value {
            Scalar::Text(value) => value.len(),
            Scalar::Bytes(value) => value.len(),
            Scalar::Integer(value) => usize::try_from(value.bits().div_ceil(8))
                .map_err(|_| InstanceError::InvalidConfig("projection integer bound"))?,
            _ => 0,
        };
        *remaining = remaining
            .checked_sub(bytes)
            .ok_or(InstanceError::InvalidConfig("projection byte bound"))?;
        Ok(value)
    }
}
