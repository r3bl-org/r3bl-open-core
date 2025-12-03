// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type definitions for the `DirectToAnsi` input device.
//!
//! This module contains the main [`DirectToAnsiInputDevice`] struct and supporting
//! enums for paste handling and event loop control flow.

use crate::InputEvent;

/// State machine for collecting bracketed paste text.
///
/// When the terminal sends a bracketed paste sequence, it arrives as:
/// - `Paste(Start)` marker
/// - Multiple `Keyboard` events (the actual pasted text)
/// - `Paste(End)` marker
///
/// This state tracks whether we're currently collecting text between markers.
///
/// # Line Ending Handling
///
/// Both CR (`\r`) and LF (`\n`) are parsed by the keyboard parser as
/// [`VT100KeyCodeIR::Enter`], which is then accumulated as `'\n'`. This means:
/// - LF (`\n`) → `'\n'` ✓
/// - CR (`\r`) → `'\n'` ✓
/// - CRLF (`\r\n`) → `'\n\n'` (double newline)
///
/// Most Unix terminals normalize line endings before sending bracketed paste,
/// so CRLF sequences are uncommon in practice.
///
/// # TODO(windows)
///
/// Windows uses CRLF line endings natively. When adding Windows support for
/// [`DirectToAnsi`], consider normalizing CRLF → LF in the paste accumulator.
/// This would require either tracking the previous byte in the keyboard parser
/// or post-processing the accumulated text.
///
/// [`DirectToAnsi`]: mod@super::super
/// [`VT100KeyCodeIR::Enter`]: crate::core::ansi::vt_100_terminal_input_parser::VT100KeyCodeIR::Enter
#[derive(Debug)]
pub enum PasteCollectionState {
    /// Not currently in a paste operation.
    Inactive,
    /// Currently collecting text for a paste operation.
    Accumulating(String),
}

/// Result of applying the paste state machine to a parsed event.
#[allow(missing_debug_implementations)]
pub enum PasteAction {
    /// Emit this event to the caller.
    Emit(InputEvent),
    /// Continue collecting (event was absorbed by paste state machine).
    Continue,
}

/// Result of waiting for stdin or signal in the event loop.
#[cfg(unix)]
#[allow(missing_debug_implementations)]
pub enum WaitAction {
    /// Emit this event to the caller (e.g., Resize).
    Emit(InputEvent),
    /// EOF or error occurred, signal shutdown.
    Shutdown,
    /// Data was read or signal handled, continue parsing.
    Continue,
}
