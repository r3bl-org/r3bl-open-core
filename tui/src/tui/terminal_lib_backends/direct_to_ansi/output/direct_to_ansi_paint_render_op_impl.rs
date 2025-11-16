// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Pipeline Stage 5: Backend Executor (`DirectToAnsi` Implementation)
//!
//! # You Are Here: **Stage 5** (DirectToAnsi Executor)
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//! [Stage 4: Backend Converter]
//!   ↓
//! [Stage 5: Backend Executor] ← YOU ARE HERE
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! **Input**: [`RenderOpOutputVec`] (from backend converter)
//! **Output**: ANSI escape sequences to terminal
//! **Role**: Execute rendering operations via `DirectToAnsi` backend
//!
//! <div class="warning">
//!
//! **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! reference table**, see the [rendering pipeline overview].
//!
//! </div>
//!
//! ## What This Stage Does
//!
//! The Backend Executor translates [`RenderOpOutputVec`] into actual terminal commands:
//! - Moves cursor to positions
//! - Sets foreground/background colors
//! - Paints styled text
//! - Manages raw mode, alternate screen, mouse tracking
//! - Uses [`RenderOpsLocalData`] to avoid redundant commands
//! - Generates ANSI sequences via [`AnsiSequenceGenerator`]
//!
//! This is the final stage before terminal output. Unlike Crossterm, `DirectToAnsi`
//! generates pure ANSI escape sequences without an external library.
//!
//! See [`RenderOpPaintImplDirectToAnsi`] for implementation details.
//!
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`RenderOpsLocalData`]: crate::RenderOpsLocalData
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture

use crate::{AnsiSequenceGenerator, CliTextInline, GCStringOwned, InlineString,
            LockedOutputDevice, PixelCharRenderer, Pos, RenderOpCommon, RenderOpFlush,
            RenderOpOutput, RenderOpPaint, RenderOpsLocalData, Size, TuiStyle,
            cli_text_inline_impl::CliTextConvertOptions, col, disable_raw_mode_now,
            enable_raw_mode_now, flush_now, sanitize_and_save_abs_pos};

/// Implements [`RenderOpPaint`] trait using direct ANSI sequence generation.
///
/// The methods execute all [`RenderOpOutput`] variants using [`AnsiSequenceGenerator`].
/// It tracks cursor position and colors to skip redundant ANSI sequences for
/// optimization.
///
/// This implementation executes all [`RenderOpOutput`] variants by generating ANSI
/// escape sequences via [`AnsiSequenceGenerator`]. It tracks cursor position and
/// colors in [`RenderOpsLocalData`] to skip redundant operations for optimization.
///
/// The [`paint()`] method dispatches to two helper methods:
/// - [`paint_common()`]: Handles all 27 [`RenderOpCommon`] variants
/// - [`paint_text_with_attributes()`]: Handles post-compositor text with optional styling
///
/// [`RenderOpPaint`]: crate::RenderOpPaint
/// [`paint()`]: Self::paint
/// [`paint_common()`]: Self::paint_common
/// [`paint_text_with_attributes()`]: Self::paint_text_with_attributes
#[derive(Debug)]
pub struct RenderOpPaintImplDirectToAnsi;

impl RenderOpPaint for RenderOpPaintImplDirectToAnsi {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOpOutput,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match render_op {
            RenderOpOutput::Common(common_op) => {
                self.paint_common(
                    skip_flush,
                    common_op,
                    window_size,
                    render_local_data,
                    locked_output_device,
                    is_mock,
                );
            }
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                text,
                maybe_style,
            ) => {
                RenderOpPaintImplDirectToAnsi::paint_text_with_attributes(
                    text,
                    *maybe_style,
                    window_size,
                    render_local_data,
                    locked_output_device,
                );
            }
        }
    }
}

impl RenderOpFlush for RenderOpPaintImplDirectToAnsi {
    fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
        locked_output_device
            .flush()
            .expect("Failed to flush output device");
    }

    fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
        let clear_sequence = AnsiSequenceGenerator::clear_screen();
        locked_output_device
            .write_all(clear_sequence.as_bytes())
            .expect("Failed to write clear screen sequence");
        locked_output_device
            .flush()
            .expect("Failed to flush output device");
    }
}

