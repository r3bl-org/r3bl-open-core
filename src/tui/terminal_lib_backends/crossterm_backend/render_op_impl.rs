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

use crate::*;

pub struct RenderOpImplCrossterm;

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

fn flush() {
  exec_render_op!(stdout().flush(), "flush() -> stdout");
  exec_render_op!(stderr().flush(), "flush() -> stderr");
}

#[async_trait]
impl PaintRenderOp for RenderOpImplCrossterm {
  async fn paint(
    &self, skip_flush: &mut bool, command_ref: &RenderOp, shared_tw_data: &SharedTWData,
  ) {
    match command_ref {
      RenderOp::Noop => {}
      RenderOp::RequestShowCaretAtPositionAbs(pos) => {
        request_show_caret_at_position_abs(pos, shared_tw_data).await;
      }
      RenderOp::RequestShowCaretAtPositionRelTo(box_origin_pos, content_rel_pos) => {
        request_show_caret_at_position_rel_to(box_origin_pos, content_rel_pos, shared_tw_data)
          .await;
      }
      RenderOp::EnterRawMode => {
        raw_mode_enter(skip_flush, shared_tw_data).await;
      }
      RenderOp::ExitRawMode => {
        raw_mode_exit(skip_flush);
      }
      RenderOp::MoveCursorPositionAbs(abs_pos) => {
        move_cursor_position_abs(abs_pos, shared_tw_data).await;
      }
      RenderOp::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
        move_cursor_position_rel_to(box_origin_pos, content_rel_pos, shared_tw_data).await;
      }
      RenderOp::ClearScreen => {
        exec_render_op!(queue!(stdout(), Clear(ClearType::All)), "ClearScreen")
      }
      RenderOp::SetFgColor(color) => {
        set_fg_color(color);
      }
      RenderOp::SetBgColor(color) => {
        set_bg_color(color);
      }
      RenderOp::ResetColor => {
        exec_render_op!(queue!(stdout(), ResetColor), "ResetColor")
      }
      RenderOp::CursorShow => {
        exec_render_op!(queue!(stdout(), Show), "CursorShow")
      }
      RenderOp::CursorHide => {
        exec_render_op!(queue!(stdout(), Hide), "CursorHide")
      }
      RenderOp::ApplyColors(style) => {
        apply_colors(style);
      }
      RenderOp::PrintANSITextWithAttributes(text, maybe_style) => {
        print_ansi_text_with_attributes(text, maybe_style);
      }
      RenderOp::PrintPlainTextWithAttributes(text, maybe_style) => {
        print_plain_text_with_attributes(text, maybe_style, shared_tw_data).await;
      }
    }
  }
}

async fn move_cursor_position_rel_to(
  box_origin_pos: &Position, content_rel_pos: &Position, shared_tw_data: &SharedTWData,
) {
  let new_abs_pos = *box_origin_pos + *content_rel_pos;
  move_cursor_position_abs(&new_abs_pos, shared_tw_data).await;
}

async fn move_cursor_position_abs(abs_pos: &Position, shared_tw_data: &SharedTWData) {
  let Position { col, row } = pipeline::sanitize_abs_position(*abs_pos, shared_tw_data).await;
  exec_render_op!(
    queue!(stdout(), MoveTo(col, row)),
    format!("MoveCursorPosition(col: {}, row: {})", col, row)
  )
}

fn raw_mode_exit(skip_flush: &mut bool) {
  exec_render_op! {
    queue!(stdout(),
      Show,
      LeaveAlternateScreen,
      DisableMouseCapture
    ),
    "ExitRawMode -> Show, LeaveAlternateScreen, DisableMouseCapture"
  };
  flush();
  exec_render_op! {terminal::disable_raw_mode(), "ExitRawMode -> disable_raw_mode()"}
  *skip_flush = true;
}

async fn raw_mode_enter(skip_flush: &mut bool, shared_tw_data: &SharedTWData) {
  shared_tw_data.write().await.cursor_position = position! {col: 0, row: 0};
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
  flush();
  *skip_flush = true;
}

async fn request_show_caret_at_position_rel_to(
  box_origin_pos: &Position, content_rel_pos: &Position, shared_tw_data: &SharedTWData,
) {
  let new_abs_pos = *box_origin_pos + *content_rel_pos;
  request_show_caret_at_position_abs(&new_abs_pos, shared_tw_data).await;
}

async fn request_show_caret_at_position_abs(pos: &Position, shared_tw_data: &SharedTWData) {
  let sanitized_pos = pipeline::sanitize_abs_position(*pos, shared_tw_data).await;
  let Position { col, row } = sanitized_pos;
  exec_render_op!(
    queue!(stdout(), MoveTo(col, row), Show),
    format!("ShowCaretAt -> MoveTo(col: {}, row: {}) & Show", col, row)
  );
}

fn set_fg_color(color: &TWColor) {
  let color = color_converter::to_crossterm_color(*color);
  exec_render_op!(
    queue!(stdout(), SetForegroundColor(color)),
    format!("SetFgColor({:?})", color)
  )
}

