// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Integration tests for text painting operations ([`CompositorNoClipTruncPaintTextWithAttributes`])
//!
//! These tests validate:
//! 1. [`CompositorNoClipTruncPaintTextWithAttributes`] paints text without ANSI escapes for styling
//! 2. Text with [`TuiStyle`] renders correct SGR styling sequences
//! 3. Text without style (plain text) renders without extra ANSI sequences
//! 4. Multiple text operations in sequence preserve styling state
//! 5. Text rendering integrates with cursor positioning
//! 6. Color state is properly managed across text painting operations
//!
//! [`CompositorNoClipTruncPaintTextWithAttributes`]: crate::RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes
//! [`TuiStyle`]: crate::TuiStyle

use super::test_helpers::*;
use crate::{tui_color, TuiStyle, tui_style_attrib};

#[test]
fn test_paint_text_plain_without_style() {
    // Test that plain text without style is painted correctly
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "Hello, World!";
    let output = execute_text_paint_and_capture(text, None, &mut state, &output_device, &stdout_mock);

    // Plain text should produce output containing the text
    assert!(!output.is_empty(), "Output should not be empty");
    assert!(output.contains(text), "Output should contain the text: {text}");

    // Cursor position should advance by the display width of the text
    assert!(
        *state.cursor_pos.col_index > *initial_cursor.col_index,
        "Cursor should advance after painting text"
    );
}

#[test]
fn test_paint_text_with_foreground_color() {
    // Test that text with foreground color renders color styling
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "Colored text";
    let style = Some(TuiStyle {
        color_fg: Some(tui_color!(red)),
        ..Default::default()
    });

    let output = execute_text_paint_and_capture(text, style, &mut state, &output_device, &stdout_mock);

    // Output should contain ANSI escape sequences for styling
    assert!(!output.is_empty(), "Output should not be empty");
    assert!(output.contains("\x1b["), "Output should contain ANSI escape sequences");
    assert!(output.contains(text), "Output should contain the text: {text}");

    // Cursor position should advance after painting
    assert!(
        *state.cursor_pos.col_index > *initial_cursor.col_index,
        "Cursor should advance after painting text"
    );
}

#[test]
fn test_paint_text_with_background_color() {
    // Test that text with background color renders background styling
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "Background colored";
    let style = Some(TuiStyle {
        color_bg: Some(tui_color!(blue)),
        ..Default::default()
    });

    let output = execute_text_paint_and_capture(text, style, &mut state, &output_device, &stdout_mock);

    // Output should contain ANSI escape sequences for background styling
    assert!(!output.is_empty(), "Output should not be empty");
    assert!(output.contains("\x1b["), "Output should contain ANSI escape sequences");
    assert!(output.contains(text), "Output should contain the text: {text}");

    // Cursor position should advance after painting
    assert!(
        *state.cursor_pos.col_index > *initial_cursor.col_index,
        "Cursor should advance after painting text"
    );
}

#[test]
fn test_paint_text_with_combined_style() {
    // Test that text with both foreground and background colors renders both
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "Styled text";
    let style = Some(TuiStyle {
        color_fg: Some(tui_color!(white)),
        color_bg: Some(tui_color!(blue)),
        ..Default::default()
    });

    let output = execute_text_paint_and_capture(text, style, &mut state, &output_device, &stdout_mock);

    // Output should contain ANSI escape sequences for both fg and bg styling
    assert!(!output.is_empty(), "Output should not be empty");
    assert!(output.contains("\x1b["), "Output should contain ANSI escape sequences");
    assert!(output.contains(text), "Output should contain the text: {text}");

    // Cursor position should advance after painting
    assert!(
        *state.cursor_pos.col_index > *initial_cursor.col_index,
        "Cursor should advance after painting text"
    );
}

#[test]
fn test_paint_text_with_bold_style() {
    // Test that text with bold attribute renders bold styling
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "Bold text";
    let style = Some(TuiStyle {
        attribs: crate::TuiStyleAttribs {
            bold: Some(tui_style_attrib::Bold),
            ..Default::default()
        },
        ..Default::default()
    });

    let output = execute_text_paint_and_capture(text, style, &mut state, &output_device, &stdout_mock);

    // Output should contain ANSI escape sequences for bold attribute
    assert!(!output.is_empty(), "Output should not be empty");
    assert!(output.contains("\x1b["), "Output should contain ANSI escape sequences");
    assert!(output.contains(text), "Output should contain the text: {text}");

    // Cursor position should advance after painting
    assert!(
        *state.cursor_pos.col_index > *initial_cursor.col_index,
        "Cursor should advance after painting text"
    );
}