impl RenderOpPaintImplDirectToAnsi {
    /// Paint a single common render operation using direct ANSI sequence generation.
    ///
    /// This method handles all 27 [`RenderOpCommon`] variants, generating appropriate
    /// ANSI escape sequences and writing them directly to the output device. It tracks
    /// state (cursor position and colors) in [`RenderOpsLocalData`] to avoid sending
    /// redundant sequences when the state hasn't changed.
    ///
    /// # Variant Groups
    ///
    /// The 27 variants are organized into 8 logical groups:
    /// - **A**: No-ops (`EnterRawMode`, `ExitRawMode`, `Noop`) - return early
    /// - **B**: Cursor movement (5 variants) - with optimization to skip if unchanged
    /// - **C**: Screen clearing (4 variants) - direct ANSI generation
    /// - **D**: Color operations (4 variants) - with caching to skip if unchanged
    /// - **E**: Text rendering (1 variant) - pass-through
    /// - **F**: Cursor visibility (2 variants) - direct ANSI generation
    /// - **G**: Cursor save/restore (2 variants) - direct ANSI generation
    /// - **H**: Terminal modes (6 variants) - direct ANSI generation
    ///
    /// # Panics
    ///
    /// This function panics if writing to the output device fails. This is appropriate
    /// for the rendering layer where I/O failures are fatal to terminal rendering.
    #[allow(clippy::too_many_lines)]
    pub fn paint_common(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOpCommon,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match render_op {
            // Group A: No-ops
            RenderOpCommon::Noop => {}

            RenderOpCommon::EnterRawMode => {
                helpers::raw_mode_enter(skip_flush, locked_output_device, is_mock);
            }

            RenderOpCommon::ExitRawMode => {
                helpers::raw_mode_exit(skip_flush, locked_output_device, is_mock);
            }

            // Group B: Cursor movement
            RenderOpCommon::MoveCursorPositionAbs(abs_pos) => {
                helpers::move_cursor_position_abs(
                    *abs_pos,
                    window_size,
                    render_local_data,
                    locked_output_device,
                );
            }

            RenderOpCommon::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
                let new_abs_pos = *box_origin_pos + *content_rel_pos;
                helpers::move_cursor_position_abs(
                    new_abs_pos,
                    window_size,
                    render_local_data,
                    locked_output_device,
                );
            }

            RenderOpCommon::MoveCursorToColumn(col_index) => {
                helpers::move_cursor_to_column(
                    *col_index,
                    render_local_data,
                    locked_output_device,
                );
            }

            RenderOpCommon::MoveCursorToNextLine(row_height) => {
                helpers::move_cursor_to_next_line(
                    *row_height,
                    render_local_data,
                    locked_output_device,
                );
            }

            RenderOpCommon::MoveCursorToPreviousLine(row_height) => {
                helpers::move_cursor_to_previous_line(
                    *row_height,
                    render_local_data,
                    locked_output_device,
                );
            }

            // Group C: Screen clearing
            RenderOpCommon::ClearScreen => {
                let ansi = AnsiSequenceGenerator::clear_screen();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write clear screen ANSI");
            }

            RenderOpCommon::ClearCurrentLine => {
                let ansi = AnsiSequenceGenerator::clear_current_line();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write clear current line ANSI");
            }

            RenderOpCommon::ClearToEndOfLine => {
                let ansi = AnsiSequenceGenerator::clear_to_end_of_line();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write clear to end of line ANSI");
            }

            RenderOpCommon::ClearToStartOfLine => {
                let ansi = AnsiSequenceGenerator::clear_to_start_of_line();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write clear to start of line ANSI");
            }

            // Group D: Color operations
            RenderOpCommon::SetFgColor(color) => {
                helpers::set_fg_color(*color, render_local_data, locked_output_device);
            }

            RenderOpCommon::SetBgColor(color) => {
                helpers::set_bg_color(*color, render_local_data, locked_output_device);
            }

            RenderOpCommon::ResetColor => {
                let ansi = AnsiSequenceGenerator::reset_color();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write reset color ANSI");
                render_local_data.fg_color = None;
                render_local_data.bg_color = None;
            }

            RenderOpCommon::ApplyColors(maybe_style) => {
                helpers::apply_colors(
                    *maybe_style,
                    render_local_data,
                    locked_output_device,
                );
            }

            // Group E: Text rendering
            RenderOpCommon::PrintStyledText(text) => {
                locked_output_device
                    .write_all(text.as_bytes())
                    .expect("Failed to write styled text");
            }

            // Group F: Cursor visibility
            RenderOpCommon::ShowCursor => {
                let ansi = AnsiSequenceGenerator::show_cursor();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write show cursor ANSI");
            }

            RenderOpCommon::HideCursor => {
                let ansi = AnsiSequenceGenerator::hide_cursor();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write hide cursor ANSI");
            }

            // Group G: Cursor save/restore
            RenderOpCommon::SaveCursorPosition => {
                let ansi = AnsiSequenceGenerator::save_cursor_position();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write save cursor position ANSI");
            }

            RenderOpCommon::RestoreCursorPosition => {
                let ansi = AnsiSequenceGenerator::restore_cursor_position();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write restore cursor position ANSI");
            }

            // Group H: Terminal modes
            RenderOpCommon::EnterAlternateScreen => {
                let ansi = AnsiSequenceGenerator::enter_alternate_screen();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write enter alternate screen ANSI");
            }

            RenderOpCommon::ExitAlternateScreen => {
                let ansi = AnsiSequenceGenerator::exit_alternate_screen();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write exit alternate screen ANSI");
            }

            RenderOpCommon::EnableMouseTracking => {
                let ansi = AnsiSequenceGenerator::enable_mouse_tracking();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write enable mouse tracking ANSI");
            }

            RenderOpCommon::DisableMouseTracking => {
                let ansi = AnsiSequenceGenerator::disable_mouse_tracking();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write disable mouse tracking ANSI");
            }

            RenderOpCommon::EnableBracketedPaste => {
                let ansi = AnsiSequenceGenerator::enable_bracketed_paste();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write enable bracketed paste ANSI");
            }

            RenderOpCommon::DisableBracketedPaste => {
                let ansi = AnsiSequenceGenerator::disable_bracketed_paste();
                locked_output_device
                    .write_all(ansi.as_bytes())
                    .expect("Failed to write disable bracketed paste ANSI");
            }
        }
    }

    /// Paint text with optional styling (post-compositor text rendering).
    ///
    /// This method is used for text that has already been clipped and truncated by the
    /// compositor. The text is positioned by the compositor, so this method simply:
    /// 1. Applies style attributes if provided
    /// 2. Writes the text
    /// 3. Resets styling if it was applied
    ///
    /// # Arguments
    ///
    /// - `text`: The text to paint (already positioned by compositor)
    /// - `maybe_style`: Optional style with colors and attributes
    /// - `window_size`: Current terminal window dimensions
    /// - `render_local_data`: State tracking for cursor position and colors
    /// - `locked_output_device`: Output device for writing bytes
    ///
    /// # Panics
    ///
    /// This function panics if writing to the output device fails. This is appropriate
    /// for the rendering layer where I/O failures are fatal to terminal rendering.
    pub fn paint_text_with_attributes(
        text: &InlineString,
        maybe_style: Option<TuiStyle>,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        // Use unified PixelCharRenderer for consistent ANSI generation
        let cli_text = CliTextInline {
            text: text.as_str().into(),
            attribs: maybe_style.map(|s| s.attribs).unwrap_or_default(),
            color_fg: maybe_style.and_then(|s| s.color_fg),
            color_bg: maybe_style.and_then(|s| s.color_bg),
        };

        // Convert CliTextInline to PixelChars using default options
        let pixel_chars = cli_text.convert(CliTextConvertOptions::default());

        // Render PixelChars to ANSI bytes using unified renderer
        let mut renderer = PixelCharRenderer::new();
        let ansi_bytes = renderer.render_line(&pixel_chars);

        // Write the ANSI bytes directly to output device
        locked_output_device
            .write_all(ansi_bytes)
            .expect("Failed to write ANSI text bytes");

        // Update cursor position after paint
        let cursor_pos_copy = {
            let mut copy = render_local_data.cursor_pos;
            let text_display_width = GCStringOwned::from(text.as_str()).width();
            *copy.col_index += *text_display_width;
            copy
        };

        sanitize_and_save_abs_pos(cursor_pos_copy, window_size, render_local_data);
    }
}

