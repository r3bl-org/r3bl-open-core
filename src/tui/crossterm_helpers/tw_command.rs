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

use std::{collections::HashMap,
          fmt::Debug,
          io::{stderr, stdout, Write},
          ops::{Add, AddAssign}};

use crossterm::{cursor::*,
                event::*,
                style::*,
                terminal::{self, *},
                *};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::*;

const DEBUG: bool = true;

// ╭┄┄┄┄┄┄┄╮
// │ exec! │
// ╯       ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Given a crossterm command, this will run it and [log!] the [Result] that is returned. If [log!]
/// fails, then it will print a message to stderr.
///
/// Paste docs: <https://github.com/dtolnay/paste>
#[macro_export]
macro_rules! exec {
  (
    $arg_cmd: expr,
    $arg_log_msg: expr
  ) => {{
    // Generate a new function that returns [CommonResult]. This needs to be called. The only
    // purpose of this generated method is to handle errors that may result from calling log! macro
    // when there are issues accessing the log file for whatever reason.
    let _fn_wrap_for_logging_err = || -> CommonResult<()> {
      throws!({
        // Execute the command.
        if let Err(err) = $arg_cmd {
          call_if_true!(
            DEBUG,
            log!(
              ERROR,
              "crossterm: ❌ Failed to {} due to {}",
              $arg_log_msg,
              err
            )
          );
        } else {
          call_if_true! {
            DEBUG,
            log!(INFO, "crossterm: ✅ {} successfully", $arg_log_msg)
          };
        }
      })
    };

    // Call this generated function. It will fail if there are problems w/ log!(). In this case, if
    // `DEBUG` is true, then it will dump the error to stderr.
    if let Err(logging_err) = _fn_wrap_for_logging_err() {
      let msg = format!(
        "❌ Failed to log exec output of {}, {}",
        stringify!($arg_cmd),
        $arg_log_msg
      );
      call_if_true! {
        DEBUG,
        debug!(ERROR_RAW &msg, logging_err)
      };
    }
  }};
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ tw_command_queue! │
// ╯                   ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This works together w/ [TWCommand] to enqueue commands, but not flush them. It will return a
/// [TWCommandQueue]. Here's an example.
///
/// ```ignore
/// let mut queue = tw_command_queue!(
///   TWCommand::ClearScreen,
///   TWCommand::ResetColor
/// ); // Returns the newly created queue.
/// ```
///
/// Another example.
///
/// ```ignore
/// let mut queue = tw_command_queue!(); // Returns the newly created queue.
/// tw_command_queue!(
///   queue push
///   TWCommand::ClearScreen,
///   TWCommand::ResetColor
/// ); // Returns nothing.
/// ```
///
/// Decl macro docs:
/// - <https://veykril.github.io/tlborm/decl-macros/macros-methodical.html#repetitions>
#[macro_export]
macro_rules! tw_command_queue {
  // Create a new queue & return it. If any ($element)* are passed, then add it to new queue.
  (
    $(                      /* Start a repetition. */
      $element:expr         /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      let mut queue = TWCommandQueue::default();
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $element replaced. */
        queue.push($element);
      )*
      queue
    }
  };
  // Add a bunch of TWCommands $element+ to the existing $queue & return nothing.
  ($queue:ident push $($element:expr),+) => {
    $(
      /* Each repeat will contain the following statement, with $element replaced. */
      $queue.push($element);
    )*
  };
  // Add a bunch of TWCommandQueues $element+ to the new queue, drop them, and return queue.
  (@join_and_drop $($element:expr),+) => {{
    let mut queue = TWCommandQueue::default();
    $(
      /* Each repeat will contain the following statement, with $element replaced. */
      queue.join_into($element);
    )*
    queue
  }};
  // New.
  (@new) => {
    TWCommandQueue::default()
  };
}

