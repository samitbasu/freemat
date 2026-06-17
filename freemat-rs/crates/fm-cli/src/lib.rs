//! `fm-cli` library surface: the embedded graphics webserver.
//!
//! The binary (`src/main.rs`) drives the REPL; the server lives here so the
//! Milestone-2 integration test (`tests/milestone2.rs`) can start it, connect a
//! websocket client, and assert the scene JSON it receives.

pub mod server;

pub use server::{ServerHandle, start};
