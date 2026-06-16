//! Span-carrying parse diagnostics.
//!
//! Errors are `miette` [`Diagnostic`]s built on `thiserror`. Each error owns the
//! full source (`#[source_code]`) plus one labelled span (`#[label]`) so the
//! `fancy` renderer prints an annotated, source-highlighted message. The CLI (or
//! a test) enables miette's `fancy` feature; the library only defines the types.

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::span::Span;

/// A parse error: a message anchored to a labelled span within the source.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
pub struct ParseError {
    /// The human-readable error message.
    pub message: String,
    /// The full source text the offsets index into.
    #[source_code]
    pub src: String,
    /// The span the label points at.
    #[label("{label}")]
    pub at: SourceSpan,
    /// The text rendered next to the labelled span.
    pub label: String,
    /// Optional remediation hint shown below the snippet.
    #[help]
    pub help: Option<String>,
}

impl ParseError {
    /// Build a parse error pointing at `span` within `src`.
    pub(crate) fn new(
        src: impl Into<String>,
        span: Span,
        message: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            src: src.into(),
            at: span.into(),
            label: label.into(),
            help: None,
        }
    }

    /// Attach a help hint, rendered below the snippet.
    #[must_use]
    pub(crate) fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// The byte span this error points at (for tests / callers).
    #[must_use]
    pub fn span(&self) -> Span {
        Span::new(self.at.offset(), self.at.offset() + self.at.len())
    }

    /// A by-value copy. `SourceSpan` is not `Clone`-friendly to derive through
    /// `thiserror`, so the parser uses this to stash the deepest error.
    pub(crate) fn clone_shallow(&self) -> Self {
        Self {
            message: self.message.clone(),
            src: self.src.clone(),
            at: SourceSpan::new(self.at.offset().into(), self.at.len()),
            label: self.label.clone(),
            help: self.help.clone(),
        }
    }
}

/// Result alias for parser operations.
pub type Result<T> = std::result::Result<T, ParseError>;
