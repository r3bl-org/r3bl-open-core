// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for screen operations ([`ClearScreen`], [`ShowCursor`],
//! [`HideCursor`], etc.)
//!
//! These tests validate:
//! 1. [`ClearScreen`] generates correct CSI 2J sequence
//! 2. [`ShowCursor`] generates DECTCEM set (show) sequence
//! 3. [`HideCursor`] generates DECTCEM reset (hide) sequence
//! 4. [`EnterAlternateScreen`] / [`ExitAlternateScreen`] for full-screen apps
//! 5. [`ClearCurrentLine`], [`ClearToEndOfLine`], [`ClearToStartOfLine`] operations
//! 6. State preservation across screen operations
//!
//! [`ClearScreen`]: crate::render_op::RenderOpCommon::ClearScreen
//! [`ShowCursor`]: crate::render_op::RenderOpCommon::ShowCursor
//! [`HideCursor`]: crate::render_op::RenderOpCommon::HideCursor
//! [`EnterAlternateScreen`]: crate::render_op::RenderOpCommon::EnterAlternateScreen
//! [`ExitAlternateScreen`]: crate::render_op::RenderOpCommon::ExitAlternateScreen
//! [`ClearCurrentLine`]: crate::render_op::RenderOpCommon::ClearCurrentLine
//! [`ClearToEndOfLine`]: crate::render_op::RenderOpCommon::ClearToEndOfLine
//! [`ClearToStartOfLine`]: crate::render_op::RenderOpCommon::ClearToStartOfLine

use super::test_helpers::*;
use crate::{AnsiSequenceGenerator, col, pos, render_op::RenderOpCommon, row};

#[test]
fn test_clear_screen() {
    // Test that ClearScreen generates correct CSI 2J sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearScreen;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 2J clears entire screen
    assert_eq!(output, AnsiSequenceGenerator::clear_screen());
}

#[test]
fn test_show_cursor() {
    // Test that ShowCursor generates correct DECTCEM set sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ShowCursor;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI ?25h sets DECTCEM (show cursor)
    assert_eq!(output, AnsiSequenceGenerator::show_cursor());
}

#[test]
fn test_hide_cursor() {
    // Test that HideCursor generates correct DECTCEM reset sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::HideCursor;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI ?25l resets DECTCEM (hide cursor)
    assert_eq!(output, AnsiSequenceGenerator::hide_cursor());
}

#[test]
fn test_clear_current_line() {
    // Test that ClearCurrentLine generates correct CSI 2K sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearCurrentLine;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 2K clears current line
    assert_eq!(output, AnsiSequenceGenerator::clear_current_line());
}

#[test]
fn test_clear_to_end_of_line() {
    // Test that ClearToEndOfLine generates correct CSI 0K sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearToEndOfLine;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 0K clears to end of line
    assert_eq!(output, AnsiSequenceGenerator::clear_to_end_of_line());
}

#[test]
fn test_clear_to_start_of_line() {
    // Test that ClearToStartOfLine generates correct CSI 1K sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ClearToStartOfLine;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI 1K clears to start of line
    assert_eq!(output, AnsiSequenceGenerator::clear_to_start_of_line());
}

#[test]
fn test_enter_alternate_screen() {
    // Test that EnterAlternateScreen generates correct sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::EnterAlternateScreen;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI ?1049h enters alternate screen
    assert_eq!(output, AnsiSequenceGenerator::enter_alternate_screen());
}

#[test]
fn test_exit_alternate_screen() {
    // Test that ExitAlternateScreen generates correct sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let op = RenderOpCommon::ExitAlternateScreen;
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // CSI ?1049l exits alternate screen
    assert_eq!(output, AnsiSequenceGenerator::exit_alternate_screen());
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
fn test_hide_and_show_cursor_sequence() {
    // Test hiding and then showing cursor
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![RenderOpCommon::HideCursor, RenderOpCommon::ShowCursor];

    let output =
        execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Should contain both sequences
    assert!(output.contains(&AnsiSequenceGenerator::hide_cursor()));
    assert!(output.contains(&AnsiSequenceGenerator::show_cursor()));
}

#[test]
fn test_enter_and_exit_alternate_screen_sequence() {
    // Test entering and exiting alternate screen
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![
        RenderOpCommon::EnterAlternateScreen,
        RenderOpCommon::ExitAlternateScreen,
    ];

    let output =
        execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Should contain both sequences
    assert!(output.contains(&AnsiSequenceGenerator::enter_alternate_screen()));
    assert!(output.contains(&AnsiSequenceGenerator::exit_alternate_screen()));
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
    assert!(output.contains(&AnsiSequenceGenerator::save_cursor_position()));
    assert!(output.contains(&AnsiSequenceGenerator::restore_cursor_position()));
}