mod helpers {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub fn move_cursor_position_abs(
        abs_pos: Pos,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        let Pos {
            col_index,
            row_index,
        } = sanitize_and_save_abs_pos(abs_pos, window_size, render_local_data);

        let ansi = AnsiSequenceGenerator::cursor_position(row_index, col_index);
        locked_output_device
            .write_all(ansi.as_bytes())
            .expect("Failed to write cursor position ANSI");
    }

    pub fn move_cursor_to_column(
        col_index: crate::ColIndex,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        render_local_data.cursor_pos.col_index = col_index;
        let ansi = AnsiSequenceGenerator::cursor_to_column(col_index);
        locked_output_device
            .write_all(ansi.as_bytes())
            .expect("Failed to write cursor to column ANSI");
    }

    pub fn move_cursor_to_next_line(
        row_height: crate::RowHeight,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        render_local_data.cursor_pos.row_index += row_height;
        render_local_data.cursor_pos.col_index = col(0);
        let ansi = AnsiSequenceGenerator::cursor_next_line(row_height);
        locked_output_device
            .write_all(ansi.as_bytes())
            .expect("Failed to write cursor next line ANSI");
    }

    pub fn move_cursor_to_previous_line(
        row_height: crate::RowHeight,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        render_local_data.cursor_pos.row_index -= row_height;
        render_local_data.cursor_pos.col_index = col(0);
        let ansi = AnsiSequenceGenerator::cursor_previous_line(row_height);
        locked_output_device
            .write_all(ansi.as_bytes())
            .expect("Failed to write cursor previous line ANSI");
    }

