// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{FlexBox, FlexBoxProps, LayoutDirection, LayoutError, LayoutErrorType,
            LayoutManagement, PerformPositioningAndSizing, SurfaceProps};
use crate::{CommonResult, InlineVec, Pos, ReqSizePc, Size, TuiStyle, TuiStylesheet,
            height, throws, unwrap_or_err, width};

/// Represents a rectangular area of the terminal screen, and not necessarily the full
/// terminal screen.
#[derive(Clone, Debug, Default)]
pub struct Surface {
    pub origin_pos: Pos,
    pub box_size: Size,
    pub stack_of_boxes: Vec<FlexBox>,
    pub stylesheet: TuiStylesheet,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SurfaceBounds {
    pub origin_pos: Pos,
    pub box_size: Size,
}

mod impl_surface_bounds {
    #[allow(clippy::wildcard_imports)]
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
    ) -> CommonResult {
        throws!({
            // Expect stack to be empty!
            if !self.no_boxes_added() {
                LayoutError::new_error_result(
                    LayoutErrorType::MismatchedSurfaceStart,
                    LayoutError::format_msg_with_stack_len(
                        &self.stack_of_boxes,
                        "Stack of boxes should be empty",
                    ),
                )?;
            }
            self.origin_pos = pos;
            self.box_size = size;
        });
    }

    fn surface_end(&mut self) -> CommonResult {
        throws!({
            // Expect stack to be empty!
            if !self.no_boxes_added() {
                LayoutError::new_error_result(
                    LayoutErrorType::MismatchedSurfaceEnd,
                    LayoutError::format_msg_with_stack_len(
                        &self.stack_of_boxes,
                        "Stack of boxes should be empty",
                    ),
                )?;
            }
        });
    }

    fn box_start(&mut self, flex_box_props: FlexBoxProps) -> CommonResult {
        throws!({
            if self.no_boxes_added() {
                self.add_root_box(flex_box_props)
            } else {
                self.add_non_root_box(flex_box_props)
            }?;
        });
    }

    fn box_end(&mut self) -> CommonResult {
        throws!({
            // Expect stack not to be empty!
            if self.no_boxes_added() {
                LayoutError::new_error_result(
                    LayoutErrorType::MismatchedBoxEnd,
                    LayoutError::format_msg_with_stack_len(
                        &self.stack_of_boxes,
                        "Stack of boxes should not be empty",
                    ),
                )?;
            }
            self.stack_of_boxes.pop();
        });
    }
}

impl PerformPositioningAndSizing for Surface {
    /// Gets the last box on the stack (if none found then return Err).
    fn current_box(&mut self) -> CommonResult<&mut FlexBox> {
        // Expect stack of boxes not to be empty!
        if self.no_boxes_added() {
            LayoutError::new_error_result_with_only_type(
                LayoutErrorType::StackOfBoxesShouldNotBeEmpty,
            )?;
        }
        if let Some(it) = self.stack_of_boxes.last_mut() {
            Ok(it)
        } else {
            LayoutError::new_error_result_with_only_type(
                LayoutErrorType::StackOfBoxesShouldNotBeEmpty,
            )?
        }
    }

    fn no_boxes_added(&self) -> bool { self.stack_of_boxes.is_empty() }

