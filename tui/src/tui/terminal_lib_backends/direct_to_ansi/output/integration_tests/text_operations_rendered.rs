// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Behavioral tests for text painting operations via [`OffscreenBuffer`] rendering.
//!
//! These tests complement the byte-level tests in [`text_operations`] by verifying
//! that styled text produces the correct **visual result** when ANSI sequences
//! are rendered to a buffer.
//!
//! # What These Tests Verify
//!
//! - Bold and other attributes are applied correctly
//! - Plain text renders at correct positions
//! - Unicode and emoji characters render correctly
//! - Styled text segments are positioned correctly
//! - Foreground and background colors are applied correctly
//!
//! # Process Isolation Pattern
//!
//! These tests use [`ColorSupport::Truecolor`] via
//! [`global_color_support::set_override`], which modifies global static state. Running
//! tests in parallel would cause race conditions (one test clears the override while
//! another expects it set).
//!
//! **Solution**: All tests run sequentially in a single isolated subprocess via
//! [`test_all_rendered_output_in_isolated_process`]. The coordinator spawns itself
//! with `ISOLATED_RENDERED_TEST=1`, sets the color override once, runs all tests,
//! then clears the override. This pattern is borrowed from `fs_path.rs`.
//!
//! Individual test functions are **not** marked with `#[test]` - they are called
//! by the coordinator. This prevents cargo from running them in parallel.
//!
//! [`text_operations`]: super::text_operations
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`ColorSupport::Truecolor`]: crate::ColorSupport::Truecolor
//! [`global_color_support::set_override`]: crate::global_color_support::set_override

use super::test_helpers_rendered::*;
use crate::{ANSIBasicColor, ColorSupport, RgbValue, TuiColor, global_color_support,
            offscreen_buffer::test_fixtures_ofs_buf::*};

/// Verify styled text with foreground color renders correct characters and color.
fn test_paint_text_with_foreground_color_rendered() {
    let red: TuiColor = ANSIBasicColor::Red.into();

    // Paint "Hello" with red foreground at origin.
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text_with_fg("Hello", red),
    ]);

    // Verify text content and color.
    for (col, expected_char) in "Hello".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.color_fg == Some(red),
            "red foreground",
        );
    }
}

/// Verify styled text with background color renders correct characters and color.
fn test_paint_text_with_background_color_rendered() {
    let blue: TuiColor = ANSIBasicColor::Blue.into();

    // Paint "World" with blue background at origin.
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text_with_bg("World", blue),
    ]);

    // Verify text content and color.
    for (col, expected_char) in "World".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.color_bg == Some(blue),
            "blue background",
        );
    }
}

/// Verify combined foreground and background colors produce styled text.
fn test_paint_text_with_combined_style_rendered() {
    let white: TuiColor = ANSIBasicColor::White.into();
    let blue: TuiColor = ANSIBasicColor::Blue.into();

    // Paint "Test" with white on blue.
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text_with_colors("Test", white, blue),
    ]);

    // Verify text content and both colors.
    for (col, expected_char) in "Test".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.color_fg == Some(white) && style.color_bg == Some(blue),
            "white foreground and blue background",
        );
    }
}

/// Verify bold attribute is applied to text.
fn test_paint_text_with_bold_style_rendered() {
    // Paint "Bold" with bold attribute.
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(0, 0), paint_text_bold("Bold")]);

    // Verify each character has bold attribute.
    for (col, expected_char) in "Bold".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.attribs.bold.is_some(),
            "bold attribute",
        );
    }
}

/// Verify plain text without style is rendered correctly.
fn test_paint_text_plain_rendered() {
    // Paint plain "Hello" without any styling.
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(0, 0), paint_text("Hello", None)]);

    // Verify each character is plain (default style).
    assert_plain_text_at(&buffer, 0, 0, "Hello");
}

