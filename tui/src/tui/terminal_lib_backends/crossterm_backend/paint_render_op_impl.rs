// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
//! # Pipeline Stage 5: Backend Executor (Crossterm Implementation)
//!
//! # You Are Here
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
//! **Role**: Execute rendering operations via Crossterm backend
//!
//! > **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! > reference table**, see the [`render_pipeline` mod docs].
//!
//! ## What This Stage Does
//!
//! The Backend Executor translates [`RenderOpOutputVec`] into actual terminal commands:
//! - Moves cursor to positions
//! - Sets foreground/background colors
//! - Paints styled text
//! - Manages raw mode, alternate screen, mouse tracking
//! - Uses [`RenderOpsLocalData`] to avoid redundant commands
//! - Flushes output to ensure immediate display
//!
//! This is the final stage before terminal output. The Crossterm library handles
//! converting commands to ANSI escape sequences appropriate for the terminal.
//!
//! [`RenderOpOutputVec`]: crate::RenderOpOutputVec
//! [`RenderOpsLocalData`]: crate::RenderOpsLocalData
//! [`render_pipeline` mod docs]: mod@crate::tui::terminal_lib_backends::render_pipeline

// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use crate::{CliTextInline, GCStringOwned, LockedOutputDevice, Pos, RenderOpCommon,
            RenderOpFlush, RenderOpOutput, RenderOpPaint, RenderOpsLocalData, Size,
            TuiColor, TuiStyle, cli_text_inline_impl::CliTextConvertOptions,
            disable_raw_mode_now, enable_raw_mode_now, flush_now,
            queue_terminal_command, sanitize_and_save_abs_pos,
            tui::terminal_lib_backends::direct_to_ansi::PixelCharRenderer};
use crossterm::{cursor::{Hide, MoveTo, Show},
                event::{DisableBracketedPaste, DisableMouseCapture,
                        EnableBracketedPaste, EnableMouseCapture},
                style::{ResetColor, SetBackgroundColor, SetForegroundColor},
                terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen}};

/// Struct representing the Crossterm implementation of [`RenderOpPaint`] trait.
/// This empty struct is needed since the `RenderOpFlush` trait needs to be implemented.
///
/// [`RenderOpPaint`]: crate::RenderOpPaint
#[derive(Debug)]
pub struct PaintRenderOpImplCrossterm;

impl RenderOpPaint for PaintRenderOpImplCrossterm {
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
                PaintRenderOpImplCrossterm::paint_text_with_attributes(
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

impl RenderOpFlush for PaintRenderOpImplCrossterm {
    fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
        flush_now!(locked_output_device, "flush() -> output_device");
    }

    fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
        queue_terminal_command!(
            locked_output_device,
            "flush() -> after ResetColor, Clear",
            ResetColor,
            Clear(ClearType::All),
        );
    }
}

