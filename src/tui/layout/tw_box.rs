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
#[derive(Clone, Default, Builder)]
pub struct TWBox {
  pub id: String,
  pub dir: Direction,
  pub origin_pos: Position,
  pub bounding_size: Size,
  pub req_size_percent: RequestedSizePercent,
  pub box_cursor_pos: Option<Position>,
  pub computed_style: Option<Style>,
}

impl TWBox {
  pub fn get_computed_style(&self) -> Option<Style> { self.computed_style.clone() }

  /// Explicitly set the position & size of our box.
  pub fn make_root_box(
    id: String, size: Size, origin_pos: Position, width_pc: Percent, height_pc: Percent,
    dir: Direction, computed_style: Option<Style>,
  ) -> TWBox {
    let bounds_size = Size::from((
      calc_percentage(width_pc, size.cols),
      calc_percentage(height_pc, size.rows),
    ));
    TWBoxBuilder::new()
      .set_id(id)
      .set_dir(dir)
      .set_origin_pos(origin_pos)
      .set_bounding_size(bounds_size)
      .set_req_size_percent((width_pc, height_pc).into())
      .set_box_cursor_pos(origin_pos.as_some())
      .set_computed_style(computed_style)
      .build()
  }

  /// Actual position and size for our box will be calculated based on provided
  /// hints.
  pub fn make_box(
    id: String, dir: Direction, container_bounds: Size, origin_pos: Position, width_pc: Percent,
    height_pc: Percent, computed_style: Option<Style>,
  ) -> Self {
    // Adjust `bounds_size` & `origin` based on the style's margin.
    let mut style_adjusted_origin = origin_pos;

    let mut style_adjusted_bounds_size = Size::from((
      calc_percentage(width_pc, container_bounds.cols),
      calc_percentage(height_pc, container_bounds.rows),
    ));

    if let Some(ref style) = computed_style {
      if let Some(margin) = style.margin {
        style_adjusted_origin += margin;
        style_adjusted_bounds_size -= margin * 2;
      };
    }

    let req_size_pc: RequestedSizePercent = (width_pc, height_pc).into();

    TWBoxBuilder::new()
      .set_id(id)
      .set_dir(dir)
      .set_origin_pos(style_adjusted_origin)
      .set_bounding_size(style_adjusted_bounds_size)
      .set_req_size_percent(req_size_pc)
      .set_computed_style(computed_style)
      .build()
  }
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
      .field("origin", &self.origin_pos)
      .field("bounds_size", &self.bounding_size)
      .field("req_size_percent", &self.req_size_percent)
      .field("box_cursor_pos", format_option!(&self.box_cursor_pos))
      .field("styles", format_option!(&self.computed_style))
      .finish()
  }
}
