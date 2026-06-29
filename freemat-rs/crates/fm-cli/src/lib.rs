//! `fm-cli` library surface: the embedded graphics webserver and the help-system
//! fragment capture engine.
//!
//! The binary (`src/main.rs`) drives the REPL; the server lives here so the
//! Milestone-2 integration test (`tests/milestone2.rs`) can start it, connect a
//! websocket client, and assert the scene JSON it receives. The [`capture`]
//! module (help-system phase P2) runs scripted REPL sessions headlessly and
//! captures their transcripts byte-for-byte for the docs pipeline.

pub mod capture;
pub mod engine;
mod help;
pub mod server;

// The canonical model lives in fm-doc (P3 unification); re-exported here so the
// fm-cli surface (`fm_cli::FragmentScript`, `fm_cli::CapturedFragment`) and the
// `--capture-fragment` CLI path are unchanged for downstream callers.
pub use capture::run_fragment;
pub use engine::{Engine, EvalOutcome, EvalReply, StopInfo};
pub use fm_doc::{CapturedFragment, FragmentScript};
pub use server::{ServerHandle, start};
