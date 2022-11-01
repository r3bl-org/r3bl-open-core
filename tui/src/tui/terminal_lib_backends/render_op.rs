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

use std::{fmt::{Debug, Formatter, Result},
          ops::{Deref, DerefMut}};

use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use super::TERMINAL_LIB_BACKEND;
use crate::*;

// ┏━━━━━━━━━━━━━┓
// ┃ render_ops! ┃
// ┛             ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Here's an example. Refer to [RenderOps] for more details.
/// ```rust
/// use r3bl_tui::*;
///
/// let mut render_ops = render_ops!(
///   @new
///   RenderOp::ClearScreen, RenderOp::CursorShow
/// );
/// let len = render_ops.len();
/// let iter = render_ops.iter();
/// ```
#[macro_export]
macro_rules! render_ops {
  // Empty.
  () => {
    RenderOps::default()
  };

  // @new: Create a RenderOps. If any ($arg_render_op)* are passed, then add it to its list. Finally
  // return it.
  (
    @new
    $(                      /* Start a repetition. */
      $arg_render_op: expr  /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      let mut render_ops = RenderOps::default();
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $arg_render_op replaced. */
        render_ops.list.push($arg_render_op);
      )*
      render_ops
    }
  };

  // @add_to: If any ($arg_render_op)* are passed, then add to it.
  (
    @add_to
    $arg_render_ops: expr
    =>
    $(                      /* Start a repetition. */
      $arg_render_op: expr  /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $arg_render_op replaced. */
        $arg_render_ops.list.push($arg_render_op);
      )*
    }
  };
}

// ┏━━━━━━━━━━━┓
// ┃ RenderOps ┃
// ┛           ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// It is a collection of *atomic* paint operations (aka [`RenderOps`] at various [`ZOrder`]s); each
/// [`RenderOps`] is made up of a [vec] of [`RenderOp`]. It contains `Map<ZOrder, Vec<RenderOps>>`,
/// eg:
/// - [`ZOrder::Normal`] => vec![[`RenderOp::ResetColor`], [`RenderOp::MoveCursorPositionAbs(..)`],
///   [`RenderOp::PrintTextWithAttributes(..)`]]
/// - [`ZOrder::Glass`] => vec![[`RenderOp::ResetColor`], [`RenderOp::MoveCursorPositionAbs(..)`],
///   [`RenderOp::PrintTextWithAttributes(..)`]]
/// - etc.
///
/// # What is an atomic paint operation?
/// 1. It moves the cursor using:
///     1. [`RenderOp::MoveCursorPositionAbs`]
///     2. [`RenderOp::MoveCursorPositionRelTo`]
/// 2.  And it does not assume that the cursor is in the correct position from some other previously
///     executed operation!
/// 3. So there are no side effects when re-ordering or omitting painting an atomic paint operation
///    (eg in the case where it has already been painted before).
///
/// Here's an example. Consider using the macro for convenience (see [render_ops!]). Also see
/// [TWData] for more information on scoping the [cursor_position](TWData::cursor_position) rules.
/// ```rust
/// use r3bl_tui::*;
///
/// let mut render_ops = RenderOps::default();
/// render_ops.push(RenderOp::ClearScreen);
/// render_ops.push(RenderOp::CursorShow);
/// let len = render_ops.len();
/// let iter = render_ops.iter();
/// ```
///
/// # Paint optimization
/// In order to ensure that things on the terminal screen aren't being needlessly drawn (when they
/// have already been drawn before and are on screen), it is important to keep track of the position
/// and bounds of each [RenderOps]. This allows [optimized_paint::clear_flex_box] to perform its
/// magic by clearing out the space for a [RenderOps] before it is painted. This is needed for
/// managing cursor movement (cursors are painted when moved, but the old cursor doesn't get
/// cleared).
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RenderOps {
  pub list: Vec<RenderOp>,
  pub flex_box: Option<FlexBox>,
}

pub mod render_ops_helpers {
  use super::*;

  impl Deref for RenderOps {
    type Target = Vec<RenderOp>;

    fn deref(&self) -> &Self::Target { &self.list }
  }

  impl DerefMut for RenderOps {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
  }

  impl Debug for RenderOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let mut vec_lines: Vec<String> = vec![];

      // First line.
      let first_line: String = {
        let mut line = format!("RenderOps.len(): {}", self.list.len());
        if let Some(ref flex_box) = self.flex_box {
          let flex_box_str = format!(
            ", origin: {:?}, size: {:?}",
            flex_box.style_adjusted_origin_pos, flex_box.style_adjusted_bounds_size
          );
          line.push_str(&flex_box_str);
        } else {
          line.push_str(", flex_box: None");
        }
        line
      };
      vec_lines.push(first_line);

      // Subsequent lines (optional).
      for render_op in self.iter() {
        let line: String = format!("[{render_op:?}]");
        vec_lines.push(line);
      }

      // Join all lines.
      write!(f, "\n    - {}", vec_lines.join("\n      - "))
    }
  }
}

