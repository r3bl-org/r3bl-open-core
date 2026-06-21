// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Character insertion, deletion, and erasure operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_char_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_char_ops`] - Full pipeline testing via public API
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
//! # [`CSI`] Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC [2P" (delete 2 chars)
//!         ↓
//!     PTY Controlled (escape sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses `ESC [`...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to ops module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)                           ╭───────────╮
//!       - char_ops:: for chars (@,P,X)                      <- │THIS MODULE│
//!         ↓                                                    ╰───────────╯
//!     Update OffscreenBuffer state
//! ```
//!
//! # [`VT-100`] Protocol Conventions
//!
//! This shim layer sits at the boundary between [`VT-100`] wire format and internal
//! types.
//!
//! ## Parameter Handling
//!
//! **Missing or zero parameters default to 1:**
//! - `ESC [@` (missing param) → insert 1 character
//! - `ESC [0@` (explicit zero) → insert 1 character
//! - `ESC [5@` (explicit value) → insert 5 characters
//!
//! This is handled by [`extract_nth_single_non_zero()`] which returns [`NonZeroU16`].
//!
//! [`CSI`]: crate::CsiSequence
//! [`extract_nth_single_non_zero()`]: crate::ParamsExt::extract_nth_single_non_zero
//! [`NonZeroU16`]: std::num::NonZeroU16
//! [`test_char_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_char_ops
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_impl_char_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_char_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::ParamsExt;

/// Handle ICH (Insert Character) - insert n blank characters at cursor position.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Characters to the right of cursor shift right, characters beyond margin
/// are lost.
///
/// See [`OfsBufVT100::insert_chars_at_cursor`] for the implementation of this
/// shim.
///
/// [`OfsBufVT100::insert_chars_at_cursor`]: crate::OfsBufVT100::insert_chars_at_cursor
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn insert_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.insert_chars_at_cursor(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to insert {:?} chars at cursor position {:?}",
        how_many,
        performer.ofs_buf_vt_100.cursor_pos
    );
}

/// Handle DCH (Delete Character) - delete n characters at cursor position.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Characters to the right of cursor shift left, blanks are inserted at
/// line end.
///
/// See [`OfsBufVT100::delete_chars_at_cursor`] for the implementation of this
/// shim.
///
/// [`OfsBufVT100::delete_chars_at_cursor`]: crate::OfsBufVT100::delete_chars_at_cursor
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn delete_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.delete_chars_at_cursor(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to delete {:?} chars at cursor position {:?}",
        how_many,
        performer.ofs_buf_vt_100.cursor_pos
    );
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Characters are replaced with blanks, no shifting occurs (unlike DCH).
///
/// See [`OfsBufVT100::erase_chars_at_cursor`] for the implementation of this shim.
///
/// [`OfsBufVT100::erase_chars_at_cursor`]: crate::OfsBufVT100::erase_chars_at_cursor
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn erase_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.erase_chars_at_cursor(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to erase {:?} chars at cursor position {:?}",
        how_many,
        performer.ofs_buf_vt_100.cursor_pos
    );
}

/// Handle REP (Repeat Character) - repeat last printable character n times.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: The last printable character (tracked by parser state) is repeated
/// at the current cursor position.
///
/// See [`OfsBufVT100::repeat_chars_at_cursor`] for the implementation of this shim.
///
/// [`OfsBufVT100::repeat_chars_at_cursor`]: crate::OfsBufVT100::repeat_chars_at_cursor
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn repeat_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.repeat_chars_at_cursor(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to repeat {:?} chars at cursor position {:?}",
        how_many,
        performer.ofs_buf_vt_100.cursor_pos
    );
}

/// Handles printable character printing - display character at cursor position.
///
/// **[`VT-100`] Behavior**: Character set translation applied if [`DEC`] graphics mode is
/// active.
///
/// See [`OfsBufVT100::print_char`] for the implementation of this shim.
///
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`OfsBufVT100::print_char`]: crate::OfsBufVT100::print_char
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
pub fn print_char(performer: &mut AnsiToOfsBufPerformer, ch: char) {
    let result = performer.ofs_buf_vt_100.print_char(ch);
    debug_assert!(
        result.is_ok(),
        "Failed to print char {:?} at cursor position {:?}",
        ch,
        performer.ofs_buf_vt_100.cursor_pos
    );
}
