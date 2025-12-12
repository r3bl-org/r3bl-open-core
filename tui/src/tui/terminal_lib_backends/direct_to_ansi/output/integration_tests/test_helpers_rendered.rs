// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test helpers for rendered output integration tests.
//!
//! These helpers execute render operations via [`StdoutMock`], capture the ANSI output,
//! then apply those bytes to an [`OffscreenBuffer`] for behavioral verification.
//!
//! # Testing Pattern
//!
//! ```text
//! RenderOpOutput → RenderOpPaintImplDirectToAnsi → StdoutMock (ANSI bytes)
//!                                                       ↓
//!                                    OffscreenBuffer::apply_ansi_bytes()
//!                                                       ↓
//!                                    Assert buffer state (chars, colors, positions)
//! ```
//!
//! # Color Support
//!
//! These tests require [`ColorSupport::Truecolor`] to be set via
//! [`global_color_support::set_override`] before execution. This is handled by the
//! process-isolated test coordinator in `text_operations_rendered.rs`, which sets
//! the override once for all tests running in the isolated subprocess.
//!
//! **Important**: Do not call `set_override`/`clear_override` in individual test
//! helpers - the coordinator manages global state to avoid race conditions.
//!
//! [`StdoutMock`]: crate::StdoutMock
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`global_color_support::set_override`]: crate::global_color_support::set_override
//! [`ColorSupport::Truecolor`]: crate::ColorSupport::Truecolor

use crate::{LockedOutputDevice, OffscreenBuffer, OutputDevice, RenderOpOutput,
            RenderOpPaint, RenderOpsLocalData, Size, col, height,
            lock_output_device_as_mut, pos, render_op::RenderOpCommon, row,
            terminal_lib_backends::direct_to_ansi::RenderOpPaintImplDirectToAnsi,
            test_fixtures::output_device_fixtures::OutputDeviceExt, width};

/// Standard buffer size for rendered tests (matches typical terminal).
pub fn rendered_test_buffer_size() -> Size { Size::new((width(80), height(24))) }

/// Creates initial test state with cursor at origin.
pub fn create_rendered_test_state() -> RenderOpsLocalData {
    RenderOpsLocalData {
        cursor_pos: pos(row(0) + col(0)),
        fg_color: None,
        bg_color: None,
    }
}

/// Execute render operations via [`StdoutMock`] and render result to [`OffscreenBuffer`].
///
/// This is the core helper for behavioral testing. It:
/// 1. Creates a [`StdoutMock`] output device
/// 2. Executes render operations via [`RenderOpPaintImplDirectToAnsi`]
/// 3. Captures the ANSI byte output
/// 4. Creates an [`OffscreenBuffer`] and applies the captured bytes
/// 5. Returns the buffer for assertions
///
/// [`StdoutMock`]: crate::StdoutMock
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
/// [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
pub fn execute_and_render_to_buffer(ops: Vec<RenderOpOutput>) -> OffscreenBuffer {
    let buffer_size = rendered_test_buffer_size();
    execute_and_render_to_buffer_with_size(ops, buffer_size)
}

/// Execute render operations and render to buffer with custom size.
///
/// # Color Support
///
/// This function assumes [`ColorSupport::Truecolor`] has already been set via
/// [`global_color_support::set_override`] by the process-isolated test coordinator.
/// Do not call `set_override`/`clear_override` here - the coordinator manages
/// global state to avoid race conditions when tests run in parallel.
///
/// [`ColorSupport::Truecolor`]: crate::ColorSupport::Truecolor
/// [`global_color_support::set_override`]: crate::global_color_support::set_override
pub fn execute_and_render_to_buffer_with_size(
    ops: Vec<RenderOpOutput>,
    buffer_size: Size,
) -> OffscreenBuffer {
    // Step 1: Create mock output device.
    let (output_device, stdout_mock) = OutputDevice::new_mock();

    // Step 2: Execute operations via DirectToAnsi painter.
    let mut state = create_rendered_test_state();
    let mut skip_flush = false;
    let mut painter = RenderOpPaintImplDirectToAnsi;

    for op in &ops {
        let mut_ref: LockedOutputDevice<'_> = lock_output_device_as_mut!(output_device);
        painter.paint(
            &mut skip_flush,
            op,
            buffer_size,
            &mut state,
            mut_ref,
            output_device.is_mock,
        );
    }

    // Step 3: Get captured ANSI bytes.
    let ansi_bytes = stdout_mock.get_copy_of_buffer_as_string();

    // Step 4: Create OffscreenBuffer and apply bytes.
    let mut ofs_buf =
        OffscreenBuffer::new_empty(buffer_size.row_height + buffer_size.col_width);
    let (_osc_events, _dsr_responses) = ofs_buf.apply_ansi_bytes(ansi_bytes);

    ofs_buf
}