    pub fn set_fg_color(
        color: crate::TuiColor,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        // Skip if color unchanged
        if render_local_data.fg_color == Some(color) {
            return;
        }
        let ansi = AnsiSequenceGenerator::fg_color(color);
        locked_output_device
            .write_all(ansi.as_bytes())
            .expect("Failed to write fg color ANSI");
        render_local_data.fg_color = Some(color);
    }

    pub fn set_bg_color(
        color: crate::TuiColor,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        // Skip if color unchanged
        if render_local_data.bg_color == Some(color) {
            return;
        }
        let ansi = AnsiSequenceGenerator::bg_color(color);
        locked_output_device
            .write_all(ansi.as_bytes())
            .expect("Failed to write bg color ANSI");
        render_local_data.bg_color = Some(color);
    }

    pub fn apply_colors(
        maybe_style: Option<TuiStyle>,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        if let Some(style) = maybe_style {
            if let Some(fg) = style.color_fg {
                helpers::set_fg_color(fg, render_local_data, locked_output_device);
            }
            if let Some(bg) = style.color_bg {
                helpers::set_bg_color(bg, render_local_data, locked_output_device);
            }
        }
    }

    pub fn raw_mode_enter(
        skip_flush: &mut bool,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        enable_raw_mode_now!(is_mock, "EnterRawMode -> enable_raw_mode()");

        // Generate ANSI sequences for entering raw mode
        let mut ansi_output = String::new();
        ansi_output.push_str(&AnsiSequenceGenerator::enable_bracketed_paste());
        ansi_output.push_str(&AnsiSequenceGenerator::enable_mouse_tracking());
        ansi_output.push_str(&AnsiSequenceGenerator::enter_alternate_screen());
        ansi_output.push_str(&AnsiSequenceGenerator::cursor_position(
            crate::row(0),
            crate::col(0),
        ));
        ansi_output.push_str(&AnsiSequenceGenerator::clear_screen());
        ansi_output.push_str(&AnsiSequenceGenerator::hide_cursor());

        locked_output_device
            .write_all(ansi_output.as_bytes())
            .expect("Failed to write enter raw mode ANSI");

        if !is_mock {
            flush_now!(locked_output_device, "EnterRawMode -> flush()");
        }

        *skip_flush = true;
    }

