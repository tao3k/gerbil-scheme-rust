// SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
use gerbil_scheme_python::{
    OwnedValue, ValueLimits,
    pyo3::{
        Python,
        types::{PyAnyMethods, PyDictMethods},
    },
};
use std::ffi::CString;

#[test]
fn owned_transport_preserves_integer_boolean_bytes_and_nested_data_without_vm() {
    Python::initialize();
    Python::attach(|py| {
        let source = CString::new("{'integer': 2**300, 'boolean': False, 'bytes': b'\\x00\\xff', 'rows': [[1, 'x'], None]}").unwrap();
        let value = py.eval(&source, None, None).unwrap();
        let owned = OwnedValue::from_python(&value, ValueLimits::default()).unwrap();
        let projected = owned.to_python(py).unwrap();
        assert!(value.eq(projected.bind(py)).unwrap());
        let OwnedValue::Map(fields) = owned else {
            panic!("map");
        };
        assert!(matches!(fields["boolean"], OwnedValue::Boolean(false)));
        assert!(matches!(&fields["integer"], OwnedValue::Integer(v) if v.bits() == 301));
        assert_eq!(fields["bytes"], OwnedValue::Bytes(vec![0, 255]));
    });
}

#[test]
fn owned_transport_rejects_cycles_caps_and_foreign_objects() {
    Python::initialize();
    Python::attach(|py| {
        for expression in ["object()", "{1: 'not a string key'}", "[1, 2]"] {
            let value = py
                .eval(&CString::new(expression).unwrap(), None, None)
                .unwrap();
            assert!(OwnedValue::from_python(&value, ValueLimits { depth: 2, nodes: 2 }).is_err());
        }
        let locals = gerbil_scheme_python::pyo3::types::PyDict::new(py);
        py.run(
            &CString::new("a = []; a.append(a)").unwrap(),
            None,
            Some(&locals),
        )
        .unwrap();
        let value = locals.get_item("a").unwrap().unwrap();
        assert!(
            OwnedValue::from_python(
                &value,
                ValueLimits {
                    depth: 4,
                    nodes: 32
                }
            )
            .is_err()
        );
    });
}

#[test]
fn ceilings_preserve_boolean_integer_separation_and_host_word_bounds() {
    assert_eq!(OwnedValue::Integer(7.into()).positive_usize().unwrap(), 7);
    for value in [
        OwnedValue::Boolean(true),
        OwnedValue::Integer(0.into()),
        OwnedValue::Integer((-1).into()),
        OwnedValue::Integer(num_bigint::BigInt::from(1) << usize::BITS),
    ] {
        assert!(value.positive_usize().is_err());
    }
}
