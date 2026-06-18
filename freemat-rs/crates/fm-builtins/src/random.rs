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

/// `randi(imax)` or `randi(imax, ...)` — uniform integers in `[1, imax]`.
fn b_randi(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "randi")?;
    let imax = args[0].as_f64().unwrap_or(1.0).max(1.0) as i64;
    let dims = dims_from_args(&args[1..]);
    let count: usize = dims.iter().product();
    let data = draw(count, |rng| rng.random_range(1..=imax) as f64);
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
