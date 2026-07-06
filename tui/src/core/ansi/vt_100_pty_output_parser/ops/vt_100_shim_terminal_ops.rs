// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal state operations for [`ESC`] sequences.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_terminal_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_terminal_ops`] - Full pipeline testing via public API
//!
//! # Testing Strategy
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in the conformance tests validating the full pipeline
//!
//! For the complete testing philosophy and rationale behind this approach,
//! see the [ops module].
//!
//! # Architecture Overview
//!
//! See the [module-level Architecture Overview].
//!
//! # [`ESC`] Sequence Architecture
//!
//! ```text
//! Application sends "ESC c" (reset terminal)
//!         ↓
//!     PTY Controlled (escape sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses `ESC` char pattern)
//!         ↓
//!     esc_dispatch() [calls functions in this module]
//!         ↓
//!     terminal_ops functions:
//!       - reset_terminal() for `ESC c` (`RIS`)
//!       - select_ascii_character_set() for `ESC ( B`
//!       - select_dec_graphics_character_set() for `ESC ( 0`
//!         ↓
//!     Update OfsBuf state
//! ```
//!
//! Note: Cursor save/restore [`ESC`] sequences (`ESC 7`/`ESC 8`) are handled by
//! [`cursor_ops`] functions to maintain consistency with `CSI` equivalents (`CSI s`/`CSI
//! u`).
//!
//! [`cursor_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_cursor_ops
//! [`ESC`]: crate::EscSequence
//! [`test_terminal_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_terminal_ops
//! [`vt_100_impl_terminal_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_terminal_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::{Pos, TuiStyle};

/// Clears all buffer content.
fn clear_buffer(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.clear();
}

/// Reset all [`SGR`] attributes to default state.
///
/// [`SGR`]: crate::SgrCode
fn reset_sgr_attributes(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.parser_global_state.current_style = TuiStyle::default();
}

/// Reset terminal to initial state (`ESC c`).
/// Clears the buffer, resets cursor, and clears saved state.
/// Clears [`DECSTBM`] scroll region margins.
///
/// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
pub fn reset_terminal(performer: &mut AnsiToOfsBufPerformer) {
    clear_buffer(performer);
    let _unused = performer.ofs_buf_vt_100.erase_display_scrollback();

    // Reset cursor to home position.
    performer.ofs_buf_vt_100.set_cursor_pos(Pos::default());

    // Clear saved cursor state.
    performer
        .ofs_buf_vt_100
        .parser_global_state
        .cursor_pos_for_esc_save_and_restore = None;

    // Reset to ASCII character set.
    select_ascii_character_set(performer);

    // Clear DECSTBM scroll region margins.
    performer
        .ofs_buf_vt_100
        .parser_global_state
        .scroll_region_top = None;
    performer
        .ofs_buf_vt_100
        .parser_global_state
        .scroll_region_bottom = None;

    // Clear any `SGR` attributes.
    reset_sgr_attributes(performer);
}

/// Select [`ASCII`] character set (`ESC ( B`).
/// Switches to normal [`ASCII`] character set for standard text rendering.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
pub fn select_ascii_character_set(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.select_ascii_character_set();
}

/// Select [`DEC`] Special Graphics character set (`ESC ( 0`).
/// Switches to [`DEC`] Special Graphics character set for box-drawing characters.
///
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
pub fn select_dec_graphics_character_set(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf_vt_100.select_dec_graphics_character_set();
}
