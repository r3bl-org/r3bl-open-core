/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */
use std::borrow::Cow;

use crossterm::{self,
                cursor::{Hide, MoveTo, Show},
                event::{DisableMouseCapture, EnableMouseCapture},
                style::{Attribute,
                        Print,
                        ResetColor,
                        SetAttribute,
                        SetBackgroundColor,
                        SetForegroundColor},
                terminal::{Clear,
                           ClearType,
                           EnterAlternateScreen,
                           LeaveAlternateScreen}};
use smallvec::smallvec;

use crate::{crossterm_color_converter::convert_from_tui_color_to_crossterm_color,
            disable_raw_mode_now,
            enable_raw_mode_now,
            flush_now,
            queue_render_op,
            sanitize_and_save_abs_pos,
            Flush,
            GCString,
            InlineVec,
            LockedOutputDevice,
            PaintRenderOp,
            Pos,
            RenderOp,
            RenderOpsLocalData,
            Size,
            TuiColor,
            TuiStyle};

/// Struct representing the implementation of [RenderOp] for crossterm terminal backend.
/// This empty struct is needed since the [Flush] trait needs to be implemented.
pub struct RenderOpImplCrossterm;

mod impl_trait_paint_render_op {
    use super::*;

    impl PaintRenderOp for RenderOpImplCrossterm {
        fn paint(
            &mut self,
            skip_flush: &mut bool,
            command_ref: &RenderOp,
            window_size: Size,
            local_data: &mut RenderOpsLocalData,
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
                        local_data,
                        locked_output_device,
                    );
                }
                RenderOp::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
                    RenderOpImplCrossterm::move_cursor_position_rel_to(
                        *box_origin_pos,
                        *content_rel_pos,
                        window_size,
                        local_data,
                        locked_output_device,
                    );
                }
                RenderOp::ClearScreen => {
                    queue_render_op!(
                        locked_output_device,
                        "ClearScreen",
                        Clear(ClearType::All),
                    )
                }
                RenderOp::SetFgColor(color) => {
                    RenderOpImplCrossterm::set_fg_color(*color, locked_output_device);
                }
                RenderOp::SetBgColor(color) => {
                    RenderOpImplCrossterm::set_bg_color(*color, locked_output_device);
                }
                RenderOp::ResetColor => {
                    queue_render_op!(locked_output_device, "ResetColor", ResetColor)
                }
                RenderOp::ApplyColors(style) => {
                    RenderOpImplCrossterm::apply_colors(style, locked_output_device);
                }
                RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                    text,
                    maybe_style,
                ) => {
                    RenderOpImplCrossterm::paint_text_with_attributes(
                        text,
                        maybe_style,
                        window_size,
                        local_data,
                        locked_output_device,
                    );
                }
                RenderOp::PaintTextWithAttributes(_text, _maybe_style) => {
                    // This should never be executed! The compositor always renders to an
                    // offscreen buffer first, then that is diff'd and
                    // then painted via calls to
                    // CompositorNoClipTruncPaintTextWithAttributes.
                }
            }
        }
    }
}

pub mod impl_trait_flush {
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
    use super::*;

    impl RenderOpImplCrossterm {
        pub fn move_cursor_position_rel_to(
            box_origin_pos: Pos,
            content_rel_pos: Pos,
            window_size: Size,
            local_data: &mut RenderOpsLocalData,
            locked_output_device: LockedOutputDevice<'_>,
        ) {
            let new_abs_pos = box_origin_pos + content_rel_pos;
            Self::move_cursor_position_abs(
                new_abs_pos,
                window_size,
                local_data,
                locked_output_device,
            );
        }

        pub fn move_cursor_position_abs(
            abs_pos: Pos,
            window_size: Size,
            local_data: &mut RenderOpsLocalData,
            locked_output_device: LockedOutputDevice<'_>,
        ) {
            let Pos {
                col_index,
                row_index,
            } = sanitize_and_save_abs_pos(abs_pos, window_size, local_data);

            let col = col_index.as_u16();
            let row = row_index.as_u16();

            queue_render_op!(
                locked_output_device,
                format!("MoveCursorPosition(col: {:?}, row: {:?})", col, row),
                MoveTo(col, row)
            )
        }

        pub fn raw_mode_exit(
            skip_flush: &mut bool,
            locked_output_device: LockedOutputDevice<'_>,
            is_mock: bool,
        ) {
            queue_render_op!(
                locked_output_device,
                "ExitRawMode -> Show, LeaveAlternateScreen, DisableMouseCapture",
                Show,
                LeaveAlternateScreen,
                DisableMouseCapture
            );

            flush_now!(locked_output_device, "ExitRawMode -> flush()");

            disable_raw_mode_now!(is_mock, "ExitRawMode -> disable_raw_mode()");

            *skip_flush = true;
        }