impl PaintRenderOpImplCrossterm {
    /// Paint a single common render operation.
    ///
    /// This method handles rendering of `RenderOpCommon` operations, which are shared
    /// between IR (app/component) and Output (backend) contexts. This is called by the
    /// render pipeline when executing IR-level operations that have been routed to the
    /// backend.
    #[allow(clippy::too_many_lines)]
    pub fn paint_common(
        &mut self,
        skip_flush: &mut bool,
        command_ref: &RenderOpCommon,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match command_ref {
            RenderOpCommon::Noop => {}
            RenderOpCommon::EnterRawMode => {
                PaintRenderOpImplCrossterm::raw_mode_enter(
                    skip_flush,
                    locked_output_device,
                    is_mock,
                );
            }
            RenderOpCommon::ExitRawMode => {
                PaintRenderOpImplCrossterm::raw_mode_exit(
                    skip_flush,
                    locked_output_device,
                    is_mock,
                );
            }
            RenderOpCommon::MoveCursorPositionAbs(abs_pos) => {
                PaintRenderOpImplCrossterm::move_cursor_position_abs(
                    *abs_pos,
                    window_size,
                    render_local_data,
                    locked_output_device,
                );
            }
            RenderOpCommon::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
                PaintRenderOpImplCrossterm::move_cursor_position_rel_to(
                    *box_origin_pos,
                    *content_rel_pos,
                    window_size,
                    render_local_data,
                    locked_output_device,
                );
            }
            RenderOpCommon::ClearScreen => {
                queue_terminal_command!(
                    locked_output_device,
                    "ClearScreen",
                    Clear(ClearType::All),
                );
            }
            RenderOpCommon::SetFgColor(color) => {
                PaintRenderOpImplCrossterm::set_fg_color(*color, locked_output_device);
            }
            RenderOpCommon::SetBgColor(color) => {
                PaintRenderOpImplCrossterm::set_bg_color(*color, locked_output_device);
            }
            RenderOpCommon::ResetColor => {
                queue_terminal_command!(locked_output_device, "ResetColor", ResetColor);
            }
            RenderOpCommon::ApplyColors(style) => {
                PaintRenderOpImplCrossterm::apply_colors(*style, locked_output_device);
            }
            RenderOpCommon::PrintStyledText(text) => {
                PaintRenderOpImplCrossterm::print_styled_text(text, locked_output_device);
            }
            RenderOpCommon::MoveCursorToColumn(col_index) => {
                PaintRenderOpImplCrossterm::move_cursor_to_column(
                    *col_index,
                    render_local_data,
                    locked_output_device,
                );
            }
            RenderOpCommon::MoveCursorToNextLine(row_height) => {
                PaintRenderOpImplCrossterm::move_cursor_to_next_line(
                    *row_height,
                    render_local_data,
                    locked_output_device,
                );
            }
            RenderOpCommon::MoveCursorToPreviousLine(row_height) => {
                PaintRenderOpImplCrossterm::move_cursor_to_previous_line(
                    *row_height,
                    render_local_data,
                    locked_output_device,
                );
            }
            RenderOpCommon::ClearCurrentLine => {
                queue_terminal_command!(
                    locked_output_device,
                    "ClearCurrentLine",
                    Clear(ClearType::CurrentLine),
                );
            }
            RenderOpCommon::ClearToEndOfLine => {
                PaintRenderOpImplCrossterm::clear_to_end_of_line(locked_output_device);
            }
            RenderOpCommon::ClearToStartOfLine => {
                PaintRenderOpImplCrossterm::clear_to_start_of_line(locked_output_device);
            }
            RenderOpCommon::ShowCursor => {
                queue_terminal_command!(locked_output_device, "ShowCursor", Show,);
            }
            RenderOpCommon::HideCursor => {
                queue_terminal_command!(locked_output_device, "HideCursor", Hide,);
            }
            RenderOpCommon::SaveCursorPosition => {
                PaintRenderOpImplCrossterm::save_cursor_position(locked_output_device);
            }
            RenderOpCommon::RestoreCursorPosition => {
                PaintRenderOpImplCrossterm::restore_cursor_position(locked_output_device);
            }
            RenderOpCommon::EnterAlternateScreen => {
                queue_terminal_command!(
                    locked_output_device,
                    "EnterAlternateScreen",
                    EnterAlternateScreen
                );
            }
            RenderOpCommon::ExitAlternateScreen => {
                queue_terminal_command!(
                    locked_output_device,
                    "ExitAlternateScreen",
                    LeaveAlternateScreen
                );
            }
            RenderOpCommon::EnableMouseTracking => {
                queue_terminal_command!(
                    locked_output_device,
                    "EnableMouseTracking",
                    EnableMouseCapture
                );
            }
            RenderOpCommon::DisableMouseTracking => {
                queue_terminal_command!(
                    locked_output_device,
                    "DisableMouseTracking",
                    DisableMouseCapture
                );
            }
            RenderOpCommon::EnableBracketedPaste => {
                queue_terminal_command!(
                    locked_output_device,
                    "EnableBracketedPaste",
                    EnableBracketedPaste
                );
            }
            RenderOpCommon::DisableBracketedPaste => {
                queue_terminal_command!(
                    locked_output_device,
                    "DisableBracketedPaste",
                    DisableBracketedPaste
                );
            }
        }
    }

    pub fn move_cursor_position_rel_to(
        box_origin_pos: Pos,
        content_rel_pos: Pos,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        let new_abs_pos = box_origin_pos + content_rel_pos;
        Self::move_cursor_position_abs(
            new_abs_pos,
            window_size,
            render_local_data,
            locked_output_device,
        );
    }

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

        let col = col_index.as_u16();
        let row = row_index.as_u16();

        queue_terminal_command!(
            locked_output_device,
            "MoveCursorPosition",
            MoveTo(col, row)
        );
    }

    pub fn raw_mode_exit(
        skip_flush: &mut bool,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        queue_terminal_command!(
            locked_output_device,
            "ExitRawMode -> DisableBracketedPaste, Show, LeaveAlternateScreen, DisableMouseCapture",
            DisableBracketedPaste,
            Show,
            LeaveAlternateScreen,
            DisableMouseCapture
        );

        flush_now!(locked_output_device, "ExitRawMode -> flush()");

        disable_raw_mode_now!(is_mock, "ExitRawMode -> disable_raw_mode()");

        *skip_flush = true;
    }

    /// Enter raw mode, enabling bracketed paste, mouse capture, and entering the
    /// alternate screen. This is used to prepare the terminal for rendering.
    /// It also clears the screen and hides the cursor.
    ///
    /// Bracketed paste allows the terminal to distinguish between typed text and
    /// pasted text. See [`crate::InputEvent::BracketedPaste`] for details on how
    /// paste events work.
    ///
    /// More info: <https://en.wikipedia.org/wiki/Bracketed-paste>
    pub fn raw_mode_enter(
        skip_flush: &mut bool,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        enable_raw_mode_now!(is_mock, "EnterRawMode -> enable_raw_mode()");

        queue_terminal_command!(
            locked_output_device,
            "EnterRawMode -> EnableBracketedPaste, EnableMouseCapture, EnterAlternateScreen, MoveTo(0,0), Clear(ClearType::All), Hide",
            EnableBracketedPaste,
            EnableMouseCapture,
            EnterAlternateScreen,
            MoveTo(0, 0),
            Clear(ClearType::All),
            Hide,
        );

        if !is_mock {
            flush_now!(locked_output_device, "EnterRawMode -> flush()");
        }

        *skip_flush = true;
    }

    pub fn set_fg_color(color: TuiColor, locked_output_device: LockedOutputDevice<'_>) {
        let color = color.into();

        queue_terminal_command!(
            locked_output_device,
            "SetFgColor",
            SetForegroundColor(color),
        );
    }

    pub fn set_bg_color(color: TuiColor, locked_output_device: LockedOutputDevice<'_>) {
        let color: crossterm::style::Color = color.into();

        queue_terminal_command!(
            locked_output_device,
            "SetBgColor",
            SetBackgroundColor(color),
        );
    }

    pub fn paint_text_with_attributes(
        text_arg: &str,
        maybe_style: Option<TuiStyle>,
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        // Phase 6: Use unified PixelCharRenderer for ANSI generation instead of
        // individual crossterm commands. This provides smart style diffing and
        // consistent ANSI output across all rendering paths.

        // Create CliTextInline from text and style
        let cli_text = CliTextInline {
            text: text_arg.into(),
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
        if let Err(e) = locked_output_device.write_all(ansi_bytes) {
            eprintln!("Failed to write ANSI bytes: {e}");
        }

        // Update cursor position after paint
        let cursor_pos_copy = {
            let mut copy = render_local_data.cursor_pos;
            let text_display_width = GCStringOwned::from(text_arg).width();
            *copy.col_index += *text_display_width;
            copy
        };

        sanitize_and_save_abs_pos(cursor_pos_copy, window_size, render_local_data);
    }

    /// Use [`crossterm::style::Color`] to set crossterm Colors.
    /// Docs: <https://docs.rs/crossterm/latest/crossterm/style/index.html#colors>
    pub fn apply_colors(
        maybe_style: Option<TuiStyle>,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        if let Some(style) = maybe_style {
            // Handle background color.
            if let Some(tui_color_bg) = style.color_bg {
                let color_bg: crossterm::style::Color = tui_color_bg.into();

                queue_terminal_command!(
                    locked_output_device,
                    "ApplyColors -> SetBgColor",
                    SetBackgroundColor(color_bg),
                );
            }

            // Handle foreground color.
            if let Some(tui_color_fg) = style.color_fg {
                let color_fg: crossterm::style::Color = tui_color_fg.into();

                queue_terminal_command!(
                    locked_output_device,
                    "ApplyColors -> SetFgColor",
                    SetForegroundColor(color_fg),
                );
            }
        }
    }

    // ===== Incremental Rendering Operations (Phase 1) =====

    /// Move cursor to specific column in current row (row unchanged).
    /// Maps to CSI `<n>G` ANSI sequence.
    pub fn move_cursor_to_column(
        col_index: crate::ColIndex,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        use crossterm::cursor::MoveToColumn;

        let col = col_index.as_u16();
        render_local_data.cursor_pos.col_index = col_index;

        queue_terminal_command!(
            locked_output_device,
            "MoveCursorToColumn",
            MoveToColumn(col),
        );
    }

    /// Move cursor down by N lines and to column 0.
    /// Maps to CSI `<n>E` ANSI sequence.
    pub fn move_cursor_to_next_line(
        row_height: crate::RowHeight,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        use crate::col;
        use crossterm::cursor::MoveToNextLine;

        let n = row_height.as_u16();
        // Add RowHeight to current RowIndex position
        render_local_data.cursor_pos.row_index += row_height;
        render_local_data.cursor_pos.col_index = col(0);

        queue_terminal_command!(
            locked_output_device,
            "MoveCursorToNextLine",
            MoveToNextLine(n),
        );
    }

    /// Move cursor up by N lines and to column 0.
    /// Maps to CSI `<n>F` ANSI sequence.
    pub fn move_cursor_to_previous_line(
        row_height: crate::RowHeight,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        use crate::col;
        use crossterm::cursor::MoveToPreviousLine;

        let n = row_height.as_u16();
        // Subtract RowHeight from current RowIndex position
        render_local_data.cursor_pos.row_index -= row_height;
        render_local_data.cursor_pos.col_index = col(0);

        queue_terminal_command!(
            locked_output_device,
            "MoveCursorToPreviousLine",
            MoveToPreviousLine(n),
        );
    }

    /// Clear from cursor to end of line.
    /// Maps to CSI `0K` (or `CSI K`) ANSI sequence.
    pub fn clear_to_end_of_line(locked_output_device: LockedOutputDevice<'_>) {
        queue_terminal_command!(
            locked_output_device,
            "ClearToEndOfLine",
            Clear(ClearType::UntilNewLine),
        );
    }

    /// Clear from cursor to beginning of line.
    /// Maps to CSI `1K` ANSI sequence.
    pub fn clear_to_start_of_line(locked_output_device: LockedOutputDevice<'_>) {
        queue_terminal_command!(
            locked_output_device,
            "ClearToStartOfLine",
            Clear(ClearType::FromCursorUp),
        );
    }

    /// Print text that already contains ANSI escape codes.
    /// No additional styling applied - text rendered as-is.
    pub fn print_styled_text(text: &str, locked_output_device: LockedOutputDevice<'_>) {
        use crossterm::style::Print;

        queue_terminal_command!(locked_output_device, "PrintStyledText", Print(text),);
    }

    /// Save cursor position to be restored later.
    /// Maps to CSI `s` ANSI sequence (DECSC - save cursor).
    pub fn save_cursor_position(locked_output_device: LockedOutputDevice<'_>) {
        // crossterm doesn't have a direct SaveCursorPosition command,
        // so we write the ANSI sequence directly.
        if let Err(e) = locked_output_device.write_all(b"\x1b[s") {
            eprintln!("Failed to write SaveCursorPosition ANSI sequence: {e}");
        }
    }

    /// Restore cursor position previously saved with [`SaveCursorPosition`].
    /// Maps to CSI `u` ANSI sequence (DECRC - restore cursor).
    ///
    /// [`SaveCursorPosition`]: PaintRenderOpImplCrossterm::save_cursor_position
    pub fn restore_cursor_position(locked_output_device: LockedOutputDevice<'_>) {
        // crossterm doesn't have a direct RestoreCursorPosition command,
        // so we write the ANSI sequence directly.
        if let Err(e) = locked_output_device.write_all(b"\x1b[u") {
            eprintln!("Failed to write RestoreCursorPosition ANSI sequence: {e}");
        }
    }
}

