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

use crate::*;

/// Public API interface to create nested & responsive layout based UIs.
pub trait LayoutManagement {
  /// Set the origin pos (x, y) & tw_surface size (width, height) of our box (container).
  fn surface_start(&mut self, bounds_props: SurfaceProps) -> CommonResult<()>;

  fn surface_end(&mut self) -> CommonResult<()>;

  /// Add a new layout on the stack w/ the direction & (width, height) percentages.
  fn box_start(&mut self, tw_box_props: TWBoxProps) -> CommonResult<()>;

  fn box_end(&mut self) -> CommonResult<()>;
}

/// Methods that actually perform the layout and positioning.
pub trait PerformPositioningAndSizing {
  /// Update `box_cursor_pos`. This needs to be called before adding a new [TWBox].
  fn calc_where_to_insert_new_box_in_tw_surface(
    &mut self, allocated_size: Size,
  ) -> CommonResult<Position>;

  /// Get the [TWBox] at the "top" of the `stack`.
  fn current_box(&mut self) -> CommonResult<&mut TWBox>;

  fn no_boxes_added(&self) -> bool;

  /// Add the first [TWBox] to the [TWSurface].
  /// 1. This one is explicitly sized.
  /// 2. there can be only one.
  fn add_root_box(&mut self, props: TWBoxProps) -> CommonResult<()>;

  /// Add non-root [TWBox].
  fn add_box(&mut self, props: TWBoxProps) -> CommonResult<()>;
}

/// Properties that are needed to create a [TWBox].
#[derive(Clone, Debug, Default)]
pub struct TWBoxProps {
  pub id: String,
  pub dir: Direction,
  pub req_size: RequestedSizePercent,
  pub styles: Option<Vec<Style>>,
}

/// Properties that are needed to create a [TWSurface].
#[derive(Clone, Debug, Default)]
pub struct SurfaceProps {
  pub pos: Position,
  pub size: Size,
}
