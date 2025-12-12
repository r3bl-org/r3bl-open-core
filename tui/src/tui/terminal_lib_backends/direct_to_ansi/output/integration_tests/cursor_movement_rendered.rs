// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Behavioral tests for cursor movement operations via [`OffscreenBuffer`] rendering.
//!
//! These tests complement the byte-level tests in [`cursor_movement`] by verifying
//! that cursor positioning produces the correct **visual result** when ANSI sequences
//! are rendered to a buffer.
//!
//! # What These Tests Verify
//!
//! - Cursor absolute positioning places characters at correct coordinates
//! - 0-based to 1-based coordinate conversion works correctly
//! - Relative cursor movement calculates positions correctly
//!
//! [`cursor_movement`]: super::cursor_movement
//! [`OffscreenBuffer`]: crate::OffscreenBuffer

use super::test_helpers_rendered::*;
use crate::{col, offscreen_buffer::test_fixtures_ofs_buf::*, pos,
            render_op::RenderOpCommon, row};

/// Verify cursor at origin (0,0) places character at top-left of buffer.
#[test]
fn test_move_cursor_absolute_origin_rendered() {
    // Move cursor to origin and paint 'X'.
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(0, 0), paint_text("X", None)]);

    // Verify 'X' appears at (0,0).
    assert_plain_char_at(&buffer, 0, 0, 'X');
}

/// Verify cursor positioning at non-origin coordinates.
///
/// This tests the critical 0-based to 1-based coordinate conversion:
/// - Our API uses 0-based indices (row 5, col 10)
/// - ANSI CSI CUP uses 1-based indices (row 6, col 11)
/// - The character should appear at buffer[5][10]
#[test]
fn test_move_cursor_absolute_5_10_rendered() {
    // Move cursor to (5,10) and paint 'X'.
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(5, 10), paint_text("X", None)]);

    // Verify 'X' appears at (5,10), not at origin.
    assert_plain_char_at(&buffer, 5, 10, 'X');

    // Verify origin is empty (didn't accidentally go there).
    assert_empty_at(&buffer, 0, 0);
}

/// Verify relative cursor positioning calculates origin + offset correctly.
///
/// MoveCursorPositionRelTo(origin=(5,3), relative=(2,7)) should place cursor at (7,10).
#[test]
fn test_move_cursor_relative_to_rendered() {
    // Create a relative positioning operation.
    // MoveCursorPositionRelTo adds origin + relative positions.
    let origin = pos(row(5) + col(3));
    let relative = pos(row(2) + col(7));
    let move_op = RenderOpCommon::MoveCursorPositionRelTo(origin, relative);

    let buffer = execute_ops_and_render(vec![
        crate::RenderOpOutput::Common(move_op),
        paint_text("X", None),
    ]);

    // Verify 'X' appears at (5+2=7, 3+7=10).
    assert_plain_char_at(&buffer, 7, 10, 'X');
}

/// Verify multiple cursor movements and text paints produce correct layout.
#[test]
fn test_multiple_cursor_positions_rendered() {
    // Paint characters at different positions.
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text("A", None),
        move_cursor_abs(5, 5),
        paint_text("B", None),
        move_cursor_abs(10, 10),
        paint_text("C", None),
    ]);

    // Verify all characters at correct positions.
    assert_plain_char_at(&buffer, 0, 0, 'A');
    assert_plain_char_at(&buffer, 5, 5, 'B');
    assert_plain_char_at(&buffer, 10, 10, 'C');

    // Verify positions in between are empty.
    assert_empty_at(&buffer, 0, 1);
    assert_empty_at(&buffer, 5, 6);
}

/// Verify cursor positioning with text string (cursor advances per character).
#[test]
fn test_cursor_position_with_text_string_rendered() {
    // Move to (2,3) and paint "Hello".
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(2, 3), paint_text("Hello", None)]);

    // Verify "Hello" starts at (2,3).
    assert_plain_text_at(&buffer, 2, 3, "Hello");

    // Verify preceding columns are empty.
    assert_empty_at(&buffer, 2, 0);
    assert_empty_at(&buffer, 2, 1);
    assert_empty_at(&buffer, 2, 2);
}

/// Verify cursor at edge of buffer (last row, last column).
#[test]
fn test_cursor_at_buffer_edge_rendered() {
    // Our test buffer is 80x24. Move to (23, 79) - last cell.
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(23, 79), paint_text("Z", None)]);

    // Verify 'Z' appears at the last cell.
    assert_plain_char_at(&buffer, 23, 79, 'Z');
}
