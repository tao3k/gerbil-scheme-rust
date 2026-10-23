//! Name/identity boundary, owned UTF-8 packets and failure-preserving publication.
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, RootedObjectBatch,
};
use gerbil_scheme_sys as sys;
pub(super) fn run(runtime: &GerbilRuntime) {
    identity(runtime);
    let before = unsafe { sys::gerbil_scheme_rust_root_count_raw() };
    let mut output = IdentifierBuffer::default();
    for names in [
        vec!["", "plain", "λ中", "😀", "nul\0name", "e\u{301}"],
        vec!["changed"],
        vec![],
        vec!["long", "prefix/λ😀suffix"],
    ] {
        let roots: Vec<_> = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                runtime
                    .intern_identifier(
                        if index % 2 == 0 {
                            IdentifierKind::Symbol
                        } else {
                            IdentifierKind::Keyword
                        },
                        name,
                    )
                    .unwrap()
            })
            .collect();
        let borrowed: Vec<_> = roots.iter().collect();
        let collection = runtime
            .collection_from_objects(CollectionKind::Vector, &borrowed)
            .unwrap();
        let mut batch = RootedObjectBatch::default();
        collection.copy_into(&mut batch).unwrap();
        drop(collection);
        drop(borrowed);
        drop(roots);
        assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
        batch.copy_identifier_names(&mut output).unwrap();
        assert_eq!(output.len(), names.len());
        for (index, name) in names.iter().enumerate() {
            assert_eq!(
                output.get(index),
                Some((
                    if index % 2 == 0 {
                        IdentifierKind::Symbol
                    } else {
                        IdentifierKind::Keyword
                    },
                    *name
                ))
            );
        }
        assert_eq!(output.get(names.len()), None);
        let capacity = output.reserved_bytes();
        let wrong = runtime
            .root_value(runtime.fixnum_value(0).unwrap())
            .unwrap();
        assert!(wrong.as_ref().copy_identifier_names(&mut output).is_err());
        assert_eq!(output.len(), names.len());
        assert_eq!(output.reserved_bytes(), capacity);
        let mixed = runtime
            .collection_from_objects(CollectionKind::Vector, &[&wrong])
            .unwrap();
        mixed.copy_into(&mut batch).unwrap();
        assert!(batch.copy_identifier_names(&mut output).is_err());
        assert_eq!(output.len(), names.len());
        drop(batch);
        assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
        for (index, name) in names.iter().enumerate() {
            assert_eq!(output.get(index).unwrap().1, *name);
        }
    }
    let root = runtime
        .intern_identifier(IdentifierKind::Keyword, "owned")
        .unwrap();
    root.as_ref().copy_identifier_names(&mut output).unwrap();
    root.as_ref().copy_identifier_names(&mut output).unwrap();
    let capacity = output.reserved_bytes();
    for _ in 0..20 {
        root.as_ref().copy_identifier_names(&mut output).unwrap();
        assert_eq!(output.reserved_bytes(), capacity);
    }
    drop(root);
    raw_failures(runtime);
    assert_eq!(unsafe { sys::gerbil_scheme_rust_root_count_raw() }, before);
    assert_eq!(output.get(0), Some((IdentifierKind::Keyword, "owned")));
}
fn identity(runtime: &GerbilRuntime) {
    let interned = runtime
        .intern_identifier(IdentifierKind::Symbol, "same-name")
        .unwrap();
    let repeated = runtime
        .intern_identifier(IdentifierKind::Symbol, "same-name")
        .unwrap();
    let keyword = runtime
        .intern_identifier(IdentifierKind::Keyword, "same-name")
        .unwrap();
    let first = runtime.uninterned_symbol("same-name").unwrap();
    let second = runtime.uninterned_symbol("same-name").unwrap();
    assert!(interned.as_ref().is_identical(repeated.as_ref()));
    assert!(!interned.as_ref().is_identical(keyword.as_ref()));
    assert!(!first.as_ref().is_identical(second.as_ref()));
    let collection = runtime
        .collection_from_objects(CollectionKind::Vector, &[&first, &second])
        .unwrap();
    let mut batch = RootedObjectBatch::default();
    collection.copy_into(&mut batch).unwrap();
    drop(collection);
    drop(first);
    drop(second);
    assert_eq!(unsafe { sys::gerbil_scheme_rust_force_gc_raw() }, 0);
    assert!(!batch.get(0).unwrap().is_identical(batch.get(1).unwrap()));
    let mut names = IdentifierBuffer::default();
    batch.copy_identifier_names(&mut names).unwrap();
    assert_eq!(names.get(0), names.get(1));
}
fn raw_failures(runtime: &GerbilRuntime) {
    let bytes = runtime.bytevector_from_bytes("λ".as_bytes()).unwrap();
    let mut name = sys::GerbilRootId(0);
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_root_utf8_to_string(bytes.root_id(), &raw mut name) },
        sys::GerbilStatus::Ok
    );
    let root = unsafe { sys::gerbil_scheme_rust_identifier_create(name.0, 0) };
    assert!(root > 0);
    let mut metadata = [0; 2];
    let packet =
        unsafe { sys::gerbil_scheme_rust_identifiers_prepare(root, -1, metadata.as_mut_ptr()) };
    assert!(packet > 0);
    assert_eq!(metadata, [1, 4]);
    let mut bytes = [0xa5_u8; 8];
    let mut offsets = [123_u64; 2];
    let mut kinds = [9_u8];
    for (capacity, count) in [(3, 1), (8, 2)] {
        assert_eq!(
            unsafe {
                sys::gerbil_scheme_rust_identifiers_copy(
                    packet,
                    bytes.as_mut_ptr(),
                    capacity,
                    offsets.as_mut_ptr(),
                    kinds.as_mut_ptr(),
                    count,
                )
            },
            -1
        );
        assert_eq!(bytes, [0xa5; 8]);
        assert_eq!(offsets, [123; 2]);
        assert_eq!(kinds, [9]);
    }
    assert_eq!(
        unsafe {
            sys::gerbil_scheme_rust_identifiers_copy(
                packet,
                bytes.as_mut_ptr(),
                8,
                offsets.as_mut_ptr(),
                kinds.as_mut_ptr(),
                1,
            )
        },
        2
    );
    assert_eq!(&bytes[..2], "λ".as_bytes());
    assert_eq!(offsets, [0, 2]);
    assert_eq!(kinds, [0]);
    for token in [packet, root, name.0] {
        assert_eq!(
            unsafe { sys::gerbil_scheme_rust_root_release(sys::GerbilRootId(token)) },
            sys::GerbilStatus::Ok
        );
    }
    assert_eq!(
        unsafe { sys::gerbil_scheme_rust_identifiers_prepare(root, -1, metadata.as_mut_ptr()) },
        0
    );
}
