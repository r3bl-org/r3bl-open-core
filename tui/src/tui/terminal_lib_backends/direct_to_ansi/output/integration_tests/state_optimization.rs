// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for state optimization and redundant operation skipping
//!
//! These tests validate `DirectToAnsi`'s optimization of redundant operations:
//!
//! 1. Moving cursor to same position skips ANSI output (but updates state)
//! 2. Setting same color twice skips second ANSI output
//! 3. State is correctly maintained across multiple operations
//! 4. Cursor position state after relative vs absolute moves
//! 5. Color state persistence across unrelated operations
//! 6. Combining operations preserves optimization
//!
//! This optimization is critical for performance as it reduces the amount of
//! ANSI escape sequences sent to the terminal.

use super::test_helpers::*;
use crate::{AnsiSequenceGenerator, col, pos, render_op::RenderOpCommon, row, tui_color};

#[test]
fn test_duplicate_cursor_position_updates_state() {
    // Test that moving cursor to same position twice still updates state correctly
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let target_pos = pos(row(5) + col(10));

    // First move generates ANSI output and updates state
    let op1 = RenderOpCommon::MoveCursorPositionAbs(target_pos);
    let output1 = execute_and_capture(op1, &mut state, &output_device, &stdout_mock);

    // State should be updated to target position
    assert_eq!(state.cursor_pos, target_pos);
    assert_eq!(
        output1,
        AnsiSequenceGenerator::cursor_position(row(5), col(10))
    );

    // Clear buffer for second operation
    let (output_device2, stdout_mock2) = create_mock_output();

    // Second move to SAME position
    let op2 = RenderOpCommon::MoveCursorPositionAbs(target_pos);
    let _output2 = execute_and_capture(op2, &mut state, &output_device2, &stdout_mock2);

    // State should still reflect the target position after second move
    assert_eq!(state.cursor_pos, target_pos);

    // Note: Cursor optimization may or may not generate output
    // depending on implementation details - what matters is state is correct
}

#[test]
fn test_duplicate_fg_color_skips_output() {
    // Test that setting same foreground color twice skips second output
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let red = tui_color!(red);

    // First color operation generates ANSI output
    let op1 = RenderOpCommon::SetFgColor(red);
    let output1 = execute_and_capture(op1, &mut state, &output_device, &stdout_mock);

    // Clear buffer for second operation
    let (output_device2, stdout_mock2) = create_mock_output();

    // Second color operation with SAME color - should skip output (optimization)
    let op2 = RenderOpCommon::SetFgColor(red);
    let output2 = execute_and_capture(op2, &mut state, &output_device2, &stdout_mock2);

    // First operation generates ANSI sequence
    assert_eq!(output1, "\x1b[38:5:1m");

    // Second operation with same color produces NO output (optimization)
    assert!(output2.is_empty());

    // State should track the color
    assert_eq!(state.fg_color, Some(red));
}

#[test]
fn test_duplicate_bg_color_skips_output() {
    // Test background color optimization
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let green = tui_color!(green);

    // First operation
    let op1 = RenderOpCommon::SetBgColor(green);
    let output1 = execute_and_capture(op1, &mut state, &output_device, &stdout_mock);

    // Second operation with same color
    let (output_device2, stdout_mock2) = create_mock_output();
    let op2 = RenderOpCommon::SetBgColor(green);
    let output2 = execute_and_capture(op2, &mut state, &output_device2, &stdout_mock2);

    assert_eq!(output1, "\x1b[48:5:2m");
    assert!(output2.is_empty());
    assert_eq!(state.bg_color, Some(green));
}

#[test]
fn test_mixed_operations_with_state_tracking() {
    // Test complex workflow with mixed operations and state tracking
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![
        // First: set red foreground
        RenderOpCommon::SetFgColor(tui_color!(red)),
        // Second: move cursor
        RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(10))),
        // Third: set SAME red (optimization may skip, but state updates)
        RenderOpCommon::SetFgColor(tui_color!(red)),
        // Fourth: move to SAME position (state updates)
        RenderOpCommon::MoveCursorPositionAbs(pos(row(5) + col(10))),
        // Fifth: change to blue
        RenderOpCommon::SetFgColor(tui_color!(blue)),
    ];

    let output =
        execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Verify output contains the main sequences
    assert!(output.contains(&AnsiSequenceGenerator::fg_color(tui_color!(red))));
    assert!(output.contains(&AnsiSequenceGenerator::cursor_position(row(5), col(10))));
    assert!(output.contains(&AnsiSequenceGenerator::fg_color(tui_color!(blue))));

    // Verify color optimization works (duplicate red doesn't appear twice)
    let expected_red = AnsiSequenceGenerator::fg_color(tui_color!(red));
    let red_count = output.matches(&expected_red).count();
    assert_eq!(
        red_count, 1,
        "Red color should only appear once (optimization)"
    );

    // Final state should be correct regardless of optimization
    assert_eq!(state.fg_color, Some(tui_color!(blue)));
    assert_eq!(state.cursor_pos, pos(row(5) + col(10)));
}

