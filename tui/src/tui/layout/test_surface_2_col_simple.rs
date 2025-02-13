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

#[cfg(test)]
mod tests {
    use r3bl_core::{assert_eq2,
                    col,
                    get_tui_styles,
                    height,
                    requested_size_percent,
                    row,
                    throws,
                    throws_with_return,
                    tui_stylesheet,
                    width,
                    CommonResult,
                    RgbValue,
                    TuiColor,
                    TuiStylesheet};
    use r3bl_macro::tui_style;

    use crate::{box_end,
                box_props,
                box_start,
                FlexBoxId,
                FlexBoxProps,
                LayoutDirection,
                LayoutManagement,
                Surface,
                SurfaceProps};

    #[test]
    fn test_surface_2_col_simple() -> CommonResult<()> {
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

            println!("{:?}", &surface.render_pipeline);
        });
    }

    /// Main container 0.
    fn create_main_container(surface: &mut Surface) -> CommonResult<()> {
        throws!({
            surface.box_start(FlexBoxProps {
                id: FlexBoxId::from(0),
                dir: LayoutDirection::Horizontal,
                requested_size_percent: requested_size_percent!(width:100, height:100),
                maybe_styles: None,
            })?;

            make_container_assertions(surface)?;

            create_left_col(surface)?;
            create_right_col(surface)?;

            surface.box_end()?;
        });

        fn make_container_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let layout_item = surface.stack_of_boxes.first().unwrap();
                assert_eq2!(layout_item.id, FlexBoxId::from(0));
                assert_eq2!(layout_item.dir, LayoutDirection::Horizontal);
                assert_eq2!(layout_item.origin_pos, col(0) + row(0));
                assert_eq2!(layout_item.bounds_size, width(500) + height(500)); // due to `padding: 1`
                assert_eq2!(
                    layout_item.requested_size_percent,
                    requested_size_percent!(width:100, height:100)
                );
                assert_eq2!(
                    layout_item.insertion_pos_for_next_box,
                    Some(col(0) + row(0))
                );
                assert_eq2!(layout_item.get_computed_style(), None);
            });
        }
    }

    /// Left column 1.
    fn create_left_col(surface: &mut Surface) -> CommonResult<()> {
        throws! {{
            // With macro.
            box_start! {
                in:                     surface,
                id:                     FlexBoxId::from(1),
                dir:                    LayoutDirection::Vertical,
                requested_size_percent: requested_size_percent!(width:50, height:100),
                styles:                 [1]
            }
            make_left_col_assertions(surface)?;
            box_end!(in: surface);

        }}

        fn make_left_col_assertions(surface: &Surface) -> CommonResult<()> {
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
                    requested_size_percent!(width:50, height:100)
                );
                assert_eq2!(layout_item.insertion_pos_for_next_box, None);
                assert_eq2!(
                    layout_item.get_computed_style(),
                    TuiStylesheet::compute(&surface.stylesheet.find_styles_by_ids(&[1]))
                );
            });
        }
    }

    /// Right column 2.
    fn create_right_col(surface: &mut Surface) -> CommonResult<()> {
        throws!({
            // No macro.
            surface.box_start(FlexBoxProps {
                maybe_styles: get_tui_styles! { @from: surface.stylesheet, [2] },
                id: FlexBoxId::from(2),
                dir: LayoutDirection::Vertical,
                requested_size_percent: requested_size_percent!(width:50, height:100),
            })?;
            make_right_col_assertions(surface)?;
            surface.box_end()?;
        });

        fn make_right_col_assertions(surface: &Surface) -> CommonResult<()> {
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
                    requested_size_percent!(width: 50, height: 100)
                );
                assert_eq2!(current_box.insertion_pos_for_next_box, None);
                assert_eq2!(
                    current_box.get_computed_style(),
                    TuiStylesheet::compute(&surface.stylesheet.find_styles_by_ids(&[2]))
                );
            });
        }
    }

    /// Create a stylesheet containing styles using DSL.
    fn dsl_stylesheet() -> CommonResult<TuiStylesheet> {
        throws_with_return!({
            tui_stylesheet! {
              tui_style! {
                id: 1
                attrib: [dim, bold]
                padding: 2
                color_fg: TuiColor::Rgb (RgbValue{ red: 255, green: 255, blue: 0 }) /* Yellow. */
                color_bg: TuiColor::Rgb (RgbValue{ red: 128, green: 128, blue: 128 }) /* Grey. */
              },
              tui_style! {
                id: 2
                attrib: [underline, strikethrough]
                padding: 3
                color_fg: TuiColor::Rgb (RgbValue{ red: 0, green: 0, blue: 0 }) /* Black. */
                color_bg: TuiColor::Rgb (RgbValue{ red: 255, green: 255, blue: 255 }) /* White. */
              }
            }
        })
    }
}
