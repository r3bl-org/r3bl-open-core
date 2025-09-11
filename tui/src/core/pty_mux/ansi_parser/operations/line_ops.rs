// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line insertion and deletion operations.

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MovementCount, term_units::TermRow};
use crate::{RowIndex, len, row};

/// Handle IL (Insert Line) - insert n blank lines at cursor position.
/// Lines below cursor and within scroll region shift down.
/// Lines scrolled off the bottom are lost.
///
/// Example - Inserting 2 blank lines at cursor position
///
/// ```text
/// Before:        Row: 0-based
/// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
///              │  1  │ Line A                              │
///              │  2  │ Line B  ← cursor (row 2, 0-based)   │ ← Insert 2 lines here
///              │  3  │ Line C                              │
///              │  4  │ Line D                              │
///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// After IL 2:
/// max_height=6 ╮     ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤
///              │  1  │ Line A                              │
///              │  2  │ (blank line)  ← cursor stays here   │
///              │  3  │ (blank line)                        │
///              │  4  │ Line B                              │
///              │     ├─────────────────────────────────────┤
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// Result: 2 blank lines inserted, Line B-C-D shifted down, Line C-D lost beyond scroll_bottom
/// ```
pub fn insert_lines(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    let current_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;

    for _ in 0..how_many.as_u16() {
        insert_line_at(performer, current_row);
    }
}

/// Handle DL (Delete Line) - delete n lines starting at cursor position.
/// Lines below cursor and within scroll region shift up.
/// Blank lines are added at the bottom of the scroll region.
///
/// Example - Deleting 2 lines at cursor position
///
/// ```text
/// Before:        Row: 0-based
/// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤ ← scroll_top
///              │  1  │ Line A                              │   (row 1, 0-based)
///              │  2  │ Line B  ← cursor (row 2, 0-based)   │ ← Delete 2 lines here
///              │  3  │ Line C                              │
///              │  4  │ Line D                              │
///              │     ├─────────────────────────────────────┤ ← scroll_bottom
///              ╰  5  │ Footer line (outside scroll region) │   (row 4, 0-based)
///                    └─────────────────────────────────────┘
///
/// After DL 2:
/// max_height=6 ╮     ┌─────────────────────────────────────┐
/// (1-based)    │  0  │ Header line (outside scroll region) │
///              │     ├─────────────────────────────────────┤
///              │  1  │ Line A                              │
///              │  2  │ Line D  ← cursor stays here         │
///              │  3  │ (blank line)                        │
///              │  4  │ (blank line)                        │
///              │     ├─────────────────────────────────────┤
///              ╰  5  │ Footer line (outside scroll region) │
///                    └─────────────────────────────────────┘
///
/// Result: Line B and C deleted, Line D shifted up, blank lines added at bottom
/// ```
pub fn delete_lines(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    let how_many = /* 1-based */ MovementCount::parse_as_row_height(params);
    let current_row = /* 0-based */ performer.ofs_buf.my_pos.row_index;

    for _ in 0..how_many.as_u16() {
        delete_line_at(performer, current_row);
    }
}

/// Insert a single blank line at the specified row.
/// Lines below shift down within the scroll region.
/// The bottom line of the scroll region is lost.
fn insert_line_at(
    performer: &mut AnsiToOfsBufPerformer,
    row_index: /* 0-based */ RowIndex,
) {
    let max_row = /* 1-based */ performer.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Only operate within scroll region and if cursor is within region.
    if row_index < scroll_top || row_index > scroll_bottom {
        return;
    }

    // Use shift_lines_down to shift lines down and clear the newly inserted line
    performer.ofs_buf.shift_lines_down(
        {
            let start = row_index;
            let end = scroll_bottom + 1;
            start..end
        },
        len(1),
    );

    // Clear the newly inserted line (shift_lines_down fills with blanks at the top)
    clear_line(performer, row_index);
}

/// Delete a single line at the specified row.
/// Lines below shift up within the scroll region.
/// A blank line is added at the bottom of the scroll region.
fn delete_line_at(
    performer: &mut AnsiToOfsBufPerformer,
    row_index: /* 0-based */ RowIndex,
) {
    let max_row = /* 1-based */ performer.ofs_buf.window_size.row_height;

    // Get top boundary of scroll region (or 0 if no region set).
    let maybe_scroll_region_top = performer.ofs_buf.ansi_parser_support.scroll_region_top;
    let scroll_top = maybe_scroll_region_top
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(/* None */ row(0), /* Some */ Into::into);

    // Get bottom boundary of scroll region (or screen bottom if no region set).
    let maybe_scroll_region_bottom =
        performer.ofs_buf.ansi_parser_support.scroll_region_bottom;
    let scroll_bottom = maybe_scroll_region_bottom
        .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
        .map_or(
            /* None */ max_row.convert_to_row_index(),
            /* Some */ Into::into,
        );

    // Only operate within scroll region and if cursor is within region.
    if row_index < scroll_top || row_index > scroll_bottom {
        return;
    }

    // Use shift_lines_up to shift lines up and clear the bottom line
    performer.ofs_buf.shift_lines_up(
        {
            let start = row_index;
            let end = scroll_bottom + 1;
            start..end
        },
        len(1),
    );

    // Clear the bottom line of the scroll region (shift_lines_up fills with blanks at the
    // bottom)
    clear_line(performer, scroll_bottom);
}

/// Clear a line by filling it with blanks.
fn clear_line(performer: &mut AnsiToOfsBufPerformer, row_index: RowIndex) {
    performer.ofs_buf.clear_line(row_index);
}
