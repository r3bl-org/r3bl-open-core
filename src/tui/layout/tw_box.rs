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

use std::fmt::Debug;

use crate::*;

/// Direction of the layout of the box.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
  Horizontal,
  Vertical,
}

impl Default for Direction {
  fn default() -> Direction { Direction::Horizontal }
}

/// A box is a rectangle with a position and size. The direction of the box
/// determines how it's contained elements are positioned.
#[derive(Clone, Default)]
pub struct TWBox {
  pub id: String,
  pub dir: Direction,
  pub origin_pos: Position,
  pub bounds_size: Size,
  pub style_adjusted_origin_pos: Position,
  pub style_adjusted_bounds_size: Size,
  pub requested_size_percent: RequestedSizePercent,
  pub insertion_pos_for_next_box: Option<Position>,
  pub maybe_computed_style: Option<Style>,
}

impl TWBox {
  pub fn get_computed_style(&self) -> Option<Style> { self.maybe_computed_style.clone() }
}

macro_rules! format_option {
  ($opt:expr) => {
    match ($opt) {
      Some(v) => v,
      None => &FormatMsg::None,
    }
  };
}

#[derive(Clone, Copy, Debug)]
enum FormatMsg {
  None,
}

impl Debug for TWBox {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TWBox")
      .field("id", &self.id)
      .field("dir", &self.dir)
      .field("origin_pos", &self.origin_pos)
      .field("bounds_size", &self.bounds_size)
      .field("style_adjusted_origin_pos", &self.style_adjusted_origin_pos)
      .field(
        "style_adjusted_bounds_size",
        &self.style_adjusted_bounds_size,
      )
      .field("requested_size_percent", &self.requested_size_percent)
      .field(
        "insertion_pos_for_next_box",
        format_option!(&self.insertion_pos_for_next_box),
      )
      .field(
        "maybe_computed_style",
        format_option!(&self.maybe_computed_style),
      )
      .finish()
  }
}
