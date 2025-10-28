// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for color operations ([`SetFgColor`], [`SetBgColor`], [`ResetColor`])
//!
//! These tests validate:
//! 1. [`SetFgColor`] [`RenderOpCommon`] generates correct SGR foreground ANSI sequences via the full paint pipeline
//! 2. [`SetBgColor`] [`RenderOpCommon`] generates correct SGR background ANSI sequences
//! 3. Color state tracking in [`RenderOpsLocalData`] (`fg_color`, `bg_color` fields)
//! 4. [`ResetColor`] clears both foreground and background color state
//! 5. Multiple color operations in sequence preserve state correctly
//! 6. ANSI escape sequence format validation (colon-separated extended palette format)
//!
//! [`SetFgColor`]: crate::render_op::RenderOpCommon::SetFgColor
//! [`SetBgColor`]: crate::render_op::RenderOpCommon::SetBgColor
//! [`ResetColor`]: crate::render_op::RenderOpCommon::ResetColor
//! [`RenderOpCommon`]: crate::render_op::RenderOpCommon
//! [`RenderOpsLocalData`]: crate::RenderOpsLocalData

use super::test_helpers::*;
use crate::{tui_color, AnsiSequenceGenerator};
use crate::render_op::RenderOpCommon;

#[test]
fn test_set_fg_color_basic_red() {
    // Test that SetFgColor(red) generates correct ANSI foreground escape sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = tui_color!(red);

    let op = set_fg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 38:5:1 for ANSI red in extended palette format
    assert_eq!(output, AnsiSequenceGenerator::fg_color(color));
    // State should track the foreground color
    assert_eq!(state.fg_color, Some(color));
}

#[test]
fn test_set_fg_color_basic_blue() {
    // Test SetFgColor with blue
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = tui_color!(blue);

    let op = set_fg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 38:5:4 for ANSI blue
    assert_eq!(output, AnsiSequenceGenerator::fg_color(color));
    assert_eq!(state.fg_color, Some(color));
}

#[test]
fn test_set_fg_color_basic_green() {
    // Test SetFgColor with green
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = tui_color!(green);

    let op = set_fg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 38:5:2 for ANSI green
    assert_eq!(output, AnsiSequenceGenerator::fg_color(color));
    assert_eq!(state.fg_color, Some(color));
}

#[test]
fn test_set_bg_color_basic_red() {
    // Test that SetBgColor(red) generates correct ANSI background escape sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = tui_color!(red);

    let op = set_bg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 48:5:1 for ANSI red background
    assert_eq!(output, AnsiSequenceGenerator::bg_color(color));
    // State should track the background color
    assert_eq!(state.bg_color, Some(color));
}

#[test]
fn test_set_bg_color_basic_green() {
    // Test SetBgColor with green
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = tui_color!(green);

    let op = set_bg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 48:5:2 for ANSI green background
    assert_eq!(output, AnsiSequenceGenerator::bg_color(color));
    assert_eq!(state.bg_color, Some(color));
}

#[test]
fn test_set_bg_color_extended_palette_226() {
    // Test background color with extended palette index (yellow in 256-color palette)
    use crate::AnsiValue;
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = crate::TuiColor::Ansi(AnsiValue::new(226));

    let op = set_bg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 48:5:226 for background extended palette
    assert_eq!(output, AnsiSequenceGenerator::bg_color(color));
    assert_eq!(state.bg_color, Some(color));
}

#[test]
fn test_set_fg_color_rgb_orange() {
    // Test foreground color with RGB (24-bit truecolor)
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = tui_color!(255, 165, 0); // Orange

    let op = set_fg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 38:2:R:G:B format
    assert_eq!(output, AnsiSequenceGenerator::fg_color(color));
    assert_eq!(state.fg_color, Some(color));
}

#[test]
fn test_set_bg_color_rgb_cyan() {
    // Test background color with RGB
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let color = tui_color!(0, 255, 255); // Cyan

    let op = set_bg_color_op(color);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Should generate SGR 48:2:R:G:B format
    assert_eq!(output, AnsiSequenceGenerator::bg_color(color));
    assert_eq!(state.bg_color, Some(color));
}

