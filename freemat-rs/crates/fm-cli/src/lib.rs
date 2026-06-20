//! `fm-cli` library surface: the embedded graphics webserver and the help-system
//! fragment capture engine.
//!
//! The binary (`src/main.rs`) drives the REPL; the server lives here so the
//! Milestone-2 integration test (`tests/milestone2.rs`) can start it, connect a
//! websocket client, and assert the scene JSON it receives. The [`capture`]
//! module (help-system phase P2) runs scripted REPL sessions headlessly and
//! captures their transcripts byte-for-byte for the docs pipeline.

pub mod capture;
pub mod server;

pub use capture::{CapturedFragment, FragmentScript, run_fragment};
pub use server::{ServerHandle, start};