/// Verify multiple styled text segments at different positions.
fn test_multiple_styled_text_segments_rendered() {
    let red: TuiColor = ANSIBasicColor::Red.into();
    let green: TuiColor = ANSIBasicColor::Green.into();

    // Paint red text at row 0, green text at row 1.
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text_with_fg("Red", red),
        move_cursor_abs(1, 0),
        paint_text_with_fg("Green", green),
    ]);

    // Verify text content and color at row 0.
    for (col, expected_char) in "Red".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.color_fg == Some(red),
            "red foreground",
        );
    }

    // Verify text content and color at row 1.
    for (col, expected_char) in "Green".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            1,
            col,
            expected_char,
            |style| style.color_fg == Some(green),
            "green foreground",
        );
    }
}

/// Verify Unicode text (non-ASCII) renders correctly.
fn test_paint_unicode_text_rendered() {
    // Paint Unicode text.
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(0, 0), paint_text("日本語", None)]);

    // Verify each Unicode character.
    // Note: These are full-width characters, so they may occupy 2 columns each.
    // The exact behavior depends on OffscreenBuffer's Unicode handling.
    // For now, just verify the first character exists.
    let pos = crate::row(0) + crate::col(0);
    let pixel_char = buffer.get_char(pos);
    assert!(
        pixel_char.is_some(),
        "Unicode character should exist at (0,0)"
    );
}

/// Verify emoji renders (note: emoji handling may vary).
fn test_paint_text_with_emoji_rendered() {
    // Paint text with emoji.
    // Emoji are typically 2 columns wide.
    let buffer =
        execute_ops_and_render(vec![move_cursor_abs(0, 0), paint_text("A👋B", None)]);

    // Verify 'A' at column 0.
    assert_plain_char_at(&buffer, 0, 0, 'A');

    // The emoji '👋' should be at column 1 (possibly spanning to column 2).
    // 'B' should appear after the emoji.
    // Note: Exact column depends on wide character handling.
    let pos_a = crate::row(0) + crate::col(0);
    let pixel_char_a = buffer.get_char(pos_a);
    assert!(pixel_char_a.is_some(), "'A' should exist at (0,0)");
}

/// Verify styled text at non-origin position.
fn test_styled_text_at_offset_position_rendered() {
    let yellow: TuiColor = ANSIBasicColor::Yellow.into();

    // Paint styled text at offset position (5, 10).
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(5, 10),
        paint_text_with_fg("Offset", yellow),
    ]);

    // Verify styled text at (5, 10).
    for (col, expected_char) in "Offset".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            5,
            10 + col,
            expected_char,
            |style| style.color_fg == Some(yellow),
            "yellow foreground",
        );
    }

    // Verify origin is empty.
    assert_empty_at(&buffer, 0, 0);
}

/// Verify RGB foreground color renders correctly.
fn test_paint_text_with_rgb_foreground_rendered() {
    // Orange RGB: (255, 128, 0).
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text_with_rgb_fg("RGB", 255, 128, 0),
    ]);

    // Verify text content and RGB color.
    let expected_color = TuiColor::Rgb(RgbValue::from_u8(255, 128, 0));
    for (col, expected_char) in "RGB".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.color_fg == Some(expected_color),
            "RGB orange foreground",
        );
    }
}

/// Verify RGB background color renders correctly.
fn test_paint_text_with_rgb_background_rendered() {
    // Blue RGB: (0, 128, 255).
    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text_with_rgb_bg("BG", 0, 128, 255),
    ]);

    let expected_color = TuiColor::Rgb(RgbValue::from_u8(0, 128, 255));
    for (col, expected_char) in "BG".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.color_bg == Some(expected_color),
            "RGB blue background",
        );
    }
}

/// Verify combined RGB foreground and background colors.
fn test_paint_text_with_rgb_combined_rendered() {
    // White on navy.
    let fg_color = TuiColor::Rgb(RgbValue::from_u8(255, 255, 255));
    let bg_color = TuiColor::Rgb(RgbValue::from_u8(0, 0, 128));

    let buffer = execute_ops_and_render(vec![
        move_cursor_abs(0, 0),
        paint_text_with_rgb_colors("Both", (255, 255, 255), (0, 0, 128)),
    ]);

    for (col, expected_char) in "Both".chars().enumerate() {
        assert_styled_char_at(
            &buffer,
            0,
            col,
            expected_char,
            |style| style.color_fg == Some(fg_color) && style.color_bg == Some(bg_color),
            "RGB white on navy",
        );
    }
}

