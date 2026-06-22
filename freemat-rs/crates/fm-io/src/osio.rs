//! Operating-system / session builtins: `system`, `diary`, `license`.
//!
//! `diary` state (on/off plus a target filename) lives in a thread-local,
//! mirroring the open-file table precedent in [`crate::fileio`]: the interpreter
//! carries no per-instance session state, and the single-threaded REPL / per-test
//! isolation make a thread-local the cleanest fit. The diary is a faithful
//! state-tracking no-op — it does not actually tee console output, but the
//! state round-trips so `state = diary` reports correctly.

use std::cell::RefCell;
use std::process::Command;

use fm_core::Array;
use fm_interp::error::Flow;

use crate::builtins::err;

thread_local! {
    /// `(enabled, filename)` for the current thread's diary.
    static DIARY: RefCell<(bool, String)> = RefCell::new((false, "diary".to_string()));
}

/// `system(cmd)` — run a shell command and capture its output.
///
/// One output (`y = system(cmd)`): `y` is a cell array with one entry per line
/// of captured stdout (FreeMat's convention). Two outputs
/// (`[status, output] = system(cmd)`): `status` is the integer exit code and
/// `output` is the captured stdout as a single char string.
pub fn system(args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    let Some(cmd) = args.first().and_then(Array::as_string) else {
        return err("system: expected a command string");
    };

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(&cmd).output()
    } else {
        Command::new("sh").arg("-c").arg(&cmd).output()
    };

    let (status, stdout) = match output {
        Ok(out) => {
            let code = out.status.code().unwrap_or(-1);
            (code, String::from_utf8_lossy(&out.stdout).into_owned())
        }
        Err(e) => (-1, format!("system: failed to run command: {e}")),
    };

    if nargout >= 2 {
        // [status, output] form: numeric status + raw stdout string.
        return Ok(vec![
            Array::double(f64::from(status)),
            Array::char_string(&stdout),
        ]);
    }

    // Single-output (or statement) form: a cell array of stdout lines.
    let lines: Vec<Array> = stdout.lines().map(Array::char_string).collect::<Vec<_>>();
    let n = lines.len();
    Ok(vec![Array::cell(&[n, 1], lines)])
}

/// `diary` / `diary on|off` / `diary filename` / `diary(filename)` — control the
/// session log. With no input and one output, returns the current logical state.
pub fn diary(args: &[Array], nargout: usize) -> Flow<Vec<Array>> {
    if args.is_empty() {
        // Query/toggle. If a value is requested, report state without toggling;
        // otherwise toggle the current state.
        if nargout >= 1 {
            let on = DIARY.with(|d| d.borrow().0);
            return Ok(vec![Array::bool(on)]);
        }
        DIARY.with(|d| {
            let mut d = d.borrow_mut();
            d.0 = !d.0;
        });
        return Ok(vec![]);
    }

    let Some(arg) = args[0].as_string() else {
        return err("diary: argument must be a string");
    };
    match arg.as_str() {
        "on" => DIARY.with(|d| d.borrow_mut().0 = true),
        "off" => DIARY.with(|d| d.borrow_mut().0 = false),
        name => DIARY.with(|d| {
            let mut d = d.borrow_mut();
            d.1 = name.to_string();
            d.0 = true;
        }),
    }
    Ok(vec![])
}

/// `license` — return the FreeMat license identifier. FreeMat is distributed
/// under the GNU General Public License (version 2); we return that as a fixed
/// deterministic string.
pub fn license(_args: &[Array]) -> Flow<Vec<Array>> {
    Ok(vec![Array::char_string("GNU General Public License")])
}