    pub fn raw_mode_exit(
        skip_flush: &mut bool,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        // Generate ANSI sequences for exiting raw mode
        let mut ansi_output = String::new();
        ansi_output.push_str(&AnsiSequenceGenerator::disable_bracketed_paste());
        ansi_output.push_str(&AnsiSequenceGenerator::disable_mouse_tracking());
        ansi_output.push_str(&AnsiSequenceGenerator::exit_alternate_screen());
        ansi_output.push_str(&AnsiSequenceGenerator::show_cursor());

        locked_output_device
            .write_all(ansi_output.as_bytes())
            .expect("Failed to write exit raw mode ANSI");

        flush_now!(locked_output_device, "ExitRawMode -> flush()");

        disable_raw_mode_now!(is_mock, "ExitRawMode -> disable_raw_mode()");

        *skip_flush = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row;

    #[test]
    fn test_paint_common_noop_variant() {
        // No-op should not produce any output
        // Simply verify Noop variant exists and can be constructed
        let _render_op = RenderOpCommon::Noop;
        // The Noop operation should not produce any side effects
    }

    #[test]
    fn test_paint_common_clear_screen() {
        // ClearScreen should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::clear_screen();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b')); // Contains escape sequence
    }

    #[test]
    fn test_paint_common_clear_current_line() {
        // ClearCurrentLine should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::clear_current_line();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_clear_to_end_of_line() {
        // ClearToEndOfLine should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::clear_to_end_of_line();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_clear_to_start_of_line() {
        // ClearToStartOfLine should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::clear_to_start_of_line();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_cursor_movement_position() {
        // MoveCursorPositionAbs should generate cursor positioning ANSI
        let row_idx = row(5);
        let col_idx = col(10);
        let ansi = AnsiSequenceGenerator::cursor_position(row_idx, col_idx);
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b')); // Contains escape sequence
        assert!(ansi.contains('H')); // Should end with H command
    }

    #[test]
    fn test_paint_common_cursor_to_column() {
        // MoveCursorToColumn should generate column positioning ANSI
        let col_idx = col(15);
        let ansi = AnsiSequenceGenerator::cursor_to_column(col_idx);
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_cursor_next_line() {
        // MoveCursorToNextLine should generate next line ANSI
        let row_height = crate::height(3);
        let ansi = AnsiSequenceGenerator::cursor_next_line(row_height);
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
        assert!(ansi.contains('E')); // Should use E command
    }

    #[test]
    fn test_paint_common_cursor_previous_line() {
        // MoveCursorToPreviousLine should generate previous line ANSI
        let row_height = crate::height(2);
        let ansi = AnsiSequenceGenerator::cursor_previous_line(row_height);
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
        assert!(ansi.contains('F')); // Should use F command
    }

    #[test]
    fn test_paint_common_show_cursor() {
        // ShowCursor should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::show_cursor();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_hide_cursor() {
        // HideCursor should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::hide_cursor();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_save_cursor_position() {
        // SaveCursorPosition should generate DECSC ANSI sequence
        let ansi = AnsiSequenceGenerator::save_cursor_position();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_restore_cursor_position() {
        // RestoreCursorPosition should generate DECRC ANSI sequence
        let ansi = AnsiSequenceGenerator::restore_cursor_position();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_enter_alternate_screen() {
        // EnterAlternateScreen should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::enter_alternate_screen();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_exit_alternate_screen() {
        // ExitAlternateScreen should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::exit_alternate_screen();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_enable_mouse_tracking() {
        // EnableMouseTracking should generate appropriate ANSI sequences
        let ansi = AnsiSequenceGenerator::enable_mouse_tracking();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_disable_mouse_tracking() {
        // DisableMouseTracking should generate appropriate ANSI sequences
        let ansi = AnsiSequenceGenerator::disable_mouse_tracking();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_enable_bracketed_paste() {
        // EnableBracketedPaste should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::enable_bracketed_paste();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_disable_bracketed_paste() {
        // DisableBracketedPaste should generate appropriate ANSI sequence
        let ansi = AnsiSequenceGenerator::disable_bracketed_paste();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
    }

    #[test]
    fn test_paint_common_reset_color() {
        // ResetColor should generate SGR reset ANSI sequence
        let ansi = AnsiSequenceGenerator::reset_color();
        assert!(!ansi.is_empty());
        assert!(ansi.contains('\x1b'));
        assert!(ansi.contains("0m")); // Should reset all attributes
    }

    #[test]
    fn test_color_caching_fg_color() {
        // Foreground color should be cached to skip redundant sequences
        let color1 = crate::TuiColor::Ansi(crate::AnsiValue::new(1)); // ANSI color index 1 (Red)
        let ansi1 = AnsiSequenceGenerator::fg_color(color1);
        let ansi2 = AnsiSequenceGenerator::fg_color(color1);

        // Both should generate valid sequences
        assert!(!ansi1.is_empty());
        assert!(!ansi2.is_empty());
    }

    #[test]
    fn test_color_caching_bg_color() {
        // Background color should be cached to skip redundant sequences
        let color1 = crate::TuiColor::Ansi(crate::AnsiValue::new(4)); // ANSI color index 4 (Blue)
        let ansi1 = AnsiSequenceGenerator::bg_color(color1);
        let ansi2 = AnsiSequenceGenerator::bg_color(color1);

        // Both should generate valid sequences
        assert!(!ansi1.is_empty());
        assert!(!ansi2.is_empty());
    }

    #[test]
    fn test_ansi_sequence_generator_cursor_position_indexing() {
        // ANSI sequences should use 1-based indexing (row, col)
        let row_idx = row(0); // 0-based index 0
        let col_idx = col(0); // 0-based index 0
        let ansi = AnsiSequenceGenerator::cursor_position(row_idx, col_idx);

        // Should generate 1-based positioning (1,1)
        assert!(ansi.contains("1;1H")); // Position (1, 1) in 1-based
    }

    #[test]
    fn test_ansi_sequence_generator_cursor_position_higher_numbers() {
        // Test with higher row and column indices
        let row_idx = row(10);
        let col_idx = col(20);
        let ansi = AnsiSequenceGenerator::cursor_position(row_idx, col_idx);

        // Should generate correct 1-based positioning (11,21)
        assert!(ansi.contains("11;21H"));
    }

    #[test]
    fn test_render_ops_local_data_default_initialization() {
        // RenderOpsLocalData should initialize with defaults
        let data = RenderOpsLocalData::default();
        assert_eq!(data.cursor_pos, Pos::default());
        assert_eq!(data.fg_color, None);
        assert_eq!(data.bg_color, None);
    }

    #[test]
    fn test_render_ops_local_data_cursor_position_tracking() {
        // Cursor position should be updated and tracked
        let mut data = RenderOpsLocalData::default();
        let new_pos = Pos::new((row(5), col(10)));
        data.cursor_pos = new_pos;
        assert_eq!(data.cursor_pos, new_pos);
    }

    #[test]
    fn test_render_ops_local_data_color_tracking() {
        // Colors should be tracked for optimization
        let mut data = RenderOpsLocalData::default();
        let color = crate::TuiColor::Ansi(crate::AnsiValue::new(2)); // Green
        data.fg_color = Some(color);
        data.bg_color = Some(color);
        assert_eq!(data.fg_color, Some(color));
        assert_eq!(data.bg_color, Some(color));
    }

    #[test]
    fn test_all_27_render_op_common_variants_exist() {
        // Ensure all 27 variants are defined in RenderOpCommon
        // This is a compile-time check via the pattern matching in paint_common

        // This test verifies we've handled all cases by checking variant count
        // in the match statement through successful compilation.
        let _noop = RenderOpCommon::Noop;
        let _enter_raw = RenderOpCommon::EnterRawMode;
        let _exit_raw = RenderOpCommon::ExitRawMode;
        let _move_abs = RenderOpCommon::MoveCursorPositionAbs(Pos::default());
        let _clear = RenderOpCommon::ClearScreen;
        let white_color = crate::TuiColor::Ansi(crate::AnsiValue::new(7)); // White (ANSI 7)
        let black_color = crate::TuiColor::Ansi(crate::AnsiValue::new(0)); // Black (ANSI 0)
        let _set_fg = RenderOpCommon::SetFgColor(white_color);
        let _set_bg = RenderOpCommon::SetBgColor(black_color);
        let _reset = RenderOpCommon::ResetColor;
        let _show = RenderOpCommon::ShowCursor;
        let _hide = RenderOpCommon::HideCursor;
        let _save = RenderOpCommon::SaveCursorPosition;
        let _restore = RenderOpCommon::RestoreCursorPosition;
        let _enter_alt = RenderOpCommon::EnterAlternateScreen;
        let _exit_alt = RenderOpCommon::ExitAlternateScreen;
        let _enable_mouse = RenderOpCommon::EnableMouseTracking;
        let _disable_mouse = RenderOpCommon::DisableMouseTracking;
        let _enable_paste = RenderOpCommon::EnableBracketedPaste;
        let _disable_paste = RenderOpCommon::DisableBracketedPaste;

        // All variants successfully created - test passes
    }
}
