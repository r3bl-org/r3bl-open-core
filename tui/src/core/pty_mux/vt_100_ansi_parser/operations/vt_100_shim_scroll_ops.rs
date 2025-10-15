// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scrolling operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation] for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_scroll_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_scroll_ops`] - Full pipeline testing via public API
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
//! Application sends "ESC[3S" (scroll up 3 lines)
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
//!       - cursor_ops:: for movement (A,B,C,D,H) ╭───────────╮
//!       - scroll_ops:: for scrolling (S,T) <--- │THIS MODULE│
//!       - sgr_ops:: for styling (m)             ╰───────────╯
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```
//!
//! # VT100 Protocol Conventions
//!
//! This shim layer sits at the boundary between VT100 wire format and internal types.
//! Understanding VT100 parameter conventions is essential for maintaining this code.
//!
//! ## Parameter Handling
//!
//! **Missing or zero parameters default to 1:**
//! - `ESC[S` (missing param) → scroll up 1 line
//! - `ESC[0S` (explicit zero) → scroll up 1 line
//! - `ESC[5S` (explicit value) → scroll up 5 lines
//!
//! This is handled by [`extract_nth_single_non_zero()`] which returns [`NonZeroU16`].
//!
//! ## Scroll Region (DECSTBM)
//!
//! Scroll operations respect the scrolling region set by DECSTBM. The region bounds are
//! maintained internally by [`OffscreenBuffer`] and applied automatically to all scroll operations.
//!
//! [`impl_scroll_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_scroll_ops
//! [`test_scroll_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::vt_100_test_scroll_ops
//! [module-level documentation]: super::super
//! [operations module documentation]: super
//! [`vt_100_ansi_conformance_tests`]: super::super::vt_100_ansi_conformance_tests
//! [`extract_nth_single_non_zero()`]: crate::ParamsExt::extract_nth_single_non_zero
//! [`NonZeroU16`]: std::num::NonZeroU16
//! [`OffscreenBuffer`]: crate::OffscreenBuffer

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::ParamsExt;

/// Move cursor down one line, scrolling the buffer if at bottom.
///
/// Implements the ESC D (IND) escape sequence.
///
/// **VT100 Protocol**: See [module-level documentation](self) for scroll region handling.
///
/// **Behavior**: Respects DECSTBM scroll region margins.
pub fn index_down(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf.index_down();
    debug_assert!(
        result.is_ok(),
        "Failed to index down at cursor position {:?}",
        performer.ofs_buf.cursor_pos
    );
}

/// Move cursor up one line, scrolling the buffer if at top.
///
/// Implements the ESC M (RI) escape sequence.
///
/// **VT100 Protocol**: See [module-level documentation](self) for scroll region handling.
///
/// **Behavior**: Respects DECSTBM scroll region margins.
pub fn reverse_index_up(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf.reverse_index_up();
    debug_assert!(
        result.is_ok(),
        "Failed to reverse index up at cursor position {:?}",
        performer.ofs_buf.cursor_pos
    );
}

/// Scroll buffer content up by one line (for ESC D at bottom).
///
/// **VT100 Protocol**: See [module-level documentation](self) for scroll region handling.
///
/// **Behavior**: The top line is lost, and a new empty line appears at bottom.
/// Respects DECSTBM scroll region margins.
pub fn scroll_buffer_up(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf.scroll_buffer_up();
    debug_assert!(
        result.is_ok(),
        "Failed to scroll buffer up at cursor position {:?}",
        performer.ofs_buf.cursor_pos
    );
}

/// Scroll buffer content down by one line (for ESC M at top).
///
/// **VT100 Protocol**: See [module-level documentation](self) for scroll region handling.
///
/// **Behavior**: The bottom line is lost, and a new empty line appears at top.
/// Respects DECSTBM scroll region margins.
pub fn scroll_buffer_down(performer: &mut AnsiToOfsBufPerformer) {
    let result = performer.ofs_buf.scroll_buffer_down();
    debug_assert!(
        result.is_ok(),
        "Failed to scroll buffer down at cursor position {:?}",
        performer.ofs_buf.cursor_pos
    );
}

/// Handle SU (Scroll Up) - scroll display up by n lines.
///
/// **VT100 Protocol**: See [module-level documentation](self) for parameter handling
/// (missing/zero parameters default to 1) and scroll region handling.
pub fn scroll_up(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf.scroll_up(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to scroll up {:?} lines at cursor position {:?}",
        how_many,
        performer.ofs_buf.cursor_pos
    );
}

/// Handle SD (Scroll Down) - scroll display down by n lines.
///
/// **VT100 Protocol**: See [module-level documentation](self) for parameter handling
/// (missing/zero parameters default to 1) and scroll region handling.
pub fn scroll_down(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = params.extract_nth_single_non_zero(0).get().into();
    let result = performer.ofs_buf.scroll_down(how_many);
    debug_assert!(
        result.is_ok(),
        "Failed to scroll down {:?} lines at cursor position {:?}",
        how_many,
        performer.ofs_buf.cursor_pos
    );
}
