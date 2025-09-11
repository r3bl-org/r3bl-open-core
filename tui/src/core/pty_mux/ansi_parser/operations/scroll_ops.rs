// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Scrolling operations.
//!
//! # CSI Sequence Architecture
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
//!     csi_dispatch() [THIS METHOD]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use super::{super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                    protocols::csi_codes::MovementCount, term_units::TermRow},
            cursor_ops};
use crate::{len, row};

/// Move cursor down one line, scrolling the buffer if at bottom.
/// Implements the ESC D (IND) escape sequence.
/// Respects DECSTBM scroll region margins.
pub fn index_down(performer: &mut AnsiToOfsBufPerformer) {
    let max_row = /* 1-based */ performer.ofs_buf.window_size.row_height;
    let current_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region = performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom_boundary = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Check if we're at the bottom of the scroll region.
    if current_row >= scroll_bottom_boundary {
        // At scroll region bottom - scroll buffer content up by one line.
        scroll_buffer_up(performer);
    } else {
        // Not at scroll region bottom - just move cursor down.
        cursor_ops::cursor_down_by_n(performer, row(1));
    }
}

/// Move cursor up one line, scrolling the buffer if at top.
/// Implements the ESC M (RI) escape sequence.
/// Respects DECSTBM scroll region margins.
pub fn reverse_index_up(performer: &mut AnsiToOfsBufPerformer) {
    let current_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top_boundary = maybe_scroll_region
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Check if we're at the top of the scroll region.
    if current_row <= scroll_top_boundary {
        // At scroll region top - scroll buffer content down by one line.
        scroll_buffer_down(performer);
    } else {
        // Not at scroll region top - just move cursor up.
        cursor_ops::cursor_up_by_n(performer, row(1));
    }
}

/// Scroll buffer content up by one line (for ESC D at bottom).
/// The top line is lost, and a new empty line appears at bottom.
/// Respects DECSTBM scroll region margins.
///
/// Example - scrolling buffer up within scroll region
///
/// ```text
/// Before:        Row: 0-based
/// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
///              │  1  │ Line A (will be lost)               │ ← Top line lost
///              │  2  │ Line B                              │
///              │  3  │ Line C                              │
///              │  4  │ Line D  ← cursor at scroll_bottom   │
///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// After scroll up:
/// max_height=6 ╮     ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤
///              │  1  │ Line B (moved up)                   │
///              │  2  │ Line C (moved up)                   │
///              │  3  │ Line D (moved up)                   │
///              │  4  │ (blank line)  ← cursor stays here   │
///              │     ├─────────────────────────────────────┤
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// Result: Content scrolls up, Line A lost, blank line added at bottom
/// ```
pub fn scroll_buffer_up(performer: &mut AnsiToOfsBufPerformer) {
    let max_row = /* 1-based */ performer.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ row(0),
            /* Some */ |row_index| row_index,
        );

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ |row| row,
        );

    // Use shift_lines_up to shift lines up within the scroll region
    performer.ofs_buf.shift_lines_up(
        {
            let start = scroll_top;
            let end = scroll_bottom + 1;
            start..end
        },
        len(1),
    );
}

/// Scroll buffer content down by one line (for ESC M at top).
/// The bottom line is lost, and a new empty line appears at top.
/// Respects DECSTBM scroll region margins.
///
/// Example - Scrolling buffer down within scroll region
///
/// ```text
/// Before:        Row: 0-based
/// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
///              │  1  │ Line A  ← cursor at scroll_top      │
///              │  2  │ Line B                              │
///              │  3  │ Line C                              │
///              │  4  │ Line D                              │
///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// After scroll down:
/// max_height=6 ╮     ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤
///              │  1  │ (blank line)  ← cursor stays here   │
///              │  2  │ Line A (moved down)                 │
///              │  3  │ Line B (moved down)                 │
///              │  4  │ Line C (moved down)                 │
///              │     ├─────────────────────────────────────┤
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// Result: Content scrolls down, Line D lost, blank line added at top
/// ```
pub fn scroll_buffer_down(performer: &mut AnsiToOfsBufPerformer) {
    let max_row = /* 1-based */ performer.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ row(0),
            /* Some */ |row_index| row_index,
        );

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ |row_index| row_index,
        );

    // Use shift_lines_down to shift lines down within the scroll region
    performer.ofs_buf.shift_lines_down(
        {
            let start = scroll_top;
            let end = scroll_bottom + 1;
            start..end
        },
        len(1),
    );
}

/// Handle SU (Scroll Up) - scroll display up by n lines.
pub fn scroll_up(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    for _ in 0..how_many.as_u16() {
        scroll_buffer_up(performer);
    }
}

/// Handle SD (Scroll Down) - scroll display down by n lines.
pub fn scroll_down(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    for _ in 0..how_many.as_u16() {
        scroll_buffer_down(performer);
    }
}
