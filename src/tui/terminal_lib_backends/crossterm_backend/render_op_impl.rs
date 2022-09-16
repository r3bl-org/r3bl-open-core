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
      RenderOp::PrintTextWithAttributes(text, maybe_style) => {
        print_text_with_attributes(text, maybe_style, shared_tw_data).await;
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
  let Position { col, row } =
    pipeline::sanitize_and_save_abs_position(*abs_pos, shared_tw_data).await;
  exec_render_op!(
    queue!(stdout(), MoveTo(*col, *row)),
    format!("MoveCursorPosition(col: {}, row: {})", *col, *row)
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
  let sanitized_pos = pipeline::sanitize_and_save_abs_position(*pos, shared_tw_data).await;
  let Position { col, row } = sanitized_pos;
  exec_render_op!(
    queue!(stdout(), MoveTo(*col, *row), Show),
    format!("ShowCaretAt -> MoveTo(col: {}, row: {}) & Show", *col, *row)
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

enum TruncationPolicy {
  ANSIText,
  PlainText,
}

async fn print_text_with_attributes(
  text_arg: &String, maybe_style: &Option<Style>, shared_tw_data: &SharedTWData,
) {
  // Are ANSI codes present?
  let truncation_policy = {
    if try_strip_ansi(text_arg).is_some() {
      TruncationPolicy::ANSIText
    } else {
      TruncationPolicy::PlainText
    }
  };

  // Gen log_msg.
  let mut log_msg = match truncation_policy {
    TruncationPolicy::PlainText => {
      format!("\"{}\"", text_arg)
    }
    TruncationPolicy::ANSIText => {
      call_if_true!(
        DEBUG_SHOW_PIPELINE_EXPANDED,
        log_no_err!(
          DEBUG,
          "ANSI {:?}, len: {:?}, plain: {:?}",
          text_arg,
          text_arg.len(),
          try_strip_ansi(text_arg)
        )
      );
      format!("ANSI detected, size: {} bytes", text_arg.len())
    }
  };

  let mut text: Cow<'_, str> = Cow::from(text_arg);

  // TK: handle ANSI text truncation using ANSIText
  if let TruncationPolicy::PlainText = truncation_policy {
    // Check whether the text needs to be truncated to fit the terminal window.
    let cursor_position = shared_tw_data.read().await.cursor_position;
    let max_cols = shared_tw_data.read().await.size.col;
    let plain_text_unicode_string = text_arg.to_string().unicode_string();
    let plain_text_len = plain_text_unicode_string.display_width;
    if cursor_position.col + plain_text_len > max_cols {
      let trunc_plain_text = plain_text_unicode_string
        .truncate_to_fit_display_cols(max_cols - cursor_position.col)
        .to_string();
      // Update plain_text & log_msg after truncating.
      text = Cow::from(trunc_plain_text);
      log_msg = format!("\"{}✂️\"", text);
    }
  };

  // Print plain_text.
  paint_style_and_text(maybe_style, text, &log_msg, shared_tw_data).await;
}

/// Use [Style] to set crossterm [Attributes] ([docs](
/// https://docs.rs/crossterm/latest/crossterm/style/index.html#attributes)).
async fn paint_style_and_text(
  maybe_style: &Option<Style>, text: Cow<'_, str>, log_msg: &str, shared_tw_data: &SharedTWData,
) {
  let mut needs_reset = false;
  if let Some(style) = maybe_style {
    let mask = style.clone().get_bitflags();
    STYLE_TO_ATTRIBUTE_MAP.iter().for_each(|(flag, attr)| {
      if mask.contains(*flag) {
        exec_render_op!(
          queue!(stdout(), SetAttribute(*attr)),
          format!("PrintWithAttributes -> SetAttribute({:?})", attr)
        );
        needs_reset = true;
      }
    });
  }

  paint_text(text, log_msg, shared_tw_data).await;

  if needs_reset {
    exec_render_op!(
      queue!(stdout(), SetAttribute(Attribute::Reset)),
      format!("PrintWithAttributes -> SetAttribute(Reset))")
    );
  }
}

async fn paint_text(text: Cow<'_, str>, log_msg: &str, shared_tw_data: &SharedTWData) {
  let unicode_string = text.unicode_string();
  let mut cursor_position_copy = shared_tw_data.read().await.cursor_position;

  match unicode_string.contains_wide_segments() {
    true => {
      for ref seg in unicode_string.vec_segment {
        // Special handling of cursor based on unicode width.
        if seg.unicode_width > ch!(1) {
          // Paint text.
          paint(Cow::from(&seg.string), log_msg, SegmentWidth::Wide);
          // Move cursor "manually" to cover "extra" width.
          let jump_to_col = ch!(
            @to_u16
            cursor_position_copy.col + seg.display_col_offset + seg.unicode_width);
          exec_render_op!(
            queue!(stdout(), MoveToColumn(jump_to_col)),
            format!("🦘 Jump cursor -> MoveToColumn({})", jump_to_col)
          );
        } else {
          // Paint text.
          paint(Cow::from(&seg.string), log_msg, SegmentWidth::Narrow);
        }
      }
    }
    false => {
      // Simple print operation.
      paint(text, log_msg, SegmentWidth::Narrow);
    }
  }

  enum SegmentWidth {
    Narrow,
    Wide,
  }

  fn paint(text: Cow<'_, str>, log_msg: &str, width: SegmentWidth) {
    exec_render_op!(
      queue!(stdout(), Print(text)),
      match width {
        SegmentWidth::Narrow => format!("Print( normal_segment {})", log_msg),
        SegmentWidth::Wide => format!("Print( wide_segment {})", log_msg),
      }
    );
  }

  // Update cursor position after paint.
  cursor_position_copy.col += unicode_string.display_width;
  pipeline::sanitize_and_save_abs_position(cursor_position_copy, shared_tw_data).await;
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
