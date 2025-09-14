// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use std::borrow::Cow;

use crossterm::{cursor::{Hide, MoveTo, Show},
                event::{DisableBracketedPaste, DisableMouseCapture,
                        EnableBracketedPaste, EnableMouseCapture},
                style::{Attribute, Print, ResetColor, SetAttribute, SetBackgroundColor,
                        SetForegroundColor},
                terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen}};
use smallvec::smallvec;

use crate::{Flush, GCStringOwned, InlineVec, LockedOutputDevice, PaintRenderOp, Pos,
            RenderOp, RenderOpsLocalData, Size, TuiColor, TuiStyle,
            disable_raw_mode_now, enable_raw_mode_now, flush_now, queue_render_op,
            sanitize_and_save_abs_pos};

/// Struct representing the implementation of [`RenderOp`] for crossterm terminal backend.
/// This empty struct is needed since the [Flush] trait needs to be implemented.
#[derive(Debug)]
pub struct RenderOpImplCrossterm;

mod impl_trait_paint_render_op {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl PaintRenderOp for RenderOpImplCrossterm {
        fn paint(
            &mut self,
            skip_flush: &mut bool,
            command_ref: &RenderOp,
            window_size: Size,
            render_local_data: &mut RenderOpsLocalData,
            locked_output_device: LockedOutputDevice<'_>,
            is_mock: bool,
        ) {
            match command_ref {
                RenderOp::Noop => {}
                RenderOp::EnterRawMode => {
                    RenderOpImplCrossterm::raw_mode_enter(
                        skip_flush,
                        locked_output_device,
                        is_mock,
                    );
                }
                RenderOp::ExitRawMode => {
                    RenderOpImplCrossterm::raw_mode_exit(
                        skip_flush,
                        locked_output_device,
                        is_mock,
                    );
                }
                RenderOp::MoveCursorPositionAbs(abs_pos) => {
                    RenderOpImplCrossterm::move_cursor_position_abs(
                        *abs_pos,
                        window_size,
                        render_local_data,
                        locked_output_device,
                    );
                }
                RenderOp::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
                    RenderOpImplCrossterm::move_cursor_position_rel_to(
                        *box_origin_pos,
                        *content_rel_pos,
                        window_size,
                        render_local_data,
                        locked_output_device,
                    );
                }
                RenderOp::ClearScreen => {
                    queue_render_op!(
                        locked_output_device,
                        "ClearScreen",
                        Clear(ClearType::All),
                    );
                }
                RenderOp::SetFgColor(color) => {
                    RenderOpImplCrossterm::set_fg_color(*color, locked_output_device);
                }
                RenderOp::SetBgColor(color) => {
                    RenderOpImplCrossterm::set_bg_color(*color, locked_output_device);
                }
                RenderOp::ResetColor => {
                    queue_render_op!(locked_output_device, "ResetColor", ResetColor);
                }
                RenderOp::ApplyColors(style) => {
                    RenderOpImplCrossterm::apply_colors(*style, locked_output_device);
                }
                RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                    text,
                    maybe_style,
                ) => {
                    RenderOpImplCrossterm::paint_text_with_attributes(
                        text,
                        *maybe_style,
                        window_size,
                        render_local_data,
                        locked_output_device,
                    );
                }
                RenderOp::PaintTextWithAttributes(_text, _maybe_style) => {
                    // This should never be executed! The compositor always renders to an
                    // offscreen buffer first, then that is diff'd and then painted via
                    // calls to CompositorNoClipTruncPaintTextWithAttributes.
                }
            }
        }
    }
}

pub mod impl_trait_flush {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Flush for RenderOpImplCrossterm {
        fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
            flush_now!(locked_output_device, "flush() -> output_device");
        }

        fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>) {
            crate::queue_render_op!(
                locked_output_device,
                "flush() -> after ResetColor, Clear",
                ResetColor,
                Clear(ClearType::All),
            );
        }
    }
}

mod impl_self {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl RenderOpImplCrossterm {
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

