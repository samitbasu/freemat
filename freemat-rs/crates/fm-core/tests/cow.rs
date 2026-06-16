//! Copy-on-write semantics: sharing is cheap; mutation does not affect clones.

use fm_core::Array;

#[test]
fn clone_shares_buffer_until_mutation() {
    let original = Array::double_matrix(&[1, 3], vec![1.0, 2.0, 3.0]);
    let shared = original.clone();

    // Cloning an Arc-backed array does NOT deep-copy: the buffer is shared.
    assert!(!original.is_unique());
    assert!(!shared.is_unique());

    drop(shared);
    // Now the original is sole owner again.
    assert!(original.is_unique());
}

#[test]
fn mutating_a_clone_does_not_affect_the_original() {
    let original = Array::double_matrix(&[1, 3], vec![1.0, 2.0, 3.0]);
    let mut clone = original.clone();

    // make_mut triggers the copy-on-write deep copy because the Arc is shared.
    let buf = clone.make_mut_double().unwrap();
    buf[[0, 0]] = 99.0;

    // The original is untouched.
    if let Array::Double(a) = &original {
        assert_eq!(a.as_slice_memory_order().unwrap(), &[1.0, 2.0, 3.0]);
    } else {
        panic!("expected Double");
    }
    if let Array::Double(a) = &clone {
        assert_eq!(a.as_slice_memory_order().unwrap(), &[99.0, 2.0, 3.0]);
    } else {
        panic!("expected Double");
    }
    // After the COW split, both are uniquely owned.
    assert!(original.is_unique());
    assert!(clone.is_unique());
}

#[test]
fn unique_array_mutates_in_place_without_copy() {
    let mut a = Array::double_matrix(&[1, 2], vec![1.0, 2.0]);
    // No clones exist, so make_mut should not deep-copy. We can't observe the
    // pointer directly, but uniqueness must hold before and after.
    assert!(a.is_unique());
    let buf = a.make_mut_double().unwrap();
    buf[[0, 1]] = 5.0;
    assert!(a.is_unique());
    if let Array::Double(arr) = &a {
        assert_eq!(arr[[0, 1]], 5.0);
    }
}

#[test]
fn scalars_are_always_unique() {
    let s = Array::double(3.0);
    assert!(s.is_unique());
    let c = s.clone();
    // Inline scalars own their value by copy — both are "unique".
    assert!(s.is_unique());
    assert!(c.is_unique());
}

#[test]
fn cell_cow_is_independent() {
    let original = Array::cell(&[1, 2], vec![Array::double(1.0), Array::double(2.0)]);
    let mut clone = original.clone();
    let buf = clone.make_mut_cell().unwrap();
    buf[[0, 0]] = Array::double(42.0);

    assert_eq!(
        original.as_cell().unwrap()[[0, 0]].as_f64(),
        Some(1.0),
        "original cell must be unchanged"
    );
    assert_eq!(clone.as_cell().unwrap()[[0, 0]].as_f64(), Some(42.0));
}
