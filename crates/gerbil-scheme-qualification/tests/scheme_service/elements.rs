//! Native collection projections preserve GC roots, exact values and bounds.
use super::*;
use gerbil_scheme::{BinaryInteger, ObjectKind, SchemeScalar};
fn count(vm: &NativeSession<'_>) -> f64 {
    vm.usage().unwrap()[3]
}
fn scalar_snapshot(vm: &NativeSession<'_>) -> Result<(), InstanceError> {
    let scalar = vm.scalar(SchemeScalar::Fixnum(7))?;
    let vector = vm.vector(&vec![&scalar; 16384])?;
    let baseline = count(vm);
    let old = vm.elements(&vector)?;
    assert!((count(vm) - baseline - 16384.0).abs() < f64::EPSILON);
    drop(old);
    let view = vm.elements_snapshot(&vector)?;
    assert_eq!(view.len(), 16384);
    assert!(!view.is_empty());
    assert!((count(vm) - baseline - 1.0).abs() < f64::EPSILON);
    for index in 0..view.len() {
        assert_eq!(view.scalar(index)?, Some(SchemeScalar::Fixnum(7)));
    }
    assert_eq!(view.kind(0)?, ObjectKind::ExactInteger);
    assert!(view.scalar(view.len()).is_err());
    assert!(view.kind(usize::MAX).is_err());
    assert!(view.rooted(view.len()).is_err());
    drop(vector);
    vm.collect()?;
    assert_eq!(view.scalar(16383)?, Some(SchemeScalar::Fixnum(7)));
    Ok(())
}
fn complex_snapshot(vm: &NativeSession<'_>) -> Result<(), InstanceError> {
    let big = BinaryInteger::new(false, vec![128; 32]).unwrap();
    let integer = vm.big_integer(&big)?;
    let parent = vm.vector(&[&integer, &integer])?;
    let previous = vm.elements(&parent)?;
    let view = vm.elements_snapshot(&parent)?;
    drop(vm.call_export(c"gerbil_owner_snapshot_retarget", &parent)?);
    assert_eq!(
        vm.copy_scalar(&vm.vector_element(&parent, 0)?)?,
        SchemeScalar::Boolean(false)
    );
    assert_eq!(view.kind(0)?, ObjectKind::ExactInteger);
    assert_eq!(view.scalar(0)?, None);
    let first = view.rooted(0)?;
    let second = view.rooted(1)?;
    assert!(vm.identical(&first, &second)?);
    assert!(vm.identical(&first, &previous[0])?);
    drop(previous);
    drop(view);
    drop(parent);
    drop(integer);
    drop(second);
    vm.collect()?;
    assert_eq!(vm.copy_big_integer(&first)?, big);
    let table = vm.table(&[(&first, &first)])?;
    let entries = vm.elements_snapshot(&table)?;
    assert_eq!(entries.len(), 2);
    assert!(vm.identical(&entries.rooted(0)?, &entries.rooted(1)?)?);
    let list = vm.list(&[])?;
    let empty = vm.elements_snapshot(&list)?;
    assert!(empty.is_empty());
    assert!(empty.scalar(0).is_err());
    Ok(())
}
fn primitive_snapshot(vm: &NativeSession<'_>) -> Result<(), InstanceError> {
    let boolean = vm.scalar(SchemeScalar::Boolean(false))?;
    let character = vm.scalar(SchemeScalar::Char('λ'))?;
    let real = vm.scalar(SchemeScalar::Flonum(-0.0))?;
    let list = vm.list(&[&boolean, &character, &real])?;
    let view = vm.elements_snapshot(&list)?;
    assert_eq!(view.scalar(0)?, Some(SchemeScalar::Boolean(false)));
    assert_eq!(view.scalar(1)?, Some(SchemeScalar::Char('λ')));
    let Some(SchemeScalar::Flonum(value)) = view.scalar(2)? else {
        panic!("flonum projection");
    };
    assert_eq!(value.to_bits(), (-0.0_f64).to_bits());
    Ok(())
}
pub(super) fn contract(dispatch: &InstanceDispatcher, progress: &mpsc::Sender<&'static str>) {
    dispatch
        .submit_native_on(0, |vm| {
            let baseline = count(vm);
            scalar_snapshot(vm)?;
            complex_snapshot(vm)?;
            primitive_snapshot(vm)?;
            assert!((count(vm) - baseline).abs() < f64::EPSILON);
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
    progress
        .send("element snapshots preserve roots, bounds and exact values")
        .unwrap();
}
