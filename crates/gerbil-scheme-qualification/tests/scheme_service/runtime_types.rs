//! Native numeric tower and strong table types use the existing VM.
use super::*;
use gerbil_scheme::{BinaryInteger, NumberKind, SchemeScalar};
pub(super) fn contract(dispatch: &InstanceDispatcher) {
    dispatch
        .submit_native_on(0, |session| {
            let two = session.scalar(SchemeScalar::Fixnum(2))?;
            let four = session.scalar(SchemeScalar::Fixnum(4))?;
            let zero = session.scalar(SchemeScalar::Fixnum(0))?;
            assert!(
                session
                    .number_batch(NumberKind::Rational, &[(&two, &zero)])
                    .is_err()
            );
            let fraction = session.number_batch(NumberKind::Rational, &[(&two, &four)])?;
            let components = session.number_components(&fraction, NumberKind::Rational)?;
            drop(fraction);
            session.collect()?;
            assert_eq!(
                session.copy_big_integer(&components[0])?,
                BinaryInteger::new(false, vec![1]).unwrap()
            );
            assert_eq!(
                session.copy_big_integer(&components[1])?,
                BinaryInteger::new(false, vec![2]).unwrap()
            );
            let big = BinaryInteger::new(true, vec![1, 0, 0, 0, 0, 0, 0, 0, 1]).unwrap();
            let left = session.big_integer(&big)?;
            let complex =
                session.number_batch(NumberKind::Complex, &[(&left, &two), (&four, &zero)])?;
            let parts = session.number_components(&complex, NumberKind::Complex)?;
            drop(complex);
            drop(left);
            session.collect()?;
            assert_eq!(parts.len(), 4);
            assert_eq!(session.copy_big_integer(&parts[0])?, big);
            assert_eq!(
                session.copy_big_integer(&parts[1])?,
                BinaryInteger::new(false, vec![2]).unwrap()
            );
            assert_eq!(
                session.copy_big_integer(&parts[3])?,
                BinaryInteger::default()
            );
            for gerbil in [false, true] {
                let entries = [(&two, &four), (&two, &zero)];
                let table = if gerbil {
                    session.gerbil_hash_table(gerbil_scheme::TableEquality::Structural, &entries)?
                } else {
                    session.hash_table(gerbil_scheme::TableEquality::Equivalent, &entries)?
                };
                assert_eq!(
                    session.kind(&table)?,
                    if gerbil {
                        gerbil_scheme::ObjectKind::GerbilHashTable
                    } else {
                        gerbil_scheme::ObjectKind::GambitTable
                    }
                );
                let snapshot = session.elements(&table)?;
                drop(table);
                session.collect()?;
                assert_eq!(snapshot.len(), 2);
                assert_eq!(
                    session.copy_big_integer(&snapshot[1])?,
                    BinaryInteger::default()
                );
            }
            let text = session.string("not a number")?;
            assert!(
                session
                    .number_components(&text, NumberKind::Complex)
                    .is_err()
            );
            Ok(())
        })
        .unwrap()
        .wait(GATE)
        .unwrap();
}