        pub fn raw_mode_enter(
            skip_flush: &mut bool,
            locked_output_device: LockedOutputDevice<'_>,
            is_mock: bool,
        ) {
            enable_raw_mode_now!(is_mock, "EnterRawMode -> enable_raw_mode()");

            queue_render_op!(
                locked_output_device,
                "EnterRawMode -> EnableMouseCapture, EnterAlternateScreen, MoveTo(0,0), Clear(ClearType::All), Hide",
                EnableMouseCapture,
                EnterAlternateScreen,
                MoveTo(0,0),
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
            let color = convert_from_tui_color_to_crossterm_color(color);

            queue_render_op!(
                locked_output_device,
                format!("SetFgColor({color:?})"),
                SetForegroundColor(color),
            );
        }

        pub fn set_bg_color(
            color: TuiColor,
            locked_output_device: LockedOutputDevice<'_>,
        ) {
            let color: crossterm::style::Color =
                convert_from_tui_color_to_crossterm_color(color);

            queue_render_op!(
                locked_output_device,
                format!("SetBgColor({color:?})"),
                SetBackgroundColor(color),
            )
        }

        pub fn paint_text_with_attributes(
            text_arg: &str,
            maybe_style: &Option<TuiStyle>,
            window_size: Size,
            local_data: &mut RenderOpsLocalData,
            locked_output_device: LockedOutputDevice<'_>,
        ) {
            use perform_paint::{paint_style_and_text, PaintArgs};

            // Gen log_msg.
            let log_msg = Cow::from(format!("\"{text_arg}\""));

            let text: Cow<'_, str> = Cow::from(text_arg);

            let mut paint_args = PaintArgs {
                text,
                log_msg,
                maybe_style,
                window_size,
            };

            let needs_reset = Cow::Owned(false);

            // Paint plain_text.
            paint_style_and_text(
                &mut paint_args,
                needs_reset,
                local_data,
                locked_output_device,
            );
        }

        /// Use [crossterm::style::Color] to set crossterm Colors.
        /// Docs: <https://docs.rs/crossterm/latest/crossterm/style/index.html#colors>
        pub fn apply_colors(
            maybe_style: &Option<TuiStyle>,
            locked_output_device: LockedOutputDevice<'_>,
        ) {
            if let Some(style) = maybe_style {
                // Handle background color.
                if let Some(tui_color_bg) = style.color_bg {
                    let color_bg: crossterm::style::Color =
                        crate::convert_from_tui_color_to_crossterm_color(tui_color_bg);

                    queue_render_op!(
                        locked_output_device,
                        format!("ApplyColors -> SetBgColor({color_bg:?})"),
                        SetBackgroundColor(color_bg),
                    );
                }

                // Handle foreground color.
                if let Some(tui_color_fg) = style.color_fg {
                    let color_fg: crossterm::style::Color =
                        crate::convert_from_tui_color_to_crossterm_color(tui_color_fg);

                    queue_render_op!(
                        locked_output_device,
                        format!("ApplyColors -> SetFgColor({color_fg:?})"),
                        SetForegroundColor(color_fg),
                    );
                }
            }
        }
    }
}

mod perform_paint {
    use super::*;

    #[derive(Debug)]
    pub struct PaintArgs<'a> {
        pub text: Cow<'a, str>,
        pub log_msg: Cow<'a, str>,
        pub maybe_style: &'a Option<TuiStyle>,
        pub window_size: Size,
    }

    fn style_to_attribute(&style: &TuiStyle) -> InlineVec<Attribute> {
        let mut it = smallvec![];
        if style.bold.is_some() {
            it.push(Attribute::Bold);
        }
        if style.italic.is_some() {
            it.push(Attribute::Italic);
        }
        if style.dim.is_some() {
            it.push(Attribute::Dim);
        }
        if style.underline.is_some() {
            it.push(Attribute::Underlined);
        }
        if style.reverse.is_some() {
            it.push(Attribute::Reverse);
        }
        if style.hidden.is_some() {
            it.push(Attribute::Hidden);
        }
        if style.strikethrough.is_some() {
            it.push(Attribute::Fraktur);
        }
        it
    }

    /// Use [Style] to set crossterm [Attributes] ([docs](
    /// https://docs.rs/crossterm/latest/crossterm/style/index.html#attributes)).
    pub fn paint_style_and_text(
        paint_args: &mut PaintArgs<'_>,
        mut needs_reset: Cow<'_, bool>,
        local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        let PaintArgs { maybe_style, .. } = paint_args;

        if let Some(style) = maybe_style {
            let attrib_vec = style_to_attribute(style);
            attrib_vec.iter().for_each(|attr| {
                queue_render_op!(
                    locked_output_device,
                    format!("PaintWithAttributes -> SetAttribute({attr:?})"),
                    SetAttribute(*attr),
                );
                needs_reset = Cow::Owned(true);
            });
        }

        paint_text(paint_args, local_data, locked_output_device);

        if *needs_reset {
            queue_render_op!(
                locked_output_device,
                format!("PaintWithAttributes -> SetAttribute(Reset))"),
                SetAttribute(Attribute::Reset),
            );
        }
    }

    pub fn paint_text(
        paint_args: &PaintArgs<'_>,
        local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
    ) {
        let PaintArgs {
            text,
            log_msg,
            window_size,
            ..
        } = paint_args;

        // Actually paint text.
        {
            let text = Cow::Borrowed(text);
            let log_msg: &str = log_msg;
            queue_render_op!(
                locked_output_device,
                format!("Print( {} {log_msg})", &text),
                Print(&text),
            );
        };

        // Update cursor position after paint.
        let cursor_pos_copy = {
            let mut copy = local_data.cursor_pos;
            let text_display_width = GCString::width(text);
            *copy.col_index += *text_display_width;
            copy
        };
        sanitize_and_save_abs_pos(cursor_pos_copy, *window_size, local_data);
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