    /// Calculates the coordinates where the next sibling box should be drawn, after we've
    /// allocated space for the current one.
    ///
    /// Imagine you have a [`LayoutDirection::Horizontal`] container. When you add a child
    /// box to it, the next child should be placed directly to its right. This means the
    /// [row] (Y-coordinate) should remain exactly the same as the parent container, while
    /// the [column] (X-coordinate) advances.
    ///
    /// ```text
    /// ╭───────────────────────────────────────╮  <-- Parent Container at col: 0, row: 5
    /// │                                       │
    /// │ ╭─────────────╮ ╭─────────────╮       │
    /// │ │ Child 1     │ │ Child 2     │       │
    /// │ │ (0, 5)      │ │ (50, 5)     │       │
    /// │ ╰─────────────╯ ╰─────────────╯       │
    /// │                 ▲                     │
    /// │                 │                     │
    /// │                 ╰─ Next insertion pos preserves the parent's row (5).
    /// │                    Only the col advances (0 + 50).
    /// ╰───────────────────────────────────────╯
    /// ```
    ///
    /// This updates the [`FlexBox::insertion_pos_for_next_box`] of the current
    /// [`FlexBox`].
    ///
    /// Returns the [Pos] where the next [`FlexBox`] can be added to the stack of boxes.
    ///
    /// Must be called *before* the new [`FlexBox`] is added to the stack of boxes
    /// otherwise [`LayoutErrorType::ErrorCalculatingNextBoxPos`] error is returned.
    ///
    /// [column]: crate::ColIndex
    /// [row]: crate::RowIndex
    fn update_insertion_pos_for_next_box(
        &mut self,
        allocated_size: Size,
    ) -> CommonResult<Pos> {
        let current_box = self.current_box()?;
        let current_insertion_pos = current_box.insertion_pos_for_next_box;

        let current_insertion_pos = unwrap_or_err! {
          current_insertion_pos,
          LayoutErrorType::ErrorCalculatingNextBoxPos
        };

        let new_pos = current_insertion_pos + allocated_size;

        // Adjust `new_pos` using Direction. Preserve the non-directional component:
        // - for vertical keep the column,
        // - for horizontal keep the row.
        let new_pos = match current_box.dir {
            LayoutDirection::Vertical => {
                new_pos.row_index + current_insertion_pos.col_index
            }
            LayoutDirection::Horizontal => {
                current_insertion_pos.row_index + new_pos.col_index
            }
        };

        // Update the insertion_pos_for_next_box of the current layout.
        current_box.insertion_pos_for_next_box = new_pos.into();

        Ok(new_pos)
    }

