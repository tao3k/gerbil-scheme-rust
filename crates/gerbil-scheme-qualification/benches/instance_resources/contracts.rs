use super::{GATE, Progress, config, finish, normal, observe::Observer, observed, program};
use gerbil_scheme::{
    NumericElement, SchemeScalar,
    instances::{InstanceError, InstancePool},
};
use std::fmt::Debug;
fn family<T>(pool: &InstancePool, input: Vec<T>, progress: &Progress) -> (Vec<T>, Vec<T>, Vec<T>)
where
    T: NumericElement + Send + Debug + 'static,
{
    let result = pool
        .dispatcher()
        .submit_native_on(0, move |session| {
            let input = session.numeric_vector(&input)?;
            let result = session.call(&input)?;
            let direct = session.copy_numeric::<T>(&result)?;
            let empty = session.numeric_vector::<T>(&[])?;
            let boolean = session.scalar(SchemeScalar::Boolean(false))?;
            let parent = session.vector(&[&result, &empty, &boolean])?;
            drop(boolean);
            drop(empty);
            drop(result);
            drop(input);
            session.collect()?;
            let roots = session.usage()?[3];
            let indexed = session.copy_numeric_element::<T>(&parent, 0)?;
            assert!(session.copy_numeric_element::<T>(&parent, 1)?.is_empty());
            assert!(matches!(
                session.copy_numeric_element::<T>(&parent, 2),
                Err(InstanceError::Native { .. })
            ));
            assert!(matches!(
                session.copy_numeric_element::<T>(&parent, 3),
                Err(InstanceError::Native { .. })
            ));
            assert!(matches!(
                session.copy_numeric_element::<T>(&parent, usize::MAX),
                Err(InstanceError::InvalidConfig(_))
            ));
            assert_eq!(session.usage()?[3].to_bits(), roots.to_bits());
            let before = session.usage()?[3];
            let envelope = session.numeric_batch_envelope(&[indexed.clone(), Vec::new()], 1)?;
            assert_eq!(session.usage()?[3].to_bits(), (before + 1.0).to_bits());
            session.collect()?;
            let returned = session.call(&envelope)?; // Actual Scheme echo forces another GC.
            drop(envelope);
            session.collect()?;
            let quantum = session.vector_element(&returned, 0)?;
            assert_eq!(session.copy_numeric::<i64>(&quantum)?, [1]);
            let children = session.vector_element(&returned, 1)?;
            let bulk = session.copy_numeric_element::<T>(&children, 0)?;
            assert!(session.copy_numeric_element::<T>(&children, 1)?.is_empty());
            assert!(matches!(
                session.numeric_batch_envelope::<T>(&[], 1),
                Err(InstanceError::InvalidConfig(_))
            ));
            assert!(matches!(
                session.numeric_batch_envelope(std::slice::from_ref(&indexed), 257),
                Err(InstanceError::InvalidConfig(_))
            ));
            Ok((direct, indexed, bulk))
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    observed(progress, "observed existing numeric ABI roundtrip");
    result
}
pub(super) fn types(pool: &InstancePool, progress: &Progress) {
    macro_rules! check {
        ($values:expr) => {{
            let values = $values;
            let (direct, indexed, bulk) = family(pool, values.clone(), progress);
            assert_eq!(direct, values);
            assert_eq!(indexed, values);
            assert_eq!(bulk, values);
        }};
    }
    check!(vec![0_u16, u16::MAX]);
    check!(vec![0_u32, u32::MAX]);
    check!(vec![0_u64, u64::MAX]);
    check!(vec![i16::MIN, i16::MAX]);
    check!(vec![i32::MIN, i32::MAX]);
    check!(vec![-7_i64, i64::MIN, i64::MAX]);
    check!(vec![i8::MIN, i8::MAX]);
    let a = vec![-0.0_f32, f32::from_bits(0x7fc0_1234)];
    let (direct, indexed, bulk) = family(pool, a.clone(), progress);
    for b in [direct, indexed, bulk] {
        assert_eq!(
            a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }
    let a = vec![-0.0_f64, f64::from_bits(0x7ff8_0000_0000_1234)];
    let (direct, indexed, bulk) = family(pool, a.clone(), progress);
    for b in [direct, indexed, bulk] {
        assert_eq!(
            a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }
    let result = pool
        .dispatcher()
        .submit_native_on(0, |session| {
            let text = session.string("native λ 🧪\0")?;
            let boolean = session.scalar(SchemeScalar::Boolean(false))?;
            let character = session.scalar(SchemeScalar::Char('🧪'))?;
            let integer = session.scalar(SchemeScalar::Fixnum(-91))?;
            let real = session.scalar(SchemeScalar::Flonum(-0.0))?;
            let input = session.vector(&[&text, &boolean, &character, &integer, &real])?;
            let output = session.call(&input)?;
            let text = session.copy_string(&session.vector_element(&output, 0)?)?;
            let mut scalars = Vec::new();
            for index in 1..5 {
                scalars.push(session.copy_scalar(&session.vector_element(&output, index)?)?);
            }
            Ok((text, scalars))
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    assert_eq!(result.0, "native λ 🧪\0");
    assert_eq!(
        result.1[..3],
        [
            SchemeScalar::Boolean(false),
            SchemeScalar::Char('🧪'),
            SchemeScalar::Fixnum(-91)
        ]
    );
    let SchemeScalar::Flonum(number) = result.1[3] else {
        panic!("flonum")
    };
    assert_eq!(number.to_bits(), (-0.0_f64).to_bits());
    observed(
        progress,
        "observed native scalar/string/mixed-vector roundtrip",
    );
    eprintln!(
        "RESOURCE-TYPES numeric-families=9 scalar-families=4 unicode=true nul=true mixed-vector=true nan-payload=true signed-zero=true indexed-copy=true parent-only-root=true indexed-rejections=true native-batch-constructor=true single-envelope-root=true json=false"
    );
}
pub(super) fn preflight(observer: &Observer, progress: &Progress) {
    let process = observer.emit("preflight-process-baseline", 0, &[]);
    let warm = 8;
    let samples = 128;
    let mut baseline = [0; 8];
    let missing = program(
        gerbil_scheme_qualification::owner_instance_image(),
        c"missing_native_export",
    );
    for index in 0..warm + samples {
        let error = InstancePool::start(&missing, config(1)).unwrap_err();
        let InstanceError::Startup {
            cause,
            shutdown,
            image_directory,
        } = error
        else {
            panic!("startup error")
        };
        assert!(matches!(*cause, InstanceError::Loader(_)));
        assert_eq!(shutdown.pending, 0);
        assert_eq!(shutdown.panicked, 0);
        assert_eq!(shutdown.joined, 1);
        assert!(
            !image_directory.exists(),
            "uninitialized image/directory must be reclaimed"
        );
        observed(progress, "observed rejected preflight and owner join");
        if index == warm - 1 {
            baseline = observer.emit("preflight-baseline", index, &[]);
        }
        if index >= warm && index % 16 == 15 {
            let current = observer.settled("preflight", index, process[3]);
            assert_eq!(current[2..5], process[2..5]);
            assert!(
                current[0] <= baseline[0] + 16 * 1024 * 1024,
                "preflight resident growth"
            );
        }
    }
    let mut pool = InstancePool::start(&normal(), config(1)).unwrap();
    observed(progress, "observed valid setup after rejected preflights");
    finish(&mut pool, progress);
    eprintln!(
        "RESOURCE-PREFLIGHT rejected={} uninitialized-images-unloaded=true copied-files-removed=true valid-later-setup=true",
        warm + samples
    );
}
pub(super) fn initialization(after: bool, observer: &Observer, progress: &Progress) {
    let baseline = observer.emit("init-baseline", 0, &[]);
    let fault = program(
        gerbil_scheme_qualification::owner_failure_image(after),
        c"gerbil_owner_native_call",
    );
    let error = InstancePool::start(&fault, config(2)).unwrap_err();
    let InstanceError::Startup {
        cause,
        shutdown,
        image_directory,
    } = error
    else {
        panic!("startup failure")
    };
    let InstanceError::Initialization {
        owner,
        status,
        phase,
        cleanup,
        ..
    } = *cause
    else {
        panic!("native phase failure")
    };
    assert_eq!(owner, 1);
    assert_eq!(status, if after { 62 } else { 61 });
    assert_eq!(phase, if after { 2 } else { 0 });
    assert_eq!(shutdown.joined, 2);
    assert_eq!(shutdown.pending, 0);
    assert_eq!(shutdown.panicked, 0);
    assert_eq!(shutdown.owners.len(), 1);
    assert_eq!(shutdown.owners[0].roots_before_cleanup, 0);
    assert_eq!(shutdown.owners[0].status, 0);
    if after {
        let rollback = cleanup.as_ref().unwrap();
        assert_eq!(rollback.status, 0);
        assert_eq!(rollback.roots_before_cleanup, 0);
    } else {
        assert!(cleanup.is_none());
    }
    let files = std::fs::read_dir(&image_directory).map_or(0, Iterator::count);
    assert_eq!(files, 0);
    assert!(!image_directory.exists());
    assert!(matches!(
        InstancePool::start(&normal(), config(1)),
        Err(InstanceError::BackendConflict)
    ));
    observed(
        progress,
        "observed native phase failure with bounded rollback and poisoned admission",
    );
    let current = observer.settled("init-failed-drained", usize::from(after), baseline[3]);
    assert_eq!(current[2], baseline[2]);
    assert_eq!(current[3], baseline[3]);
    assert_eq!(current[4] - baseline[4], if after { 2 } else { 1 });
    eprintln!(
        "RESOURCE-INIT after={after} status={status} phase={phase} rollback={cleanup:?} joined={} pending={} initialized-peers-cleaned={} retained-files={files} retry=REJECTED",
        shutdown.joined,
        shutdown.pending,
        shutdown.owners.len()
    );
}
