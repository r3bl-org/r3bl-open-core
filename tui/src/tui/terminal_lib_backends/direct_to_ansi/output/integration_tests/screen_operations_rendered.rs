// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Behavioral tests for screen operations via [`OffscreenBuffer`] rendering.
//!
//! These tests complement the byte-level tests in [`screen_operations`] by verifying
//! that clear operations produce the correct **visual result** when ANSI sequences
//! are rendered to a buffer.
//!
//! # Important Design Note
//!
//! The [`OffscreenBuffer`] ANSI parser **intentionally ignores** clear operations
//! (ED/EL sequences). This is by design because TUI applications repaint themselves
//! after clear operations. See [`performer.rs`] where `ED_ERASE_DISPLAY` and
//! `EL_ERASE_LINE` are explicitly ignored.
//!
//! As a result, these tests verify:
//! - Clear ANSI sequences are **generated correctly** (verified by byte-level tests)
//! - Text painting **after** clear operations works correctly (cursor positioning)
//! - Buffer state is correct for content that **isn't** cleared
//!
//! [`screen_operations`]: super::screen_operations
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`performer.rs`]: crate::vt_100_pty_output_parser::performer

use super::test_helpers_rendered::*;
use crate::{offscreen_buffer::test_fixtures_ofs_buf::*, render_op::RenderOpCommon};

/// Verify text can be painted at various positions.
///
/// NOTE: Clear operations (`ClearScreen`, `ClearLine`, etc.) are intentionally ignored
/// by [`OffscreenBuffer`]'s ANSI parser - TUI apps repaint themselves. This test
/// verifies the cursor positioning and text painting still work correctly.
#[test]
fn test_paint_after_clear_sequence_rendered() {
    // Even though ClearScreen is ignored by the parser, the cursor positioning
    // and subsequent text painting should work correctly.
    let ops = vec![
        move_cursor_abs(5, 5),
        paint_text("X", None),
        crate::RenderOpOutput::Common(RenderOpCommon::ClearScreen),
        // Move explicitly and paint new content.
        move_cursor_abs(3, 3),
        paint_text("Y", None),
    ];

    let buffer = execute_ops_and_render(ops);

    // Both 'X' and 'Y' should be present (clear is ignored).
    assert_plain_char_at(&buffer, 5, 5, 'X');
    assert_plain_char_at(&buffer, 3, 3, 'Y');
}

/// Verify painting on empty line positions cursor correctly.
#[test]
fn test_paint_on_empty_line_rendered() {
    // Paint on row 5 which was never written to.
    let ops = vec![move_cursor_abs(5, 0), paint_text("Hello", None)];

    let buffer = execute_ops_and_render(ops);

    // Verify "Hello" appears at row 5.
    assert_plain_text_at(&buffer, 5, 0, "Hello");
}

/// Verify multiple text segments at different rows.
#[test]
fn test_multiple_rows_text_rendered() {
    // Paint text on multiple rows.
    let ops = vec![
        move_cursor_abs(0, 0),
        paint_text("Row Zero", None),
        move_cursor_abs(1, 0),
        paint_text("Row One", None),
        move_cursor_abs(2, 0),
        paint_text("Row Two", None),
    ];

    let buffer = execute_ops_and_render(ops);

    // Verify all rows have content.
    assert_plain_text_at(&buffer, 0, 0, "Row Zero");
    assert_plain_text_at(&buffer, 1, 0, "Row One");
    assert_plain_text_at(&buffer, 2, 0, "Row Two");
}

/// Verify overwriting text at same position.
#[test]
fn test_overwrite_text_rendered() {
    // Paint "ABCDE", then overwrite with "XYZ" starting at column 1.
    let ops = vec![
        move_cursor_abs(0, 0),
        paint_text("ABCDE", None),
        move_cursor_abs(0, 1),
        paint_text("XYZ", None),
    ];

    let buffer = execute_ops_and_render(ops);

    // Should be "AXYZE" after overwrite.
    assert_plain_char_at(&buffer, 0, 0, 'A');
    assert_plain_char_at(&buffer, 0, 1, 'X');
    assert_plain_char_at(&buffer, 0, 2, 'Y');
    assert_plain_char_at(&buffer, 0, 3, 'Z');
    assert_plain_char_at(&buffer, 0, 4, 'E');
}

/// Verify text at buffer edges.
#[test]
fn test_text_at_edges_rendered() {
    // Paint text at origin and near edges.
    let ops = vec![
        move_cursor_abs(0, 0),
        paint_text("TopLeft", None),
        move_cursor_abs(23, 70),
        paint_text("Bottom", None),
    ];

    let buffer = execute_ops_and_render(ops);

    // Verify both texts appear at correct positions.
    assert_plain_text_at(&buffer, 0, 0, "TopLeft");
    assert_plain_text_at(&buffer, 23, 70, "Bottom");
}

/// Verify sparse text placement (gaps between text).
#[test]
fn test_sparse_text_placement_rendered() {
    // Paint text with gaps (sparse placement).
    let ops = vec![
        move_cursor_abs(0, 0),
        paint_text("A", None),
        move_cursor_abs(0, 10),
        paint_text("B", None),
        move_cursor_abs(0, 20),
        paint_text("C", None),
    ];

    let buffer = execute_ops_and_render(ops);

    // Verify characters at sparse positions.
    assert_plain_char_at(&buffer, 0, 0, 'A');
    assert_plain_char_at(&buffer, 0, 10, 'B');
    assert_plain_char_at(&buffer, 0, 20, 'C');

    // Verify gaps are empty.
    assert_empty_at(&buffer, 0, 1);
    assert_empty_at(&buffer, 0, 5);
    assert_empty_at(&buffer, 0, 11);
}