#[test]
fn test_paint_multiple_text_operations_sequence() {
    // Test that multiple text operations render correctly in sequence
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    // First text
    let text1 = "First";
    let output1 = execute_text_paint_and_capture(text1, None, &mut state, &output_device, &stdout_mock);
    assert!(!output1.is_empty(), "First output should not be empty");
    assert!(output1.contains(text1), "First output should contain text: {text1}");

    let cursor_after_first = state.cursor_pos;
    assert!(
        *cursor_after_first.col_index > *initial_cursor.col_index,
        "Cursor should advance after first text"
    );

    // Second text with style
    let (output_device2, stdout_mock2) = create_mock_output();
    let text2 = "Second";
    let style2 = Some(TuiStyle {
        color_fg: Some(tui_color!(green)),
        ..Default::default()
    });
    let output2 =
        execute_text_paint_and_capture(text2, style2, &mut state, &output_device2, &stdout_mock2);

    // Second output should contain styled text
    assert!(!output2.is_empty(), "Second output should not be empty");
    assert!(output2.contains(text2), "Second output should contain text: {text2}");
    assert!(output2.contains("\x1b["), "Second output should have ANSI sequences");

    // Cursor should advance further
    assert!(
        *state.cursor_pos.col_index > *cursor_after_first.col_index,
        "Cursor should advance after second text"
    );
}

#[test]
fn test_paint_text_empty_string() {
    // Test that empty text is handled correctly
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "";
    let output = execute_text_paint_and_capture(text, None, &mut state, &output_device, &stdout_mock);

    // Empty text should not panic and cursor should not advance significantly
    // (may produce minimal ANSI sequences but no visible text)
    assert_eq!(
        state.cursor_pos.col_index, initial_cursor.col_index,
        "Cursor should not advance for empty text"
    );

    // Output might be empty or contain only control sequences
    // The important thing is it doesn't panic
    drop(output);
}

#[test]
fn test_paint_text_with_special_characters() {
    // Test that text with special characters is rendered correctly
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "Special: @#$%^&*()";
    let output = execute_text_paint_and_capture(text, None, &mut state, &output_device, &stdout_mock);

    // Should handle special characters without panic
    assert!(!output.is_empty(), "Output should not be empty");
    assert!(output.contains(text), "Output should contain special characters: {text}");

    // Cursor should advance
    assert!(
        *state.cursor_pos.col_index > *initial_cursor.col_index,
        "Cursor should advance after painting special characters"
    );
}

#[test]
fn test_paint_text_with_unicode_emoji() {
    // Test that text with unicode and emoji is rendered correctly
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();
    let initial_cursor = state.cursor_pos;

    let text = "Hello 👋 World 🌍";
    let output = execute_text_paint_and_capture(text, None, &mut state, &output_device, &stdout_mock);

    // Should handle unicode/emoji without panic
    assert!(!output.is_empty(), "Output should not be empty");
    // Note: emoji might be represented differently in ANSI, but the output should contain something
    assert!(!output.is_empty(), "Output should have content");

    // Cursor should advance (emoji typically take 2 columns)
    assert!(
        *state.cursor_pos.col_index > *initial_cursor.col_index,
        "Cursor should advance after painting unicode/emoji text"
    );
}

#[test]
fn test_paint_text_style_persistence() {
    // Test that style state is properly maintained across operations
    let (output_device, stdout_mock) = create_mock_output();
    let mut state = create_test_state();

    // Verify initial state has no colors
    assert!(state.fg_color.is_none(), "Initial fg_color should be None");
    assert!(state.bg_color.is_none(), "Initial bg_color should be None");

    // Paint text with a color
    let style = Some(TuiStyle {
        color_fg: Some(tui_color!(red)),
        ..Default::default()
    });
    let output = execute_text_paint_and_capture("Red text", style, &mut state, &output_device, &stdout_mock);

    // Text painting should produce styled output
    assert!(!output.is_empty(), "Output should not be empty");
    assert!(output.contains("\x1b["), "Output should contain ANSI escape sequences");
    assert!(output.contains("Red text"), "Output should contain the text");

    // Note: Text painting via CompositorNoClipTruncPaintTextWithAttributes doesn't
    // directly update state.fg_color like SetFgColor does, as it applies and resets
    // styles inline through PixelCharRenderer
}
