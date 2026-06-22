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
use rand_distr::{
    Beta, Binomial, ChiSquared, Distribution, Exp, FisherF, Gamma, Normal, Poisson, StandardNormal,
};

use crate::util::{err_signal, need};

thread_local! {
    static SEEDED: RefCell<Option<StdRng>> = const { RefCell::new(None) };
    /// The 64-bit seed last applied via a control-string form (or `seed`), so
    /// `rand('state')` can report a state vector that round-trips through
    /// `rand('state', s)`.
    static STATE_SEED: RefCell<Option<u64>> = const { RefCell::new(None) };
}

/// Apply a deterministic seed to the generator and remember it.
fn apply_seed(seed: u64) {
    SEEDED.with(|cell| *cell.borrow_mut() = Some(StdRng::seed_from_u64(seed)));
    STATE_SEED.with(|cell| *cell.borrow_mut() = Some(seed));
}

/// Recognize the RNG-control string forms shared by `rand`/`randn`:
///   `rand('state')` / `'seed'` / `'twister'`            → return state vector
///   `rand('state', s)` (scalar) or (captured vector)    → reseed, return []
/// Returns `Some(result)` if `args` is a control-string call, else `None` so the
/// caller proceeds with normal size-argument handling.
fn rng_control(args: &[Array]) -> Option<Flow<Vec<Array>>> {
    let first = args.first()?;
    let kind = first.as_string()?;
    let kind = kind.to_ascii_lowercase();
    if !matches!(kind.as_str(), "state" | "seed" | "twister") {
        return None;
    }
    if args.len() == 1 {
        // Report the current state as a single-element vector (the stored seed,
        // defaulting to 0). This round-trips through `rand('state', s)`.
        let s = STATE_SEED.with(|c| c.borrow().unwrap_or(0));
        return Some(Ok(vec![build_real(
            DataClass::Double,
            &[1, 1],
            vec![s as f64],
        )]));
    }
    // `rand('state', s)` — `s` may be the scalar 0 (reset), any scalar, or a
    // previously captured state vector (we use its first element).
    let arg = &args[1];
    let seed = if arg.numel() == 0 {
        0u64
    } else {
        to_f64_vec(arg).first().copied().unwrap_or(0.0) as u64
    };
    apply_seed(seed);
    Some(Ok(vec![Array::empty()]))
}

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("rand", b_rand);
    table.add_builtin("randn", b_randn);
    table.add_builtin("randi", b_randi);
    table.add_builtin("randperm", b_randperm);
    table.add_builtin("seed", b_seed);
    // FreeMat-specific distribution generators (element-wise over their params,
    // scalars broadcast). All draw from the shared seeded generator so the doc
    // fragments are reproducible under `seed(1,0)`.
    table.add_builtin("randbeta", b_randbeta);
    table.add_builtin("randbin", b_randbin);
    table.add_builtin("randchi", b_randchi);
    table.add_builtin("randexp", b_randexp);
    table.add_builtin("randf", b_randf);
    table.add_builtin("randgamma", b_randgamma);
    table.add_builtin("randmulti", b_randmulti);
    table.add_builtin("randnbin", b_randnbin);
    table.add_builtin("randnchi", b_randnchi);
    table.add_builtin("randnf", b_randnf);
    table.add_builtin("randp", b_randp);
}

/// Broadcast the (already-flattened) arguments element-wise: the output length is
/// the size of the single non-scalar argument (or 1 if all are scalars), and its
/// shape is taken from that argument. Two non-scalars of differing size are an
/// error (FreeMat requires same-size or scalar inputs).
fn broadcast_dims(args: &[&Array], name: &str) -> Flow<(Vec<usize>, usize)> {
    let mut dims = vec![1usize, 1];
    let mut len = 1usize;
    for a in args {
        let n = a.numel();
        if n != 1 {
            if len != 1 && len != n {
                return Err(err_signal(format!(
                    "{name}: arguments must be the same size or scalar"
                )));
            }
            dims = a.dims().to_vec();
            len = n;
        }
    }
    Ok((dims, len))
}

