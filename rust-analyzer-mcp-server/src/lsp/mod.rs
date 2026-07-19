// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Language Server Protocol (LSP) client subsystem for `rust-analyzer`.
//!
//! This module encapsulates the `rust-analyzer` child process lifecycle, asynchronous
//! stdio framing reader threads, synchronous JSON-RPC request-response routing, AST
//! inspection queries, compiler diagnostics aggregation, and server readiness tracking.

mod client;
mod diagnostics;
mod protocol;
mod queries;
mod readiness_monitor;
mod subprocess;
mod transport;

pub use client::*;
pub use diagnostics::*;
pub use protocol::*;
pub use readiness_monitor::*;