// ╭┄┄┄┄┄┄┄┄┄┄┄╮
// │ TWCommand │
// ╯           ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TWCommand {
  EnterRawMode,
  ExitRawMode,
  /// [Position] is the absolute column and row on the terminal screen. This uses
  /// [sanitize_abs_position] to clean up the given [Position].
  MoveCursorPositionAbs(Position),
  /// 1st [Position] is the origin column and row, and the 2nd [Position] is the offset column and
  /// row. They are added together to move the absolute position on the terminal screen. This uses
  /// [sanitize_abs_position] to clean up the absolute [Position].
  MoveCursorPositionRelTo(Position, Position),
  ClearScreen,
  /// Directly set the fg color for crossterm w/out using [Style].
  SetFgColor(TWColor),
  /// Directly set the bg color for crossterm w/out using [Style].
  SetBgColor(TWColor),
  ResetColor,
  /// Translate [Style] into fg and bg colors for crossterm.
  ApplyColors(Option<Style>),
  /// Translate [Style] into attributes [static@STYLE_TO_ATTRIBUTE_MAP] for crossterm (bold,
  /// underline, strikethrough, etc). The [String] argument shouldn't contained any ANSI escape
  /// sequences (since it will be stripped out in order to figure out where to clip the text to the
  /// available width of the terminal screen).
  PrintPlainTextWithAttributes(String, Option<Style>),
  /// Translate [Style] into attributes [static@STYLE_TO_ATTRIBUTE_MAP] for crossterm (bold,
  /// underline, strikethrough, etc). You are responsible for handling clipping of the text to the
  /// bounds of the terminal screen.
  PrintANSITextWithAttributes(String, Option<Style>),
  CursorShow,
  CursorHide,
  /// [Position] is the absolute column and row on the terminal screen. This uses
  /// [sanitize_abs_position] to clean up the given [Position].
  ///
  /// 1. [handle_draw_cursor] is actually used to draw the cursor.
  /// 2. [handle_maybe_draw_caret_at_overwrite_attempt] is used to ensure that this is not an
  ///    overwrite attempt.
  RequestShowCaretAtPositionAbs(Position),
  /// 1st [Position] is the origin column and row, and the 2nd [Position] is the offset column and
  /// row. They are added together to move the absolute position on the terminal screen. This uses
  /// [sanitize_abs_position] to clean up the absolute [Position].
  ///
  /// 1. [handle_draw_cursor] is actually used to draw the cursor.
  /// 2. [handle_maybe_draw_caret_at_overwrite_attempt] is used to ensure that this is not an
  ///    overwrite attempt.
  RequestShowCaretAtPositionRelTo(Position, Position),
}

pub enum FlushKind {
  JustFlushQueue,
  ClearBeforeFlushQueue,
}

mod command_helpers {
  use super::*;

  impl Debug for TWCommand {
    /// When [TWCommandQueue] is printed as debug, each [TWCommand] is printed using this method.
    /// Also [exec!] does not use this; it has its own way of logging output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{}",
        match self {
          TWCommand::EnterRawMode => "EnterRawMode".into(),
          TWCommand::ExitRawMode => "ExitRawMode".into(),
          TWCommand::MoveCursorPositionAbs(pos) => format!("MoveCursorPositionAbs({:?})", pos),
          TWCommand::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => format!(
            "MoveCursorPositionRelTo({:?}, {:?})",
            box_origin_pos, content_rel_pos
          ),
          TWCommand::ClearScreen => "ClearScreen".into(),
          TWCommand::SetFgColor(fg_color) => format!("SetFgColor({:?})", fg_color),
          TWCommand::SetBgColor(bg_color) => format!("SetBgColor({:?})", bg_color),
          TWCommand::ResetColor => "ResetColor".into(),
          TWCommand::ApplyColors(maybe_style) => match maybe_style {
            Some(style) => format!("ApplyColors({:?})", style),
            None => "ApplyColors(None)".into(),
          },
          TWCommand::PrintPlainTextWithAttributes(text, maybe_style)
          | TWCommand::PrintANSITextWithAttributes(text, maybe_style) => {
            match try_strip_ansi(text) {
              Some(plain_text) => {
                // Successfully stripped ANSI escape codes.
                match maybe_style {
                  Some(style) => format!("PrintWithAttributes(\"{}\", {:?})", plain_text, style),
                  None => format!("PrintWithAttributes(\"{}\", None)", plain_text),
                }
              }
              None => {
                // Couldn't strip ANSI, so just print the text.
                match maybe_style {
                  Some(style) => format!("PrintWithAttributes({} bytes, {:?})", text.len(), style),
                  None => format!("PrintWithAttributes({} bytes, None)", text.len()),
                }
              }
            }
          }
          TWCommand::CursorShow => "CursorShow".into(),
          TWCommand::CursorHide => "CursorHide".into(),
          TWCommand::RequestShowCaretAtPositionAbs(pos) =>
            format!("ShowCursorAtPosition({:?})", pos),
          TWCommand::RequestShowCaretAtPositionRelTo(box_origin_pos, content_rel_pos) => format!(
            "ShowCursorAtPosition({:?}, {:?})",
            box_origin_pos, content_rel_pos
          ),
        }
      )
    }
  }
}

