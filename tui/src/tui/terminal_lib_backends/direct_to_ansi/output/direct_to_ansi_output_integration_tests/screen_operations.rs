// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for screen operations ([`ClearScreen`], [`ClearCurrentLine`],
//! etc.)
//!
//! These tests validate:
//! 1. [`ClearScreen`] generates correct [`CSI`] 2J sequence
//! 2. [`ClearCurrentLine`], [`ClearToEndOfLine`], [`ClearToStartOfLine`] operations
//! 3. State preservation across screen operations
//!
//! [`ClearCurrentLine`]: crate::render_op::RenderOpCommon::ClearCurrentLine
//! [`ClearScreen`]: crate::render_op::RenderOpCommon::ClearScreen
//! [`ClearToEndOfLine`]: crate::render_op::RenderOpCommon::ClearToEndOfLine
//! [`ClearToStartOfLine`]: crate::render_op::RenderOpCommon::ClearToStartOfLine
//! [`CSI`]: crate::CsiSequence

use super::test_helpers::*;
use crate::{col, pos, render_op::RenderOpCommon, row};
use crate::ansi_output::{cursor_save_restore, screen_clearing};

#[test]
fn test_clear_screen() {
    // Test that ClearScreen generates correct CSI 2J sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearScreen;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 2J clears entire screen
    assert_eq!(output, screen_clearing::clear_screen());
}

#[test]
fn test_clear_current_line() {
    // Test that ClearCurrentLine generates correct CSI 2K sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearCurrentLine;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 2K clears current line
    assert_eq!(
        output,
        screen_clearing::clear_current_line()
    );
}

#[test]
fn test_clear_to_end_of_line() {
    // Test that ClearToEndOfLine generates correct CSI 0K sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearToEndOfLine;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 0K clears to end of line
    assert_eq!(
        output,
        screen_clearing::clear_to_end_of_line()
    );
}

#[test]
fn test_clear_to_start_of_line() {
    // Test that ClearToStartOfLine generates correct CSI 1K sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearToStartOfLine;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 1K clears to start of line
    assert_eq!(
        output,
        screen_clearing::clear_to_start_of_line()
    );
}

#[test]
fn test_screen_operations_preserve_cursor_state() {
    // Test that screen operations don't affect cursor state
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // Set cursor position
    let move_op = RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(10)));
    let _unused = execute_and_capture(move_op, &mut state, &output_device, &stdout_mock);
    let saved_pos = state.cursor_pos;

    // Do a screen operation
    let (output_device2, stdout_mock2) = create_mock_output();
    let clear_op = RenderOpCommon::ClearScreen;
    let _unused =
        execute_and_capture(clear_op, &mut state, &output_device2, &stdout_mock2);

    // Cursor position should be unchanged
    assert_eq!(state.cursor_pos, saved_pos);
}

#[test]
fn test_save_and_restore_cursor_position() {
    // Test saving and restoring cursor position
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![
        RenderOpCommon::SaveCursorPosition,
        RenderOpCommon::RestoreCursorPosition,
    ];

    let output =
        execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Should contain both sequences (CSI format: ESC [ s and ESC [ u)
    assert!(
        output.contains(cursor_save_restore::save_cursor_position())
    );
    assert!(
        output.contains(
            cursor_save_restore::restore_cursor_position()
        )
    );
}