#[macro_export]
macro_rules! queue_terminal_command {
    ($writer: expr, $arg_log_msg: expr $(, $command: expr)* $(,)?) => {{
        use ::crossterm::QueueableCommand;
        $(
            $crate::crossterm_op!(
                $arg_log_msg,
                QueueableCommand::queue($writer, $command),
                "crossterm: ✅ Succeeded",
                "crossterm: ❌ Failed"
            );
        )*
    }};
}

#[macro_export]
macro_rules! flush_now {
    ($writer: expr, $arg_log_msg: expr) => {{
        $crate::crossterm_op!(
            $arg_log_msg,
            $writer.flush(),
            "crossterm: ✅ Succeeded",
            "crossterm: ❌ Failed"
        );
    }};
}

#[macro_export]
macro_rules! disable_raw_mode_now {
    (
        $arg_is_mock: expr,
        $arg_log_msg: expr
    ) => {{
        $crate::crossterm_op!(
            $arg_is_mock,
            $arg_log_msg,
            crossterm::terminal::disable_raw_mode(),
            "crossterm: ✅ Succeeded",
            "crossterm: ❌ Failed"
        );
    }};
}

#[macro_export]
macro_rules! enable_raw_mode_now {
    (
        $arg_is_mock: expr,
        $arg_log_msg: expr
    ) => {{
        $crate::crossterm_op!(
            $arg_is_mock,
            $arg_log_msg,
            crossterm::terminal::enable_raw_mode(),
            "crossterm: ✅ Succeeded",
            "crossterm: ❌ Failed"
        );
    }};
}