/// Index into a broadcast operand: element `i`, or the lone value if scalar.
fn at(v: &[f64], i: usize) -> f64 {
    if v.len() == 1 { v[0] } else { v[i] }
}

/// Like [`draw`], but the per-element closure may fail (e.g. invalid
/// distribution parameters), threading the index through so each draw can pick
/// its own broadcast parameters.
fn draw_try<F>(count: usize, mut f: F) -> Flow<Vec<f64>>
where
    F: FnMut(usize, &mut dyn rand::RngCore) -> Flow<f64>,
{
    SEEDED.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(rng) = slot.as_mut() {
            (0..count).map(|i| f(i, rng)).collect()
        } else {
            let mut rng = rand::rng();
            (0..count).map(|i| f(i, &mut rng)).collect()
        }
    })
}

/// `randbeta(alpha, beta)` — Beta-distributed deviates (PDF
/// $x^{a-1}(1-x)^{b-1}/B(a,b)$ on $[0,1]$).
fn b_randbeta(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randbeta")?;
    let a = to_f64_vec(&args[0]);
    let b = to_f64_vec(&args[1]);
    let (dims, len) = broadcast_dims(&[&args[0], &args[1]], "randbeta")?;
    let data = draw_try(len, |i, rng| {
        let d =
            Beta::new(at(&a, i), at(&b, i)).map_err(|e| err_signal(format!("randbeta: {e}")))?;
        Ok(d.sample(rng))
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randbin(N, p)` — number of successes in `N` Bernoulli trials with success
/// probability `p`.
fn b_randbin(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randbin")?;
    let nv = to_f64_vec(&args[0]);
    let pv = to_f64_vec(&args[1]);
    let (dims, len) = broadcast_dims(&[&args[0], &args[1]], "randbin")?;
    let data = draw_try(len, |i, rng| {
        let trials = at(&nv, i).max(0.0).round() as u64;
        let d =
            Binomial::new(trials, at(&pv, i)).map_err(|e| err_signal(format!("randbin: {e}")))?;
        Ok(d.sample(rng) as f64)
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randchi(n)` — chi-square deviate with `n` degrees of freedom.
fn b_randchi(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "randchi")?;
    let nv = to_f64_vec(&args[0]);
    let (dims, len) = broadcast_dims(&[&args[0]], "randchi")?;
    let data = draw_try(len, |i, rng| {
        let d = ChiSquared::new(at(&nv, i)).map_err(|e| err_signal(format!("randchi: {e}")))?;
        Ok(d.sample(rng))
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randexp(lambda)` — exponential deviate with rate `lambda` (PDF
/// $\lambda e^{-\lambda x}$, mean $1/\lambda$).
fn b_randexp(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "randexp")?;
    let lv = to_f64_vec(&args[0]);
    let (dims, len) = broadcast_dims(&[&args[0]], "randexp")?;
    let data = draw_try(len, |i, rng| {
        let d = Exp::new(at(&lv, i)).map_err(|e| err_signal(format!("randexp: {e}")))?;
        Ok(d.sample(rng))
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randf(n, m)` — F-distributed deviate, the ratio
/// $(\chi^2_n/n)/(\chi^2_m/m)$ of two chi-squares.
fn b_randf(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randf")?;
    let nv = to_f64_vec(&args[0]);
    let mv = to_f64_vec(&args[1]);
    let (dims, len) = broadcast_dims(&[&args[0], &args[1]], "randf")?;
    let data = draw_try(len, |i, rng| {
        // FisherF's first parameter is the numerator dof.
        let d =
            FisherF::new(at(&nv, i), at(&mv, i)).map_err(|e| err_signal(format!("randf: {e}")))?;
        Ok(d.sample(rng))
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randgamma(a, r)` — Gamma deviate with shape `r` and rate `a` (PDF
/// $a^r x^{r-1} e^{-ax}/\Gamma(r)$, mean $r/a$).
fn b_randgamma(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randgamma")?;
    let av = to_f64_vec(&args[0]);
    let rv = to_f64_vec(&args[1]);
    let (dims, len) = broadcast_dims(&[&args[0], &args[1]], "randgamma")?;
    let data = draw_try(len, |i, rng| {
        // rand_distr's Gamma takes (shape, scale); scale = 1/rate.
        let d = Gamma::new(at(&rv, i), 1.0 / at(&av, i))
            .map_err(|e| err_signal(format!("randgamma: {e}")))?;
        Ok(d.sample(rng))
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randmulti(N, pvec)` — one multinomial draw: counts of each outcome over `N`
/// trials, returned as a vector the same shape as `pvec`. Sampled via the
/// sequential conditional-binomial method.
fn b_randmulti(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randmulti")?;
    let total = args[0].as_f64().unwrap_or(0.0).max(0.0).round() as u64;
    let p = to_f64_vec(&args[1]);
    let dims = args[1].dims();
    let out = multinomial_draw(total, &p)?;
    Ok(vec![build_real(DataClass::Double, &dims, out)])
}

/// Helper performing the actual multinomial draw from the shared RNG.
fn multinomial_draw(total: u64, p: &[f64]) -> Flow<Vec<f64>> {
    SEEDED.with(|cell| {
        let mut slot = cell.borrow_mut();
        let do_draw = |rng: &mut dyn rand::RngCore| -> Flow<Vec<f64>> {
            let mut remaining = total;
            let mut prob_left: f64 = p.iter().sum();
            let mut out = vec![0.0; p.len()];
            for (k, &pk) in p.iter().enumerate() {
                if k + 1 == p.len() {
                    out[k] = remaining as f64;
                    break;
                }
                if remaining == 0 || prob_left <= 0.0 {
                    break;
                }
                let cond = (pk / prob_left).clamp(0.0, 1.0);
                let d = Binomial::new(remaining, cond)
                    .map_err(|e| err_signal(format!("randmulti: {e}")))?;
                let c = d.sample(rng);
                out[k] = c as f64;
                remaining -= c;
                prob_left -= pk;
            }
            Ok(out)
        };
        if let Some(rng) = slot.as_mut() {
            do_draw(rng)
        } else {
            let mut rng = rand::rng();
            do_draw(&mut rng)
        }
    })
}

/// `randnbin(r, p)` — negative-binomial deviate: number of failures before the
/// `r`-th success, each trial succeeding with probability `p`. Sampled as a
/// Gamma–Poisson mixture.
fn b_randnbin(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randnbin")?;
    let rv = to_f64_vec(&args[0]);
    let pv = to_f64_vec(&args[1]);
    let (dims, len) = broadcast_dims(&[&args[0], &args[1]], "randnbin")?;
    let data = draw_try(len, |i, rng| {
        let r = at(&rv, i);
        let p = at(&pv, i);
        if !(0.0..=1.0).contains(&p) || r <= 0.0 {
            return Err(err_signal("randnbin: need r>0 and 0<=p<=1"));
        }
        // NB(r,p) ~ Poisson(Gamma(shape=r, scale=(1-p)/p)).
        let lambda = Gamma::new(r, (1.0 - p) / p)
            .map_err(|e| err_signal(format!("randnbin: {e}")))?
            .sample(rng);
        let d = Poisson::new(lambda.max(f64::MIN_POSITIVE))
            .map_err(|e| err_signal(format!("randnbin: {e}")))?;
        Ok(d.sample(rng))
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// Draw one non-central chi-square: a central chi-square with `df-1` degrees of
/// freedom plus the square of a `Normal(shift, 1)` deviate.
fn noncentral_chi2(df: f64, shift: f64, rng: &mut dyn rand::RngCore) -> Flow<f64> {
    let base = if df > 1.0 {
        ChiSquared::new(df - 1.0)
            .map_err(|e| err_signal(format!("randnchi: {e}")))?
            .sample(rng)
    } else {
        0.0
    };
    let z: f64 = Normal::new(shift, 1.0)
        .map_err(|e| err_signal(format!("randnchi: {e}")))?
        .sample(rng);
    Ok(base + z * z)
}

/// `randnchi(n, mu)` — non-central chi-square deviate with `n` degrees of
/// freedom and non-centrality shift `mu`.
fn b_randnchi(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "randnchi")?;
    let nv = to_f64_vec(&args[0]);
    let muv = to_f64_vec(&args[1]);
    let (dims, len) = broadcast_dims(&[&args[0], &args[1]], "randnchi")?;
    let data = draw_try(len, |i, rng| noncentral_chi2(at(&nv, i), at(&muv, i), rng))?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randnf(n, m, c)` — non-central F deviate: the ratio of a non-central
/// chi-square (numerator dof `n`, shift `c`) over `n` to a central chi-square
/// (`m` dof) over `m`.
fn b_randnf(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 3, "randnf")?;
    let nv = to_f64_vec(&args[0]);
    let mv = to_f64_vec(&args[1]);
    let cv = to_f64_vec(&args[2]);
    let (dims, len) = broadcast_dims(&[&args[0], &args[1], &args[2]], "randnf")?;
    let data = draw_try(len, |i, rng| {
        let n = at(&nv, i);
        let m = at(&mv, i);
        let num = noncentral_chi2(n, at(&cv, i), rng)? / n;
        let den = ChiSquared::new(m)
            .map_err(|e| err_signal(format!("randnf: {e}")))?
            .sample(rng)
            / m;
        Ok(num / den)
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randp(nu)` — Poisson deviate with rate `nu`.
fn b_randp(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "randp")?;
    let nuv = to_f64_vec(&args[0]);
    let (dims, len) = broadcast_dims(&[&args[0]], "randp")?;
    let data = draw_try(len, |i, rng| {
        let nu = at(&nuv, i);
        if nu <= 0.0 {
            return Ok(0.0);
        }
        let d = Poisson::new(nu).map_err(|e| err_signal(format!("randp: {e}")))?;
        Ok(d.sample(rng))
    })?;
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

/// `randperm(n)` — a random permutation of `1:n` (Fisher–Yates).
fn b_randperm(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "randperm")?;
    let n = args[0].as_f64().unwrap_or(0.0).max(0.0) as usize;
    let mut perm: Vec<f64> = (1..=n).map(|x| x as f64).collect();
    // Fisher–Yates using the shared (possibly seeded) generator.
    if n > 1 {
        let swaps = draw(n - 1, |rng| rng.random::<f64>());
        for (i, &r) in swaps.iter().enumerate() {
            // pick j in [i, n)
            let j = i + ((r * (n - i) as f64) as usize).min(n - i - 1);
            perm.swap(i, j);
        }
    }
    Ok(vec![build_real(DataClass::Double, &[1, n], perm)])
}

/// Run `f` with the active generator: the seeded one if `seed`/`rand('state',…)`
/// has been applied, else a fresh thread RNG. Exposed so other modules (e.g. the
/// sparse constructors) draw from the same reproducible stream, which is what
/// makes their doc fragments deterministic under `seed(1,0)`.
pub(crate) fn with_rng<T>(f: impl FnOnce(&mut dyn rand::RngCore) -> T) -> T {
    SEEDED.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(rng) = slot.as_mut() {
            f(rng)
        } else {
            let mut rng = rand::rng();
            f(&mut rng)
        }
    })
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
    if let Some(r) = rng_control(args) {
        return r;
    }
    let dims = dims_from_args(args);
    let count: usize = dims.iter().product();
    let data = draw(count, |rng| rng.random::<f64>());
    Ok(vec![build_real(DataClass::Double, &dims, data)])
}

fn b_randn(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if let Some(r) = rng_control(args) {
        return r;
    }
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
    apply_seed(seed);
    Ok(vec![])
}