            queue_render_op!(
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
            queue_render_op!(
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

            queue_render_op!(
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

        pub fn set_fg_color(
            color: TuiColor,
            locked_output_device: LockedOutputDevice<'_>,
        ) {
            let color = color.into();

            queue_render_op!(
                locked_output_device,
                "SetFgColor",
                SetForegroundColor(color),
            );
        }

        pub fn set_bg_color(
            color: TuiColor,
            locked_output_device: LockedOutputDevice<'_>,
        ) {
            let color: crossterm::style::Color = color.into();

            queue_render_op!(
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
            use perform_paint::{PaintArgs, paint_style_and_text};

            let text: Cow<'_, str> = Cow::from(text_arg);

            let mut paint_args = PaintArgs {
                text,
                maybe_style,
                window_size,
            };

            let needs_reset = Cow::Owned(false);

            // Paint plain_text.
            paint_style_and_text(
                &mut paint_args,
                needs_reset,
                render_local_data,
                locked_output_device,
            );
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

                    queue_render_op!(
                        locked_output_device,
                        "ApplyColors -> SetBgColor",
                        SetBackgroundColor(color_bg),
                    );
                }

                // Handle foreground color.
                if let Some(tui_color_fg) = style.color_fg {
                    let color_fg: crossterm::style::Color = tui_color_fg.into();

                    queue_render_op!(
                        locked_output_device,
                        "ApplyColors -> SetFgColor",
                        SetForegroundColor(color_fg),
                    );
                }
            }
        }
    }
}

mod perform_paint {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(Debug)]
    pub struct PaintArgs<'a> {
        pub text: Cow<'a, str>,
        pub maybe_style: Option<TuiStyle>,
        pub window_size: Size,
    }

    fn style_to_attribute(&style: &TuiStyle) -> InlineVec<Attribute> {
        let mut it = smallvec![];
        if style.attribs.bold.is_some() {
            it.push(Attribute::Bold);
        }
        if style.attribs.italic.is_some() {
            it.push(Attribute::Italic);
        }
        if style.attribs.dim.is_some() {
            it.push(Attribute::Dim);
        }
        if style.attribs.underline.is_some() {
            it.push(Attribute::Underlined);
        }
        if style.attribs.reverse.is_some() {
            it.push(Attribute::Reverse);
        }
        if style.attribs.hidden.is_some() {
            it.push(Attribute::Hidden);
        }
        if style.attribs.strikethrough.is_some() {
            it.push(Attribute::Fraktur);
        }
        it
    }

    /// Use [`crate::TuiStyle`] to set crossterm [`Attribute`]. Read more about attributes
    /// in the [crossterm docs](https://docs.rs/crossterm/latest/crossterm/style/index.html#attributes).
    pub fn paint_style_and_text(
        paint_args: &mut PaintArgs<'_>,
        mut needs_reset: Cow<'_, bool>,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        let PaintArgs { maybe_style, .. } = paint_args;

        if let Some(style) = maybe_style {
            let attrib_vec = style_to_attribute(style);
            attrib_vec.iter().for_each(|attr| {
                queue_render_op!(
                    locked_output_device,
                    "PaintWithAttributes -> SetAttribute",
                    SetAttribute(*attr),
                );
                needs_reset = Cow::Owned(true);
            });
        }

        paint_text(paint_args, render_local_data, locked_output_device);

        if *needs_reset {
            queue_render_op!(
                locked_output_device,
                "PaintWithAttributes -> SetAttribute(Reset)",
                SetAttribute(Attribute::Reset),
            );
        }
    }

    pub fn paint_text(
        paint_args: &PaintArgs<'_>,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        let PaintArgs {
            text, window_size, ..
        } = paint_args;

        // Actually paint text.
        {
            let text = Cow::Borrowed(text);
            queue_render_op!(locked_output_device, "Print", Print(&text),);
        };

        // Update cursor position after paint.
        let cursor_pos_copy = {
            let mut copy = render_local_data.cursor_pos;
            let text_display_width = GCStringOwned::from(text.as_ref()).width();
            *copy.col_index += *text_display_width;
            copy
        };
        sanitize_and_save_abs_pos(cursor_pos_copy, *window_size, render_local_data);
    }
}

#[macro_export]
macro_rules! queue_render_op {
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

        // Conditionally skip execution if mock.
        if $arg_is_mock {
            return;
        }

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
