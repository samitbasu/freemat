//! Random-number builtins: `rand` (uniform [0,1)), `randn` (standard normal),
//! `randi` (uniform integers). Backed by `rand` + `rand_distr`.

use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};
use rand::Rng;
use rand_distr::{Distribution, StandardNormal};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("rand", b_rand);
    table.add_builtin("randn", b_randn);
    table.add_builtin("randi", b_randi);
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
    let mut rng = rand::rng();
    let data: Vec<f64> = (0..count).map(|_| rng.random::<f64>()).collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_randn(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let dims = dims_from_args(args);
    let count: usize = dims.iter().product();
    let mut rng = rand::rng();
    let normal = StandardNormal;
    let data: Vec<f64> = (0..count).map(|_| normal.sample(&mut rng)).collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randi(imax)` or `randi(imax, ...)` — uniform integers in `[1, imax]`.
fn b_randi(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "randi")?;
    let imax = args[0].as_f64().unwrap_or(1.0).max(1.0) as i64;
    let dims = dims_from_args(&args[1..]);
    let count: usize = dims.iter().product();
    let mut rng = rand::rng();
    let data: Vec<f64> = (0..count)
        .map(|_| rng.random_range(1..=imax) as f64)
        .collect();
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}
