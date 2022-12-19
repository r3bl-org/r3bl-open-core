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
    use r3bl_rs_utils_core::*;
    use r3bl_rs_utils_macro::style;

    use crate::*;

    #[test]
    fn test_surface_2_col_simple() -> CommonResult<()> {
        throws!({
            let mut surface = Surface {
                stylesheet: dsl_stylesheet()?,
                ..Default::default()
            };

            surface.surface_start(SurfaceProps {
                pos: position!(col_index: 0, row_index: 0),
                size: size!(col_count:500, row_count:500),
            })?;

            create_main_container(&mut surface)?;

            surface.surface_end()?;

            println!("{:?}", &surface.render_pipeline);
            println!(
                "{}",
                serde_json::to_string_pretty(&surface.render_pipeline).unwrap()
            );
        });
    }

    /// Main container 0.
    fn create_main_container(surface: &mut Surface) -> CommonResult<()> {
        throws!({
            surface.box_start(FlexBoxProps {
                id: 0,
                dir: Direction::Horizontal,
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
                assert_eq2!(layout_item.id, 0);
                assert_eq2!(layout_item.dir, Direction::Horizontal);
                assert_eq2!(
                    layout_item.origin_pos,
                    position!(col_index: 0, row_index: 0)
                );
                assert_eq2!(layout_item.bounds_size, size!(col_count:500, row_count:500)); // due to `padding: 1`
                assert_eq2!(
                    layout_item.requested_size_percent,
                    requested_size_percent!(width:100, height:100)
                );
                assert_eq2!(
                    layout_item.insertion_pos_for_next_box,
                    Some(position!(col_index:0, row_index:0))
                );
                assert_eq2!(layout_item.get_computed_style(), None);
            });
        }
    }

    /// Left column 1.
    fn create_left_col(surface: &mut Surface) -> CommonResult<()> {
        throws!({
            // With macro.
            box_start! {
              in:                     surface,
              id:                     1,
              dir:                    Direction::Vertical,
              requested_size_percent: requested_size_percent!(width:50, height:100),
              styles:                 ["1"]
            }
            make_left_col_assertions(surface)?;
            box_end!(in: surface);
        });

        fn make_left_col_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let layout_item = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(layout_item.id, 1);
                assert_eq2!(layout_item.dir, Direction::Vertical);

                assert_eq2!(layout_item.origin_pos, position!(col_index:0, row_index:0));
                assert_eq2!(layout_item.bounds_size, size!(col_count:250, row_count:500));

                assert_eq2!(
                    layout_item.style_adjusted_origin_pos,
                    position!(col_index:2, row_index:2)
                ); // Take padding into account.
                assert_eq2!(
                    layout_item.style_adjusted_bounds_size,
                    size!(col_count:246, row_count:496)
                ); // Take padding into account.

                assert_eq2!(
                    layout_item.requested_size_percent,
                    requested_size_percent!(width:50, height:100)
                );
                assert_eq2!(layout_item.insertion_pos_for_next_box, None);
                assert_eq2!(
                    layout_item.get_computed_style(),
                    Stylesheet::compute(&surface.stylesheet.find_styles_by_ids(vec!["1"]))
                );
            });
        }
    }

    /// Right column 2.
    fn create_right_col(surface: &mut Surface) -> CommonResult<()> {
        throws!({
            // No macro.
            surface.box_start(FlexBoxProps {
                maybe_styles: get_styles! { @from: surface.stylesheet, ["2"] },
                id: 2,
                dir: Direction::Vertical,
                requested_size_percent: requested_size_percent!(width:50, height:100),
            })?;
            make_right_col_assertions(surface)?;
            surface.box_end()?;
        });

        fn make_right_col_assertions(surface: &Surface) -> CommonResult<()> {
            throws!({
                let current_box = surface.stack_of_boxes.last().unwrap();
                assert_eq2!(current_box.id, 2);
                assert_eq2!(current_box.dir, Direction::Vertical);

                assert_eq2!(
                    current_box.origin_pos,
                    position!(col_index: 250, row_index: 0)
                );
                assert_eq2!(current_box.bounds_size, size!(col_count:250, row_count:500));

                assert_eq2!(
                    current_box.style_adjusted_origin_pos,
                    position!(col_index: 253, row_index: 3)
                ); // Take padding into account.
                assert_eq2!(
                    current_box.style_adjusted_bounds_size,
                    size!(col_count:244, row_count:494)
                ); // Take padding into account.

                assert_eq2!(
                    current_box.requested_size_percent,
                    requested_size_percent!(width: 50, height: 100)
                );
                assert_eq2!(current_box.insertion_pos_for_next_box, None);
                assert_eq2!(
                    current_box.get_computed_style(),
                    Stylesheet::compute(&surface.stylesheet.find_styles_by_ids(vec!["2"]))
                );
            });
        }
    }

    /// Create a stylesheet containing styles using DSL.
    fn dsl_stylesheet() -> CommonResult<Stylesheet> {
        throws_with_return!({
            stylesheet! {
              style! {
                id: "1"
                attrib: [dim, bold]
                padding: 2
                color_fg: TuiColor::Rgb { r: 255, g: 255, b: 0 } /* Yellow. */
                color_bg: TuiColor::Rgb { r: 128, g: 128, b: 128 } /* Grey. */
              },
              style! {
                id: "2"
                attrib: [underline, strikethrough]
                padding: 3
                color_fg: TuiColor::Rgb { r: 0, g: 0, b: 0 } /* Black. */
                color_bg: TuiColor::Rgb { r: 255, g: 255, b: 255 } /* White. */
              }
            }
        })
    }
}
