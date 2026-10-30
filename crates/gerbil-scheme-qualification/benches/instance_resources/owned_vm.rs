//! Real Scheme execution through the owned C ABI's explicit Scheme VM contract.
use super::{GATE, Progress, checkpoint, config, finish, normal, observed};
use gerbil_scheme::instances::{InstancePool, NativeRoot, NativeSession};
use gerbil_scheme_python::{
    OwnedValue, ValueLimits,
    c_abi::{self, Error, ForeignCallable, RawCallable, RawValue, View},
};
use std::{cell::Cell, ffi::c_void};
struct Scope<'a, 'vm> {
    session: &'a NativeSession<'vm>,
    releases: &'a Cell<usize>,
}
// Result owns its native root independently of the callable, on this VM owner.
struct RootedResult<'vm> {
    transport: RawValue,
    _root: NativeRoot<'vm>,
}
unsafe extern "C" fn release_result(context: *mut c_void) {
    // SAFETY: one transferred result allocation, released before the VM task ends.
    let mut result = unsafe { Box::from_raw(context.cast::<RootedResult<'_>>()) };
    if let Some(release) = result.transport.release.take() {
        // SAFETY: this wrapper owns the original independent transport allocation.
        unsafe { release(result.transport.context) };
    }
    // Dropping result also releases its real Scheme root on the same owner.
}
unsafe fn invoke(context: *mut c_void, input: *const View) -> Result<RawValue, Error> {
    // SAFETY: the test retains this scope/session on its fixed VM owner until close.
    let scope = unsafe { &*context.cast::<Scope<'_, '_>>() };
    // SAFETY: ForeignCallable retains the encoded input for the callback.
    let OwnedValue::List(values) = (unsafe { c_abi::read_input(input, ValueLimits::default()) })?
    else {
        return Err(Error::Invalid("numeric list"));
    };
    let numbers: Vec<i64> = values
        .iter()
        .map(|value| {
            if let OwnedValue::Integer(value) = value {
                i64::try_from(value).map_err(|_| Error::Invalid("i64"))
            } else {
                Err(Error::Invalid("integer"))
            }
        })
        .collect::<Result<_, _>>()?;
    let input = scope
        .session
        .numeric_vector(&numbers)
        .map_err(|_| Error::Status(90))?;
    let output = scope.session.call(&input).map_err(|_| Error::Status(91))?;
    let values = scope
        .session
        .copy_numeric::<i64>(&output)
        .map_err(|_| Error::Status(92))?;
    let transport = c_abi::publish_value(&OwnedValue::List(
        values
            .into_iter()
            .map(|v| OwnedValue::Integer(v.into()))
            .collect(),
    ))?;
    Ok(RawValue {
        abi_version: c_abi::ABI_VERSION,
        flags: c_abi::SCHEME_VM,
        view: transport.view,
        context: Box::into_raw(Box::new(RootedResult {
            transport,
            _root: output,
        }))
        .cast(),
        release: Some(release_result),
    })
}
unsafe extern "C" fn call(context: *mut c_void, input: *const View, output: *mut RawValue) -> i32 {
    // No Rust unwind crosses C, including validation/allocation failures.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: the admitted caller retains context/input throughout this call.
        unsafe { invoke(context, input) }
    }));
    match result {
        Ok(Ok(value)) => {
            // SAFETY: ForeignCallable supplies a valid, initially zeroed output.
            unsafe { output.write(value) };
            0
        }
        Ok(Err(Error::Status(status))) => status,
        Ok(Err(_)) => 93,
        Err(_) => 94,
    }
}
unsafe extern "C" fn release(context: *mut c_void) {
    // SAFETY: one allocation transferred below, consumed once on the same owner.
    let scope = unsafe { Box::from_raw(context.cast::<Scope<'_, '_>>()) };
    scope.releases.set(scope.releases.get() + 1);
}
fn input(values: &[i64]) -> OwnedValue {
    OwnedValue::List(
        values
            .iter()
            .map(|v| OwnedValue::Integer((*v).into()))
            .collect(),
    )
}
pub(super) fn run(progress: &Progress) {
    let mut pool = InstancePool::start(&normal(), config(4)).unwrap();
    observed(progress, "observed owned C ABI Scheme VM initialization");
    for owner in 0..4 {
        pool.dispatcher()
            .submit_native_on(owner, move |session| {
                let releases = Cell::new(0);
                for flags in [0, 3, 4] {
                    let invalid = RawCallable {
                        abi_version: c_abi::ABI_VERSION,
                        flags,
                        context: Box::into_raw(Box::new(Scope {
                            session,
                            releases: &releases,
                        }))
                        .cast(),
                        call: Some(call),
                        release: Some(release),
                    };
                    // SAFETY: rejection releases the same live, owner-local scope.
                    assert!(matches!(
                        unsafe { ForeignCallable::from_raw(invalid) },
                        Err(Error::Abi)
                    ));
                }
                assert_eq!(releases.get(), 3);
                let raw = RawCallable {
                    abi_version: c_abi::ABI_VERSION,
                    flags: c_abi::SCHEME_VM,
                    context: Box::into_raw(Box::new(Scope {
                        session,
                        releases: &releases,
                    }))
                    .cast(),
                    call: Some(call),
                    release: Some(release),
                };
                // SAFETY: scope/code/VM remain on this owner through all aliases and
                // values below. No borrowed handle escapes the enclosing native task.
                let callable = unsafe { ForeignCallable::from_raw(raw) }.unwrap();
                let alias = callable.clone();
                let value = callable.call(&input(&[0, 11, -7])).unwrap();
                assert_eq!(
                    value.decode(ValueLimits::default()).unwrap(),
                    input(&[40, -14])
                );
                assert!(matches!(
                    callable.call(&input(&[1])),
                    Err(Error::Status(91))
                ));
                let marker = i64::try_from(owner).unwrap() + 100;
                for (operation, expected) in [(1, 23), (2, marker + 7), (3, marker + 9), (4, 31)] {
                    let result = callable.call(&input(&[2, operation, marker])).unwrap();
                    assert_eq!(
                        result.decode(ValueLimits::default()).unwrap(),
                        input(&[expected])
                    );
                }
                session.collect()?;
                callable.close().unwrap();
                assert!(matches!(alias.call(&input(&[0, 1])), Err(Error::Closed)));
                drop(alias);
                drop(callable);
                assert_eq!(releases.get(), 4);
                // A dependent result retains its own Scheme root after callable close.
                assert!(session.usage()?[3] >= 1.0);
                session.collect()?;
                assert_eq!(
                    value.decode(ValueLimits::default()).unwrap(),
                    input(&[40, -14])
                );
                drop(value);
                assert_eq!(session.usage()?[3].to_bits(), 0.0_f64.to_bits());
                Ok(())
            })
            .unwrap()
            .wait(GATE)
            .unwrap();
        observed(
            progress,
            "observed owned Scheme call GC continuation exception and close",
        );
    }
    checkpoint(&pool, progress);
    finish(&mut pool, progress);
    eprintln!(
        "OWNED-VM owners=4 native-types=true gc=true multishot=true exception=true alias-close=true release-once=true roots=0"
    );
}
