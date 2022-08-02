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

/// Represents a rectangular area of the terminal screen, and not necessarily
/// the full terminal screen.
#[derive(Clone, Debug, Default)]
pub struct TWSurface {
  pub origin_pos: Position,
  pub box_size: Size,
  pub stack_of_boxes: Vec<TWBox>,
  pub stylesheet: Stylesheet,
  pub render_buffer: TWCommandQueue,
}

impl LayoutManagement for TWSurface {
  fn surface_start(&mut self, TWSurfaceProps { pos, size }: TWSurfaceProps) -> CommonResult<()> {
    throws!({
      // Expect stack to be empty!
      if !self.no_boxes_added() {
        LayoutError::new_err_with_msg(
          LayoutErrorType::MismatchedSurfaceStart,
          LayoutError::format_msg_with_stack_len(
            &self.stack_of_boxes,
            "Stack of boxes should be empty",
          ),
        )?
      }
      self.origin_pos = pos;
      self.box_size = size;
    });
  }

  fn surface_end(&mut self) -> CommonResult<()> {
    throws!({
      // Expect stack to be empty!
      if !self.no_boxes_added() {
        LayoutError::new_err_with_msg(
          LayoutErrorType::MismatchedSurfaceEnd,
          LayoutError::format_msg_with_stack_len(
            &self.stack_of_boxes,
            "Stack of boxes should be empty",
          ),
        )?
      }
    });
  }

  fn box_start(&mut self, tw_box_props: TWBoxProps) -> CommonResult<()> {
    throws!({
      match self.no_boxes_added() {
        true => self.add_root_box(tw_box_props),
        false => self.add_box(tw_box_props),
      }?
    });
  }

  fn box_end(&mut self) -> CommonResult<()> {
    throws!({
      // Expect stack not to be empty!
      if self.no_boxes_added() {
        LayoutError::new_err_with_msg(
          LayoutErrorType::MismatchedBoxEnd,
          LayoutError::format_msg_with_stack_len(
            &self.stack_of_boxes,
            "Stack of boxes should not be empty",
          ),
        )?
      }
      self.stack_of_boxes.pop();
    });
  }
}

impl PerformPositioningAndSizing for TWSurface {
  /// 🌳 Root: Handle first box to add to stack of boxes, explicitly sized &
  /// positioned.
  fn add_root_box(
    &mut self,
    TWBoxProps {
      id,
      dir,
      req_size: RequestedSizePercent {
        width: width_pc,
        height: height_pc,
      },
      styles,
    }: TWBoxProps,
  ) -> CommonResult<()> {
    throws!({
      self.stack_of_boxes.push(TWBox::make_root_box(
        id,
        self.box_size,
        self.origin_pos,
        width_pc,
        height_pc,
        dir,
        Stylesheet::compute(styles),
      ));
    });
  }

  /// 🍀 Non-root: Handle non-root box to add to stack of boxes. [Position] and
  /// [Size] will be calculated.
  fn add_box(
    &mut self,
    TWBoxProps {
      id,
      dir,
      req_size: RequestedSizePercent {
        width: width_pc,
        height: height_pc,
      },
      styles,
    }: TWBoxProps,
  ) -> CommonResult<()> {
    throws!({
      let current_box = self.current_box()?;

      let container_bounds = current_box.bounding_size;

      let requested_size_allocation = Size::from((
        calc_percentage(width_pc, container_bounds.cols),
        calc_percentage(height_pc, container_bounds.rows),
      ));

      let old_position = unwrap_or_err! {
        current_box.box_cursor_pos,
        LayoutErrorType::BoxCursorPositionUndefined
      };

      self.calc_where_to_insert_new_box_in_tw_surface(requested_size_allocation)?;

      self.stack_of_boxes.push(TWBox::make_box(
        id,
        dir,
        container_bounds,
        old_position,
        width_pc,
        height_pc,
        Stylesheet::compute(styles),
      ));
    });
  }

  /// Must be called *before* the new [TWBox] is added to the stack of boxes
  /// otherwise [LayoutErrorType::ErrorCalculatingNextLayoutPos] error is
  /// returned.
  ///
  /// This updates the `box_cursor_pos` of the current [TWBox].
  ///
  /// Returns the [Position] where the next [TWBox] can be added to the stack of
  /// boxes.
  fn calc_where_to_insert_new_box_in_tw_surface(
    &mut self, allocated_size: Size,
  ) -> CommonResult<Position> {
    let current_box = self.current_box()?;
    let box_cursor_pos = current_box.box_cursor_pos;

    let box_cursor_pos = unwrap_or_err! {
      box_cursor_pos,
      LayoutErrorType::ErrorCalculatingNextBoxPos
    };

    let new_pos: Position = box_cursor_pos + allocated_size;

    // Adjust `new_pos` using Direction.
    let new_pos: Position = match current_box.dir {
      Direction::Vertical => new_pos * (0, 1).into(),
      Direction::Horizontal => new_pos * (1, 0).into(),
    };

    // Update the box_cursor_pos of the current layout.
    current_box.box_cursor_pos = new_pos.as_some();

    Ok(new_pos)
  }

  /// Get the last box on the stack (if none found then return Err).
  fn current_box(&mut self) -> CommonResult<&mut TWBox> {
    // Expect stack of boxes not to be empty!
    if self.no_boxes_added() {
      LayoutError::new_err(LayoutErrorType::StackOfBoxesShouldNotBeEmpty)?
    }
    Ok(self.stack_of_boxes.last_mut().unwrap())
  }

  fn no_boxes_added(&self) -> bool { self.stack_of_boxes.is_empty() }
}
