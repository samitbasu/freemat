//! Construction of each value type, plus class/shape queries.

use fm_core::{Array, C64, DataClass, ScalarValue};

#[test]
fn scalar_constructors_are_inline() {
    assert!(matches!(Array::double(3.0), Array::Scalar(_)));
    assert!(matches!(Array::bool(true), Array::Scalar(_)));
    assert!(matches!(Array::int32(7), Array::Scalar(_)));
    assert!(matches!(Array::single(1.5), Array::Scalar(_)));
    assert!(matches!(Array::char_scalar('a'), Array::Scalar(_)));
    assert!(matches!(
        Array::complex64(C64::new(1.0, 2.0)),
        Array::Scalar(_)
    ));
}

#[test]
fn scalar_classes() {
    assert_eq!(Array::double(3.0).class(), DataClass::Double);
    assert_eq!(Array::single(3.0).class(), DataClass::Float);
    assert_eq!(Array::bool(true).class(), DataClass::Bool);
    assert_eq!(Array::int32(1).class(), DataClass::Int32);
    assert_eq!(Array::char_scalar('z').class(), DataClass::Char);
    assert_eq!(
        Array::complex64(C64::new(0.0, 1.0)).class(),
        DataClass::Double
    );
}

#[test]
fn class_names_match_matlab() {
    assert_eq!(Array::double(1.0).class_name(), "double");
    assert_eq!(Array::single(1.0).class_name(), "single");
    assert_eq!(Array::bool(true).class_name(), "logical");
    assert_eq!(Array::int32(1).class_name(), "int32");
    assert_eq!(Array::scalar(ScalarValue::UInt8(1)).class_name(), "uint8");
    assert_eq!(Array::char_scalar('a').class_name(), "char");
}

#[test]
fn dense_double_matrix_is_column_major() {
    // [1 2; 3 4] in MATLAB stores column-major: [1, 3, 2, 4].
    let a = Array::double_matrix(&[2, 2], vec![1.0, 3.0, 2.0, 4.0]);
    assert_eq!(a.class(), DataClass::Double);
    assert_eq!(a.dims(), vec![2, 2]);
    assert_eq!(a.numel(), 4);
    // Read back element (row 0, col 1) == 2.
    if let Array::Double(arr) = &a {
        assert_eq!(arr[[0, 1]], 2.0);
        assert_eq!(arr[[1, 0]], 3.0);
        // Memory order is column-major.
        assert_eq!(arr.as_slice_memory_order().unwrap(), &[1.0, 3.0, 2.0, 4.0]);
    } else {
        panic!("expected Double");
    }
}

#[test]
fn dense_variants_construct() {
    assert_eq!(
        Array::bool_matrix(&[1, 3], vec![true, false, true]).class(),
        DataClass::Bool
    );
    assert_eq!(
        Array::int32_matrix(&[2, 1], vec![1, 2]).class(),
        DataClass::Int32
    );
    assert_eq!(
        Array::single_matrix(&[1, 2], vec![1.0, 2.0]).class(),
        DataClass::Float
    );
    let c = Array::complex64_matrix(&[1, 2], vec![C64::new(1.0, 1.0), C64::new(2.0, -1.0)]);
    assert_eq!(c.class(), DataClass::Double);
    assert!(c.is_complex());
}

#[test]
fn char_string_construction() {
    let s = Array::char_string("hi");
    assert_eq!(s.class(), DataClass::Char);
    assert_eq!(s.dims(), vec![1, 2]);
    assert_eq!(s.as_string().as_deref(), Some("hi"));

    // Single char collapses to inline scalar.
    let one = Array::char_string("x");
    assert!(matches!(one, Array::Scalar(_)));
    assert_eq!(one.as_string().as_deref(), Some("x"));
}

#[test]
fn cell_construction() {
    let c = Array::cell(&[1, 2], vec![Array::double(1.0), Array::char_string("ab")]);
    assert_eq!(c.class(), DataClass::Cell);
    assert_eq!(c.numel(), 2);
    let cells = c.as_cell().unwrap();
    assert_eq!(cells[[0, 0]].as_f64(), Some(1.0));
    assert_eq!(cells[[0, 1]].as_string().as_deref(), Some("ab"));
}

#[test]
fn struct_construction() {
    use fm_core::StructArray;
    let s = StructArray::scalar([
        ("name".to_string(), Array::char_string("freemat")),
        ("count".to_string(), Array::double(3.0)),
    ]);
    let a = Array::struct_array(s);
    assert_eq!(a.class(), DataClass::Struct);
    let st = a.as_struct().unwrap();
    assert_eq!(st.field_names(), vec!["name", "count"]);
    assert_eq!(st.scalar_field("count").and_then(Array::as_f64), Some(3.0));
}

#[test]
fn empty_array() {
    let e = Array::empty();
    assert!(e.is_empty());
    assert_eq!(e.numel(), 0);
    assert_eq!(e.dims(), vec![0, 0]);
}

#[test]
fn scalar_extraction() {
    assert_eq!(Array::double(2.5).as_f64(), Some(2.5));
    assert_eq!(Array::int32(9).as_f64(), Some(9.0));
    assert_eq!(Array::bool(true).as_f64(), Some(1.0));
    // 1-element dense reads out as a scalar.
    let one = Array::double_matrix(&[1, 1], vec![42.0]);
    assert_eq!(one.as_f64(), Some(42.0));
    assert!(one.is_scalar());
    // Multi-element doesn't.
    assert_eq!(Array::double_matrix(&[1, 2], vec![1.0, 2.0]).as_f64(), None);
}
