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

use r3bl_rs_utils::*;

#[test]
fn test_surface_2_col_simple() -> CommonResult<()> {
  throws!({
    let mut tw_surface = Surface {
      stylesheet: dsl_stylesheet()?,
      ..Default::default()
    };

    tw_surface.surface_start(SurfaceProps {
      pos: (0, 0).into(),
      size: (500, 500).into(),
    })?;

    create_main_container(&mut tw_surface)?;

    tw_surface.surface_end()?;

    println!("{:?}", &tw_surface.render_buffer);
    println!(
      "{}",
      serde_json::to_string_pretty(&tw_surface.render_buffer).unwrap()
    );
  });
}

/// Main container "container".
fn create_main_container(tw_surface: &mut Surface) -> CommonResult<()> {
  throws!({
    tw_surface.box_start(TWBoxProps {
      id: "container".to_string(),
      dir: Direction::Horizontal,
      requested_size_percent: (100, 100).try_into()?,
      maybe_styles: None,
    })?;

    make_container_assertions(tw_surface)?;

    create_left_col(tw_surface)?;
    create_right_col(tw_surface)?;

    tw_surface.box_end()?;
  });

  fn make_container_assertions(tw_surface: &Surface) -> CommonResult<()> {
    throws!({
      let layout_item = tw_surface.stack_of_boxes.first().unwrap();
      assert_eq2!(layout_item.id, "container");
      assert_eq2!(layout_item.dir, Direction::Horizontal);
      assert_eq2!(layout_item.origin_pos, (0, 0).into());
      assert_eq2!(layout_item.bounds_size, (500, 500).into()); // due to `margin: 1`
      assert_eq2!(layout_item.requested_size_percent, (100, 100).try_into()?);
      assert_eq2!(layout_item.insertion_pos_for_next_box, Some((0, 0).into()));
      assert_eq2!(layout_item.get_computed_style(), None);
    });
  }
}

/// Left column "col_1".
fn create_left_col(tw_surface: &mut Surface) -> CommonResult<()> {
  throws!({
    // With macro.
    box_start! {
      in:                     tw_surface,
      id:                     "col_1",
      dir:                    Direction::Vertical,
      requested_size_percent: (50, 100).try_into()?,
      styles:                 ["col_1"]
    }
    make_left_col_assertions(tw_surface)?;
    tw_surface.box_end()?;
  });

  fn make_left_col_assertions(tw_surface: &Surface) -> CommonResult<()> {
    throws!({
      let layout_item = tw_surface.stack_of_boxes.last().unwrap();
      assert_eq2!(layout_item.id, "col_1");
      assert_eq2!(layout_item.dir, Direction::Vertical);

      assert_eq2!(layout_item.origin_pos, (0, 0).into());
      assert_eq2!(layout_item.bounds_size, (250, 500).into());

      assert_eq2!(layout_item.style_adjusted_origin_pos, (2, 2).into()); // Take margin into account.
      assert_eq2!(layout_item.style_adjusted_bounds_size, (246, 496).into()); // Take margin into account.

      assert_eq2!(layout_item.requested_size_percent, (50, 100).try_into()?);
      assert_eq2!(layout_item.insertion_pos_for_next_box, None);
      assert_eq2!(
        layout_item.get_computed_style(),
        Stylesheet::compute(&tw_surface.stylesheet.find_styles_by_ids(vec!["col_1"]))
      );
    });
  }
}

/// Right column "col_2".
fn create_right_col(tw_surface: &mut Surface) -> CommonResult<()> {
  throws!({
    // No macro.
    tw_surface.box_start(TWBoxProps {
      maybe_styles: get_styles! { from: tw_surface.stylesheet, ["col_2"] },
      id: "col_2".to_string(),
      dir: Direction::Vertical,
      requested_size_percent: (50, 100).try_into()?,
    })?;
    make_right_col_assertions(tw_surface)?;
    tw_surface.box_end()?;
  });

  fn make_right_col_assertions(tw_surface: &Surface) -> CommonResult<()> {
    throws!({
      let current_box = tw_surface.stack_of_boxes.last().unwrap();
      assert_eq2!(current_box.id, "col_2");
      assert_eq2!(current_box.dir, Direction::Vertical);

      assert_eq2!(current_box.origin_pos, (250, 0).into());
      assert_eq2!(current_box.bounds_size, (250, 500).into());

      assert_eq2!(current_box.style_adjusted_origin_pos, (253, 3).into()); // Take margin into account.
      assert_eq2!(current_box.style_adjusted_bounds_size, (244, 494).into()); // Take margin into account.

      assert_eq2!(current_box.requested_size_percent, (50, 100).try_into()?);
      assert_eq2!(current_box.insertion_pos_for_next_box, None);
      assert_eq2!(
        current_box.get_computed_style(),
        Stylesheet::compute(&tw_surface.stylesheet.find_styles_by_ids(vec!["col_2"]))
      );
    });
  }
}

/// Create a stylesheet containing styles using DSL.
fn dsl_stylesheet() -> CommonResult<Stylesheet> {
  throws_with_return!({
    stylesheet! {
      style! {
        id: "col_1"
        attrib: [dim, bold]
        margin: 2
        color_fg: TWColor::Rgb { r: 255, g: 255, b: 0 } /* Yellow. */
        color_bg: TWColor::Rgb { r: 128, g: 128, b: 128 } /* Grey. */
      },
      style! {
        id: "col_2"
        attrib: [underline, strikethrough]
        margin: 3
        color_fg: TWColor::Rgb { r: 0, g: 0, b: 0 } /* Black. */
        color_bg: TWColor::Rgb { r: 255, g: 255, b: 255 } /* White. */
      }
    }
  })
}
