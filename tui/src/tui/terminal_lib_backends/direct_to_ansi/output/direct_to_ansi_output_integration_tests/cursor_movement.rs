// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for cursor movement operations
//!
//! These tests validate:
//! 1. [`MoveCursorPositionAbs`] generates correct CUP (Cursor Position) [`ANSI`]
//!    sequences
//! 2. [`MoveCursorPositionRelTo`] correctly adds origin + relative offset
//! 3. Cursor state tracking in [`RenderOpsLocalData`] after movement
//! 4. [`MoveCursorToColumn`], [`MoveCursorToNextLine`], [`MoveCursorToPreviousLine`]
//!    operations
//! 5. Multiple cursor moves in sequence preserve correct final position
//! 6. Cursor position state matches [`ANSI`] output
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`MoveCursorPositionAbs`]: crate::render_op::RenderOpCommon::MoveCursorPositionAbs
//! [`MoveCursorPositionRelTo`]: crate::render_op::RenderOpCommon::MoveCursorPositionRelTo
//! [`MoveCursorToColumn`]: crate::render_op::RenderOpCommon::MoveCursorToColumn
//! [`MoveCursorToNextLine`]: crate::render_op::RenderOpCommon::MoveCursorToNextLine
//! [`MoveCursorToPreviousLine`]: crate::render_op::RenderOpCommon::MoveCursorToPreviousLine
//! [`RenderOpsLocalData`]: crate::tui::RenderOpsLocalData

use super::test_helpers::*;
use crate::{ansi_output, render_op::RenderOpCommon, term_row_delta, tui_color, vp_col,
            vp_height, vp_row};

#[test]
fn test_move_cursor_absolute_origin() {
    // Test moving cursor to origin (0,0)
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::MoveCursorPositionAbs(vp_row(0) + vp_col(0));
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI H with 1-based indexing: row 0 (0-based) = 1 (1-based), col 0 = 1
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_position(vp_row(0).into(), vp_col(0).into())
    );
    assert_eq!(state.cursor_pos, vp_row(0) + vp_col(0));
}

#[test]
fn test_move_cursor_absolute_5_10() {
    // Test moving cursor to (5, 10) in 0-based indices
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::MoveCursorPositionAbs(vp_row(5) + vp_col(10));
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI H with 1-based: row 5 (0-based) = 6 (1-based), col 10 = 11
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_position(
            vp_row(5).into(),
            vp_col(10).into()
        )
    );
    assert_eq!(state.cursor_pos, vp_row(5) + vp_col(10));
}

#[test]
fn test_move_cursor_absolute_20_40() {
    // Test moving cursor to further position
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::MoveCursorPositionAbs(vp_row(20) + vp_col(40));
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // row 20 = 21, col 40 = 41 in 1-based
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_position(
            vp_row(20).into(),
            vp_col(40).into()
        )
    );
    assert_eq!(state.cursor_pos, vp_row(20) + vp_col(40));
}

#[test]
fn test_move_cursor_relative_to() {
    // Test MoveCursorPositionRelTo which adds origin + relative
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let origin = vp_row(5) + vp_col(3);
    let relative = vp_row(2) + vp_col(7);
    let op = RenderOpCommon::MoveCursorPositionRelTo(origin, relative);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Final position: vp_row(5+2) + vp_col(3+7) = vp_row(7) + vp_col(10)
    // ANSI: row 7 = 8, col 10 = 11
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_position(
            vp_row(7).into(),
            vp_col(10).into()
        )
    );
    assert_eq!(state.cursor_pos, vp_row(7) + vp_col(10));
}

#[test]
fn test_move_cursor_to_column() {
    // Test MoveCursorToColumn which moves to a column in current row
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // First move to a specific position
    let move_abs = RenderOpCommon::MoveCursorPositionAbs(vp_row(5) + vp_col(5));
    let _unused = execute_and_capture(move_abs, &mut state, &output_device, &stdout_mock);
    let initial_row = state.cursor_pos.row_index;

    // Now move to column 20 (should keep same row)
    let (output_device2, stdout_mock2) = create_mock_output();
    let op = RenderOpCommon::MoveCursorToColumn(vp_col(20));
    let output = execute_and_capture(op, &mut state, &output_device2, &stdout_mock2);

    // CSI 21G (1-based column index)
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_to_column(vp_col(20).into())
    );
    // Row should remain unchanged
    assert_eq!(state.cursor_pos.row_index, initial_row);
    // Column should be updated
    assert_eq!(state.cursor_pos.col_index, vp_col(20));
}

