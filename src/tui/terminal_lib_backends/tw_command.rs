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

use std::{fmt::{self, Debug, Result},
          io::{stderr, stdout, Write}};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::BACKEND;
use crate::*;

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

// ╭┄┄┄┄┄┄┄┄┄┄┄╮
// │ TWCommand │
// ╯           ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TWCommand {
  EnterRawMode,

  ExitRawMode,

  /// [Position] is the absolute column and row on the terminal screen. This uses
  /// [process_queue::sanitize_abs_position] to clean up the given [Position].
  MoveCursorPositionAbs(Position),

  /// 1st [Position] is the origin column and row, and the 2nd [Position] is the offset column and
  /// row. They are added together to move the absolute position on the terminal screen. Then
  /// [TWCommand::MoveCursorPositionAbs] is used.
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
  ///
  /// | Variant | Auto clipping support |
  /// | --- | ---  |
  /// | `PrintPlainTextWithAttributes(String, Option<Style>)` | YES |
  /// | `PrintANSITextWithAttributes(String, Option<Style>)` | NO |
  PrintPlainTextWithAttributes(String, Option<Style>),

  /// Translate [Style] into attributes [static@STYLE_TO_ATTRIBUTE_MAP] for crossterm (bold,
  /// underline, strikethrough, etc). You are responsible for handling clipping of the text to the
  /// bounds of the terminal screen.
  ///
  /// | Variant | Auto clipping support |
  /// | --- | ---  |
  /// | `PrintPlainTextWithAttributes(String, Option<Style>)` | YES |
  /// | `PrintANSITextWithAttributes(String, Option<Style>)` | NO |
  PrintANSITextWithAttributes(String, Option<Style>),

  CursorShow,
  CursorHide,

  /// [Position] is the absolute column and row on the terminal screen. This uses
  /// [process_queue::sanitize_abs_position] to clean up the given [Position].
  ///
  /// 1. [process_queue::handle_draw_caret_on_top] is actually used to draw the cursor.
  /// 2. [process_queue::log_maybe_draw_caret_at_overwrite_attempt] is used to log when there's an
  ///    overwrite attempt.
  RequestShowCursorAtPositionAbs(Position),

  /// 1st [Position] is the origin column and row, and the 2nd [Position] is the offset column and
  /// row. They are added together to move the absolute position on the terminal screen. Then
  /// [TWCommand::RequestShowCursorAtPositionAbs].
  RequestShowCursorAtPositionRelTo(Position, Position),
}

pub enum FlushKind {
  JustFlushQueue,
  ClearBeforeFlushQueue,
}

impl TWCommand {
  pub fn flush() {
    exec!(stdout().flush(), "flush() -> stdout");
    exec!(stderr().flush(), "flush() -> stderr");
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Debug formatter │
// ╯                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
impl Debug for TWCommand {
  /// When [TWCommandQueue] is printed as debug, each [TWCommand] is printed using this method.
  /// Also [exec!] does not use this; it has its own way of logging output.
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
    match BACKEND {
      Backend::Crossterm => CrosstermDebugFormatCommand {}.debug_fmt(self, f),
      Backend::Termion => todo!(), // TODO: implement debug formatter for termion
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ DebugFormatCommand trait │
// ╯                          ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub trait DebugFormatCommand {
  fn debug_fmt(&self, this: &TWCommand, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Route command │
// ╯               ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub async fn route_command(
  maybe_sanitized_draw_caret_at: &mut Option<Position>, skip_flush: &mut bool,
  command_ref: &TWCommand, shared_tw_data: &SharedTWData,
) {
  match BACKEND {
    Backend::Crossterm => {
      CommandImplCrossterm {}
        .run_command(
          maybe_sanitized_draw_caret_at,
          skip_flush,
          command_ref,
          shared_tw_data,
        )
        .await;
    }
    Backend::Termion => todo!(), // TODO: implement RunCommand trait for termion
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ RunCommand trait │
// ╯                  ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[async_trait]
pub trait RunCommand {
  async fn run_command(
    &self, maybe_sanitized_draw_caret_at: &mut Option<Position>, skip_flush: &mut bool,
    command_ref: &TWCommand, shared_tw_data: &SharedTWData,
  );
}
