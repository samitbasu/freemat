//! Linear-algebra builtins wrapping [`fm_linalg`]: `inv`, `det`, `eig`, `svd`,
//! `lu`, `qr`, `chol`, `norm`, `rank`, `pinv`, `trace`, `transpose`, `mtimes`.

use fm_core::{Array, DataClass};
use fm_interp::error::{Flow, InterpError, Signal};
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};
use fm_linalg::{LinalgError, NormKind};

use crate::util::need;

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("inv", b_inv);
    table.add_builtin("det", b_det);
    table.add_builtin("eig", b_eig);
    table.add_builtin("eigs", b_eigs);
    table.add_builtin("svd", b_svd);
    table.add_builtin("lu", b_lu);
    table.add_builtin("qr", b_qr);
    table.add_builtin("chol", b_chol);
    table.add_builtin("norm", b_norm);
    table.add_builtin("xnrm2", b_xnrm2);
    table.add_builtin("rank", b_rank);
    table.add_builtin("pinv", b_pinv);
    table.add_builtin("trace", b_trace);
    table.add_builtin("cond", b_cond);
    table.add_builtin("rcond", b_rcond);
    table.add_builtin("rref", b_rref);
    table.add_builtin("kron", b_kron);
    table.add_builtin("null", b_null);
    table.add_builtin("orth", b_orth);
    table.add_builtin("tril", b_tril);
    table.add_builtin("triu", b_triu);
}

fn b_cond(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "cond")?;
    Ok(vec![fm_linalg::cond(&args[0]).map_err(wrap)?])
}

fn b_rcond(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "rcond")?;
    Ok(vec![fm_linalg::rcond(&args[0]).map_err(wrap)?])
}

fn b_rref(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "rref")?;
    Ok(vec![fm_linalg::rref(&args[0]).map_err(wrap)?])
}

fn b_kron(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 2, "kron")?;
    Ok(vec![fm_linalg::kron(&args[0], &args[1]).map_err(wrap)?])
}

fn b_null(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "null")?;
    Ok(vec![fm_linalg::null(&args[0]).map_err(wrap)?])
}

fn b_orth(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "orth")?;
    Ok(vec![fm_linalg::orth(&args[0]).map_err(wrap)?])
}

fn b_tril(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "tril")?;
    let k = args.get(1).and_then(Array::as_f64).unwrap_or(0.0) as i64;
    Ok(vec![fm_linalg::tril(&args[0], k).map_err(wrap)?])
}

fn b_triu(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "triu")?;
    let k = args.get(1).and_then(Array::as_f64).unwrap_or(0.0) as i64;
    Ok(vec![fm_linalg::triu(&args[0], k).map_err(wrap)?])
}

fn wrap(e: LinalgError) -> Signal {
    Signal::Error(InterpError::msg(e.message))
}

fn b_inv(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "inv")?;
    Ok(vec![fm_linalg::inv(&args[0]).map_err(wrap)?])
}

fn b_det(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "det")?;
    Ok(vec![fm_linalg::det(&args[0]).map_err(wrap)?])
}

fn b_eig(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "eig")?;
    // `eig(A, B)` — generalized eigenproblem of the pencil (A, B) — but only when
    // the second argument is a numeric matrix. A char second argument is an
    // option string (e.g. `eig(A, 'nobalance')`), handled by the standard path.
    if let Some(b) = args.get(1)
        && b.class() != DataClass::Char
    {
        return fm_linalg::eig_gen(&args[0], b, nargout).map_err(wrap);
    }
    fm_linalg::eig(&args[0], nargout).map_err(wrap)
}

fn b_svd(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "svd")?;
    fm_linalg::svd(&args[0], nargout).map_err(wrap)
}

