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

use r3bl_core::{size,
                throws,
                CommonResult,
                Position,
                RequestedSizePercent,
                Size,
                TuiStyle,
                TuiStylesheet};
use serde::{Deserialize, Serialize};

use super::{FlexBox,
            FlexBoxProps,
            LayoutDirection,
            LayoutManagement,
            PerformPositioningAndSizing,
            SurfaceProps};
use crate::{unwrap_or_err, LayoutError, LayoutErrorType, RenderPipeline};

/// Represents a rectangular area of the terminal screen, and not necessarily the full terminal
/// screen.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Surface {
    pub origin_pos: Position,
    pub box_size: Size,
    pub stack_of_boxes: Vec<FlexBox>,
    pub stylesheet: TuiStylesheet,
    pub render_pipeline: RenderPipeline,
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct SurfaceBounds {
    pub origin_pos: Position,
    pub box_size: Size,
}

mod surface_bounds_impl {
    use super::*;

    impl From<&Surface> for SurfaceBounds {
        fn from(surface: &Surface) -> Self {
            Self {
                origin_pos: surface.origin_pos,
                box_size: surface.box_size,
            }
        }
    }
}

#[macro_export]
macro_rules! surface {
    (
        stylesheet: $arg_stylesheet : expr
    ) => {
        $crate::Surface {
            stylesheet: $arg_stylesheet,
            ..Default::default()
        }
    };

    (
        origin_pos: $arg_origin_pos : expr,
        box_size:   $arg_box_size   : expr,
        stylesheet: $arg_stylesheet : expr
    ) => {
        $crate::Surface {
            origin_pos: $arg_origin_pos,
            box_size: $arg_box_size,
            stylesheet: $arg_stylesheet,
            ..Default::default()
        }
    };
}