    /// 🍀 Handle non-root box to add to stack of boxes. [Pos] and [Size] will be
    /// calculated. `insertion_pos_for_next_box` will also be updated.
    fn add_non_root_box(&mut self, flex_box_props: FlexBoxProps) -> CommonResult {
        throws!({
            let container_box = self.current_box()?;
            let container_bounds = container_box.bounds_size;

            let maybe_cascaded_style: Option<TuiStyle> =
                cascade_styles(container_box, &flex_box_props);

            let ReqSizePc {
                width_pc,
                height_pc,
            } = flex_box_props.requested_size_percent;

            let requested_size_allocation = {
                let width_val = width_pc.apply_to(*container_bounds.col_width);
                let height_val = height_pc.apply_to(*container_bounds.row_height);
                width(width_val) + height(height_val)
            };

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

    /// 🌳 Handle root (first) box to add to stack of boxes, explicitly sized &
    /// positioned.
    fn add_root_box(&mut self, flex_box_props: FlexBoxProps) -> CommonResult {
        throws!({
            let ReqSizePc {
                width_pc,
                height_pc,
            } = flex_box_props.requested_size_percent;

            let bounds_size = {
                let width_val = width_pc.apply_to(*self.box_size.col_width);
                let height_val = height_pc.apply_to(*self.box_size.row_height);
                width(width_val) + height(height_val)
            };

            self.stack_of_boxes.push(make_root_box_with_style(
                flex_box_props,
                self.origin_pos,
                bounds_size,
            ));
        });
    }
}

/// - If `is_root` is true:
///   - The `insertion_pos_for_next_box` is `origin_pos` + padding adjustment (from style)
/// - If `is_root` is false:
///   - The `insertion_pos_for_next_box` is `None` non-root box; it needs to be calculated
///     by `update_box_cursor_pos_for_next_box_insertion()`
fn make_non_root_box_with_style(
    FlexBoxProps {
        id,
        dir,
        requested_size_percent:
            ReqSizePc {
                width_pc,
                height_pc,
            },
        maybe_styles: _,
    }: FlexBoxProps,
    origin_pos: Pos,
    container_bounds: Size,
    maybe_cascaded_style: Option<TuiStyle>,
) -> FlexBox {
    let bounds_size = {
        let width_val = width_pc.apply_to(*container_bounds.col_width);
        let height_val = height_pc.apply_to(*container_bounds.row_height);
        width(width_val) + height(height_val)
    };

    // Adjust `bounds_size` & `origin` based on the style's padding.
    let (style_adjusted_origin_pos, style_adjusted_bounds_size) =
        adjust_with_style(maybe_cascaded_style, origin_pos, bounds_size);

    FlexBox {
        id,
        dir,
        origin_pos,
        bounds_size,
        style_adjusted_origin_pos,
        style_adjusted_bounds_size,
        requested_size_percent: ReqSizePc {
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
    origin_pos: Pos,
    bounds_size: Size,
) -> FlexBox {
    let computed_style = TuiStylesheet::compute(&maybe_styles);

    // Adjust `bounds_size` & `origin` based on the style's padding.
    let (style_adjusted_origin_pos, style_adjusted_bounds_size) =
        adjust_with_style(computed_style, origin_pos, bounds_size);

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
    maybe_computed_style: Option<TuiStyle>,
    origin_pos: Pos,
    bounds_size: Size,
) -> (Pos, Size) {
    let mut style_adjusted_origin_pos = origin_pos;
    let mut style_adjusted_bounds_size = bounds_size;

    if let Some(style) = maybe_computed_style
        && let Some(padding) = style.padding
    {
        style_adjusted_origin_pos += padding;
        style_adjusted_bounds_size -= padding * 2;
    }

    (style_adjusted_origin_pos, style_adjusted_bounds_size)
}

fn cascade_styles(
    parent_box: &FlexBox,
    self_box_props: &FlexBoxProps,
) -> Option<TuiStyle> {
    let mut style_vec = InlineVec::<TuiStyle>::new();

    if let Some(parent_style) = parent_box.get_computed_style() {
        style_vec.push(parent_style);
    }

    if let Some(ref self_styles) = self_box_props.maybe_styles {
        self_styles.iter().for_each(|style| style_vec.push(*style));
    }

    if style_vec.is_empty() {
        None
    } else {
        TuiStylesheet::compute(&Some(style_vec))
    }
}

#[cfg(test)]
mod test_surface_2_col_complex {
    use crate::{CommonResult, FlexBoxId, FlexBoxProps, LayoutDirection,
                LayoutManagement, Surface, SurfaceProps, TuiStylesheet, assert_eq2,
                box_end, box_start, ch, col, console_log, get_tui_styles, height,
                new_style, req_size_pc, row, throws, throws_with_return, tui_color,
                tui_stylesheet, width};

    #[test]
    fn test_surface_2_col_complex() -> CommonResult {
        throws!({
            let mut surface = Surface {
                stylesheet: dsl_stylesheet()?,
                ..Default::default()
            };

            surface.surface_start(SurfaceProps {
                pos: col(0) + row(0),
                size: width(500) + height(500),
            })?;

            create_main_container(&mut surface)?;

            surface.surface_end()?;
        });
    }

    /// Main container "container".
    fn create_main_container(surface: &mut Surface) -> CommonResult {
        fn make_container_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let layout_item = surface.stack_of_boxes.first().unwrap();
                assert_eq2!(layout_item.id, FlexBoxId::from(0));
                assert_eq2!(layout_item.dir, LayoutDirection::Horizontal);

                assert_eq2!(layout_item.origin_pos, col(0) + row(0));
                assert_eq2!(layout_item.bounds_size, width(500) + height(500));

                assert_eq2!(layout_item.style_adjusted_origin_pos, col(1) + row(1)); // due to `padding: 1`
                assert_eq2!(
                    layout_item.style_adjusted_bounds_size,
                    width(498) + height(498)
                ); // due to `padding: 1`

                assert_eq2!(
                    layout_item.requested_size_percent,
                    req_size_pc!(width:100, height:100)
                );

                assert_eq2!(
                    layout_item.insertion_pos_for_next_box,
                    Some(col(0) + row(0))
                );

                assert!(layout_item.get_computed_style().is_some());
                assert_eq2!(
                    layout_item.get_computed_style().unwrap().padding,
                    Some(ch(1))
                );
            });
        }

        throws!({
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(0),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width:100, height:100),
                maybe_styles: get_tui_styles! { @from: surface.stylesheet, [0] },
            })?;

            make_container_assertions(surface)?;

            create_left_col(surface)?;
            create_right_col(surface)?;

            surface.box_end()?;
        });
    }

    /// Left column 1.
    fn create_left_col(surface: &mut Surface) -> CommonResult {
        fn make_left_col_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let layout_item = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(layout_item.id, FlexBoxId::from(1));
                assert_eq2!(layout_item.dir, LayoutDirection::Vertical);

                assert_eq2!(layout_item.origin_pos, col(0) + row(0));
                assert_eq2!(layout_item.bounds_size, width(250) + height(500));

                console_log!(layout_item);

                assert_eq2!(layout_item.style_adjusted_origin_pos, col(3) + row(3)); // Take padding into account.
                assert_eq2!(
                    layout_item.style_adjusted_bounds_size,
                    width(244) + height(494)
                ); // Take padding into account.

                assert_eq2!(
                    layout_item.requested_size_percent,
                    req_size_pc!(width:50, height:100)
                );
                assert_eq2!(layout_item.insertion_pos_for_next_box, None);

                assert_ne!(
                    layout_item.get_computed_style(),
                    TuiStylesheet::compute(&surface.stylesheet.find_styles_by_ids(&[1]))
                );
            });
        }

        throws!({
            // With macro.
            box_start! {
              in:                     surface,
              id:                     FlexBoxId::from(1),
              dir:                    LayoutDirection::Vertical,
              requested_size_percent: req_size_pc!(width:50, height:100),
              styles:                 [1]
            }
            make_left_col_assertions(surface)?;
            box_end!(in: surface);
        });
    }

    /// Right column 2.
    fn create_right_col(surface: &mut Surface) -> CommonResult {
        fn make_right_col_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let current_box = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(current_box.id, FlexBoxId::from(2));
                assert_eq2!(current_box.dir, LayoutDirection::Vertical);

                assert_eq2!(current_box.origin_pos, col(250) + row(0));
                assert_eq2!(current_box.bounds_size, width(250) + height(500));

                assert_eq2!(current_box.style_adjusted_origin_pos, col(254) + row(4)); // Take padding into account.
                assert_eq2!(
                    current_box.style_adjusted_bounds_size,
                    width(242) + height(492)
                ); // Take padding into account.

                assert_eq2!(
                    current_box.requested_size_percent,
                    req_size_pc!(width:50, height:100)
                );
                assert_eq2!(current_box.insertion_pos_for_next_box, None);

                assert_ne!(
                    current_box.get_computed_style(),
                    TuiStylesheet::compute(&surface.stylesheet.find_styles_by_ids(&[2]))
                );
            });
        }

        throws!({
            // No macro.
            surface.box_start(FlexBoxProps {
                maybe_styles: get_tui_styles! { @from: surface.stylesheet, [2] },
                id: FlexBoxId::from(2),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width:50, height:100),
            })?;
            make_right_col_assertions(surface)?;
            surface.box_end()?;
        });
    }

    /// Creates a stylesheet containing styles using DSL.
    fn dsl_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
                new_style!(
                    id: {0}
                    padding: {1}
                ),
                new_style!(
                    id: {1}
                    dim bold
                    padding: {2}
                    color_fg: {tui_color!(255, 255, 0)} /* Yellow. */
                    color_bg: {tui_color!(128, 128, 128)} /* Grey. */
                ),
                new_style!(
                    id: {2}
                    underline strikethrough
                    padding: {3}
                    color_fg: {tui_color!(black)} /* Black. */
                    color_bg: {tui_color!(white)} /* White. */
                )
            }
        })
    }
}

