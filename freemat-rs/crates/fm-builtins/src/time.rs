//! Time / date builtins: `tic`, `toc`, `clock`, `cputime`, `etime`, `pause`,
//! `now` — MATLAB/FreeMat semantics.
//!
//! `tic`/`toc` implement a stopwatch. The default (no-handle) start time lives
//! in a thread-local (mirroring the `fm-io` open-file table precedent: the
//! interpreter carries no per-instance timer state, and the single-threaded
//! REPL / per-test conformance isolation make a thread-local the cleanest fit).
//! The handle form `id = tic; toc(id)` encodes the start instant as nanoseconds
//! since a fixed process anchor so it round-trips through a `double`.

use std::cell::Cell;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Local, TimeZone, Timelike};
use fm_core::{Array, DataClass};
use fm_interp::error::Flow;
use fm_interp::value::{build_real, to_f64_vec};
use fm_interp::{FunctionTable, Interpreter};

use crate::util::err;

thread_local! {
    /// A fixed per-thread anchor instant. `tic` handles are encoded as
    /// nanoseconds elapsed since this anchor so they survive a round-trip
    /// through an `f64` and can be decoded back into an elapsed measurement.
    static ANCHOR: Instant = Instant::now();
    /// The most recent default (`tic` with no output) start time, as nanos
    /// since `ANCHOR`. `None` until the first `tic`.
    static LAST_TIC: Cell<Option<u128>> = const { Cell::new(None) };
}

/// Nanoseconds elapsed from the thread anchor to now.
fn now_nanos() -> u128 {
    ANCHOR.with(|a| a.elapsed().as_nanos())
}

pub(crate) fn register(table: &mut FunctionTable) {
    table.add_builtin("tic", b_tic);
    table.add_builtin("toc", b_toc);
    table.add_builtin("clock", b_clock);
    table.add_builtin("cputime", b_cputime);
    table.add_builtin("etime", b_etime);
    table.add_builtin("pause", b_pause);
    table.add_builtin("now", b_now);
}

/// `tic` — start the stopwatch. Always records the default (no-handle) start
/// time so a later bare `toc` works, and also returns an opaque timer handle
/// (nanos since the thread anchor) so the handle form `id = tic; toc(id)` works.
///
/// (The interpreter evaluates a bare `tic` statement with `nargout == 1`, so a
/// builtin can't distinguish `tic` from `id = tic`; recording the default *and*
/// returning the handle is correct for both — a bare `tic;` simply also sets
/// `ans`, a harmless cosmetic deviation from MATLAB suppressing it.)
fn b_tic(_i: &mut Interpreter, _args: &[Array], _nargout: usize) -> Flow<Vec<Array>> {
    let t = now_nanos();
    LAST_TIC.with(|c| c.set(Some(t)));
    Ok(vec![Array::double(t as f64)])
}

/// `toc` / `toc(id)` — elapsed seconds since the matching `tic`. With no output
/// it prints `Elapsed time is N seconds.`; otherwise it returns the value.
fn b_toc(i: &mut Interpreter, args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    let start = if let Some(id) = args.first() {
        // Handle form: the argument is a `tic` start time (nanos since anchor).
        let Some(v) = id.as_f64() else {
            return err("toc: handle argument must be a scalar from `tic`");
        };
        v as u128
    } else {
        match LAST_TIC.with(Cell::get) {
            Some(t) => t,
            None => return err("toc: you must call `tic` before `toc`"),
        }
    };
    let now = now_nanos();
    let elapsed = now.saturating_sub(start) as f64 / 1e9;
    if nargout == 0 {
        i.emit(&format!("Elapsed time is {elapsed} seconds.\n"));
        Ok(vec![])
    } else {
        Ok(vec![Array::double(elapsed)])
    }
}

/// `clock` — `[year month day hour minute seconds]` in local time. The seconds
/// field carries the fractional part (MATLAB semantics).
fn b_clock(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let now = Local::now();
    let secs = f64::from(now.second()) + f64::from(now.nanosecond()) / 1e9;
    let v = vec![
        f64::from(now.year()),
        f64::from(now.month()),
        f64::from(now.day()),
        f64::from(now.hour()),
        f64::from(now.minute()),
        secs,
    ];
    Ok(vec![build_real(DataClass::Double, &[1, 6], v)])
}

/// `cputime` — process CPU seconds (approximated by wall-clock seconds since the
/// Unix epoch; portable without platform CPU-time syscalls). Differences between
/// two `cputime` reads give elapsed CPU-ish time, which is what callers use it
/// for.
fn b_cputime(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    Ok(vec![Array::double(secs)])
}

/// `etime(t2, t1)` — seconds from clock vector `t1` to `t2` (each a 6-element
/// `[year month day hour minute seconds]` vector, e.g. from `clock`).
fn b_etime(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if args.len() < 2 {
        return err("etime: expected two clock vectors");
    }
    let t2 = clock_to_secs(&args[0])?;
    let t1 = clock_to_secs(&args[1])?;
    Ok(vec![Array::double(t2 - t1)])
}

/// Convert a 6-element clock vector to absolute seconds (local-time epoch).
fn clock_to_secs(a: &Array) -> Flow<f64> {
    let v = to_f64_vec(a);
    if v.len() != 6 {
        return err("etime: clock vectors must have 6 elements");
    }
    let year = v[0] as i32;
    let month = v[1] as u32;
    let day = v[2] as u32;
    let hour = v[3] as u32;
    let minute = v[4] as u32;
    let whole_sec = v[5].trunc() as u32;
    let frac = v[5].fract();
    match Local.with_ymd_and_hms(year, month, day, hour, minute, whole_sec) {
        chrono::LocalResult::Single(dt) => Ok(dt.timestamp() as f64 + frac),
        _ => err("etime: invalid clock vector"),
    }
}

/// `pause(n)` — sleep `n` seconds. `pause` with no argument is a no-op in this
/// headless build (there is no keypress to wait for; we must not block forever).
fn b_pause(_i: &mut Interpreter, args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    if let Some(secs) = args.first().and_then(Array::as_f64)
        && secs.is_finite()
        && secs > 0.0
    {
        std::thread::sleep(Duration::from_secs_f64(secs));
    }
    Ok(vec![])
}

/// `now` — MATLAB serial date number for the current local time. Day 1 is
/// `0000-01-01`; the Unix epoch (`1970-01-01`) is serial day `719529`.
fn b_now(_i: &mut Interpreter, _args: &[Array], _n: usize) -> Flow<Vec<Array>> {
    Ok(vec![Array::double(datenum_now())])
}

/// Serial date number (MATLAB convention) for the current local time.
fn datenum_now() -> f64 {
    const UNIX_EPOCH_DATENUM: f64 = 719_529.0;
    let now = Local::now();
    // Seconds (with sub-second precision) since the local Unix epoch.
    let secs = now.timestamp() as f64 + f64::from(now.timestamp_subsec_nanos()) / 1e9;
    // Add the local UTC offset so the serial number reflects local wall-clock.
    let offset = f64::from(now.offset().local_minus_utc());
    UNIX_EPOCH_DATENUM + (secs + offset) / 86_400.0
}
