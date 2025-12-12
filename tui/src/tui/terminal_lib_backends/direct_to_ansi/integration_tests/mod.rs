// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! **Output** integration tests for [`RenderOpPaintImplDirectToAnsi`] (using [`StdoutMock`]).
//!
//! These tests verify the **output painting pipeline** ([`RenderOpOutput`] → ANSI escape
//! sequences). They use [`StdoutMock`] to capture output—no real terminal or PTY is needed
//! because we're testing ANSI sequence generation, not terminal I/O.
//!
//! **Looking for input tests?** See the [PTY Tests for Input Handling] section below.
//!
//! # Testing Strategy
//!
//! This module tests the **output painting pipeline**:
//! - [`RenderOpOutput`] → ANSI escape sequences via [`RenderOpPaintImplDirectToAnsi`]
//! - State tracking in [`RenderOpsLocalData`]
//! - Output captured via [`StdoutMock`]
//!
//! ## PTY Tests for Input Handling
//!
//! End-to-end PTY tests for [`DirectToAnsiInputDevice`] (the input side of [`DirectToAnsi`])
//! are intentionally located in [`integration_tests`]. Those 8 PTY tests validate the
//! complete input parsing pipeline in real pseudo-terminals:
//!
//! | Test Module                        | What it validates                     |
//! |:-----------------------------------|:--------------------------------------|
//! | [`pty_input_device_test`]          | Basic async I/O and buffer management |
//! | [`pty_keyboard_modifiers_test`]    | Keyboard modifiers (Shift, Ctrl, Alt) |
//! | [`pty_mouse_events_test`]          | Mouse clicks, drags, scrolling        |
//! | [`pty_terminal_events_test`]       | Focus events, window resize           |
//! | [`pty_utf8_text_test`]             | UTF-8 text input handling             |
//! | [`pty_bracketed_paste_test`]       | Bracketed paste mode                  |
//! | [`pty_new_keyboard_features_test`] | Extended keyboard protocol            |
//! | [`pty_sigwinch_test`]              | SIGWINCH signal handling              |
//!
//! The PTY tests live with the parser because they primarily validate **parser correctness**
//! (raw bytes → [`InputEvent`]), even though they exercise [`DirectToAnsiInputDevice`].
//! See the [parser module's testing strategy] for the full rationale on validation vs.
//! generated sequences.
//!
//! # Module Organization
//!
//! Tests are organized by operation type and variant:
//!
//! **[`RenderOpOutput::Common`] Tests:**
//! - [`color_operations`]: Tests for [`SetFgColor`], [`SetBgColor`], [`ResetColor`]
//!   operations
//! - [`cursor_movement`]: Tests for [`MoveCursorPositionAbs`], [`MoveCursorPositionRelTo`]
//!   operations
//! - [`screen_operations`]: Tests for [`ClearScreen`], [`ShowCursor`], [`HideCursor`]
//!   operations
//! - [`state_optimization`]: Tests for redundant operation skipping and state persistence
//!
//! **[`RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes`] Tests:**
//! - [`text_operations`]: Tests for painted text with various styles (colors, attributes)
//!
//! # Implementation Notes
//!
//! Tests cover both [`RenderOpOutput::Common`] (for cursor/color/screen operations) and
//! [`RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes`] (for painted text).
//!
//! The test pattern:
//!
//! 1. Create mock [`OutputDevice`] with [`StdoutMock`] (captures ANSI output)
//! 2. Create test state ([`RenderOpsLocalData`] - tracks cursor and colors)
//! 3. Create operation (either [`RenderOpCommon`] or text paint with style)
//! 4. Wrap in appropriate [`RenderOpOutput`] variant
//! 5. Execute via [`RenderOpPaint`] trait on [`RenderOpPaintImplDirectToAnsi`]
//! 6. Verify BOTH:
//!    - ANSI output captured in [`StdoutMock`]
//!    - State changes in [`RenderOpsLocalData`] (optimization tracking)
//!
//! # Key Types Under Test
//!
//! **[`RenderOpOutput`] Variants:**
//! - [`RenderOpOutput::Common`]: Wraps [`RenderOpCommon`] for cursor/color/screen
//!   operations
//! - [`RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes`]: Paints styled text
//!
//! **Supporting Types:**
//! - [`RenderOpsLocalData`]: Tracks cursor position, `fg_color`, `bg_color` for
//!   optimization
//! - [`Pos`]: Position with `row_index` and `col_index` fields (0-based indices)
//! - [`RenderOpCommon`]: Enum variants for common operations ([`SetFgColor`], [`SetBgColor`],
//!   [`MoveCursorPositionAbs`], [`ClearScreen`], [`ShowCursor`], [`HideCursor`], etc.)
//! - [`TuiStyle`]: Styling information for text (foreground color, background color,
//!   attributes)
//! - [`StdoutMock`]: Captures ANSI output for verification
//! - [`OutputDeviceExt::new_mock()`]: Creates ([`OutputDevice`], [`StdoutMock`]) pair
//!
//! [`DirectToAnsi`]: crate::terminal_lib_backends::direct_to_ansi::DirectToAnsi
//! [`OutputDevice`]: crate::OutputDevice
//! [`StdoutMock`]: crate::StdoutMock
//! [`RenderOpsLocalData`]: crate::RenderOpsLocalData
//! [`Pos`]: crate::Pos
//! [`RenderOpCommon`]: crate::render_op::RenderOpCommon
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [`RenderOpPaint`]: crate::RenderOpPaint
//! [`RenderOpPaintImplDirectToAnsi`]: crate::terminal_lib_backends::direct_to_ansi::output::direct_to_ansi_paint_render_op_impl::RenderOpPaintImplDirectToAnsi
//! [`TuiStyle`]: crate::TuiStyle
//! [`OutputDeviceExt`]: crate::test_fixtures::output_device_fixtures::OutputDeviceExt
//! [`SetFgColor`]: crate::render_op::RenderOpCommon::SetFgColor
//! [`SetBgColor`]: crate::render_op::RenderOpCommon::SetBgColor
//! [`ResetColor`]: crate::render_op::RenderOpCommon::ResetColor
//! [`MoveCursorPositionAbs`]: crate::render_op::RenderOpCommon::MoveCursorPositionAbs
//! [`MoveCursorPositionRelTo`]: crate::render_op::RenderOpCommon::MoveCursorPositionRelTo
//! [`ClearScreen`]: crate::render_op::RenderOpCommon::ClearScreen
//! [`ShowCursor`]: crate::render_op::RenderOpCommon::ShowCursor
//! [`HideCursor`]: crate::render_op::RenderOpCommon::HideCursor
//! [`RenderOpOutput::Common`]: crate::RenderOpOutput::Common
//! [`RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes`]: crate::RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes
//! [parser module's testing strategy]: mod@crate::core::ansi::vt_100_terminal_input_parser#testing-strategy
//! [`integration_tests`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests
//! [`pty_input_device_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_input_device_test
//! [`pty_keyboard_modifiers_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_keyboard_modifiers_test
//! [`pty_mouse_events_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mouse_events_test
//! [`pty_terminal_events_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_terminal_events_test
//! [`pty_utf8_text_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_utf8_text_test
//! [`pty_bracketed_paste_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_bracketed_paste_test
//! [`pty_new_keyboard_features_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_new_keyboard_features_test
//! [`pty_sigwinch_test`]: mod@crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_sigwinch_test
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`InputEvent`]: crate::InputEvent
//! [PTY Tests for Input Handling]: #pty-tests-for-input-handling
//! [`color_operations`]: mod@crate::terminal_lib_backends::direct_to_ansi::integration_tests::color_operations
//! [`cursor_movement`]: mod@crate::terminal_lib_backends::direct_to_ansi::integration_tests::cursor_movement
//! [`screen_operations`]: mod@crate::terminal_lib_backends::direct_to_ansi::integration_tests::screen_operations
//! [`state_optimization`]: mod@crate::terminal_lib_backends::direct_to_ansi::integration_tests::state_optimization
//! [`text_operations`]: mod@crate::terminal_lib_backends::direct_to_ansi::integration_tests::text_operations