/// Execute a sequence of operations including cursor movement and text paint.
///
/// This is useful for tests that need to position cursor before painting text.
pub fn execute_ops_and_render(ops: Vec<RenderOpOutput>) -> OffscreenBuffer {
    execute_and_render_to_buffer(ops)
}

/// Helper to create a cursor move operation.
pub fn move_cursor_abs(row_idx: usize, col_idx: usize) -> RenderOpOutput {
    RenderOpOutput::Common(RenderOpCommon::MoveCursorPositionAbs(pos(
        row(row_idx) + col(col_idx)
    )))
}

/// Helper to create a text paint operation.
pub fn paint_text(text: &str, style: Option<crate::TuiStyle>) -> RenderOpOutput {
    RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
        crate::InlineString::from(text),
        style,
    )
}

/// Helper to create a text paint operation with foreground color.
pub fn paint_text_with_fg(text: &str, fg_color: crate::TuiColor) -> RenderOpOutput {
    let style = crate::TuiStyle {
        color_fg: Some(fg_color),
        ..Default::default()
    };
    paint_text(text, Some(style))
}

/// Helper to create a text paint operation with background color.
pub fn paint_text_with_bg(text: &str, bg_color: crate::TuiColor) -> RenderOpOutput {
    let style = crate::TuiStyle {
        color_bg: Some(bg_color),
        ..Default::default()
    };
    paint_text(text, Some(style))
}

/// Helper to create a text paint operation with both fg and bg colors.
pub fn paint_text_with_colors(
    text: &str,
    fg_color: crate::TuiColor,
    bg_color: crate::TuiColor,
) -> RenderOpOutput {
    let style = crate::TuiStyle {
        color_fg: Some(fg_color),
        color_bg: Some(bg_color),
        ..Default::default()
    };
    paint_text(text, Some(style))
}

/// Helper to create a text paint operation with bold attribute.
pub fn paint_text_bold(text: &str) -> RenderOpOutput {
    let style = crate::TuiStyle {
        attribs: crate::TuiStyleAttribs {
            bold: Some(crate::tui_style_attrib::Bold),
            ..Default::default()
        },
        ..Default::default()
    };
    paint_text(text, Some(style))
}

/// Helper to create a text paint operation with RGB foreground color.
pub fn paint_text_with_rgb_fg(text: &str, r: u8, g: u8, b: u8) -> RenderOpOutput {
    let style = crate::TuiStyle {
        color_fg: Some(crate::TuiColor::Rgb(crate::RgbValue::from_u8(r, g, b))),
        ..Default::default()
    };
    paint_text(text, Some(style))
}

/// Helper to create a text paint operation with RGB background color.
pub fn paint_text_with_rgb_bg(text: &str, r: u8, g: u8, b: u8) -> RenderOpOutput {
    let style = crate::TuiStyle {
        color_bg: Some(crate::TuiColor::Rgb(crate::RgbValue::from_u8(r, g, b))),
        ..Default::default()
    };
    paint_text(text, Some(style))
}

/// Helper to create a text paint operation with RGB foreground and background colors.
pub fn paint_text_with_rgb_colors(
    text: &str,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) -> RenderOpOutput {
    let style = crate::TuiStyle {
        color_fg: Some(crate::TuiColor::Rgb(crate::RgbValue::from_u8(fg.0, fg.1, fg.2))),
        color_bg: Some(crate::TuiColor::Rgb(crate::RgbValue::from_u8(bg.0, bg.1, bg.2))),
        ..Default::default()
    };
    paint_text(text, Some(style))
}
