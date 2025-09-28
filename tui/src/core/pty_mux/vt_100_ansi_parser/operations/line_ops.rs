// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line insertion and deletion operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation](super::super) for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_line_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_line_ops`] - Full pipeline testing via public API
//!
//! [`impl_line_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_line_ops
//! [`test_line_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::test_line_ops
//!
//! # Architecture Overview
//!
//! ```text
//! ╭─────────────────╮    ╭───────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │───▶│ PTY Master    │───▶│ VTE Parser      │───▶│ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream) │    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰───────────────╯    ╰─────────────────╯    │  buffer)     │
//!        │                                            │              ╰──────────────╯
//!        │                                            ▼                      │
//!        │                                   ╔═════════════════╗             │
//!        │                                   ║ Perform Trait   ║             │
//!        │                                   ║ Implementation  ║             │
//!        │                                   ╚═════════════════╝             │
//!        │                                                                   │
//!        │                                   ╭─────────────────╮             │
//!        │                                   │ RenderPipeline  │◀────────────╯
//!        │                                   │ paint()         │
//!        ╰───────────────────────────────────▶ Terminal Output │
//!                                            ╰─────────────────╯
//! ```
//!
//! # CSI Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC[2L" (insert 2 lines)
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
//!       - sgr_ops:: for styling (m)     ╭───────────╮
//!       - line_ops:: for lines (L,M) <- │THIS MODULE│
//!       - char_ops:: for chars (@,P,X)  ╰───────────╯
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MovementCount};
use crate::{RowHeight, RowIndex};

/// Handle IL (Insert Line) - insert n blank lines at cursor position.
/// Lines below cursor and within scroll region shift down.
/// See [`OffscreenBuffer::shift_lines_down`] for detailed behavior and examples.
///
/// [`OffscreenBuffer::shift_lines_down`]: crate::OffscreenBuffer::shift_lines_down
pub fn insert_lines(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = MovementCount::parse_as_row_height_non_zero(params);
    let at = performer.ofs_buf.cursor_pos.row_index;

    insert_lines_at(performer, at, how_many);
}

/// Handle DL (Delete Line) - delete n lines starting at cursor position.
/// Lines below cursor and within scroll region shift up.
/// Blank lines are added at the bottom of the scroll region.
///
/// See [`OffscreenBuffer::shift_lines_up`] for detailed behavior and examples.
///
/// [`OffscreenBuffer::shift_lines_up`]: crate::OffscreenBuffer::shift_lines_up
pub fn delete_lines(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = MovementCount::parse_as_row_height_non_zero(params);
    let at = performer.ofs_buf.cursor_pos.row_index;

    delete_lines_at(performer, at, how_many);
}

/// Insert multiple blank lines at the specified row.
/// Lines below shift down within the scroll region.
/// The bottom lines of the scroll region are lost.
fn insert_lines_at(
    performer: &mut AnsiToOfsBufPerformer,
    row_index: RowIndex,
    how_many: RowHeight,
) {
    // Get scroll region boundaries using helper methods.
    let scroll_top = performer.ofs_buf.get_scroll_top_boundary();
    let scroll_bottom = performer.ofs_buf.get_scroll_bottom_boundary();

    // Only operate within scroll region and if cursor is within region.
    if row_index < scroll_top || row_index > scroll_bottom {
        return;
    }

    // Use shift_lines_down to shift lines down by how_many positions.
    let result = performer.ofs_buf.shift_lines_down(
        {
            let start = row_index;
            let end = scroll_bottom + 1;
            start..end
        },
        how_many.into(),
    );
    debug_assert!(
        result.is_ok(),
        "Failed to shift lines down for line insertion at row {row_index:?}, scroll region {row_index:?}..{scroll_bottom:?}"
    );

    // Clear the newly inserted lines (shift_lines_down fills with blanks at the top).
    for offset in 0..how_many.as_u16() {
        if let Some(clear_row_u16) = row_index.as_u16().checked_add(offset) {
            let clear_row = RowIndex::from(clear_row_u16);
            if clear_row <= scroll_bottom {
                clear_line(performer, clear_row);
            }
        }
    }
}

/// Delete multiple lines at the specified row.
/// Lines below shift up within the scroll region.
/// Blank lines are added at the bottom of the scroll region.
fn delete_lines_at(
    performer: &mut AnsiToOfsBufPerformer,
    row_index: RowIndex,
    how_many: RowHeight,
) {
    // Get scroll region boundaries using helper methods.
    let scroll_top = performer.ofs_buf.get_scroll_top_boundary();
    let scroll_bottom = performer.ofs_buf.get_scroll_bottom_boundary();

    // Only operate within scroll region and if cursor is within region.
    if row_index < scroll_top || row_index > scroll_bottom {
        return;
    }

    // Use shift_lines_up to shift lines up by how_many positions.
    let result = performer.ofs_buf.shift_lines_up(
        {
            let start = row_index;
            let end = scroll_bottom + 1;
            start..end
        },
        how_many.into(),
    );
    debug_assert!(
        result.is_ok(),
        "Failed to shift lines up for line deletion at row {row_index:?}, scroll region {row_index:?}..{scroll_bottom:?}"
    );

    // Clear the bottom lines of the scroll region (shift_lines_up fills with blanks at
    // the bottom).
    for offset in 0..how_many.as_u16() {
        if let Some(clear_row_u16) = scroll_bottom.as_u16().checked_sub(offset) {
            let clear_row = RowIndex::from(clear_row_u16);
            if clear_row >= row_index {
                clear_line(performer, clear_row);
            }
        }
    }
}

/// Clear a line by filling it with blanks.
fn clear_line(performer: &mut AnsiToOfsBufPerformer, row_index: RowIndex) {
    let result = performer.ofs_buf.clear_line(row_index);
    debug_assert!(result.is_ok(), "Failed to clear line at row {row_index:?}");
}
