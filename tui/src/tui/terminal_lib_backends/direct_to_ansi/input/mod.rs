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
//! - **Buffering**: Ring buffer for handling partial/incomplete ANSI sequences
//! - **Timeout Management**: 150ms timeout for sequence completion
//! - **Parser Dispatch**: Route buffer content to appropriate protocol parsers
//! - **Event Generation**: Convert parsed results to InputEvent

mod input_device_impl;

// Re-exports - flatten the public API
pub use input_device_impl::DirectToAnsiInputDevice;

// Tests
#[cfg(test)]
mod tests;
