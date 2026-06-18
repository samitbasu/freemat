//! Random-number builtins: `rand` (uniform [0,1)), `randn` (standard normal),
//! `randi` (uniform integers), and `seed` (deterministic reseeding). Backed by
//! `rand` + `rand_distr`.
//!
//! A thread-local optional [`StdRng`] makes `seed(...)` reproducible: once set,
//! all draws come from the seeded generator (FreeMat's `seed` semantics); until
//! then, draws use the OS entropy source via `rand::rng()`.

use std::cell::RefCell;

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, StandardNormal};

use crate::util::need;

thread_local! {
    static SEEDED: RefCell<Option<StdRng>> = const { RefCell::new(None) };
}

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("rand", b_rand);
    table.add_builtin("randn", b_randn);
    table.add_builtin("randi", b_randi);
    table.add_builtin("seed", b_seed);
}

/// Draw `count` values using `f`, from the seeded generator if one is active,
/// else from the thread RNG.
fn draw<F: FnMut(&mut dyn rand::RngCore) -> f64>(count: usize, mut f: F) -> Vec<f64> {
    SEEDED.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(rng) = slot.as_mut() {
            (0..count).map(|_| f(rng)).collect()
        } else {
            let mut rng = rand::rng();
            (0..count).map(|_| f(&mut rng)).collect()
        }
    })
}

fn dims_from_args(args: &[Array]) -> Vec<usize> {
    if args.is_empty() {
        return vec![1, 1];
    }
    if args.len() == 1 {
        if args[0].numel() == 1 {
            let n = args[0].as_f64().unwrap_or(0.0).max(0.0) as usize;
            return vec![n, n];
        }
        return to_f64_vec(&args[0])
            .into_iter()
            .map(|x| x.max(0.0) as usize)
            .collect();
    }
    args.iter()
        .map(|a| a.as_f64().unwrap_or(0.0).max(0.0) as usize)
        .collect()
}

fn b_rand(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let dims = dims_from_args(args);
    let count: usize = dims.iter().product();
    let data = draw(count, |rng| rng.random::<f64>());
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_randn(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let dims = dims_from_args(args);
    let count: usize = dims.iter().product();
    let normal = StandardNormal;
    let data = draw(count, |rng| normal.sample(rng));
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randi(low, high)` — element-wise uniform integers in `[low(i), high(i)]`.
///
/// This is FreeMat's `RandIFunction` (a binary dot-op over `low`/`high`), **not**
/// MATLAB's `randi(imax, dims)`. The two arguments broadcast: a scalar pairs
/// with every element of the other operand, otherwise the shapes must match.
fn b_randi(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randi")?;
    let lo = to_f64_vec(&args[0]);
    let hi = to_f64_vec(&args[1]);
    // Broadcast: pick the non-scalar shape (or the first if both match/scalar).
    let (dims, count) = if args[0].numel() == 1 {
        (args[1].dims(), hi.len())
    } else {
        (args[0].dims(), lo.len())
    };
    if lo.len() != 1 && hi.len() != 1 && lo.len() != hi.len() {
        return Err(crate::util::err_signal(
            "randi: low and high must be the same size or scalar",
        ));
    }
    let at = |v: &[f64], i: usize| if v.len() == 1 { v[0] } else { v[i] };
    let data = {
        let mut idx = 0;
        draw(count, move |rng| {
            let l = at(&lo, idx) as i64;
            let h = at(&hi, idx) as i64;
            idx += 1;
            let (l, h) = if l <= h { (l, h) } else { (h, l) };
            rng.random_range(l..=h) as f64
        })
    };
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `seed(a)` / `seed(a, b)` — reseed the generator deterministically. The
/// argument(s) are combined into a 64-bit seed so that an identical call
/// reproduces an identical sequence (FreeMat semantics).
fn b_seed(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "seed")?;
    let a = args[0].as_f64().unwrap_or(0.0) as u64;
    let b = args.get(1).and_then(Array::as_f64).unwrap_or(0.0) as u64;
    let seed = a.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(b);
    SEEDED.with(|cell| {
        *cell.borrow_mut() = Some(StdRng::seed_from_u64(seed));
    });
    Ok(vec![])
}