#[cfg(test)]
mod test_surface_2_col_simple {
    use crate::{CommonResult, FlexBoxId, FlexBoxProps, LayoutDirection,
                LayoutManagement, Surface, SurfaceProps, TuiStylesheet, assert_eq2,
                box_end, box_start, col, get_tui_styles, height, new_style, req_size_pc,
                row, throws, throws_with_return, tui_color, tui_stylesheet, width};

    #[test]
    fn test_surface_2_col_simple() -> CommonResult {
        throws!({
            let mut surface = Surface {
                stylesheet: dsl_stylesheet()?,
                ..Default::default()
            };

            surface.surface_start(SurfaceProps {
                pos: col(0) + row(0),
                size: width(500) + height(500),
            })?;

            create_main_container(&mut surface)?;

            surface.surface_end()?;
        });
    }

    /// Main container 0.
    fn create_main_container(surface: &mut Surface) -> CommonResult {
        fn make_container_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let layout_item = surface.stack_of_boxes.first().unwrap();
                assert_eq2!(layout_item.id, FlexBoxId::from(0));
                assert_eq2!(layout_item.dir, LayoutDirection::Horizontal);
                assert_eq2!(layout_item.origin_pos, col(0) + row(0));
                assert_eq2!(layout_item.bounds_size, width(500) + height(500)); // due to `padding: 1`
                assert_eq2!(
                    layout_item.requested_size_percent,
                    req_size_pc!(width:100, height:100)
                );
                assert_eq2!(
                    layout_item.insertion_pos_for_next_box,
                    Some(col(0) + row(0))
                );
                assert_eq2!(layout_item.get_computed_style(), None);
            });
        }

        throws!({
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(0),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width:100, height:100),
                maybe_styles: None,
            })?;

            make_container_assertions(surface)?;

            create_left_col(surface)?;
            create_right_col(surface)?;

            surface.box_end()?;
        });
    }

    /// Left column 1.
    fn create_left_col(surface: &mut Surface) -> CommonResult {
        fn make_left_col_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let layout_item = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(layout_item.id, FlexBoxId::from(1));
                assert_eq2!(layout_item.dir, LayoutDirection::Vertical);

                assert_eq2!(layout_item.origin_pos, col(0) + row(0));
                assert_eq2!(layout_item.bounds_size, width(250) + height(500));

                assert_eq2!(layout_item.style_adjusted_origin_pos, col(2) + row(2)); // Take padding into account.
                assert_eq2!(
                    layout_item.style_adjusted_bounds_size,
                    width(246) + height(496)
                ); // Take padding into account.

                assert_eq2!(
                    layout_item.requested_size_percent,
                    req_size_pc!(width:50, height:100)
                );
                assert_eq2!(layout_item.insertion_pos_for_next_box, None);
                assert_eq2!(
                    layout_item.get_computed_style(),
                    TuiStylesheet::compute(&surface.stylesheet.find_styles_by_ids(&[1]))
                );
            });
        }

        throws! {{
            // With macro.
            box_start! {
                in:                     surface,
                id:                     FlexBoxId::from(1),
                dir:                    LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width:50, height:100),
                styles:                 [1]
            }
            make_left_col_assertions(surface)?;
            box_end!(in: surface);

        }}
    }

    /// Right column 2.
    fn create_right_col(surface: &mut Surface) -> CommonResult {
        fn make_right_col_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let current_box = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(current_box.id, FlexBoxId::from(2));
                assert_eq2!(current_box.dir, LayoutDirection::Vertical);

                assert_eq2!(current_box.origin_pos, col(250) + row(0));
                assert_eq2!(current_box.bounds_size, width(250) + height(500));

                assert_eq2!(current_box.style_adjusted_origin_pos, col(253) + row(3)); // Take padding into account.
                assert_eq2!(
                    current_box.style_adjusted_bounds_size,
                    width(244) + height(494)
                ); // Take padding into account.

                assert_eq2!(
                    current_box.requested_size_percent,
                    req_size_pc!(width: 50, height: 100)
                );
                assert_eq2!(current_box.insertion_pos_for_next_box, None);
                assert_eq2!(
                    current_box.get_computed_style(),
                    TuiStylesheet::compute(&surface.stylesheet.find_styles_by_ids(&[2]))
                );
            });
        }

        throws!({
            // No macro.
            surface.box_start(FlexBoxProps {
                maybe_styles: get_tui_styles! { @from: surface.stylesheet, [2] },
                id: FlexBoxId::from(2),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width:50, height:100),
            })?;
            make_right_col_assertions(surface)?;
            surface.box_end()?;
        });
    }

    /// Creates a stylesheet containing styles using DSL.
    fn dsl_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
                new_style!(
                    id: {1}
                    dim bold
                    padding: {2}
                    color_fg: {tui_color!(255, 255, 0)} /* Yellow. */
                    color_bg: {tui_color!(128, 128, 128)} /* Grey. */
                ),
                new_style!(
                    id: {2}
                    underline strikethrough
                    padding: {3}
                    color_fg: {tui_color!(0, 0, 0)} /* Black. */
                    color_bg: {tui_color!(255, 255, 255)} /* White. */
                )
            }
        })
    }
}