#[cfg(test)]
mod color_operations;

#[cfg(test)]
mod cursor_movement;

#[cfg(test)]
mod screen_operations;

#[cfg(test)]
mod state_optimization;

#[cfg(test)]
mod text_operations;

#[cfg(test)]
mod test_helpers {
    use crate::{LockedOutputDevice, OutputDevice, RenderOpOutput, RenderOpPaint,
                RenderOpsLocalData, Size, StdoutMock, TuiColor, col, height,
                lock_output_device_as_mut, pos, render_op::RenderOpCommon, row,
                terminal_lib_backends::direct_to_ansi::RenderOpPaintImplDirectToAnsi,
                test_fixtures::output_device_fixtures::OutputDeviceExt, width};

    /// Creates initial test state with default values
    pub fn create_test_state() -> RenderOpsLocalData {
        RenderOpsLocalData {
            cursor_pos: pos(row(0) + col(0)),
            fg_color: None,
            bg_color: None,
        }
    }

    /// Standard window size for tests
    pub fn test_window_size() -> Size { Size::new((width(80), height(24))) }

    /// Creates a mock output device for testing
    pub fn create_mock_output() -> (OutputDevice, StdoutMock) { OutputDevice::new_mock() }

    /// Helper to create a [`SetFgColor`] [`RenderOpCommon`] variant
    pub fn set_fg_color_op(color: TuiColor) -> RenderOpCommon {
        RenderOpCommon::SetFgColor(color)
    }

