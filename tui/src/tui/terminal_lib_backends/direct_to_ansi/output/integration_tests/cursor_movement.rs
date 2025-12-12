// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for cursor movement operations
//!
//! These tests validate:
//! 1. [`MoveCursorPositionAbs`] generates correct CUP (Cursor Position) ANSI sequences
//! 2. [`MoveCursorPositionRelTo`] correctly adds origin + relative offset
//! 3. Cursor state tracking in [`RenderOpsLocalData`] after movement
//! 4. [`MoveCursorToColumn`], [`MoveCursorToNextLine`], [`MoveCursorToPreviousLine`]
//!    operations
//! 5. Multiple cursor moves in sequence preserve correct final position
//! 6. Cursor position state matches ANSI output
//!
//! [`MoveCursorPositionAbs`]: crate::render_op::RenderOpCommon::MoveCursorPositionAbs
//! [`MoveCursorPositionRelTo`]: crate::render_op::RenderOpCommon::MoveCursorPositionRelTo
//! [`MoveCursorToColumn`]: crate::render_op::RenderOpCommon::MoveCursorToColumn
//! [`MoveCursorToNextLine`]: crate::render_op::RenderOpCommon::MoveCursorToNextLine
//! [`MoveCursorToPreviousLine`]: crate::render_op::RenderOpCommon::MoveCursorToPreviousLine
//! [`RenderOpsLocalData`]: crate::RenderOpsLocalData

use super::test_helpers::*;
use crate::{AnsiSequenceGenerator, ColIndex, RowIndex, col, height, pos,
            render_op::RenderOpCommon, row, term_row_delta, tui_color};

#[test]
fn test_move_cursor_absolute_origin() {
    // Test moving cursor to origin (0,0)
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::MoveCursorPositionAbs(pos(row(0) + col(0)));
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI H with 1-based indexing: row 0 (0-based) = 1 (1-based), col 0 = 1
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_position(row(0), col(0))
    );
    assert_eq!(state.cursor_pos, pos(row(0) + col(0)));
}

#[test]
fn test_move_cursor_absolute_5_10() {
    // Test moving cursor to (5, 10) in 0-based indices
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(10)));
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI H with 1-based: row 5 (0-based) = 6 (1-based), col 10 = 11
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_position(row(5), col(10))
    );
    assert_eq!(state.cursor_pos, pos(row(5) + col(10)));
}

#[test]
fn test_move_cursor_absolute_20_40() {
    // Test moving cursor to further position
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::MoveCursorPositionAbs(pos(row(20) + col(40)));
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // row 20 = 21, col 40 = 41 in 1-based
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_position(row(20), col(40))
    );
    assert_eq!(state.cursor_pos, pos(row(20) + col(40)));
}

#[test]
fn test_move_cursor_relative_to() {
    // Test MoveCursorPositionRelTo which adds origin + relative
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let origin = pos(row(5) + col(3));
    let relative = pos(row(2) + col(7));
    let op = RenderOpCommon::MoveCursorPositionRelTo(origin, relative);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Final position: row(5+2) + col(3+7) = row(7) + col(10)
    // ANSI: row 7 = 8, col 10 = 11
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_position(row(7), col(10))
    );
    assert_eq!(state.cursor_pos, pos(row(7) + col(10)));
}

#[test]
fn test_move_cursor_to_column() {
    // Test MoveCursorToColumn which moves to a column in current row
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // First move to a specific position
    let move_abs = RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(5)));
    let _unused = execute_and_capture(move_abs, &mut state, &output_device, &stdout_mock);
    let initial_row = state.cursor_pos.row_index;

    // Now move to column 20 (should keep same row)
    let (output_device2, stdout_mock2) = create_mock_output();
    let op = RenderOpCommon::MoveCursorToColumn(ColIndex::new(20));
    let output = execute_and_capture(op, &mut state, &output_device2, &stdout_mock2);

    // CSI 21G (1-based column index)
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_to_column(ColIndex::new(20))
    );
    // Row should remain unchanged
    assert_eq!(state.cursor_pos.row_index, initial_row);
    // Column should be updated
    assert_eq!(state.cursor_pos.col_index, ColIndex::new(20));
}

#[test]
fn test_move_cursor_to_next_line() {
    // Test MoveCursorToNextLine which moves down N lines and to column 0
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // First position cursor at (5, 10)
    let move_abs = RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(10)));
    let _unused = execute_and_capture(move_abs, &mut state, &output_device, &stdout_mock);

    // Move down 3 lines (to row 8, column 0)
    let (output_device2, stdout_mock2) = create_mock_output();
    let op = RenderOpCommon::MoveCursorToNextLine(height(3));
    let output = execute_and_capture(op, &mut state, &output_device2, &stdout_mock2);

    // CSI 3E (move down 3 lines and to column 0)
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_next_line(term_row_delta(3).unwrap())
    );
    assert_eq!(state.cursor_pos.row_index, RowIndex::new(8));
    assert_eq!(state.cursor_pos.col_index, ColIndex::new(0));
}

#[test]
fn test_move_cursor_to_previous_line() {
    // Test MoveCursorToPreviousLine which moves up N lines and to column 0
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // First position cursor at (10, 15)
    let move_abs = RenderOpCommon::MoveCursorPositionAbs(pos(row(10) + col(15)));
    let _unused = execute_and_capture(move_abs, &mut state, &output_device, &stdout_mock);

    // Move up 3 lines (to row 7, column 0)
    let (output_device2, stdout_mock2) = create_mock_output();
    let op = RenderOpCommon::MoveCursorToPreviousLine(height(3));
    let output = execute_and_capture(op, &mut state, &output_device2, &stdout_mock2);

    // CSI 3F (move up 3 lines and to column 0)
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_previous_line(term_row_delta(3).unwrap())
    );
    assert_eq!(state.cursor_pos.row_index, RowIndex::new(7));
    assert_eq!(state.cursor_pos.col_index, ColIndex::new(0));
}

#[test]
fn test_multiple_cursor_moves_sequence() {
    // Test multiple cursor movements in sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![
        RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(5))),
        RenderOpCommon::MoveCursorPositionAbs(pos(row(10) + col(20))),
        RenderOpCommon::MoveCursorPositionAbs(pos(row(0) + col(0))),
    ];

    let output =
        execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Should contain all three ANSI sequences
    assert!(output.contains(&AnsiSequenceGenerator::cursor_position(row(5), col(5))));
    assert!(output.contains(&AnsiSequenceGenerator::cursor_position(row(10), col(20))));
    assert!(output.contains(&AnsiSequenceGenerator::cursor_position(row(0), col(0))));

    // Final state should match last position
    assert_eq!(state.cursor_pos, pos(row(0) + col(0)));
}

#[test]
fn test_cursor_state_persists_across_operations() {
    // Test that cursor state persists after other operations
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // Set cursor position
    let move_op = RenderOpCommon::MoveCursorPositionAbs(pos(row(7) + col(12)));
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

    let pos_val = pos(row(8) + col(15));
    let op1 = RenderOpCommon::MoveCursorPositionAbs(pos_val);
    let _unused = execute_and_capture(op1, &mut state, &output_device, &stdout_mock);

    // Move to same position again
    let (output_device2, stdout_mock2) = create_mock_output();
    let op2 = RenderOpCommon::MoveCursorPositionAbs(pos_val);
    let output = execute_and_capture(op2, &mut state, &output_device2, &stdout_mock2);

    // Both should generate same ANSI sequence
    assert_eq!(
        output,
        AnsiSequenceGenerator::cursor_position(row(8), col(15))
    );
    assert_eq!(state.cursor_pos, pos_val);
}
