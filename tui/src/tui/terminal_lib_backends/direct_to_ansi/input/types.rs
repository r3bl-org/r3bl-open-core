// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared type definitions for the `DirectToAnsi` input device.
//!
//! This module contains types used across multiple input submodules:
//! - [`LoopContinuationSignal`]: Event loop control flow
//! - [`StdinReadResult`]: Channel message from stdin reader thread

use crate::InputEvent;

/// Signal from a processing stage to the event loop indicating how to proceed.
///
/// Used by both the paste state machine and I/O wait operations to communicate
/// back to the main event loop. Each stage returns this signal to indicate:
/// - An event is ready to emit
/// - Processing should continue (more data needed or event absorbed)
/// - The loop should terminate (EOF/error)
#[allow(missing_debug_implementations)]
pub enum LoopContinuationSignal {
    /// Emit this event to the caller and return from the loop.
    Emit(InputEvent),
    /// Continue the loop (data received, event absorbed, or signal handled).
    Continue,
    /// EOF or error occurred - terminate the loop.
    Shutdown,
}

/// Result of a stdin read operation, sent through the channel from the stdin
/// reader thread.
#[derive(Debug)]
pub enum StdinReadResult {
    /// Successfully read bytes from stdin.
    Data(Vec<u8>),
    /// EOF reached (0 bytes read).
    Eof,
    /// Error occurred during read.
    Error(std::io::ErrorKind),
}