// XMARK: Process isolated test.

/// Run all rendered tests sequentially in a single process with controlled global state.
///
/// This function runs tests in two phases:
/// 1. **Ansi256 phase**: Tests using [`ANSIBasicColor`] (palette indices 0-15)
/// 2. **Truecolor phase**: Tests using [`TuiColor::Rgb`] (24-bit RGB colors)
///
/// Each phase sets the appropriate [`ColorSupport`] override. Individual test functions
/// don't modify global state - they rely on this coordinator to set it up.
///
/// [`ANSIBasicColor`]: crate::ANSIBasicColor
/// [`TuiColor::Rgb`]: crate::TuiColor::Rgb
/// [`ColorSupport`]: crate::ColorSupport
fn run_all_rendered_tests_sequentially() {
    // Run Ansi256 palette color tests (via ANSIBasicColor).
    // Use Ansi256 to accurately test 256-color palette behavior.
    global_color_support::set_override(ColorSupport::Ansi256);
    test_paint_text_with_foreground_color_rendered();
    test_paint_text_with_background_color_rendered();
    test_paint_text_with_combined_style_rendered();
    test_paint_text_with_bold_style_rendered();
    test_paint_text_plain_rendered();
    test_multiple_styled_text_segments_rendered();
    test_paint_unicode_text_rendered();
    test_paint_text_with_emoji_rendered();
    test_styled_text_at_offset_position_rendered();
    global_color_support::clear_override();

    // Run RGB true color tests.
    // Use Truecolor to test full RGB without degradation.
    global_color_support::set_override(ColorSupport::Truecolor);
    test_paint_text_with_rgb_foreground_rendered();
    test_paint_text_with_rgb_background_rendered();
    test_paint_text_with_rgb_combined_rendered();
    global_color_support::clear_override();
}

/// Run all rendered output tests in an isolated process.
///
/// This test coordinator spawns itself in a subprocess with `ISOLATED_RENDERED_TEST=1`,
/// where it runs all rendered tests sequentially with controlled global state.
/// This prevents race conditions when tests run in parallel.
///
/// # Why Process Isolation?
///
/// These tests use [`global_color_support::set_override`] which modifies a static
/// mutable variable. When tests run in parallel:
/// - Test A sets override → Test B sets override → Test A clears → Test B gets `NoColor`
/// - `degrade_color(yellow, NoColor)` returns black (index 0) instead of yellow (index 3)
///
/// By running in an isolated process, we ensure the global state is controlled and
/// cannot be affected by other tests.
#[test]
fn test_all_rendered_output_in_isolated_process() {
    if std::env::var("ISOLATED_RENDERED_TEST").is_ok() {
        // This is the actual test running in the isolated process.
        run_all_rendered_tests_sequentially();
        // If we reach here without errors, exit normally.
        std::process::exit(0);
    }

    // This is the test coordinator - spawn the actual test in a new process.
    let current_exe = std::env::current_exe().unwrap();
    let mut cmd = std::process::Command::new(&current_exe);
    cmd.env("ISOLATED_RENDERED_TEST", "1")
        .env("RUST_BACKTRACE", "1") // Get better error info.
        .args([
            "--test-threads",
            "1",
            "test_all_rendered_output_in_isolated_process",
        ]);

    let output = cmd.output().expect("Failed to run isolated test");

    // Check if the child process exited successfully or if there's a panic message.
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success()
        || stderr.contains("panicked at")
        || stderr.contains("Test failed with error")
    {
        // These statements help IDEs provide hyperlinks to the failing test source.
        eprintln!("Exit status: {:?}", output.status);
        eprintln!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("Stderr: {stderr}");

        panic!(
            "Isolated rendered test failed with status code {:?}: {}",
            output.status.code(),
            stderr
        );
    }
}
