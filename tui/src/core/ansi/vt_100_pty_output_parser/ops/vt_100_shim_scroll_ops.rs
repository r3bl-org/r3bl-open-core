// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scrolling operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_scroll_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_scroll_ops`] - Full pipeline testing via public API
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
//! Application sends "ESC [3S" (scroll up 3 lines)
//!         ↓
//!     PTY Controlled (escape sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC [...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to ops module:
//!       - cursor_ops:: for movement (A,B,C,D,H)                ╭───────────╮
//!       - scroll_ops:: for scrolling (S,T)                  <- │THIS MODULE│
//!       - sgr_ops:: for styling (m)                            ╰───────────╯
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OfsBuf state
//! ```
//!
//! # [`VT-100`] Protocol Conventions
//!
//! This shim layer sits at the boundary between [`VT-100`] wire format and internal
//! types. Understanding [`VT-100`] parameter conventions is essential for maintaining
//! this code.
//!
//! ## Parameter Handling
//!
//! **Missing or zero parameters default to 1:**
//! - `ESC [S` (missing param) → scroll up 1 line
//! - `ESC [0S` (explicit zero) → scroll up 1 line
//! - `ESC [5S` (explicit value) → scroll up 5 lines
//!
//! This is handled by [`extract_nth_single_non_zero()`] which returns [`NonZeroU16`].
//!
//! ## Scroll Region ([`DECSTBM`])
//!
//! Scroll operations respect the scrolling region set by [`DECSTBM`]. The region bounds
//! are maintained internally by [`OfsBuf`] and applied automatically to all
//! scroll operations.
//!
//! [`CSI`]: crate::CsiSequence
//! [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
//! [`extract_nth_single_non_zero()`]: crate::ParamsExt::extract_nth_single_non_zero
//! [`NonZeroU16`]: std::num::NonZeroU16
//! [`OfsBuf`]: crate::OfsBuf
//! [`test_scroll_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_scroll_ops
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100_impl_scroll_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_scroll_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::ParamsExt;
use std::debug_assert_matches;

/// Move cursor down one line, scrolling the buffer if at bottom.
///
/// Implements the `ESC D` (`IND`) escape sequence.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for scroll region handling.
///
/// **Behavior**: Respects [`DECSTBM`] scroll region margins.
///
/// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn index_down(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf_vt_100.index_down();
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to index down at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}

/// Move cursor up one line, scrolling the buffer if at top.
///
/// Implements the `ESC M` (`RI`) escape sequence.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for scroll region handling.
///
/// **Behavior**: Respects [`DECSTBM`] scroll region margins.
///
/// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn reverse_index_up(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf_vt_100.reverse_index_up();
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to reverse index up at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}

/// Scroll buffer content up by one line (for `ESC D` at bottom).
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for scroll region handling.
///
/// **Behavior**: The top line is lost, and a new empty line appears at bottom.
/// Respects [`DECSTBM`] scroll region margins.
///
/// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn scroll_buffer_up(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf_vt_100.scroll_buffer_up();
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to scroll buffer up at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}

/// Scroll buffer content down by one line (for `ESC M` at top).
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for scroll region handling.
///
/// **Behavior**: The bottom line is lost, and a new empty line appears at top.
/// Respects [`DECSTBM`] scroll region margins.
///
/// [`DECSTBM`]: https://vt100.net/docs/vt510-rm/DECSTBM.html
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn scroll_buffer_down(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf_vt_100.scroll_buffer_down();
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to scroll buffer down at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}

/// Handle SU (Scroll Up) - scroll display up by n lines.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1) and scroll region handling.
///
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn scroll_up(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.scroll_up(how_many);
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to scroll up {how_many:?} lines at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}

/// Handle SD (Scroll Down) - scroll display down by n lines.
///
/// **[`VT-100`] Protocol**: See [module-level documentation] for parameter handling
/// (missing/zero parameters default to 1) and scroll region handling.
///
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level documentation]: self
pub fn scroll_down(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf_vt_100.scroll_down(how_many);
    debug_assert_matches!(
        result,
        Ok(()),
        "Failed to scroll down {how_many:?} lines at cursor position {:?}",
        performer.ofs_buf_vt_100.get_cursor_pos()
    );
}
