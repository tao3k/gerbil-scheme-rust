//! Whole rooted numeric/table snapshots versus per-element strong rooting.
use gerbil_scheme::{
    GerbilRuntime, NumberKind, RootedObjectBatch, RootedSchemeObject, SchemeObjectRef,
    SchemeScalar, TableEquality,
};
use std::hint::black_box;
fn int(runtime: &GerbilRuntime, n: isize) -> RootedSchemeObject<'_> {
    runtime
        .root_value(runtime.fixnum_value(n).unwrap())
        .unwrap()
}
fn number(value: SchemeObjectRef<'_>) -> isize {
    match value.scalar().unwrap() {
        SchemeScalar::Fixnum(n) => n,
        _ => panic!("fixture type"),
    }
}
fn validate<'a>(family: usize, row: usize, references: impl Iterator<Item = SchemeObjectRef<'a>>) {
    let mut values = references.map(number);
    for _ in 0..8 {
        let pair = [values.next().unwrap(), values.next().unwrap()];
        match family {
            0 => {
                assert_eq!(pair[1], 2);
                assert!(pair[0] % 2 == 1);
            }
            1 => {
                assert!(pair[0] > isize::try_from(row * 2).unwrap());
                assert_eq!(pair[1], -3);
            }
            _ => assert_eq!(pair[1], pair[0] * 3),
        }
    }
    assert!(values.next().is_none());
}
fn snapshot<'r>(family: usize, root: &RootedSchemeObject<'r>, out: &mut RootedObjectBatch<'r>) {
    if family == 2 {
        root.copy_table_entries(out).unwrap();
    } else {
        root.copy_number_components(
            if family == 0 {
                NumberKind::Rational
            } else {
                NumberKind::Complex
            },
            out,
        )
        .unwrap();
    }
}
fn family(runtime: &GerbilRuntime, family: usize, name: &str) -> bool {
    let numbers: Vec<Vec<_>> = (0..5)
        .map(|row| {
            (0..8)
                .map(|index| int(runtime, isize::try_from(index * 2 + row * 2 + 1).unwrap()))
                .collect()
        })
        .collect();
    let second = int(runtime, if family == 0 { 2 } else { -3 });
    let roots: Vec<_> = numbers
        .iter()
        .map(|row| {
            if family == 2 {
                let values: Vec<_> = row
                    .iter()
                    .map(|key| int(runtime, number(key.as_ref()) * 3))
                    .collect();
                let entries: Vec<_> = row.iter().zip(&values).collect();
                runtime
                    .hash_table(TableEquality::Equivalent, &entries)
                    .unwrap()
            } else {
                let pairs: Vec<_> = row.iter().map(|left| (left, &second)).collect();
                runtime
                    .number_batch(
                        if family == 0 {
                            NumberKind::Rational
                        } else {
                            NumberKind::Complex
                        },
                        &pairs,
                    )
                    .unwrap()
            }
        })
        .collect();
    let mut output = RootedObjectBatch::default();
    snapshot(family, &roots[0], &mut output);
    let mut individual = Vec::with_capacity(16);
    let memory = [
        output.reserved_bytes()
            + 16 * std::mem::size_of::<RootedSchemeObject<'_>>()
            + std::mem::size_of_val(&individual),
        output.reserved_bytes(),
    ];
    let mut pass = true;
    for load in [1000, 10_000, 100_000] {
        pass &= super::measure(name, load, memory, |mode, request| {
            let row = request % roots.len();
            individual.clear();
            snapshot(family, &roots[row], &mut output);
            if mode == 0 {
                for index in 0..output.len() {
                    individual.push(output.root_element(index).unwrap());
                }
                validate(
                    family,
                    row,
                    individual.iter().map(RootedSchemeObject::as_ref),
                );
            } else {
                validate(
                    family,
                    row,
                    (0..output.len()).map(|i| output.get(i).unwrap()),
                );
            }
            black_box(&output);
        });
    }
    pass
}
pub(super) fn run(runtime: &GerbilRuntime) -> bool {
    family(runtime, 0, "rational8") & family(runtime, 1, "complex8") & family(runtime, 2, "table8")
}