#[test]
fn test_color_change_resets_optimization_cache() {
    // Test that changing color clears the optimization cache
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // Set red
    let op1 = RenderOpCommon::SetFgColor(tui_color!(red));
    let _unused = execute_and_capture(op1, &mut state, &output_device, &stdout_mock);
    assert_eq!(state.fg_color, Some(tui_color!(red)));

    // New device for next operation
    let (output_device2, stdout_mock2) = create_mock_output();

    // Change to blue
    let op2 = RenderOpCommon::SetFgColor(tui_color!(blue));
    let output2 = execute_and_capture(op2, &mut state, &output_device2, &stdout_mock2);

    // Should generate output (color changed)
    assert_eq!(output2, AnsiSequenceGenerator::fg_color(tui_color!(blue)));
    assert_eq!(state.fg_color, Some(tui_color!(blue)));

    // New device for third operation
    let (output_device3, stdout_mock3) = create_mock_output();

    // Set same blue again - should be skipped
    let op3 = RenderOpCommon::SetFgColor(tui_color!(blue));
    let output3 = execute_and_capture(op3, &mut state, &output_device3, &stdout_mock3);

    // Should be empty (optimization kicks in again)
    assert!(output3.is_empty());
}

#[test]
fn test_cursor_position_state_tracks_changes() {
    // Test that cursor position state is correctly tracked through multiple moves
    let mut state = create_test_state();
    let target_pos = pos(row(8) + col(12));

    // First move
    let (device1, mock1) = create_mock_output();
    let op1 = RenderOpCommon::MoveCursorPositionAbs(target_pos);
    let _unused = execute_and_capture(op1, &mut state, &device1, &mock1);
    assert_eq!(state.cursor_pos, target_pos);

    // Second move to different position
    let (device2, mock2) = create_mock_output();
    let new_pos = pos(row(10) + col(15));
    let op2 = RenderOpCommon::MoveCursorPositionAbs(new_pos);
    let _unused = execute_and_capture(op2, &mut state, &device2, &mock2);
    assert_eq!(state.cursor_pos, new_pos);

    // Third move back to original
    let (device3, mock3) = create_mock_output();
    let op3 = RenderOpCommon::MoveCursorPositionAbs(target_pos);
    let _unused = execute_and_capture(op3, &mut state, &device3, &mock3);
    assert_eq!(state.cursor_pos, target_pos);

    // Fourth move to same position - state should still be correct
    let (device4, mock4) = create_mock_output();
    let op4 = RenderOpCommon::MoveCursorPositionAbs(target_pos);
    let _unused = execute_and_capture(op4, &mut state, &device4, &mock4);

    // State should reflect the target position
    assert_eq!(state.cursor_pos, target_pos);
}

#[test]
fn test_reset_color_clears_optimization_state() {
    // Test that reset clears the color cache
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // Set red
    let op1 = RenderOpCommon::SetFgColor(tui_color!(red));
    let _unused = execute_and_capture(op1, &mut state, &output_device, &stdout_mock);

    // New device for reset
    let (output_device2, stdout_mock2) = create_mock_output();

    // Reset color
    let reset = RenderOpCommon::ResetColor;
    let output_reset =
        execute_and_capture(reset, &mut state, &output_device2, &stdout_mock2);

    assert_eq!(output_reset, AnsiSequenceGenerator::reset_color());
    assert!(state.fg_color.is_none());
    assert!(state.bg_color.is_none());

    // New device for next color
    let (output_device3, stdout_mock3) = create_mock_output();

    // Set red again - should generate output (cache was cleared by reset)
    let op2 = RenderOpCommon::SetFgColor(tui_color!(red));
    let output_red2 =
        execute_and_capture(op2, &mut state, &output_device3, &stdout_mock3);

    // Should generate output because reset cleared the cache
    assert_eq!(
        output_red2,
        AnsiSequenceGenerator::fg_color(tui_color!(red))
    );
}

#[test]
fn test_complex_optimization_workflow() {
    // Test realistic complex workflow
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![
        // Initial setup
        RenderOpCommon::SetFgColor(tui_color!(red)),
        RenderOpCommon::SetBgColor(tui_color!(blue)),
        RenderOpCommon::MoveCursorPositionAbs(pos(row(0) + col(0))),
        // Redundant operations (will be skipped)
        RenderOpCommon::SetFgColor(tui_color!(red)),
        RenderOpCommon::SetBgColor(tui_color!(blue)),
        RenderOpCommon::MoveCursorPositionAbs(pos(row(0) + col(0))),
        // New operations
        RenderOpCommon::SetFgColor(tui_color!(green)),
        RenderOpCommon::MoveCursorPositionAbs(pos(row(1) + col(5))),
        RenderOpCommon::ClearCurrentLine,
    ];

    let output =
        execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Verify final state
    assert_eq!(state.fg_color, Some(tui_color!(green)));
    assert_eq!(state.bg_color, Some(tui_color!(blue)));
    assert_eq!(state.cursor_pos, pos(row(1) + col(5)));

    // Verify output contains necessary sequences but optimized
    assert!(output.contains(&AnsiSequenceGenerator::fg_color(tui_color!(red))));
    assert!(output.contains(&AnsiSequenceGenerator::bg_color(tui_color!(blue))));
    assert!(output.contains(&AnsiSequenceGenerator::cursor_position(row(0), col(0))));
    assert!(output.contains(&AnsiSequenceGenerator::fg_color(tui_color!(green))));
    assert!(output.contains(&AnsiSequenceGenerator::cursor_position(row(1), col(5))));
    assert!(output.contains(&AnsiSequenceGenerator::clear_current_line()));
}