    /// Helper to create a [`SetBgColor`] [`RenderOpCommon`] variant
    pub fn set_bg_color_op(color: TuiColor) -> RenderOpCommon {
        RenderOpCommon::SetBgColor(color)
    }

    /// Executes a [`RenderOpCommon`] via the paint pipeline and returns the captured ANSI
    /// output
    pub fn execute_and_capture(
        op: RenderOpCommon,
        state: &mut RenderOpsLocalData,
        output_device: &OutputDevice,
        stdout_mock: &StdoutMock,
    ) -> String {
        let render_op = RenderOpOutput::Common(op);
        let window_size = test_window_size();
        let mut skip_flush = false;

        let mut painter = RenderOpPaintImplDirectToAnsi;

        {
            let mut_ref: LockedOutputDevice<'_> =
                lock_output_device_as_mut!(output_device);
            painter.paint(
                &mut skip_flush,
                &render_op,
                window_size,
                state,
                mut_ref,
                output_device.is_mock,
            );
        }

        stdout_mock.get_copy_of_buffer_as_string()
    }

    /// Executes multiple [`RenderOpCommon`] operations in sequence
    pub fn execute_sequence_and_capture(
        ops: Vec<RenderOpCommon>,
        state: &mut RenderOpsLocalData,
        output_device: &OutputDevice,
        stdout_mock: &StdoutMock,
    ) -> String {
        let window_size = test_window_size();
        let mut skip_flush = false;
        let mut painter = RenderOpPaintImplDirectToAnsi;

        for op in ops {
            let render_op = RenderOpOutput::Common(op);
            let mut_ref: LockedOutputDevice<'_> =
                lock_output_device_as_mut!(output_device);
            painter.paint(
                &mut skip_flush,
                &render_op,
                window_size,
                state,
                mut_ref,
                output_device.is_mock,
            );
        }

        stdout_mock.get_copy_of_buffer_as_string()
    }

    /// Executes a [`CompositorNoClipTruncPaintTextWithAttributes`] [`RenderOpOutput`] and
    /// returns captured ANSI output
    ///
    /// [`CompositorNoClipTruncPaintTextWithAttributes`]: crate::RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes
    pub fn execute_text_paint_and_capture(
        text: &str,
        style: Option<crate::TuiStyle>,
        state: &mut RenderOpsLocalData,
        output_device: &OutputDevice,
        stdout_mock: &StdoutMock,
    ) -> String {
        let window_size = test_window_size();
        let mut skip_flush = false;
        let mut painter = RenderOpPaintImplDirectToAnsi;
        let render_op = RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
            crate::InlineString::from(text),
            style,
        );

        {
            let mut_ref: LockedOutputDevice<'_> =
                lock_output_device_as_mut!(output_device);
            painter.paint(
                &mut skip_flush,
                &render_op,
                window_size,
                state,
                mut_ref,
                output_device.is_mock,
            );
        }

        stdout_mock.get_copy_of_buffer_as_string()
    }
}