#[test]
fn test_reset_color_clears_both_colors() {
    // Test that ResetColor clears both fg and bg colors from state
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // First set both colors
    let fg_op = set_fg_color_op(tui_color!(red));
    let bg_op = set_bg_color_op(tui_color!(blue));
    let _unused = execute_and_capture(fg_op, &mut state, &output_device, &stdout_mock);
    let _unused = execute_and_capture(bg_op, &mut state, &output_device, &stdout_mock);

    // Verify colors are set
    assert!(state.fg_color.is_some());
    assert!(state.bg_color.is_some());

    // Clear the output buffer before testing reset
    let (output_device2, stdout_mock2) = create_mock_output();

    // Now reset
    let reset_op = RenderOpCommon::ResetColor;
    let output = execute_and_capture(reset_op, &mut state, &output_device2, &stdout_mock2);

    // Should generate SGR 0 (reset all attributes)
    assert_eq!(output, AnsiSequenceGenerator::reset_color());
    // Both colors should be cleared
    assert!(state.fg_color.is_none());
    assert!(state.bg_color.is_none());
}

#[test]
fn test_multiple_color_changes_sequence() {
    // Test multiple color operations in sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let ops = vec![
        set_fg_color_op(tui_color!(red)),
        set_bg_color_op(tui_color!(blue)),
        set_fg_color_op(tui_color!(green)),
    ];

    let output = execute_sequence_and_capture(ops, &mut state, &output_device, &stdout_mock);

    // Should contain all three ANSI sequences
    assert!(output.contains(&AnsiSequenceGenerator::fg_color(tui_color!(red))));
    assert!(output.contains(&AnsiSequenceGenerator::bg_color(tui_color!(blue))));
    assert!(output.contains(&AnsiSequenceGenerator::fg_color(tui_color!(green))));

    // Final state should have green foreground and blue background
    assert_eq!(state.fg_color, Some(tui_color!(green)));
    assert_eq!(state.bg_color, Some(tui_color!(blue)));
}

#[test]
fn test_fg_color_overwrite() {
    // Test that setting new foreground color overwrites previous in state
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let red_op = set_fg_color_op(tui_color!(red));
    let _unused = execute_and_capture(red_op, &mut state, &output_device, &stdout_mock);
    assert_eq!(state.fg_color, Some(tui_color!(red)));

    // Create fresh output device for second operation
    let (output_device2, stdout_mock2) = create_mock_output();
    let blue_op = set_fg_color_op(tui_color!(blue));
    let output = execute_and_capture(blue_op, &mut state, &output_device2, &stdout_mock2);

    assert_eq!(output, AnsiSequenceGenerator::fg_color(tui_color!(blue)));
    assert_eq!(state.fg_color, Some(tui_color!(blue)));
}

#[test]
fn test_dark_color_variants() {
    // Test dark color variants (ANSI indices 8-15)
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let dark_red = tui_color!(dark_red);

    let op = set_fg_color_op(dark_red);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    // Dark red is ANSI index 9
    assert_eq!(output, AnsiSequenceGenerator::fg_color(dark_red));
    assert_eq!(state.fg_color, Some(dark_red));
}

#[test]
fn test_pure_black_color() {
    // Test pure black color (ANSI index 0)
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let black = tui_color!(black);

    let op = set_fg_color_op(black);
    let output = execute_and_capture(op, &mut state, &output_device, &stdout_mock);

    assert_eq!(output, AnsiSequenceGenerator::fg_color(black));
    assert_eq!(state.fg_color, Some(black));
}

#[test]
fn test_color_sequence_after_reset() {
    // Test that we can set colors again after reset clears them
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    let first_color = set_fg_color_op(tui_color!(red));
    let _unused = execute_and_capture(first_color, &mut state, &output_device, &stdout_mock);
    assert_eq!(state.fg_color, Some(tui_color!(red)));

    // Reset
    let (output_device2, stdout_mock2) = create_mock_output();
    let reset = RenderOpCommon::ResetColor;
    let _unused = execute_and_capture(reset, &mut state, &output_device2, &stdout_mock2);
    assert!(state.fg_color.is_none());

    // Set new color
    let (output_device3, stdout_mock3) = create_mock_output();
    let second_color = set_fg_color_op(tui_color!(blue));
    let output = execute_and_capture(second_color, &mut state, &output_device3, &stdout_mock3);

    // Should produce correct ANSI sequence
    assert_eq!(output, AnsiSequenceGenerator::fg_color(tui_color!(blue)));
    assert_eq!(state.fg_color, Some(tui_color!(blue)));
}