fn set_bg_color(color: &TWColor) {
  let color: crossterm::style::Color = color_converter::to_crossterm_color(*color);
  exec_render_op!(
    queue!(stdout(), SetBackgroundColor(color)),
    format!("SetBgColor({:?})", color)
  )
}

async fn print_plain_text_with_attributes(
  text_arg: &String, maybe_style: &Option<Style>, shared_tw_data: &SharedTWData,
) {
  // Try and strip ansi codes & prepare the log message.
  let mut plain_text: Cow<'_, str> = Cow::Borrowed(text_arg);
  let maybe_stripped_text = try_strip_ansi(text_arg);
  let log_msg: String = match maybe_stripped_text {
    Some(ref stripped_text) => {
      plain_text = Cow::Borrowed(stripped_text);
      format!("\"{}\"", stripped_text)
    }
    None => format!("bytes {}", text_arg.len()),
  };

  // Check whether the plain_text needs to be truncated to fit the terminal window.
  let cursor_position = shared_tw_data.read().await.cursor_position;
  let max_cols = shared_tw_data.read().await.size.col;
  let plain_text_unicode_string = plain_text.to_string().unicode_string();
  let plain_text_len = plain_text_unicode_string.display_width;
  if cursor_position.col + plain_text_len > max_cols {
    let trunc_plain_text = plain_text_unicode_string
      .truncate_to_fit_display_cols(max_cols - cursor_position.col)
      .to_string();
    plain_text = Cow::Owned(trunc_plain_text);
  }

  // Print plain_text.
  match maybe_style {
    Some(style) => {
      paint_with_style(style, plain_text, log_msg);
    }
    None => {
      paint_no_style(plain_text, log_msg);
    }
  }
}

fn print_ansi_text_with_attributes(text_arg: &String, maybe_style: &Option<Style>) {
  // Try and strip ansi codes & prepare the log message.
  let ansi_text: Cow<'_, str> = Cow::Borrowed(text_arg);
  let maybe_stripped_text = try_strip_ansi(text_arg);
  let log_msg: String = match maybe_stripped_text {
    Some(ref stripped_text) => {
      format!("\"{}\"", stripped_text)
    }
    None => format!("bytes {}", text_arg.len()),
  };

  // Print plain_text.
  match maybe_style {
    Some(style) => {
      paint_with_style(style, ansi_text, log_msg);
    }
    None => {
      paint_no_style(ansi_text, log_msg);
    }
  }
}

fn paint_no_style(plain_text: Cow<'_, str>, log_msg: String) {
  exec_render_op!(
    queue!(stdout(), Print(plain_text)),
    format!("PrintWithAttributes -> None + Print({})", log_msg)
  )
}

/// Use [Style] to set crossterm [Attributes].
/// Docs: https://docs.rs/crossterm/latest/crossterm/style/index.html#attributes
fn paint_with_style(style: &Style, plain_text: Cow<'_, str>, log_msg: String) {
  let mask = style.clone().get_bitflags();
  let mut needs_reset = false;
  STYLE_TO_ATTRIBUTE_MAP.iter().for_each(|(flag, attr)| {
    if mask.contains(*flag) {
      exec_render_op!(
        queue!(stdout(), SetAttribute(*attr)),
        format!("PrintWithAttributes -> SetAttribute({:?})", attr)
      );
      needs_reset = true;
    }
  });
  exec_render_op!(
    queue!(stdout(), Print(plain_text)),
    format!("PrintWithAttributes -> Style + Print({})", log_msg)
  );
  if needs_reset {
    exec_render_op!(
      queue!(stdout(), SetAttribute(Attribute::Reset)),
      format!("PrintWithAttributes -> SetAttribute(Reset))")
    );
  }
}

fn apply_colors(style: &Option<Style>) {
  if style.is_some() {
    // Use Style to set crossterm Colors.
    // Docs: https://docs.rs/crossterm/latest/crossterm/style/index.html#colors
    let mut style = style.clone().unwrap();
    let mask = style.get_bitflags();
    if mask.contains(StyleFlag::COLOR_BG_SET) {
      let color_bg = style.color_bg.unwrap();
      let color_bg: crossterm::style::Color = color_converter::to_crossterm_color(color_bg);
      exec_render_op!(
        queue!(stdout(), SetBackgroundColor(color_bg)),
        format!("ApplyColors -> SetBackgroundColor({:?})", color_bg)
      )
    }
    if mask.contains(StyleFlag::COLOR_FG_SET) {
      let color_fg = style.color_fg.unwrap();
      let color_fg: crossterm::style::Color = color_converter::to_crossterm_color(color_fg);
      exec_render_op!(
        queue!(stdout(), SetForegroundColor(color_fg)),
        format!("ApplyColors -> SetForegroundColor({:?})", color_fg)
      )
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Style to attribute map │
// ╯                        ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
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
