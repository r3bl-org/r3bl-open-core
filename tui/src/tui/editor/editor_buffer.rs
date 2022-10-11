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

use std::fmt::{Debug, Result};

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::*;

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ EditorBuffer struct │
// ╯                     ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Stores the data for a single editor buffer.
///
/// 1. This struct is stored in the [r3bl_redux::Store].
/// 2. And it is paired w/ [EditorEngine] at runtime; which is responsible for rendering it to TUI,
///    and handling user input.
///
/// # Modifying the buffer
///
/// You have to supply an [EditorEvent] to the [EditorBuffer] to modify it via:
/// 1. [apply_editor_event](EditorEvent::apply_editor_event)
/// 2. [apply_editor_events](EditorEvent::apply_editor_events)
///
/// In order for the commands to be executed, the functions in [EditorEngineDataApi] are used.
///
/// These functions take any one of the following args:
/// 1. [EditorArgsMut]
/// 2. [EditorArgs]
/// 3. [EditorBuffer] and [EditorEngine]
///
/// # Accessing and mutating the fields (w/ validation)
///
/// All the fields in this struct are private. In order to access them you have to use the accessor
/// associated functions. To mutate them, you have to use the [get_mut](EditorBuffer::get_mut)
/// method, which returns a tuple w/ mutable references to the fields. This rather strange design
/// allows for all mutations to be tracked easily and allows for validation operations to be applied
/// post mutation (by [validate::apply_change]).
///
/// # Different kinds of caret positions
///
/// There are two variants for the caret position value:
/// 1. [CaretKind::Raw] - this is the position of the caret (unadjusted for scroll_offset) and this
///    represents the position of the caret in the viewport.
/// 2. [CaretKind::ScrollAdjusted] - this is the position of the caret (adjusted for scroll_offset)
///    and represents the position of the caret in the buffer (not the viewport).
///
/// # Vertical scrolling and viewport
///
/// ```text
///                    +0--------------------+
///                    0                     |
///                    |        above        | <- caret_row_adj
///                    |                     |
///                    +--- scroll_offset ---+
///              ->    |         ↑           |      ↑
///              |     |                     |      |
///   caret.row  |     |      within vp      |  vp height
///              |     |                     |      |
///              ->    |         ↓           |      ↓
///                    +--- scroll_offset ---+
///                    |    + vp height      |
///                    |                     |
///                    |        below        | <- caret_row_adj
///                    |                     |
///                    +---------------------+
/// ```
///
/// # Horizontal scrolling and viewport
///
/// ```text
///           <-   vp width   ->
/// +0--------+----------------+---------->
/// 0         |                |
/// | left of |<-  within vp ->| right of
/// |         |                |
/// +---------+----------------+---------->
///       scroll_offset    scroll_offset
///                        + vp width
/// ```

#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct EditorBuffer {
  /// A list of lines representing the document being edited.
  lines: Vec<UnicodeString>,

  /// The current caret position (relative to the
  /// [style_adjusted_origin_pos](FlexBox::style_adjusted_origin_pos) of the enclosing [FlexBox]).
  ///
  /// 1. This is the "display" and not "logical" position as defined in [UnicodeString].
  /// 2. This works w/ [crate::RenderOp::MoveCursorPositionRelTo] as well.
  /// 3. This is not marked pub in order to guard mutation. In order to access it, use
  ///    [get_mut](EditorBuffer::get_mut).
  caret: Position,

  /// The col and row offset for scrolling if active.
  /// 1. This is not marked pub in order to guard mutation. In order to access it, use
  ///    [get_mut](EditorBuffer::get_mut).
  scroll_offset: ScrollOffset,

  /// Lolcat struct for generating rainbow colors.
  lolcat: Lolcat,
}

mod constructor {
  use super::*;