// ┏━━━━━━━━━━┓
// ┃ RenderOp ┃
// ┛          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RenderOp {
  EnterRawMode,

  ExitRawMode,

  /// This is always painted on top. [Position] is the absolute column and row on the terminal
  /// screen. This uses [sanitize_and_save_abs_position] to clean up the given
  /// [Position].
  MoveCursorPositionAbs(Position),

  /// This is always painted on top. 1st [Position] is the origin column and row, and the 2nd
  /// [Position] is the offset column and row. They are added together to move the absolute position
  /// on the terminal screen. Then [RenderOp::MoveCursorPositionAbs] is used.
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
  /// underline, strikethrough, etc).
  ///
  /// 1. If the [String] argument is plain text (no ANSI sequences) then it will be clipped
  ///    available width of the terminal screen).
  ///
  /// 2. If the [String] argument contains ANSI sequences then it will be printed as-is. You are
  ///    responsible for handling clipping of the text to the bounds of the terminal screen.
  PrintTextWithAttributes(String, Option<Style>),

  CursorShow,
  CursorHide,

  /// [Position] is the absolute column and row on the terminal screen. This uses
  /// [sanitize_and_save_abs_position] to clean up the given [Position].
  RequestShowCaretAtPositionAbs(Position),

  /// 1st [Position] is the origin column and row, and the 2nd [Position] is the offset column and
  /// row. They are added together to move the absolute position on the terminal screen. Then
  /// [RenderOp::RequestShowCaretAtPositionAbs].
  RequestShowCaretAtPositionRelTo(Position, Position),

  /// For [Default] impl.
  Noop,
}

impl Default for RenderOp {
  fn default() -> Self { Self::Noop }
}

#[derive(Debug, Clone, Copy)]
pub enum FlushKind {
  JustFlush,
  ClearBeforeFlush,
}

pub trait Flush {
  fn flush(&mut self);
  fn clear_before_flush(&mut self);
}

impl Flush for RenderOp {
  fn flush(&mut self) {
    match TERMINAL_LIB_BACKEND {
      TerminalLibBackend::Crossterm => {
        RenderOpImplCrossterm {}.flush();
      }
      TerminalLibBackend::Termion => todo!(), // FUTURE: implement flush for termion
    }
  }

  fn clear_before_flush(&mut self) {
    match TERMINAL_LIB_BACKEND {
      TerminalLibBackend::Crossterm => {
        RenderOpImplCrossterm {}.clear_before_flush();
      }
      TerminalLibBackend::Termion => todo!(), // FUTURE: implement clear_before_flush for termion
    }
  }
}

// ┏━━━━━━━━━━━━━━━━━┓
// ┃ Debug formatter ┃
// ┛                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
impl Debug for RenderOp {
  /// When [RenderPipeline] is printed as debug, each [RenderOp] is printed using this method. Also
  /// [exec_render_op!] does not use this; it has its own way of logging output.
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match TERMINAL_LIB_BACKEND {
      TerminalLibBackend::Crossterm => CrosstermDebugFormatRenderOp {}.debug_format(self, f),
      TerminalLibBackend::Termion => todo!(), // FUTURE: implement debug formatter for termion
    }
  }
}

// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ DebugFormatRenderOp trait ┃
// ┛                           ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub trait DebugFormatRenderOp {
  fn debug_format(&self, this: &RenderOp, f: &mut Formatter<'_>) -> Result;
}

// ┏━━━━━━━━━━━━━━━━━┓
// ┃ exec_render_op! ┃
// ┛                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Given a crossterm command, this will run it and [log!] the [Result] that is returned. If [log!]
/// fails, then it will print a message to stderr.
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
    use $crate::tui::DEBUG_SHOW_TERMINAL_BACKEND;

    let _fn_wrap_for_logging_err = || -> CommonResult<()> {
      throws!({
        // Execute the command.
        if let Err(err) = $arg_cmd {
          call_if_true!(
            DEBUG_SHOW_TERMINAL_BACKEND,
            log!(ERROR, "crossterm: ❌ Failed to {} due to {}", $arg_log_msg, err)
          );
        } else {
          call_if_true! {
            DEBUG_SHOW_TERMINAL_BACKEND,
            log!(INFO, "crossterm: ✅ {} successfully", $arg_log_msg)
          };
        }
      })
    };

    // Call this generated function. It will fail if there are problems w/ log!(). In this case, if
    // `DEBUG_SHOW_TERMINAL_BACKEND` is true, then it will dump the error to stderr.
    if let Err(logging_err) = _fn_wrap_for_logging_err() {
      let msg = format!(
        "❌ Failed to log exec output of {}, {}",
        stringify!($arg_cmd),
        $arg_log_msg
      );
      call_if_true! {
        DEBUG_SHOW_TERMINAL_BACKEND,
        debug!(ERROR_RAW &msg, logging_err)
      };
    }
  }};
}
