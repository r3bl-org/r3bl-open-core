// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared type definitions for the `DirectToAnsi` input device.
//!
//! This module contains types used across multiple input submodules:
//! - [`LoopContinuationSignal`]: Event loop control flow
//! - [`ReaderThreadMessage`]: Channel message from stdin reader thread

use crate::InputEvent;

/// Signal from a processing stage to the event loop indicating how to proceed.
///
/// Used by the paste state machine to communicate back to the reader loop.
/// Each stage returns this signal to indicate:
/// - An event is ready to emit
/// - Processing should continue (more data needed or event absorbed)
#[allow(missing_debug_implementations)]
pub enum LoopContinuationSignal {
    /// Emit this event to the caller and return from the loop.
    Emit(InputEvent),
    /// Continue the loop (data received, event absorbed, or signal handled).
    Continue,
}

/// Message from the stdin reader thread, sent through a broadcast channel.
///
/// Requires [`Clone`] because [`tokio::sync::broadcast`] clones messages for each
/// receiver. See [`process_global_stdin`] for architecture details.
///
/// [`process_global_stdin`]: mod@super::process_global_stdin
#[derive(Debug, Clone)]
pub enum ReaderThreadMessage {
    /// Parsed input event ready for consumption.
    Event(InputEvent),
    /// EOF reached (0 bytes read).
    Eof,
    /// Error occurred during read (error details are logged in reader thread).
    Error,
    /// Terminal resize signal (`SIGWINCH`) received.
    ///
    /// The reader thread detected a window size change. The async side should
    /// query the new terminal size via [`get_size()`] and emit an
    /// [`InputEvent::Resize`].
    ///
    /// [`get_size()`]: crate::core::term::get_size
    /// [`InputEvent::Resize`]: crate::InputEvent::Resize
    Resize,
}
