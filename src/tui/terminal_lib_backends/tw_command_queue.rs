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

use std::{collections::HashMap, fmt::Debug, io::stdout};

use crossterm::{style::*, terminal::*, *};
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
///   @new ZOrder::Normal,
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
/// HashMap docs:
/// - <https://doc.rust-lang.org/std/collections/struct.HashMap.html#examples>
#[macro_export]
macro_rules! command_queue {
  // Create a new queue & return it. If any ($element)* are passed, then add it to new queue.
  (
    @new
    $arg_z_order: expr
    => $(                   /* Start a repetition. */
      $element:expr         /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      let mut tw_command_queue = TWCommandQueue::default();
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $element replaced. */
        match tw_command_queue.map_of_queue.entry($arg_z_order) {
          std::collections::hash_map::Entry::Occupied(mut entry) => {
            entry.get_mut().push($element);
          }
          std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(vec![$element]);
          }
        }
      )*
      tw_command_queue
    }
  };
  // Add a bunch of TWCommands $element+ to the existing $arg_queue & return nothing.
  (
    @push_into $arg_queue:ident
    at $arg_z_order: expr
    => $($element:expr),+
  ) => {
    $(
      /* Each repeat will contain the following statement, with $element replaced. */
      match $arg_queue.map_of_queue.entry($arg_z_order) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
          entry.get_mut().push($element);
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
          entry.insert(vec![$element]);
        }
      }
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
  (@new_empty) => {
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
  pub map_of_queue: QueueMap,
}

type QueueMap = HashMap<ZOrder, Vec<TWCommand>>;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ZOrder {
  Normal,
  High,
  Caret,
  Glass,
}

const RENDER_ORDERED_Z_ORDER_ARRAY: [ZOrder; 4] =
  [ZOrder::Normal, ZOrder::High, ZOrder::Caret, ZOrder::Glass];

impl Default for ZOrder {
  fn default() -> Self { Self::Normal }
}

impl TWCommandQueue {
  /// This will add `rhs` to `self`.
  pub fn join_into(&mut self, mut rhs: TWCommandQueue) {
    for (z_order, queue) in rhs.map_of_queue.drain() {
      match self.map_of_queue.entry(z_order) {
        std::collections::hash_map::Entry::Occupied(mut entry) => {
          entry.get_mut().extend(queue);
        }
        std::collections::hash_map::Entry::Vacant(entry) => {
          entry.insert(queue);
        }
      }
    }
  }

  pub fn push(&mut self, z_order: &ZOrder, cmd_wrapper: TWCommand) -> &mut Self {
    match self.map_of_queue.entry(*z_order) {
      std::collections::hash_map::Entry::Occupied(mut entry) => {
        entry.get_mut().push(cmd_wrapper);
      }
      std::collections::hash_map::Entry::Vacant(entry) => {
        entry.insert(vec![cmd_wrapper]);
      }
    }

    self
  }

  // FUTURE: support termion, along w/ crossterm, by providing another impl of this fn #24
  pub async fn flush(&self, flush_kind: FlushKind, shared_tw_data: &SharedTWData) {
    process_queue::run_flush(&self.map_of_queue, flush_kind, shared_tw_data).await;
  }
}

mod queue_helpers {
  use std::ops::AddAssign;

  use super::*;
  use crate::tui::terminal_lib_backends::tw_command_queue::TWCommandQueue;

  impl Debug for TWCommandQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let mut temp_vec: Vec<String> = vec![];
      for (z_order, vec_command) in &self.map_of_queue {
        let line: String = format!("[{:?}]: {:?}", z_order, vec_command);
        temp_vec.push(line);
      }
      write!(f, "\n    - {}", temp_vec.join("\n    - "))
    }
  }

  impl AddAssign for TWCommandQueue {
    fn add_assign(&mut self, other: TWCommandQueue) { self.join_into(other); }
  }

  impl AddAssign<(ZOrder, TWCommand)> for TWCommandQueue {
    fn add_assign(&mut self, other: (ZOrder, TWCommand)) { self.push(&other.0, other.1); }
  }
}

pub mod process_queue {
  use super::*;

  pub async fn run_flush(
    map_of_queue: &HashMap<ZOrder, Vec<TWCommand>>, flush_kind: FlushKind,
    shared_tw_data: &SharedTWData,
  ) {
    let mut skip_flush = false;

    if let FlushKind::ClearBeforeFlushQueue = flush_kind {
      exec! {
        queue!(stdout(),
          ResetColor,
          Clear(ClearType::All),
        ),
      "flush() -> after ResetColor, Clear"
      }
    }

    // List of special commands that should only be rendered at the very end.
    let mut hoisted_commands: Vec<TWCommand> = vec![];

    // Execute the commands in the queue, in the correct order of the ZOrder enum.
    for z_order in RENDER_ORDERED_Z_ORDER_ARRAY.iter() {
      if let Some(queue) = map_of_queue.get(z_order) {
        for command_ref in queue {
          if let TWCommand::RequestShowCaretAtPositionAbs(_)
          | TWCommand::RequestShowCaretAtPositionRelTo(_, _) = command_ref
          {
            hoisted_commands.push(command_ref.clone());
          } else {
            route_command(&mut skip_flush, command_ref, shared_tw_data).await;
          }
        }
      }
    }

    // Log error if hoisted_commands has more than one item.
    if hoisted_commands.len() > 1 {
      log_no_err!(
        WARN,
        "🥕 Too many requests to draw caret (some will be clobbered): {:?}",
        hoisted_commands,
      );
    }

    // Execute the hoisted commands (at the very end).
    for command_ref in &hoisted_commands {
      route_command(&mut skip_flush, command_ref, shared_tw_data).await;
    }

    // Flush all the commands that were added via calls to `queue!` above.
    if !skip_flush {
      TWCommand::flush()
    };
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
}