#[cfg(test)]
mod test_surface_offset_origin {
    use crate::{CommonResult, FlexBoxId, FlexBoxProps, LayoutDirection,
                LayoutManagement, Surface, SurfaceProps, assert_eq2, col, height,
                req_size_pc, row, throws, width};

    #[test]
    fn test_surface_horizontal_non_zero_origin() -> CommonResult {
        fn make_container_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let container = surface.stack_of_boxes.first().unwrap();
                assert_eq2!(container.origin_pos, col(0) + row(5));
                assert_eq2!(container.dir, LayoutDirection::Horizontal);
            });
        }

        fn make_child1_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                // First child starts at the container's insertion start = (0, 5).
                assert_eq2!(child.origin_pos, col(0) + row(5));
            });
        }

        fn make_child2_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                // Second child row must equal the container's row (5), NOT 0.
                // Before the fix this was `col(50) + row(0)`.
                assert_eq2!(child.origin_pos, col(50) + row(5));
            });
        }

        throws!({
            let mut surface = Surface::default();

            // Surface starts at row 5 – the existing tests all start at (0, 0).
            surface.surface_start(SurfaceProps {
                pos: col(0) + row(5),
                size: width(100) + height(40),
            })?;

            // Horizontal container – no padding, fills the surface.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(0),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width: 100, height: 100),
                maybe_styles: None,
            })?;
            make_container_assertions(&surface)?;

            // First child: 50 % width.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(1),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 50, height: 100),
                maybe_styles: None,
            })?;
            make_child1_assertions(&surface)?;
            surface.box_end()?;

            // Second child: 50 % width.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(2),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 50, height: 100),
                maybe_styles: None,
            })?;
            make_child2_assertions(&surface)?;
            surface.box_end()?;

            surface.box_end()?;
            surface.surface_end()?;
        });
    }

    #[test]
    fn test_surface_vertical_non_zero_origin() -> CommonResult {
        fn make_container_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let container = surface.stack_of_boxes.first().unwrap();
                assert_eq2!(container.origin_pos, col(5) + row(0));
                assert_eq2!(container.dir, LayoutDirection::Vertical);
            });
        }

        fn make_child1_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(child.origin_pos, col(5) + row(0));
            });
        }

        fn make_child2_assertions(surface: &Surface) -> CommonResult {
            throws!({
                let child = surface.stack_of_boxes.last().unwrap();
                // Second child column must equal the container's column (5), NOT 0.
                // Before the fix for Vertical this would have been `col(0) + row(20)`.
                assert_eq2!(child.origin_pos, col(5) + row(20));
            });
        }

        throws!({
            let mut surface = Surface::default();

            surface.surface_start(SurfaceProps {
                pos: col(5) + row(0),
                size: width(100) + height(40),
            })?;

            // Vertical container - no padding, fills the surface.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(0),
                dir: LayoutDirection::Vertical,
                requested_size_percent: req_size_pc!(width: 100, height: 100),
                maybe_styles: None,
            })?;
            make_container_assertions(&surface)?;

            // First child: 50 % height.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(1),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width: 100, height: 50),
                maybe_styles: None,
            })?;
            make_child1_assertions(&surface)?;
            surface.box_end()?;

            // Second child: 50 % height.
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(2),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: req_size_pc!(width: 100, height: 50),
                maybe_styles: None,
            })?;
            make_child2_assertions(&surface)?;
            surface.box_end()?;

            surface.box_end()?;
            surface.surface_end()?;
        });
    }
}
