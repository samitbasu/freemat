//! Runtime errors and control-flow signals.
//!
//! Runtime failures are [`InterpError`] — `miette` diagnostics that optionally
//! carry the source text and a span (MException-like; FreeMat raises an
//! `Exception` with a message and, when available, a location). Non-error
//! control flow (`break`/`continue`/`return`, and a caught error) is an
//! idiomatic [`Signal`] enum threaded through `Result`, never a panic.

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use fm_parser::Span;

/// A runtime error raised while evaluating a program (an MException analogue).
///
/// The optional `src`/`span` make it render as an annotated, source-highlighted
/// diagnostic when the consumer enables miette's `fancy` feature.
#[derive(Debug, Clone, Error, Diagnostic)]
#[error("{message}")]
pub struct InterpError {
    /// The (possibly MATLAB-style `identifier:component`) message.
    pub message: String,
    /// MATLAB error identifier (`error('id:x', ...)`), if any. Surfaced via an
    /// MException once Stage 6 wires `try`/`catch` to bind the exception object.
    pub identifier: Option<String>,
    /// The source unit, attached so the diagnostic can render the line.
    #[source_code]
    pub src: Option<String>,
    /// The span the error points at, within `src`.
    #[label("here")]
    pub span: Option<SourceSpan>,
}

impl InterpError {
    /// A bare error message with no location.
    #[must_use]
    pub fn msg(message: impl Into<String>) -> Self {
        InterpError {
            message: message.into(),
            identifier: None,
            src: None,
            span: None,
        }
    }

    /// An error carrying a MATLAB identifier (`error('comp:id', msg)`).
    #[must_use]
    pub fn with_id(identifier: impl Into<String>, message: impl Into<String>) -> Self {
        InterpError {
            message: message.into(),
            identifier: Some(identifier.into()),
            src: None,
            span: None,
        }
    }

    /// Attach a source unit and span (if not already located).
    #[must_use]
    pub fn located(mut self, src: &str, span: Span) -> Self {
        if self.span.is_none() {
            self.src = Some(src.to_string());
            self.span = Some(span.into());
        }
        self
    }
}

/// The outcome of executing a statement or block: normal flow, or a
/// control-flow transfer that unwinds toward the construct that handles it.
///
/// This is the idiomatic Rust replacement for FreeMat's interpreter state flags
/// (`InterpreterContinueException`, `InterpreterBreakException`, ...). It is the
/// `Err` arm of [`Flow`]; a real runtime error is [`Signal::Error`].
#[derive(Debug, Clone)]
pub enum Signal {
    /// `break` — exit the innermost loop.
    Break,
    /// `continue` — next iteration of the innermost loop.
    Continue,
    /// `return` — leave the current function/script.
    Return,
    /// A runtime error propagating toward a `try`/`catch` or the REPL.
    Error(InterpError),
}

impl Signal {
    /// Whether this signal is a (catchable) runtime error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Signal::Error(_))
    }
}

impl From<InterpError> for Signal {
    fn from(e: InterpError) -> Self {
        Signal::Error(e)
    }
}

/// Result of evaluating/executing — `Ok` for normal flow, `Err(Signal)` for a
/// control-flow transfer (including runtime errors).
pub type Flow<T> = Result<T, Signal>;

/// Convenience: build an `Err(Signal::Error(..))` from a message.
pub fn err<T>(message: impl Into<String>) -> Flow<T> {
    Err(Signal::Error(InterpError::msg(message)))
}