impl TWCommand {
  pub fn flush() {
    exec!(stdout().flush(), "flush() -> stdout");
    exec!(stderr().flush(), "flush() -> stderr");
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ TWCommandQueue │
// ╯                ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This works w/ [TWCommand] items. It allows them to be added in sequence, and then flushed at the
/// end. Here's an example.
///
/// ```ignore
/// let mut queue = CommandQueue::default();
/// queue.add(TWCommand::ClearScreen);
/// queue.add(TWCommand::CursorShow);
/// queue.flush();
/// ```
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TWCommandQueue {
  /// The queue of [TWCommand]s to execute.
  pub queue: Vec<TWCommand>,
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Static mutable size │
// ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub mod terminal_window_static_data {
  use std::sync::atomic::{AtomicU16, Ordering};

  use super::*;

  pub static mut SIZE_COL: AtomicU16 = AtomicU16::new(0);
  pub static mut SIZE_ROW: AtomicU16 = AtomicU16::new(0);

  /// Save the given size to the static mutable variables [SIZE_COL] and [SIZE_ROW].
  pub fn set_size(size: Size) {
    unsafe {
      SIZE_COL.store(size.col, Ordering::SeqCst);
      SIZE_ROW.store(size.row, Ordering::SeqCst);
    }
  }

  /// Get the size from the static mutable mutable variables [SIZE_COL] and [SIZE_ROW].
  pub fn get_size() -> Size {
    unsafe {
      let cols = SIZE_COL.load(Ordering::SeqCst);
      let rows = SIZE_ROW.load(Ordering::SeqCst);
      size!(col: cols, row: rows)
    }
  }

  pub static mut CURSOR_COL: AtomicU16 = AtomicU16::new(0);
  pub static mut CURSOR_ROW: AtomicU16 = AtomicU16::new(0);

  /// Save the given position to the static mutable variables [CURSOR_COL] and [CURSOR_ROW].
  pub fn set_cursor_pos(pos: Position) {
    unsafe {
      CURSOR_COL.store(pos.col, Ordering::SeqCst);
      CURSOR_ROW.store(pos.row, Ordering::SeqCst);
    }
  }

  /// Get the position from the static mutable mutable variables [CURSOR_COL] and [CURSOR_ROW].
  pub fn get_cursor_pos() -> Position {
    unsafe {
      let col = CURSOR_COL.load(Ordering::SeqCst);
      let row = CURSOR_ROW.load(Ordering::SeqCst);
      position!(col: col, row: row)
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Misc crossterm lookup commands │
// ╯                                ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub(crate) mod terminal_window_commands {
  use super::*;

  /// Interrogate crossterm [crossterm::terminal::size()] to get the size of the terminal window.
  pub(crate) fn lookup_size() -> CommonResult<Size> {
    let size: Size = crossterm::terminal::size()?.into();
    Ok(size)
  }
}

impl TWCommandQueue {
  /// This will add `rhs` to `self` and then drop `rhs`.
  pub fn join_into(&mut self, mut rhs: TWCommandQueue) {
    self.queue.append(&mut rhs.queue);
    drop(rhs);
  }

  pub fn push(&mut self, cmd_wrapper: TWCommand) -> &mut Self {
    self.queue.push(cmd_wrapper);
    self
  }

  pub fn push_all(&mut self, cmd_wrapper_vec: Vec<TWCommand>) -> &mut Self {
    self.queue.extend(cmd_wrapper_vec);
    self
  }

  pub fn push_another(&mut self, other: TWCommandQueue) -> &mut Self {
    self.queue.extend(other.queue);
    self
  }

  // FUTURE: support termion, along w/ crossterm, by providing another impl of this fn #24
  pub async fn flush(&self, flush_kind: FlushKind) {
    let mut skip_flush = false;

    // If set to [Position] then it will draw the cursor at that position after flushing the queue.
    // Then clear this value. It will hide the cursor if [Position] is [None].
    let mut maybe_draw_caret_at: Option<Position> = None;

    if let FlushKind::ClearBeforeFlushQueue = flush_kind {
      exec! {
        queue!(stdout(),
          ResetColor,
          Clear(ClearType::All),
        ),
      "flush() -> after ResetColor, Clear"
      }
    }

    for command_ref in &self.queue {
      command_processor::execute(&mut maybe_draw_caret_at, &mut skip_flush, command_ref);
    }

    // Flush all the commands that were added via calls to `queue!` above.
    if !skip_flush {
      TWCommand::flush()
    };

    // Handle cursor drawing.
    handle_draw_cursor(maybe_draw_caret_at);
  }
}

pub fn handle_draw_cursor(maybe_draw_caret_at: Option<Position>) {
  if let Some(draw_cursor_at) = maybe_draw_caret_at {
    let Position { col, row } = draw_cursor_at;
    exec!(
      queue!(stdout(), MoveTo(col, row)),
      format!("DrawCursorAt -> MoveTo(col: {}, row: {})", col, row)
    );
    exec!(queue!(stdout(), Show), "DrawCursorAt -> Show");
    TWCommand::flush();
  } else {
    exec!(queue!(stdout(), Hide), "DrawCursorAt -> Hide");
    TWCommand::flush();
  }
}

pub fn handle_maybe_draw_caret_at_overwrite_attempt(ignored_pos: Position) {
  call_if_debug_true! {
    log_no_err!(WARN,
      "{} -> {:?}",
      "Attempt to set maybe_draw_caret_at more than once. Ignoring {:?}", ignored_pos)
  };
}

/// Ensure that the [Position] is within the bounds of the terminal window using [this
/// method](static_terminal_window_size::get). If the [Position] is outside of the bounds of the
/// window then it is clamped to the nearest edge of the window. This clamped [Position] is
/// returned.
pub fn sanitize_abs_position(orig_abs_pos: Position) -> Position {
  let Size {
    col: max_cols,
    row: max_rows,
  } = terminal_window_static_data::get_size();

  let mut new_abs_pos: Position = orig_abs_pos;

  if orig_abs_pos.col > max_cols {
    new_abs_pos.col = max_cols;
  }

  if orig_abs_pos.row > max_rows {
    new_abs_pos.row = max_rows;
  }

  debug_sanitize_abs_position(orig_abs_pos, new_abs_pos);

  return new_abs_pos;

  fn debug_sanitize_abs_position(orig_pos: Position, sanitized_pos: Position) {
    call_if_debug_true!({
      if sanitized_pos != orig_pos {
        log_no_err!(INFO, "🔏 Attempt to set position {:?} outside of terminal window. Clamping to nearest edge of window {:?}.", 
        orig_pos,
        sanitized_pos);
      }
    });
  }
}

mod command_processor {
  use std::borrow::Cow;

  use super::*;

  pub(super) fn execute(
    maybe_draw_caret_at: &mut Option<Position>, skip_flush: &mut bool, command_ref: &TWCommand,
  ) {
    match command_ref {
      TWCommand::RequestShowCaretAtPositionAbs(pos) => {
        request_show_caret_at_position_abs(pos, maybe_draw_caret_at);
      }
      TWCommand::RequestShowCaretAtPositionRelTo(box_origin_pos, content_rel_pos) => {
        request_show_caret_at_position_rel_to(box_origin_pos, content_rel_pos, maybe_draw_caret_at);
      }
      TWCommand::EnterRawMode => {
        raw_mode_enter(skip_flush);
      }
      TWCommand::ExitRawMode => {
        raw_mode_exit(skip_flush);
      }
      TWCommand::MoveCursorPositionAbs(abs_pos) => {
        move_cursor_position_abs(abs_pos);
      }
      TWCommand::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
        move_cursor_position_rel_to(box_origin_pos, content_rel_pos);
      }
      TWCommand::ClearScreen => {
        exec!(queue!(stdout(), Clear(ClearType::All)), "ClearScreen")
      }
      TWCommand::SetFgColor(color) => {
        set_fg_color(color);
      }
      TWCommand::SetBgColor(color) => {
        set_bg_color(color);
      }
      TWCommand::ResetColor => {
        exec!(queue!(stdout(), ResetColor), "ResetColor")
      }
      TWCommand::CursorShow => {
        exec!(queue!(stdout(), Show), "CursorShow")
      }
      TWCommand::CursorHide => {
        exec!(queue!(stdout(), Hide), "CursorHide")
      }
      TWCommand::ApplyColors(style) => {
        apply_colors(style);
      }
      TWCommand::PrintANSITextWithAttributes(text, maybe_style) => {
        print_ansi_text_with_attributes(text, maybe_style);
      }
      TWCommand::PrintPlainTextWithAttributes(text, maybe_style) => {
        print_plain_text_with_attributes(text, maybe_style);
      }
    }
  }

  fn move_cursor_position_rel_to(box_origin_pos: &Position, content_rel_pos: &Position) {
    let new_abs_pos = *box_origin_pos + *content_rel_pos;
    move_cursor_position_abs(&new_abs_pos);
  }

  fn move_cursor_position_abs(abs_pos: &Position) {
    let Position { col, row } = sanitize_abs_position(*abs_pos);
    terminal_window_static_data::set_cursor_pos(position! {col: col, row: row});
    exec!(
      queue!(stdout(), MoveTo(col, row)),
      format!("MoveCursorPosition(col: {}, row: {})", col, row)
    )
  }

  fn raw_mode_exit(skip_flush: &mut bool) {
    exec! {
      queue!(stdout(),
        Show,
        LeaveAlternateScreen,
        DisableMouseCapture
      ),
      "ExitRawMode -> Show, LeaveAlternateScreen, DisableMouseCapture"
    };
    TWCommand::flush();
    exec! {terminal::disable_raw_mode(), "ExitRawMode -> disable_raw_mode()"}
    *skip_flush = true;
  }

  fn raw_mode_enter(skip_flush: &mut bool) {
    exec! {
      terminal::enable_raw_mode(),
      "EnterRawMode -> enable_raw_mode()"
    };
    exec! {
      queue!(stdout(),
        EnableMouseCapture,
        EnterAlternateScreen,
        MoveTo(0,0),
        Clear(ClearType::All),
        Hide,
      ),
    "EnterRawMode -> EnableMouseCapture, EnterAlternateScreen, MoveTo(0,0), Clear(ClearType::All), Hide"
    }
    TWCommand::flush();
    *skip_flush = true;
  }

  fn request_show_caret_at_position_rel_to(
    box_origin_pos: &Position, content_rel_pos: &Position,
    maybe_draw_caret_at: &mut Option<Position>,
  ) {
    let new_abs_pos = *box_origin_pos + *content_rel_pos;
    request_show_caret_at_position_abs(&new_abs_pos, maybe_draw_caret_at);
  }

  fn request_show_caret_at_position_abs(
    pos: &Position, maybe_draw_caret_at: &mut Option<Position>,
  ) {
    let sanitized_pos = sanitize_abs_position(*pos);
    if maybe_draw_caret_at.is_none() {
      *maybe_draw_caret_at = Some(sanitized_pos);
    } else {
      handle_maybe_draw_caret_at_overwrite_attempt(sanitized_pos);
    }
  }

  fn set_fg_color(color: &TWColor) {
    let color = color_converter::to_crossterm_color(*color);
    exec!(
      queue!(stdout(), SetForegroundColor(color)),
      format!("SetFgColor({:?})", color)
    )
  }

  fn set_bg_color(color: &TWColor) {
    let color: crossterm::style::Color = color_converter::to_crossterm_color(*color);
    exec!(
      queue!(stdout(), SetBackgroundColor(color)),
      format!("SetBgColor({:?})", color)
    )
  }

  fn print_plain_text_with_attributes(text_arg: &String, maybe_style: &Option<Style>) {
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
    let cursor_position = terminal_window_static_data::get_cursor_pos();
    let max_cols = terminal_window_static_data::get_size().col;
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
    exec!(
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
        exec!(
          queue!(stdout(), SetAttribute(*attr)),
          format!("PrintWithAttributes -> SetAttribute({:?})", attr)
        );
        needs_reset = true;
      }
    });
    exec!(
      queue!(stdout(), Print(plain_text)),
      format!("PrintWithAttributes -> Style + Print({})", log_msg)
    );
    if needs_reset {
      exec!(
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
        exec!(
          queue!(stdout(), SetBackgroundColor(color_bg)),
          format!("ApplyColors -> SetBackgroundColor({:?})", color_bg)
        )
      }
      if mask.contains(StyleFlag::COLOR_FG_SET) {
        let color_fg = style.color_fg.unwrap();
        let color_fg: crossterm::style::Color = color_converter::to_crossterm_color(color_fg);
        exec!(
          queue!(stdout(), SetForegroundColor(color_fg)),
          format!("ApplyColors -> SetForegroundColor({:?})", color_fg)
        )
      }
    }
  }
}

mod queue_helpers {
  use super::*;

  impl Debug for TWCommandQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let mut temp_vec: Vec<String> = vec![];
      for command in &self.queue {
        let line: String = format!("{:?}", command);
        temp_vec.push(line);
      }
      write!(f, "\n    - {}", temp_vec.join("\n    - "))
    }
  }

  impl AddAssign for TWCommandQueue {
    fn add_assign(&mut self, other: TWCommandQueue) { self.queue.extend(other.queue); }
  }

  impl Add<TWCommand> for TWCommandQueue {
    type Output = TWCommandQueue;
    fn add(mut self, other: TWCommand) -> TWCommandQueue {
      self.queue.push(other);
      self
    }
  }

  impl AddAssign<TWCommand> for TWCommandQueue {
    fn add_assign(&mut self, other: TWCommand) { self.queue.push(other); }
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