/// `eigs(A, k, sigma)` — a subset of the (sparse) matrix `A`'s eigenvalues.
/// `k` defaults to 6; the third argument is either a numeric shift `sigma`
/// (eigenvalues nearest it) or a mode string (`'lm'`/`'sm'`/`'lr'`/`'sr'`/
/// `'li'`/`'si'`), defaulting to largest magnitude (`'lm'`).
fn b_eigs(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    use fm_linalg::EigsWhich;
    need(args, 1, "eigs")?;
    let k = args
        .get(1)
        .and_then(Array::as_f64)
        .map_or(6, |v| v as usize);
    let which = match args.get(2) {
        None => EigsWhich::LargeMag,
        Some(s) if s.class() == DataClass::Char => {
            match s.as_string().unwrap_or_default().to_lowercase().as_str() {
                "sm" => EigsWhich::SmallMag,
                "lr" | "la" => EigsWhich::LargeReal,
                "sr" | "sa" => EigsWhich::SmallReal,
                "li" => EigsWhich::LargeImag,
                "si" => EigsWhich::SmallImag,
                _ => EigsWhich::LargeMag,
            }
        }
        Some(s) => EigsWhich::Nearest(s.as_f64().unwrap_or(0.0)),
    };
    fm_linalg::eigs(&args[0], k, which, nargout.max(1)).map_err(wrap)
}

fn b_lu(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "lu")?;
    let a = &args[0];
    // FreeMat's sparse LU (SparseLUDecompose) only handles square double /
    // dcomplex matrices; everything else errors out (the dense path, by
    // contrast, supports rectangular and all numeric classes).
    if a.is_sparse() {
        if a.class() != DataClass::Double {
            return Err(Signal::Error(InterpError::msg(
                "lu: sparse LU is only supported for double and dcomplex matrices",
            )));
        }
        let dims = a.dims();
        if dims.len() != 2 || dims[0] != dims[1] {
            return Err(Signal::Error(InterpError::msg(
                "lu: sparse LU is only supported for square matrices",
            )));
        }
    }
    fm_linalg::lu(a, nargout).map_err(wrap)
}

fn b_qr(_i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    need(args, 1, "qr")?;
    // `qr(a, 0)` requests the economy (compact) decomposition.
    let economy = args.get(1).and_then(Array::as_f64) == Some(0.0);
    fm_linalg::qr(&args[0], nargout, economy).map_err(wrap)
}

fn b_chol(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "chol")?;
    Ok(vec![fm_linalg::chol(&args[0]).map_err(wrap)?])
}

/// `norm(A)` / `norm(A, p)` where `p` is 1, 2, `inf`, or `'fro'`.
fn b_norm(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "norm")?;
    let kind = match args.get(1) {
        None => NormKind::Two,
        Some(p) => {
            if let Some(s) = p.as_string() {
                match s.to_lowercase().as_str() {
                    "fro" => NormKind::Fro,
                    "inf" => NormKind::Inf,
                    _ => return Err(Signal::Error(InterpError::msg("norm: unknown norm type"))),
                }
            } else {
                let v = p.as_f64().unwrap_or(2.0);
                if v == f64::INFINITY {
                    NormKind::Inf
                } else if v == f64::NEG_INFINITY {
                    NormKind::NegInf
                } else if v == 1.0 {
                    NormKind::One
                } else if v == 2.0 {
                    NormKind::Two
                } else {
                    // General p-norm (vectors only; matrices error in fm_linalg).
                    NormKind::P(v)
                }
            }
        }
    };
    Ok(vec![fm_linalg::norm(&args[0], kind).map_err(wrap)?])
}

/// `xnrm2(A)` — Euclidean (2-)norm of the flattened array (FreeMat's BLAS
/// `?nrm2` wrapper, used by the `wbtest_near` whitebox helper). Equivalent to
/// `norm(A(:))` / the Frobenius norm; real for real input.
fn b_xnrm2(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "xnrm2")?;
    Ok(vec![
        fm_linalg::norm(&args[0], NormKind::Fro).map_err(wrap)?,
    ])
}

fn b_rank(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "rank")?;
    Ok(vec![fm_linalg::rank(&args[0]).map_err(wrap)?])
}

fn b_pinv(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "pinv")?;
    Ok(vec![fm_linalg::pinv(&args[0]).map_err(wrap)?])
}

/// `trace(A)` — sum of the main diagonal.
fn b_trace(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    need(args, 1, "trace")?;
    let dims = args[0].dims();
    if dims.len() != 2 {
        return Err(Signal::Error(InterpError::msg("trace: input must be 2-D")));
    }
    let (r, c) = (dims[0], dims[1]);
    let data = to_f64_vec(&args[0]);
    let k = r.min(c);
    let t: f64 = (0..k).map(|i| data[i + i * r]).sum();
    Ok(vec![build_real(DataClass::Double, &[1, 1], vec![t])])
}