impl LayoutManagement for Surface {
    fn surface_start(
        &mut self,
        SurfaceProps { pos, size }: SurfaceProps,
    ) -> CommonResult<()> {
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

    fn box_start(&mut self, flex_box_props: FlexBoxProps) -> CommonResult<()> {
        throws!({
            match self.no_boxes_added() {
                true => self.add_root_box(flex_box_props),
                false => self.add_non_root_box(flex_box_props),
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

impl PerformPositioningAndSizing for Surface {
    /// Get the last box on the stack (if none found then return Err).
    fn current_box(&mut self) -> CommonResult<&mut FlexBox> {
        // Expect stack of boxes not to be empty!
        if self.no_boxes_added() {
            LayoutError::new_err(LayoutErrorType::StackOfBoxesShouldNotBeEmpty)?
        }
        if let Some(it) = self.stack_of_boxes.last_mut() {
            Ok(it)
        } else {
            LayoutError::new_err(LayoutErrorType::StackOfBoxesShouldNotBeEmpty)?
        }
    }

    fn no_boxes_added(&self) -> bool { self.stack_of_boxes.is_empty() }

    /// Must be called *before* the new [FlexBox] is added to the stack of boxes
    /// otherwise [LayoutErrorType::ErrorCalculatingNextBoxPos] error is
    /// returned.
    ///
    /// This updates the `box_cursor_pos` of the current [FlexBox].
    ///
    /// Returns the [Position] where the next [FlexBox] can be added to the stack of
    /// boxes.
    fn update_insertion_pos_for_next_box(
        &mut self,
        allocated_size: Size,
    ) -> CommonResult<Position> {
        let current_box = self.current_box()?;
        let current_insertion_pos = current_box.insertion_pos_for_next_box;

        let current_insertion_pos = unwrap_or_err! {
          current_insertion_pos,
          LayoutErrorType::ErrorCalculatingNextBoxPos
        };

        let new_pos: Position = current_insertion_pos + allocated_size;

        // Adjust `new_pos` using Direction.
        let new_pos: Position = match current_box.dir {
            LayoutDirection::Vertical => new_pos * (0, 1),
            LayoutDirection::Horizontal => new_pos * (1, 0),
        };

        // Update the box_cursor_pos of the current layout.
        current_box.insertion_pos_for_next_box = new_pos.into();

        Ok(new_pos)
    }

    /// 🍀 Handle non-root box to add to stack of boxes. [Position] and [Size] will be calculated.
    /// `insertion_pos_for_next_box` will also be updated.
    fn add_non_root_box(&mut self, flex_box_props: FlexBoxProps) -> CommonResult<()> {
        throws!({
            let container_box = self.current_box()?;
            let container_bounds = container_box.bounds_size;

            let maybe_cascaded_style: Option<TuiStyle> =
                cascade_styles(container_box, &flex_box_props);

            let RequestedSizePercent {
                width_pc,
                height_pc,
            } = flex_box_props.requested_size_percent;

            let requested_size_allocation = size!(
              col_count: width_pc.calc_percentage(container_bounds.col_count),
              row_count: height_pc.calc_percentage(container_bounds.row_count)
            );

            let origin_pos = unwrap_or_err! {
              container_box.insertion_pos_for_next_box,
              LayoutErrorType::BoxCursorPositionUndefined
            };

            self.update_insertion_pos_for_next_box(requested_size_allocation)?;

            self.stack_of_boxes.push(make_non_root_box_with_style(
                flex_box_props,
                origin_pos,
                container_bounds,
                maybe_cascaded_style,
            ));
        });
    }

    /// 🌳 Handle root (first) box to add to stack of boxes, explicitly sized & positioned.
    fn add_root_box(&mut self, flex_box_props: FlexBoxProps) -> CommonResult<()> {
        throws!({
            let RequestedSizePercent {
                width_pc,
                height_pc,
            } = flex_box_props.requested_size_percent;

            let bounds_size = size!(
              col_count: width_pc.calc_percentage(self.box_size.col_count),
              row_count: height_pc.calc_percentage(self.box_size.row_count)
            );

            self.stack_of_boxes.push(make_root_box_with_style(
                flex_box_props,
                self.origin_pos,
                bounds_size,
            ));
        });
    }
}

/// - If `is_root` is true:
///   - The `insertion_pos_for_next_box` is origin_pos + padding adjustment (from style)
/// - If `is_root` is false:
///   - The `insertion_pos_for_next_box` is `None` non-root box; it needs to be calculated by
///     `update_box_cursor_pos_for_next_box_insertion()`
fn make_non_root_box_with_style(
    FlexBoxProps {
        id,
        dir,
        requested_size_percent:
            RequestedSizePercent {
                width_pc,
                height_pc,
            },
        maybe_styles: _,
    }: FlexBoxProps,
    origin_pos: Position,
    container_bounds: Size,
    maybe_cascaded_style: Option<TuiStyle>,
) -> FlexBox {
    let bounds_size = size!(
      col_count: width_pc.calc_percentage(container_bounds.col_count),
      row_count: height_pc.calc_percentage(container_bounds.row_count)
    );

    // Adjust `bounds_size` & `origin` based on the style's padding.
    let (style_adjusted_origin_pos, style_adjusted_bounds_size) =
        adjust_with_style(&maybe_cascaded_style, origin_pos, bounds_size);

    FlexBox {
        id,
        dir,
        origin_pos,
        bounds_size,
        style_adjusted_origin_pos,
        style_adjusted_bounds_size,
        requested_size_percent: RequestedSizePercent {
            width_pc,
            height_pc,
        },
        maybe_computed_style: maybe_cascaded_style,
        insertion_pos_for_next_box: None,
    }
}

fn make_root_box_with_style(
    FlexBoxProps {
        id,
        dir,
        requested_size_percent,
        maybe_styles,
    }: FlexBoxProps,
    origin_pos: Position,
    bounds_size: Size,
) -> FlexBox {
    let computed_style = TuiStylesheet::compute(&maybe_styles);

    // Adjust `bounds_size` & `origin` based on the style's padding.
    let (style_adjusted_origin_pos, style_adjusted_bounds_size) =
        adjust_with_style(&computed_style, origin_pos, bounds_size);

    FlexBox {
        id,
        dir,
        origin_pos,
        bounds_size,
        style_adjusted_origin_pos,
        style_adjusted_bounds_size,
        requested_size_percent,
        maybe_computed_style: computed_style,
        insertion_pos_for_next_box: Some(origin_pos),
    }
}

/// Adjust `origin` & `bounds_size` based on the `maybe_style`'s padding.
fn adjust_with_style(
    maybe_computed_style: &Option<TuiStyle>,
    origin_pos: Position,
    bounds_size: Size,
) -> (Position, Size) {
    let mut style_adjusted_origin_pos = origin_pos;
    let mut style_adjusted_bounds_size = bounds_size;

    if let Some(ref style) = maybe_computed_style {
        if let Some(padding) = style.padding {
            style_adjusted_origin_pos += padding;
            style_adjusted_bounds_size -= padding * 2;
        };
    }

    (style_adjusted_origin_pos, style_adjusted_bounds_size)
}

fn cascade_styles(
    parent_box: &FlexBox,
    self_box_props: &FlexBoxProps,
) -> Option<TuiStyle> {
    let mut style_vec: Vec<TuiStyle> = vec![];

    if let Some(parent_style) = parent_box.get_computed_style() {
        style_vec.push(parent_style);
    };

    if let Some(ref self_styles) = self_box_props.maybe_styles {
        self_styles.iter().for_each(|style| style_vec.push(*style));
    }

    if style_vec.is_empty() {
        None
    } else {
        TuiStylesheet::compute(&Some(style_vec))
    }
}
