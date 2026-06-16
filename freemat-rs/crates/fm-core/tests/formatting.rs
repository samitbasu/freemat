//! Golden-string tests for MATLAB-style display.
//!
//! These assert the *value body* (no `ans =` prefix, which the interpreter
//! adds). Output follows FreeMat/MATLAB `format short` conventions: integers
//! print without a decimal point, non-integers with 4 decimals, complex as
//! `re + imi`, char as text, empty as `[]`.

use fm_core::{Array, C64, FormatMode, ScalarValue};

fn short(a: &Array) -> String {
    a.format(FormatMode::Short)
}

#[test]
fn scalar_integers() {
    assert_eq!(short(&Array::double(3.0)), "3");
    assert_eq!(short(&Array::double(-5.0)), "-5");
    assert_eq!(short(&Array::double(0.0)), "0");
    assert_eq!(short(&Array::int32(42)), "42");
    assert_eq!(short(&Array::bool(true)), "1");
    assert_eq!(short(&Array::bool(false)), "0");
}

#[test]
fn scalar_reals() {
    assert_eq!(short(&Array::double(3.5)), "3.5000");
    assert_eq!(short(&Array::double(std::f64::consts::PI)), "3.1416");
    assert_eq!(short(&Array::double(0.001)), "0.0010");
}

#[test]
fn scalar_long_mode() {
    assert_eq!(
        Array::double(std::f64::consts::PI).format(FormatMode::Long),
        "3.141592653589793"
    );
}

#[test]
fn scalar_special_values() {
    assert_eq!(short(&Array::double(f64::NAN)), "NaN");
    assert_eq!(short(&Array::double(f64::INFINITY)), "Inf");
    assert_eq!(short(&Array::double(f64::NEG_INFINITY)), "-Inf");
}

#[test]
fn scalar_exponential() {
    // Non-integer, very large magnitude -> exponential notation.
    assert_eq!(short(&Array::double(1.2345e7 + 0.5)), "1.2345e+07");
    // Very small magnitude -> exponential.
    assert_eq!(short(&Array::double(1.5e-8)), "1.5000e-08");
}

#[test]
fn large_integer_scalar_prints_as_integer() {
    // MATLAB prints integer-valued reals without a decimal point or exponent.
    assert_eq!(short(&Array::double(12_345_000.0)), "12345000");
}

#[test]
fn complex_scalars() {
    assert_eq!(short(&Array::complex64(C64::new(3.0, 4.0))), "3 + 4i");
    assert_eq!(short(&Array::complex64(C64::new(3.0, -4.0))), "3 - 4i");
    assert_eq!(
        short(&Array::complex64(C64::new(1.5, 2.5))),
        "1.5000 + 2.5000i"
    );
}

#[test]
fn char_strings() {
    assert_eq!(short(&Array::char_string("hello")), "hello");
    assert_eq!(short(&Array::char_scalar('x')), "x");
}

#[test]
fn empty_array() {
    assert_eq!(short(&Array::empty()), "[]");
}

#[test]
fn integer_matrix() {
    // [1 2; 3 4] column-major -> [1,3,2,4]
    let a = Array::double_matrix(&[2, 2], vec![1.0, 3.0, 2.0, 4.0]);
    assert_eq!(short(&a), "   1   2\n   3   4");
}

#[test]
fn real_matrix_alignment() {
    // Mixed-magnitude integers right-align in a shared column width.
    let a = Array::double_matrix(&[1, 3], vec![1.0, 20.0, 300.0]);
    assert_eq!(short(&a), "     1    20   300");
}

#[test]
fn non_integer_matrix() {
    let a = Array::double_matrix(&[1, 2], vec![1.5, 2.25]);
    assert_eq!(short(&a), "   1.5000   2.2500");
}

#[test]
fn row_vector_of_ints() {
    let a = Array::int32_matrix(&[1, 3], vec![10, 20, 30]);
    assert_eq!(short(&a), "   10   20   30");
}

#[test]
fn cell_display_lists_elements() {
    let c = Array::cell(&[1, 2], vec![Array::double(1.0), Array::char_string("hi")]);
    let out = short(&c);
    assert!(out.starts_with('{'));
    assert!(out.contains("[1]"));
    assert!(out.contains("['hi']"));
    assert!(out.ends_with('}'));
}

#[test]
fn struct_display_lists_fields() {
    use fm_core::StructArray;
    let s = StructArray::scalar([
        ("a".to_string(), Array::double(1.0)),
        ("b".to_string(), Array::double(2.0)),
    ]);
    let out = short(&Array::struct_array(s));
    assert!(out.starts_with("1x1 struct array with fields:"));
    assert!(out.contains("    a"));
    assert!(out.contains("    b"));
}

#[test]
fn uint8_scalar_via_scalarvalue() {
    assert_eq!(short(&Array::scalar(ScalarValue::UInt8(255))), "255");
}
