use gerbil_scheme::{
    ByteOrder, BytestringDelimiter, GerbilRuntime, IntegerDecoding, IntegerEncoding, IntegerWidth,
    RootedSchemeValue, RootedSchemeValueKind,
};

#[test]
fn rooted_scheme_value_preserves_typed_projections_and_single_owner_drop() {
    let runtime = GerbilRuntime::initialize().expect("initialize live Gerbil runtime");
    let exact: RootedSchemeValue<'_> = runtime
        .exact_integer_from_i64(-23)
        .expect("root exact integer")
        .into();

    let fixture = runtime
        .fixture_bytevector_value()
        .expect("export bytevector fixture");
    let borrowed = fixture
        .as_bytevector()
        .into_result()
        .expect("project bytevector fixture");
    let string: RootedSchemeValue<'_> = borrowed
        .to_bytestring(BytestringDelimiter::SPACE)
        .into_result()
        .expect("root Scheme string")
        .into();

    let width = IntegerWidth::new(2).expect("two-byte integer width");
    let encoding = IntegerEncoding::fixed(ByteOrder::Big, width);
    let decoding = IntegerDecoding::entire(ByteOrder::Big);
    let bytevector: RootedSchemeValue<'_> = runtime
        .uint_to_bytevector(258, encoding)
        .expect("root Scheme bytevector")
        .into();

    let values = [exact, string, bytevector];
    assert_eq!(values[0].kind(), RootedSchemeValueKind::ExactInteger);
    assert_eq!(values[1].kind(), RootedSchemeValueKind::String);
    assert_eq!(values[2].kind(), RootedSchemeValueKind::Bytevector);

    assert_eq!(
        values[0]
            .as_exact_integer()
            .expect("typed exact integer")
            .to_i64()
            .into_result()
            .expect("project exact integer"),
        -23,
    );
    assert!(values[0].as_string().is_none());
    assert_eq!(
        values[1]
            .as_string()
            .expect("typed Scheme string")
            .to_string()
            .into_result()
            .expect("copy Scheme string"),
        "FF 7F 0B 01 00",
    );
    assert!(values[1].as_bytevector().is_none());
    assert_eq!(
        values[2]
            .as_bytevector()
            .expect("typed Scheme bytevector")
            .to_uint(decoding)
            .into_result()
            .expect("decode Scheme bytevector"),
        258,
    );
    assert!(values[2].as_exact_integer().is_none());
}
