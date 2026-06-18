//! Type-promotion rules.

use fm_core::{DataClass, promote};

#[test]
fn double_combines() {
    assert_eq!(
        promote(DataClass::Double, DataClass::Double).unwrap(),
        DataClass::Double
    );
    assert_eq!(
        promote(DataClass::Bool, DataClass::Double).unwrap(),
        DataClass::Double
    );
    assert_eq!(
        promote(DataClass::Char, DataClass::Double).unwrap(),
        DataClass::Double
    );
}

#[test]
fn single_dominates_double() {
    // FreeMat's `ComputeTypes`: if either operand is `single`, the result is
    // `single` (single dominates double; the arithmetic is done in double then
    // cast to single).
    assert_eq!(
        promote(DataClass::Float, DataClass::Float).unwrap(),
        DataClass::Float
    );
    assert_eq!(
        promote(DataClass::Float, DataClass::Bool).unwrap(),
        DataClass::Float
    );
    assert_eq!(
        promote(DataClass::Float, DataClass::Double).unwrap(),
        DataClass::Float
    );
    assert_eq!(
        promote(DataClass::Double, DataClass::Float).unwrap(),
        DataClass::Float
    );
}

#[test]
fn bool_and_char_promote_to_double() {
    // bool + bool and char + char go to double for arithmetic.
    assert_eq!(
        promote(DataClass::Bool, DataClass::Bool).unwrap(),
        DataClass::Double
    );
    assert_eq!(
        promote(DataClass::Char, DataClass::Char).unwrap(),
        DataClass::Double
    );
}

#[test]
fn integer_wins_over_float() {
    assert_eq!(
        promote(DataClass::Int32, DataClass::Double).unwrap(),
        DataClass::Int32
    );
    assert_eq!(
        promote(DataClass::Double, DataClass::Int32).unwrap(),
        DataClass::Int32
    );
    assert_eq!(
        promote(DataClass::UInt8, DataClass::Float).unwrap(),
        DataClass::UInt8
    );
    assert_eq!(
        promote(DataClass::Int16, DataClass::Bool).unwrap(),
        DataClass::Int16
    );
    assert_eq!(
        promote(DataClass::Int64, DataClass::Int64).unwrap(),
        DataClass::Int64
    );
}

#[test]
fn mixed_integers_are_an_error() {
    assert!(promote(DataClass::Int32, DataClass::UInt8).is_err());
    assert!(promote(DataClass::Int8, DataClass::Int16).is_err());
}

#[test]
fn reference_types_are_incompatible() {
    assert!(promote(DataClass::Cell, DataClass::Double).is_err());
    assert!(promote(DataClass::Double, DataClass::Struct).is_err());
}