  impl Default for EditorBuffer {
    fn default() -> Self {
      // Potentially do any other initialization here.
      call_if_true!(DEBUG_TUI_MOD, {
        log_no_err!(DEBUG, "🪙 {}", "construct EditorBuffer { lines, caret, lolcat }");
      });

      Self {
        lines: vec![UnicodeString::default()],
        caret: Position::default(),
        lolcat: Lolcat::default(),
        scroll_offset: ScrollOffset::default(),
      }
    }
  }
}

pub enum CaretKind {
  Raw,
  ScrollAdjusted,
}

pub mod access_and_mutate {
  use super::*;

  impl EditorBuffer {
    pub fn is_empty(&self) -> bool { self.lines.is_empty() }

    pub fn len(&self) -> ChUnit { ch!(self.lines.len()) }

    pub fn get_lines(&self) -> &Vec<UnicodeString> { &self.lines }

    /// Returns the current caret position in two variants:
    /// 1. [CaretKind::Raw] -> The raw caret position not adjusted for scrolling.
    /// 2. [CaretKind::ScrollAdjusted] -> The caret position adjusted for scrolling using
    ///    scroll_offset.
    pub fn get_caret(&self, kind: CaretKind) -> Position {
      match kind {
        CaretKind::Raw => self.caret,
        CaretKind::ScrollAdjusted => {
          position! {
            col: Self::calc_scroll_adj_caret_col(&self.caret, &self.scroll_offset),
            row: Self::calc_scroll_adj_caret_row(&self.caret, &self.scroll_offset)
          }
        }
      }
    }

    /// Scroll adjusted caret row = caret.row + scroll_offset.row.
    pub fn calc_scroll_adj_caret_row(caret: &Position, scroll_offset: &ScrollOffset) -> usize {
      ch!(@to_usize caret.row + scroll_offset.row)
    }

    /// Scroll adjusted caret col = caret.col + scroll_offset.col.
    pub fn calc_scroll_adj_caret_col(caret: &Position, scroll_offset: &ScrollOffset) -> usize {
      ch!(@to_usize caret.col + scroll_offset.col)
    }

    pub fn get_scroll_offset(&self) -> ScrollOffset { self.scroll_offset }

    /// Returns:
    /// 1. /* lines */ &mut Vec<UnicodeString>,
    /// 2. /* caret */ &mut Position,
    /// 3. /* scroll_offset */ &mut ScrollOffset,
    ///
    /// Even though this struct is mutable by editor_ops.rs, this method is provided to mark when
    /// mutable access is made to this struct. This makes it easy to determine what code mutates
    /// this struct, since it is necessary to validate things after mutation quite a bit in
    /// editor_ops.rs.
    pub fn get_mut(
      &mut self,
    ) -> (
      /* lines */ &mut Vec<UnicodeString>,
      /* caret */ &mut Position,
      /* scroll_offset */ &mut ScrollOffset,
    ) {
      (&mut self.lines, &mut self.caret, &mut self.scroll_offset)
    }
  }
}

mod debug_format_helpers {
  use super::*;

  impl Debug for EditorBuffer {
    fn fmt(&self, f: &mut __private::Formatter<'_>) -> Result {
      write! { f,
        "\nEditorBuffer [ \n \
        ├ lines: {}, size: {}, \n \
        ├ caret: {:?}, \n \
        └ lolcat: [{}, {}, {}, {}] \n]",
        self.lines.len(), self.lines.get_heap_size(),
        self.caret,
        pretty_print_f64(self.lolcat.color_wheel_control.seed),
        pretty_print_f64(self.lolcat.color_wheel_control.spread),
        pretty_print_f64(self.lolcat.color_wheel_control.frequency),
        self.lolcat.color_wheel_control.color_change_speed
      }
    }
  }

  /// More info: <https://stackoverflow.com/questions/63214346/how-to-truncate-f64-to-2-decimal-places>
  fn pretty_print_f64(before: f64) -> f64 { f64::trunc(before * 100.0) / 100.0 }
}
