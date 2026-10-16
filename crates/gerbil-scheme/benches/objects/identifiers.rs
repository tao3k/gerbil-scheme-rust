//! Repeated complete-name calls versus one changing owned name packet.
use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, RootedObjectBatch,
};
use std::hint::black_box;
fn family(runtime: &GerbilRuntime, corpus: &str) -> bool {
    let rows: Vec<Vec<_>> = (0..5)
        .map(|row| {
            (0..8)
                .map(|index| {
                    let text = match corpus {
                        "ascii" => "alpha/field",
                        "bmp" => "λ中/名字",
                        "supplementary" => "😀/𐐷/field",
                        _ => "中😀/ascii-suffix",
                    };
                    (
                        if index % 2 == 0 {
                            IdentifierKind::Symbol
                        } else {
                            IdentifierKind::Keyword
                        },
                        format!("{row}/{index}/{text}"),
                    )
                })
                .collect()
        })
        .collect();
    let roots: Vec<Vec<_>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|(kind, name)| runtime.intern_identifier(*kind, name).unwrap())
                .collect()
        })
        .collect();
    let batches: Vec<_> = roots
        .iter()
        .map(|row| {
            let refs: Vec<_> = row.iter().collect();
            let collection = runtime
                .collection_from_objects(CollectionKind::Vector, &refs)
                .unwrap();
            let mut batch = RootedObjectBatch::default();
            collection.copy_into(&mut batch).unwrap();
            batch
        })
        .collect();
    let mut separate: Vec<_> = (0..8).map(|_| IdentifierBuffer::default()).collect();
    let mut bulk = IdentifierBuffer::default();
    for _ in 0..2 {
        for (root, buffer) in roots[0].iter().zip(&mut separate) {
            root.as_ref().copy_identifier_names(buffer).unwrap();
        }
        batches[0].copy_identifier_names(&mut bulk).unwrap();
    }
    let memory = [
        separate
            .iter()
            .map(IdentifierBuffer::reserved_bytes)
            .sum::<usize>()
            + separate.capacity() * std::mem::size_of::<IdentifierBuffer>()
            + std::mem::size_of_val(&separate),
        bulk.reserved_bytes() + std::mem::size_of_val(&bulk),
    ];
    let mut pass = true;
    for load in [1_000, 10_000, 100_000] {
        pass &= super::measure(
            &format!("identifiers-{corpus}"),
            load,
            memory,
            |mode, request| {
                let row = request % rows.len();
                if mode == 0 {
                    for (index, (root, buffer)) in roots[row].iter().zip(&mut separate).enumerate()
                    {
                        root.as_ref().copy_identifier_names(buffer).unwrap();
                        assert_eq!(
                            buffer.get(0),
                            Some((rows[row][index].0, rows[row][index].1.as_str()))
                        );
                    }
                    black_box(&separate);
                } else {
                    batches[row].copy_identifier_names(&mut bulk).unwrap();
                    for (index, (kind, name)) in rows[row].iter().enumerate() {
                        assert_eq!(bulk.get(index), Some((*kind, name.as_str())));
                    }
                    black_box(&bulk);
                }
            },
        );
    }
    pass
}
pub(super) fn run(runtime: &GerbilRuntime) -> bool {
    ["ascii", "bmp", "supplementary", "reverse-prefix"]
        .iter()
        .fold(true, |pass, corpus| family(runtime, corpus) & pass)
}
