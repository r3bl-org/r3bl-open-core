// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character insertion, deletion, and erasure operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation] for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_char_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_char_ops`] - Full pipeline testing via public API
//!
//! # Testing Strategy
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in [`vt_100_ansi_conformance_tests`] validating the full
//!   pipeline
//!
//! See the [operations module documentation] for the complete testing philosophy
//! and rationale behind this approach.
//!
//! # Architecture Overview
//!
//! ```text
//! ╭─────────────────╮    ╭───────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │────▶ PTY Master    │────▶ VTE Parser      │────▶ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream) │    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰───────────────╯    ╰─────────────────╯    │  buffer)     │
//!        │                                            │              ╰──────────────╯
//!        │                                            │                      │
//!        │                                   ╔════════▼════════╗             │
//!        │                                   ║ Perform Trait   ║             │
//!        │                                   ║ Implementation  ║             │
//!        │                                   ╚═════════════════╝             │
//!        │                                                                   │
//!        │                                   ╭─────────────────╮             │
//!        │                                   │ RenderPipeline  ◀─────────────╯
//!        │                                   │ paint()         │
//!        ╰───────────────────────────────────▶ Terminal Output │
//!                                            ╰─────────────────╯
//! ```
//!
//! # CSI Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC[2P" (delete 2 chars)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)      ╭───────────╮
//!       - char_ops:: for chars (@,P,X) <- │THIS MODULE│
//!         ↓                               ╰───────────╯
//!     Update OffscreenBuffer state
//! ```
//!
//! [`impl_char_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_char_ops
//! [`test_char_ops`]: super::super::vt_100_ansi_conformance_tests::tests::test_char_ops
//! [module-level documentation]: super::super
//! [operations module documentation]: super
//! [`vt_100_ansi_conformance_tests`]: super::super::vt_100_ansi_conformance_tests

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MovementCount};

/// Handle ICH (Insert Character) - insert n blank characters at cursor position.
/// Characters to the right of cursor shift right, characters beyond margin are lost.
/// See [`OffscreenBuffer::insert_chars_at_cursor`] for the implementation of this shim.
///
/// [`OffscreenBuffer::insert_chars_at_cursor`]: crate::OffscreenBuffer::insert_chars_at_cursor
pub fn insert_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = MovementCount::parse_as_length_non_zero(params);
    let result = performer.ofs_buf.insert_chars_at_cursor(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to insert {:?} chars at cursor position {:?}",
        how_many,
        performer.ofs_buf.cursor_pos
    );
}

/// Handle DCH (Delete Character) - delete n characters at cursor position.
/// Characters to the right of cursor shift left, blanks are inserted at line end.
/// See [`OffscreenBuffer::delete_chars_at_cursor`] for the implementation of this shim.
///
/// [`OffscreenBuffer::delete_chars_at_cursor`]: crate::OffscreenBuffer::delete_chars_at_cursor
pub fn delete_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = MovementCount::parse_as_length_non_zero(params);
    let result = performer.ofs_buf.delete_chars_at_cursor(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to delete {:?} chars at cursor position {:?}",
        how_many,
        performer.ofs_buf.cursor_pos
    );
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
/// Characters are replaced with blanks, no shifting occurs (unlike DCH).
/// See [`OffscreenBuffer::erase_chars_at_cursor`] for the implementation of this shim.
///
/// [`OffscreenBuffer::erase_chars_at_cursor`]: crate::OffscreenBuffer::erase_chars_at_cursor
pub fn erase_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = MovementCount::parse_as_length_non_zero(params);
    let result = performer.ofs_buf.erase_chars_at_cursor(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to erase {:?} chars at cursor position {:?}",
        how_many,
        performer.ofs_buf.cursor_pos
    );
}

/// Handle printable character printing - display character at cursor position.
/// Character set translation applied if DEC graphics mode is active.
/// Cursor advances with automatic line wrapping based on DECAWM mode.
/// See [`OffscreenBuffer::print_char`] for the implementation of this shim.
///
/// [`OffscreenBuffer::print_char`]: crate::OffscreenBuffer::print_char
pub fn print_char(performer: &mut AnsiToOfsBufPerformer, ch: char) {
    let result = performer.ofs_buf.print_char(ch);
    debug_assert!(
        result.is_ok(),
        "Failed to print char {:?} at cursor position {:?}",
        ch,
        performer.ofs_buf.cursor_pos
    );
}