#[macro_export]
macro_rules! crossterm_op {
    (
        $arg_is_mock:expr, // Optional mock flag.
        $arg_log_msg:expr, // Log message.
        $op:expr,          // The crossterm operation to perform.
        $success_msg:expr, // Success log message.
        $error_msg:expr    // Error log message.
    ) => {{
        use $crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;

        // Mock mode is handled at the OutputDevice level.
        // This macro always executes the operation; the I/O boundary decides whether
        // output is actually written.
        let _ = $arg_is_mock;

        match $op {
            Ok(_) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info!(
                        message = $success_msg,
                        details = %$arg_log_msg
                    );
                });
            }
            Err(err) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = $error_msg,
                        details = %$arg_log_msg,
                        error = %err,
                    );
                });
            }
        }
    }};
    (
        $arg_log_msg:expr, // Log message.
        $op:expr,          // The crossterm operation to perform.
        $success_msg:expr, // Success log message.
        $error_msg:expr    // Error log message.
    ) => {{
        use $crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;

        match $op {
            Ok(_) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info!(
                        message = $success_msg,
                        details = %$arg_log_msg
                    );
                });
            }
            Err(err) => {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = $error_msg,
                        details = %$arg_log_msg,
                        error = %err,
                    );
                });
            }
        }
    }};
}
