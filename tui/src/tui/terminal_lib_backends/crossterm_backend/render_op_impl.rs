/*
 *   Copyright (c) 2022 R3BL LLC
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

use std::{borrow::Cow,
          collections::HashMap,
          io::{stderr, stdout, Write}};

use async_trait::async_trait;
use crossterm::{cursor::*,
                event::*,
                queue,
                style::{Attribute, *},
                terminal::{self, *}};
use once_cell::sync::Lazy;
use r3bl_rs_utils_core::*;

use crate::*;

/// Struct representing the implementation of [RenderOp] for crossterm terminal backend. This empty
/// struct is needed since the [Flush] trait needs to be implemented.
pub struct RenderOpImplCrossterm;

mod render_op_impl_crossterm_impl_trait_paint_render_op {
    use super::*;

    #[async_trait]
    impl PaintRenderOp for RenderOpImplCrossterm {
        async fn paint(
            &mut self,
            skip_flush: &mut bool,
            command_ref: &RenderOp,
            shared_global_data: &SharedGlobalData,
            local_data: &mut RenderOpsLocalData,
        ) {
            match command_ref {
                RenderOp::Noop => {}
                RenderOp::EnterRawMode => {
                    RenderOpImplCrossterm::raw_mode_enter(skip_flush, shared_global_data).await;
                }
                RenderOp::ExitRawMode => {
                    RenderOpImplCrossterm::raw_mode_exit(skip_flush);
                }
                RenderOp::MoveCursorPositionAbs(abs_pos) => {
                    RenderOpImplCrossterm::move_cursor_position_abs(
                        abs_pos,
                        shared_global_data,
                        local_data,
                    )
                    .await;
                }
                RenderOp::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
                    RenderOpImplCrossterm::move_cursor_position_rel_to(
                        box_origin_pos,
                        content_rel_pos,
                        shared_global_data,
                        local_data,
                    )
                    .await;
                }
                RenderOp::ClearScreen => {
                    exec_render_op!(queue!(stdout(), Clear(ClearType::All)), "ClearScreen")
                }
                RenderOp::SetFgColor(color) => {
                    RenderOpImplCrossterm::set_fg_color(color);
                }
                RenderOp::SetBgColor(color) => {
                    RenderOpImplCrossterm::set_bg_color(color);
                }
                RenderOp::ResetColor => {
                    exec_render_op!(queue!(stdout(), ResetColor), "ResetColor")
                }
                RenderOp::ApplyColors(style) => {
                    RenderOpImplCrossterm::apply_colors(style);
                }
                RenderOp::CompositorNoClipTruncPaintTextWithAttributes(text, maybe_style) => {
                    RenderOpImplCrossterm::paint_text_with_attributes(
                        text,
                        maybe_style,
                        shared_global_data,
                        local_data,
                    )
                    .await;
                }
                RenderOp::PaintTextWithAttributes(_text, _maybe_style) => {
                    // This should never be executed! The compositor always renders to an offscreen
                    // buffer first, then that is diff'd and then painted via calls to
                    // CompositorNoClipTruncPaintTextWithAttributes.
                }
            }
        }
    }
}

pub mod render_op_impl_crossterm_impl_trait_flush {
    use super::*;

    impl Flush for RenderOpImplCrossterm {
        fn flush(&mut self) { flush(); }
        fn clear_before_flush(&mut self) { clear_before_flush(); }
    }

    fn clear_before_flush() {
        exec_render_op! {
          queue!(stdout(),
            ResetColor,
            Clear(ClearType::All),
          ),
        "flush() -> after ResetColor, Clear"
        }
    }

    pub fn flush() {
        exec_render_op!(stdout().flush(), "flush() -> stdout");
        exec_render_op!(stderr().flush(), "flush() -> stderr");
    }
}

mod render_op_impl_crossterm_impl {
    use super::*;

    impl RenderOpImplCrossterm {
        pub async fn move_cursor_position_rel_to(
            box_origin_pos: &Position,
            content_rel_pos: &Position,
            shared_global_data: &SharedGlobalData,
            local_data: &mut RenderOpsLocalData,
        ) {
            let new_abs_pos = *box_origin_pos + *content_rel_pos;
            Self::move_cursor_position_abs(&new_abs_pos, shared_global_data, local_data).await;
        }

        pub async fn move_cursor_position_abs(
            abs_pos: &Position,
            shared_global_data: &SharedGlobalData,
            local_data: &mut RenderOpsLocalData,
        ) {
            let Position {
                col_index: col,
                row_index: row,
            } = sanitize_and_save_abs_position(*abs_pos, shared_global_data, local_data).await;
            exec_render_op!(
                queue!(stdout(), MoveTo(*col, *row)),
                format!("MoveCursorPosition(col: {}, row: {})", *col, *row)
            )
        }

        pub fn raw_mode_exit(skip_flush: &mut bool) {
            exec_render_op! {
              queue!(stdout(),
                Show,
                LeaveAlternateScreen,
                DisableMouseCapture
              ),
              "ExitRawMode -> Show, LeaveAlternateScreen, DisableMouseCapture"
            };
            render_op_impl_crossterm_impl_trait_flush::flush();
            exec_render_op! {terminal::disable_raw_mode(), "ExitRawMode -> disable_raw_mode()"}
            *skip_flush = true;
        }

        pub async fn raw_mode_enter(skip_flush: &mut bool, _shared_global_data: &SharedGlobalData) {
            exec_render_op! {
              terminal::enable_raw_mode(),
              "EnterRawMode -> enable_raw_mode()"
            };
            exec_render_op! {
              queue!(stdout(),
                EnableMouseCapture,
                EnterAlternateScreen,
                MoveTo(0,0),
                Clear(ClearType::All),
                Hide,
              ),
            "EnterRawMode -> EnableMouseCapture, EnterAlternateScreen, MoveTo(0,0), Clear(ClearType::All), Hide"
            }
            render_op_impl_crossterm_impl_trait_flush::flush();
            *skip_flush = true;
        }

        pub fn set_fg_color(color: &TuiColor) {
            let color = color_converter::to_crossterm_color(*color);
            exec_render_op!(
                queue!(stdout(), SetForegroundColor(color)),
                format!("SetFgColor({color:?})")
            )
        }

        pub fn set_bg_color(color: &TuiColor) {
            let color: crossterm::style::Color = color_converter::to_crossterm_color(*color);
            exec_render_op!(
                queue!(stdout(), SetBackgroundColor(color)),
                format!("SetBgColor({color:?})")
            )
        }

        pub async fn paint_text_with_attributes(
            text_arg: &String,
            maybe_style: &Option<Style>,
            shared_global_data: &SharedGlobalData,
            local_data: &mut RenderOpsLocalData,
        ) {
            use perform_paint::*;

            // Are ANSI codes present?
            let it = shared_global_data
                .write()
                .await
                .get_from_cache_try_strip_ansi_text(text_arg);
            let content_type = {
                if it.is_some() {
                    ContentType::ANSIText
                } else {
                    ContentType::PlainText
                }
            };

            // Gen log_msg.
            let log_msg = Cow::from(match content_type {
                ContentType::PlainText => {
                    format!("\"{text_arg}\"")
                }
                ContentType::ANSIText => {
                    call_if_true!(DEBUG_TUI_SHOW_PIPELINE_EXPANDED, {
                        let it = shared_global_data
                            .write()
                            .await
                            .get_from_cache_try_strip_ansi_text(text_arg);

                        let msg = format!(
                            "ANSI {:?}, len: {:?}, plain: {:?}",
                            text_arg,
                            text_arg.len(),
                            it
                        );
                        log_debug(msg);
                    });
                    format!("ANSI detected, size: {} bytes", text_arg.len())
                }
            });

            let text: Cow<'_, str> = Cow::from(text_arg);

            let mut paint_args = PaintArgs {
                text,
                log_msg,
                maybe_style,
                shared_global_data,
                content_type,
            };

            let mut needs_reset = false;

            // Paint plain_text.
            paint_style_and_text(&mut paint_args, &mut needs_reset, local_data).await;
        }

        pub fn apply_colors(style: &Option<Style>) {
            if style.is_some() {
                // Use Style to set crossterm Colors.
                // Docs: https://docs.rs/crossterm/latest/crossterm/style/index.html#colors
                let mut style = (*style).unwrap();
                let mask = style.get_bitflags();
                if mask.contains(StyleFlag::COLOR_BG_SET) {
                    let color_bg = style.color_bg.unwrap();
                    let color_bg: crossterm::style::Color =
                        color_converter::to_crossterm_color(color_bg);
                    exec_render_op!(
                        queue!(stdout(), SetBackgroundColor(color_bg)),
                        format!("ApplyColors -> SetBgColor({color_bg:?})")
                    )
                }
                if mask.contains(StyleFlag::COLOR_FG_SET) {
                    let color_fg = style.color_fg.unwrap();
                    let color_fg: crossterm::style::Color =
                        color_converter::to_crossterm_color(color_fg);
                    exec_render_op!(
                        queue!(stdout(), SetForegroundColor(color_fg)),
                        format!("ApplyColors -> SetFgColor({color_fg:?})")
                    )
                }
            }
        }
    }
}

mod perform_paint {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub enum ContentType {
        ANSIText,
        PlainText,
    }

    #[derive(Debug)]
    pub struct PaintArgs<'a> {
        pub text: Cow<'a, str>,
        pub log_msg: Cow<'a, str>,
        pub maybe_style: &'a Option<Style>,
        pub shared_global_data: &'a SharedGlobalData,
        pub content_type: ContentType,
    }

    /// Use [Style] to set crossterm [Attributes] ([docs](
    /// https://docs.rs/crossterm/latest/crossterm/style/index.html#attributes)).
    pub async fn paint_style_and_text<'a>(
        paint_args: &mut PaintArgs<'a>,
        needs_reset: &mut bool,
        local_data: &mut RenderOpsLocalData,
    ) {
        let PaintArgs { maybe_style, .. } = paint_args;

        if let Some(style) = maybe_style {
            let mask = style.clone().get_bitflags();
            STYLE_TO_ATTRIBUTE_MAP.iter().for_each(|(flag, attr)| {
                if mask.contains(*flag) {
                    exec_render_op!(
                        queue!(stdout(), SetAttribute(*attr)),
                        format!("PaintWithAttributes -> SetAttribute({attr:?})")
                    );
                    *needs_reset = true;
                }
            });
        }

        paint_text(paint_args, local_data).await;

        if *needs_reset {
            exec_render_op!(
                queue!(stdout(), SetAttribute(Attribute::Reset)),
                format!("PaintWithAttributes -> SetAttribute(Reset))")
            );
        }
    }

    pub async fn paint_text<'a>(paint_args: &PaintArgs<'a>, local_data: &mut RenderOpsLocalData) {
        let PaintArgs {
            text,
            log_msg,
            shared_global_data,
            content_type,
            ..
        } = paint_args;

        let unicode_string: UnicodeString = text.as_ref().into();
        let mut cursor_position_copy = local_data.cursor_position;

        // Actually paint text.
        {
            let text = Cow::Borrowed(text);
            let log_msg: &str = log_msg;
            exec_render_op!(
                queue!(stdout(), Print(&text)),
                format!("Print( {} {log_msg})", &text)
            );
        };

        // Update cursor position after paint.
        let display_width = match content_type {
            ContentType::ANSIText => {
                let ansi_text = text.ansi_text();
                let ansi_text_segments = ansi_text.filter_segments_by_display_width(None);
                let unicode_width = ansi_text_segments.display_width;
                ch!(unicode_width)
            }
            ContentType::PlainText => unicode_string.display_width,
        };

        cursor_position_copy.col_index += display_width;
        sanitize_and_save_abs_position(cursor_position_copy, shared_global_data, local_data).await;
    }
}

pub static STYLE_TO_ATTRIBUTE_MAP: Lazy<HashMap<StyleFlag, Attribute>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(StyleFlag::BOLD_SET, Attribute::Bold);
    map.insert(StyleFlag::DIM_SET, Attribute::Dim);
    map.insert(StyleFlag::UNDERLINE_SET, Attribute::Underlined);
    map.insert(StyleFlag::REVERSE_SET, Attribute::Reverse);
    map.insert(StyleFlag::HIDDEN_SET, Attribute::Hidden);
    map.insert(StyleFlag::STRIKETHROUGH_SET, Attribute::Fraktur);
    map
});

/// Given a crossterm command, this will run it and [log_error] or [log_info] the [Result] that is
/// returned.
///
/// Paste docs: <https://github.com/dtolnay/paste>
#[macro_export]
macro_rules! exec_render_op {
    (
    $arg_cmd: expr,
    $arg_log_msg: expr
  ) => {{
        // Generate a new function that returns [CommonResult]. This needs to be called. The only
        // purpose of this generated method is to handle errors that may result from calling log! macro
        // when there are issues accessing the log file for whatever reason.
        use $crate::tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND;

        let _fn_wrap_for_logging_err = || -> CommonResult<()> {
            throws!({
                // Execute the command.
                if let Err(err) = $arg_cmd {
                    let msg = format!("crossterm: ❌ Failed to {} due to {}", $arg_log_msg, err);
                    call_if_true!(
                        DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                        log_error(msg)
                    );
                } else {
                    let msg = format!("crossterm: ✅ {} successfully", $arg_log_msg);
                    call_if_true! {
                      DEBUG_TUI_SHOW_TERMINAL_BACKEND,
                      log_info(msg)
                    };
                }
            })
        };

        // Call this generated function. It will fail if there are problems w/ log!(). In this case, if
        // `DEBUG_TUI_SHOW_TERMINAL_BACKEND` is true, then it will dump the error to stderr.
        if let Err(logging_err) = _fn_wrap_for_logging_err() {
            let msg = format!(
                "❌ Failed to log exec output of {}, {}",
                stringify!($arg_cmd),
                $arg_log_msg
            );
            call_if_true! {
              DEBUG_TUI_SHOW_TERMINAL_BACKEND,
              debug!(ERROR_RAW &msg, logging_err)
            };
        }
    }};
}
