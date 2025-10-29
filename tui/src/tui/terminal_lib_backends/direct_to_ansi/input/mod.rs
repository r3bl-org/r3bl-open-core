// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input handling module for DirectToAnsi backend.
//!
//! This module contains the async input device implementation that reads from
//! terminal stdin and parses ANSI sequences into input events.
//!
//! ## Architecture
//!
//! The input module is the **Stage 5 Backend Executor for input**, parallel to
//! the output module structure:
//!
//! ```text
//! Terminal stdin
//!    ↓
//! [DirectToAnsiInputDevice] (async I/O, buffering)
//!    ↓
//! [vt_100_terminal_input_parser] (protocol parsing)
//!    ↓
//! InputEvent (keyboard, mouse, resize, focus, paste)
//! ```
//!
//! ## Module Responsibilities
//!
//! - **Async I/O**: Non-blocking reading from tokio::io::stdin()
//! - **Buffering**: Simple `Vec<u8>` buffer for handling partial/incomplete ANSI sequences
//! - **Smart Lookahead**: Zero-latency ESC key detection (no timeout needed!)
//! - **Parser Dispatch**: Route buffer content to appropriate protocol parsers
//! - **Event Generation**: Convert parsed results to InputEvent
//!
//! ## Key Architectural Decision: No Timeout Needed
//!
//! Unlike naive implementations that wait 150ms to distinguish ESC from ESC sequences,
//! we use tokio's async I/O to yield until data is ready. This means:
//! - ESC key pressed alone → emitted immediately (0ms latency)
//! - ESC sequence arriving → parsed as complete sequence
//! - No artificial delays or timeouts required!

mod input_device_impl;

// Re-exports - flatten the public API
pub use input_device_impl::*;

// Tests
#[cfg(test)]
mod tests;
