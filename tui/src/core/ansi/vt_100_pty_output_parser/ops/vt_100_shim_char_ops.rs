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
//!     Update OfsBuf state
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
use std::debug_assert_matches;

/// Handle ICH (Insert Character) - insert n blank characters at cursor position.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Characters to the right of cursor shift right, characters beyond margin
/// are lost.
///
/// See [`OfsBufVT100::insert_chars`] for the implementation of this
/// shim.
///
/// [`OfsBufVT100::insert_chars`]: crate::OfsBufVT100::insert_chars
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn insert_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.insert_chars(how_many);
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to insert {how_many:?} chars at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}

/// Handle DCH (Delete Character) - delete n characters at cursor position.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Characters to the right of cursor shift left, blanks are inserted at
/// line end.
///
/// See [`OfsBufVT100::delete_chars`] for the implementation of this
/// shim.
///
/// [`OfsBufVT100::delete_chars`]: crate::OfsBufVT100::delete_chars
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn delete_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.delete_chars(how_many);
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to delete {how_many:?} chars at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}

/// Handle ECH (Erase Character) - erase n characters at cursor position.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling.
///
/// **Behavior**: Characters are replaced with blanks, no shifting occurs (unlike DCH).
///
/// See [`OfsBufVT100::clear_chars`] for the implementation of this shim.
///
/// [`OfsBufVT100::clear_chars`]: crate::OfsBufVT100::clear_chars
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn erase_chars(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.clear_chars(how_many);
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to erase {how_many:?} chars at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
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
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to print char {ch:?} at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}
