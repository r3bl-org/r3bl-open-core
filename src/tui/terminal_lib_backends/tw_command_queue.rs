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

use std::{fmt::Debug,
          io::stdout,
          ops::{Add, AddAssign}};

use crossterm::{cursor::*, style::*, terminal::*, *};
use serde::{Deserialize, Serialize};

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ command_queue! │
// ╯                ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This works together w/ [TWCommand] to enqueue commands, but not flush them. It will return a
/// [TWCommandQueue]. Here's an example.
///
/// ```ignore
/// let mut queue = command_queue!(
///   TWCommand::ClearScreen,
///   TWCommand::ResetColor
/// ); // Returns the newly created queue.
/// ```
///
/// Another example.
///
/// ```ignore
/// let mut queue = command_queue!(); // Returns the newly created queue.
/// command_queue!(
///   queue push
///   TWCommand::ClearScreen,
///   TWCommand::ResetColor
/// ); // Returns nothing.
/// ```
///
/// Decl macro docs:
/// - <https://veykril.github.io/tlborm/decl-macros/macros-methodical.html#repetitions>
#[macro_export]
macro_rules! command_queue {
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
  pub async fn flush(&self, flush_kind: FlushKind, shared_tw_data: &SharedTWData) {
    process_queue::run_flush(&self.queue, flush_kind, shared_tw_data).await;
  }
}

mod queue_helpers {
  use super::*;
  use crate::tui::terminal_lib_backends::tw_command_queue::TWCommandQueue;

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

pub mod process_queue {
  use super::*;

  pub async fn run_flush(
    queue: &Vec<TWCommand>, flush_kind: FlushKind, shared_tw_data: &SharedTWData,
  ) {
    let mut skip_flush = false;
    // If set to [Position] then it will draw the cursor at that position after flushing the queue.
    // Then clear this value. It will hide the cursor if [Position] is [None].
    let mut maybe_sanitized_draw_caret_at: Option<Position> = None;

    if let FlushKind::ClearBeforeFlushQueue = flush_kind {
      exec! {
        queue!(stdout(),
          ResetColor,
          Clear(ClearType::All),
        ),
      "flush() -> after ResetColor, Clear"
      }
    }

    for command_ref in queue {
      route_command(
        &mut maybe_sanitized_draw_caret_at,
        &mut skip_flush,
        command_ref,
        shared_tw_data,
      )
      .await;
    }

    // Flush all the commands that were added via calls to `queue!` above.
    if !skip_flush {
      TWCommand::flush()
    };

    // Handle caret drawing.
    process_queue::handle_draw_caret_on_top(maybe_sanitized_draw_caret_at, shared_tw_data).await;
  }

  /// This paints the caret at the very end, so its painted on top of everything else. The
  /// `maybe_draw_caret_at` has already been sanitized by the time it gets here.
  ///
  /// See:
  /// 1. [crossterm_backend::request_show_caret_at_position_abs]
  /// 2. [crossterm_backend::request_show_caret_at_position_rel_to]
  pub async fn handle_draw_caret_on_top(
    maybe_sanitized_draw_caret_at: Option<Position>, _shared_tw_data: &SharedTWData,
  ) {
    if let Some(draw_cursor_at_pos) = maybe_sanitized_draw_caret_at {
      let Position { col, row } = draw_cursor_at_pos;
      exec!(
        queue!(stdout(), MoveTo(col, row), Show),
        format!("DrawCursorAt -> MoveTo(col: {}, row: {}) & Show", col, row)
      );
      TWCommand::flush();
    } else {
      exec!(queue!(stdout(), Hide), "DrawCursorAt -> Hide");
      TWCommand::flush();
    }
  }

  /// 1. Ensure that the [Position] is within the bounds of the terminal window using
  ///    [SharedTWData].
  /// 2. If the [Position] is outside of the bounds of the window then it is clamped to the nearest
  ///    edge of the window. This clamped [Position] is returned.
  /// 3. This also saves the clamped [Position] to [SharedTWData].
  pub async fn sanitize_abs_position(
    orig_abs_pos: Position, shared_tw_data: &SharedTWData,
  ) -> Position {
    let Size {
      col: max_cols,
      row: max_rows,
    } = shared_tw_data.read().await.size;

    let mut new_abs_pos: Position = orig_abs_pos;

    if orig_abs_pos.col > max_cols {
      new_abs_pos.col = max_cols;
    }

    if orig_abs_pos.row > max_rows {
      new_abs_pos.row = max_rows;
    }

    // Save the cursor position.
    shared_tw_data.write().await.cursor_position = new_abs_pos;

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

  pub fn log_maybe_draw_caret_at_overwrite_attempt(ignored_pos: Position) {
    call_if_debug_true! {
      log_no_err!(WARN,
        "{} -> {:?}",
        "Attempt to set maybe_draw_caret_at more than once. Ignoring {:?}", ignored_pos)
    };
  }
}