#[test]
fn test_move_cursor_to_next_line() {
    // Test MoveCursorToNextLine which moves down N lines and to column 0
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // First position cursor at (5, 10)
    let move_abs = RenderOpCommon::MoveCursorPositionAbs(vp_row(5) + vp_col(10));
    let _unused = execute_and_capture(move_abs, &mut state, &output_device, &stdout_mock);

    // Move down 3 lines (to row 8, column 0)
    let (output_device2, stdout_mock2) = create_mock_output();
    let op = RenderOpCommon::MoveCursorToNextLine(vp_height(3));
    let output = execute_and_capture(op, &mut state, &output_device2, &stdout_mock2);

    // CSI 3E (move down 3 lines and to column 0)
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_next_line(
            term_row_delta(3).expect("conversion error")
        )
    );
    assert_eq!(state.cursor_pos.row_index, vp_row(8));
    assert_eq!(state.cursor_pos.col_index, vp_col(0));
}

#[test]
fn test_move_cursor_to_previous_line() {
    // Test MoveCursorToPreviousLine which moves up N lines and to column 0
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // First position cursor at (10, 15)
    let move_abs = RenderOpCommon::MoveCursorPositionAbs(vp_row(10) + vp_col(15));
    let _unused = execute_and_capture(move_abs, &mut state, &output_device, &stdout_mock);

    // Move up 3 lines (to row 7, column 0)
    let (output_device2, stdout_mock2) = create_mock_output();
    let op = RenderOpCommon::MoveCursorToPreviousLine(vp_height(3));
    let output = execute_and_capture(op, &mut state, &output_device2, &stdout_mock2);

    // CSI 3F (move up 3 lines and to column 0)
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_previous_line(
            term_row_delta(3).expect("conversion error")
        )
    );
    assert_eq!(state.cursor_pos.row_index, vp_row(7));
    assert_eq!(state.cursor_pos.col_index, vp_col(0));
}

#[test]
fn test_multiple_cursor_moves_sequence() {
    // Test multiple cursor movements in sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![
        RenderOpCommon::MoveCursorPositionAbs(vp_row(5) + vp_col(5)),
        RenderOpCommon::MoveCursorPositionAbs(vp_row(10) + vp_col(20)),
        RenderOpCommon::MoveCursorPositionAbs(vp_row(0) + vp_col(0)),
    ];

    let output =
        execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Should contain all three ANSI sequences
    assert!(
        output.contains(&ansi_output::cursor_movement::cursor_position(
            vp_row(5).into(),
            vp_col(5).into()
        ))
    );
    assert!(
        output.contains(&ansi_output::cursor_movement::cursor_position(
            vp_row(10).into(),
            vp_col(20).into()
        ))
    );
    assert!(
        output.contains(&ansi_output::cursor_movement::cursor_position(
            vp_row(0).into(),
            vp_col(0).into()
        )),
    );

    // Final state should match last position
    assert_eq!(state.cursor_pos, vp_row(0) + vp_col(0));
}

#[test]
fn test_cursor_state_persists_across_operations() {
    // Test that cursor state persists after other operations
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // Set cursor position
    let move_op = RenderOpCommon::MoveCursorPositionAbs(vp_row(7) + vp_col(12));
    let _unused = execute_and_capture(move_op, &mut state, &output_device, &stdout_mock);
    let saved_pos = state.cursor_pos;

    // Do a color operation (shouldn't affect cursor position)
    let (output_device2, stdout_mock2) = create_mock_output();
    let color_op = RenderOpCommon::SetFgColor(tui_color!(red));
    let _unused =
        execute_and_capture(color_op, &mut state, &output_device2, &stdout_mock2);

    // Cursor position should be unchanged
    assert_eq!(state.cursor_pos, saved_pos);
}

#[test]
fn test_cursor_overwrite_same_position() {
    // Test that moving to same position twice updates state correctly
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let pos_val = vp_row(8) + vp_col(15);
    let op1 = RenderOpCommon::MoveCursorPositionAbs(pos_val);
    let _unused = execute_and_capture(op1, &mut state, &output_device, &stdout_mock);

    // Move to same position again
    let (output_device2, stdout_mock2) = create_mock_output();
    let op2 = RenderOpCommon::MoveCursorPositionAbs(pos_val);
    let output = execute_and_capture(op2, &mut state, &output_device2, &stdout_mock2);

    // Both should generate same ANSI sequence
    assert_eq!(
        output,
        ansi_output::cursor_movement::cursor_position(
            vp_row(8).into(),
            vp_col(15).into()
        )
    );
    assert_eq!(state.cursor_pos, pos_val);
}
