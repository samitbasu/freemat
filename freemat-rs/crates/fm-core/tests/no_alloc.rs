//! Performance guarantee: scalar arithmetic must not heap-allocate.
//!
//! FreeMat keeps scalar temporaries in an inline union; our [`Array::Scalar`]
//! variant replicates that. This test installs a counting global allocator and
//! asserts that a tight scalar-add loop performs **zero** heap allocations.
//!
//! Both the enable flag and the counter are **thread-local**, so tests running
//! in parallel cannot pollute each other's measurements and the harness's own
//! allocations are never counted.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use fm_core::{Array, ScalarValue};

thread_local! {
    /// When `true`, allocations on this thread are counted into `ALLOCS`.
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    /// Per-thread allocation count (only bumped while `COUNTING` is set).
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

/// A global allocator that counts allocations made while this thread has
/// counting enabled, delegating to the system allocator otherwise.
struct Counting;

// Safety: every call forwards straight to `System`; we only read/write
// thread-locals around the delegation, which upholds the allocator contract.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        bump();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        bump();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Increment the per-thread counter if counting is active. Guarded with
/// `try_with` so a TLS access during thread teardown can never panic.
fn bump() {
    let _ = COUNTING.try_with(|c| {
        if c.get() {
            let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
        }
    });
}

#[global_allocator]
static GLOBAL: Counting = Counting;

/// Run `body` with allocation counting enabled on this thread; return the
/// number of allocations observed within it.
fn count_allocs(body: impl FnOnce()) -> usize {
    // Prime the thread-locals so their lazy init (which may allocate) is not
    // counted, then zero the counter and measure.
    COUNTING.with(|c| c.set(false));
    ALLOCS.with(|a| a.set(0));
    COUNTING.with(|c| c.set(true));
    body();
    COUNTING.with(|c| c.set(false));
    ALLOCS.with(Cell::get)
}

#[test]
fn scalar_loop_does_not_allocate() {
    // Construct outside the measured region.
    let mut acc = Array::double(0.0);
    let one = ScalarValue::Double(1.0);

    let allocs = count_allocs(|| {
        for _ in 0..100_000 {
            if let Array::Scalar(s) = acc {
                acc = Array::Scalar(s.add_scalar(one));
            } else {
                unreachable!("scalar add must stay on the inline path");
            }
        }
    });

    assert_eq!(allocs, 0, "scalar arithmetic allocated {allocs} times");
    assert_eq!(acc.as_f64(), Some(100_000.0));
}

#[test]
fn building_inline_scalars_does_not_allocate() {
    let mut sum = 0.0;
    let allocs = count_allocs(|| {
        for i in 0..50_000u32 {
            let a = Array::double(f64::from(i));
            sum += a.as_f64().unwrap();
        }
    });
    assert_eq!(
        allocs, 0,
        "inline scalar construction allocated {allocs} times"
    );
    assert!(sum > 0.0);
}

#[test]
fn dense_construction_does_allocate() {
    // Control: a dense array DOES hit the heap, proving the counter works.
    let allocs = count_allocs(|| {
        let a = Array::double_matrix(&[1, 4], vec![1.0, 2.0, 3.0, 4.0]);
        std::hint::black_box(&a);
    });
    assert!(allocs > 0, "dense construction should allocate");
}
