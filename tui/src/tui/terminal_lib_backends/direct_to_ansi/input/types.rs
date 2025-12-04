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

/// Message from the stdin reader thread, sent through the channel.
///
/// The dedicated reader thread uses [`mio::Poll`] to wait on both stdin and
/// `SIGWINCH` signals simultaneously. It parses stdin bytes using the crossterm
/// pattern and sends parsed events through the channel.
///
/// # Crossterm Pattern
///
/// The reader thread does parsing (not just reading):
/// 1. Read bytes from stdin into `TTY_BUFFER_SIZE` buffer
/// 2. Compute `more = read_count == TTY_BUFFER_SIZE`
/// 3. Call `parser.advance(buffer, more)` to parse with ESC disambiguation
/// 4. Send each parsed `InputEvent` through the channel
///
/// This matches crossterm's `mio.rs` architecture where parsing happens in the
/// reader thread, ensuring the `more` flag is computed correctly from the actual
/// read count.
///
/// [`mio::Poll`]: mio::Poll
#[derive(Debug)]
pub enum ReaderThreadMessage {
    /// Successfully parsed an input event from stdin.
    ///
    /// The reader thread has already parsed raw bytes into an [`InputEvent`]
    /// using the crossterm pattern with proper ESC disambiguation.
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
