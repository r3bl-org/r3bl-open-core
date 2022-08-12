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
fn test_simple_2_col_layout() -> CommonResult<()> {
  throws!({
    let mut tw_surface = TWSurface {
      stylesheet: dsl_stylesheet()?,
      ..TWSurface::default()
    };
    tw_surface.surface_start(TWSurfaceProps {
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
fn create_main_container(tw_surface: &mut TWSurface) -> CommonResult<()> {
  throws!({
    tw_surface.box_start(TWBoxProps {
      id: "container".to_string(),
      dir: Direction::Horizontal,
      req_size: (100, 100).try_into()?,
      ..Default::default()
    })?;
    make_container_assertions(tw_surface)?;
    create_left_col(tw_surface)?;
    create_right_col(tw_surface)?;
    tw_surface.box_end()?;
  });

  fn make_container_assertions(tw_surface: &TWSurface) -> CommonResult<()> {
    throws!({
      let layout_item = tw_surface.stack_of_boxes.first().unwrap();
      assert_eq!(layout_item.id, "container");
      assert_eq!(layout_item.dir, Direction::Horizontal);
      assert_eq!(layout_item.origin_pos, (0, 0).into());
      assert_eq!(layout_item.bounding_size, (500, 500).into());
      assert_eq!(layout_item.req_size_percent, (100, 100).try_into()?);
      assert_eq!(layout_item.box_cursor_pos, Some((0, 0).into()));
      assert_eq!(layout_item.get_computed_style(), None);
    });
  }
}

/// Left column "col_1".
fn create_left_col(tw_surface: &mut TWSurface) -> CommonResult<()> {
  throws!({
    tw_surface.box_start(TWBoxProps {
      styles: tw_surface.stylesheet.find_styles_by_ids(vec!["style1"]),
      id: "col_1".to_string(),
      dir: Direction::Vertical,
      req_size: (50, 100).try_into()?,
    })?;
    make_left_col_assertions(tw_surface)?;
    tw_surface.box_end()?;
  });

  fn make_left_col_assertions(tw_surface: &TWSurface) -> CommonResult<()> {
    throws!({
      let layout_item = tw_surface.stack_of_boxes.last().unwrap();
      assert_eq!(layout_item.id, "col_1");
      assert_eq!(layout_item.dir, Direction::Vertical);
      assert_eq!(layout_item.origin_pos, (2, 2).into()); // Take margin into account.
      assert_eq!(layout_item.bounding_size, (246, 496).into()); // Take margin into account.
      assert_eq!(layout_item.req_size_percent, (50, 100).try_into()?);
      assert_eq!(layout_item.box_cursor_pos, None);
      assert_eq!(
        layout_item.get_computed_style(),
        Stylesheet::compute(tw_surface.stylesheet.find_styles_by_ids(vec!["style1"]))
      );
    });
  }
}

/// Right column "col_2".
fn create_right_col(tw_surface: &mut TWSurface) -> CommonResult<()> {
  throws!({
    tw_surface.box_start(TWBoxProps {
      styles: tw_surface.stylesheet.find_styles_by_ids(vec!["style2"]),
      id: "col_2".to_string(),
      dir: Direction::Vertical,
      req_size: (50, 100).try_into()?,
    })?;
    make_right_col_assertions(tw_surface)?;
    tw_surface.box_end()?;
  });

  fn make_right_col_assertions(tw_surface: &TWSurface) -> CommonResult<()> {
    throws!({
      let current_box = tw_surface.stack_of_boxes.last().unwrap();
      assert_eq!(current_box.id, "col_2");
      assert_eq!(current_box.dir, Direction::Vertical);
      assert_eq!(current_box.origin_pos, (253, 3).into()); // Take margin into account.
      assert_eq!(current_box.bounding_size, (244, 494).into()); // Take margin into account.
      assert_eq!(current_box.req_size_percent, (50, 100).try_into()?);
      assert_eq!(current_box.box_cursor_pos, None);
      assert_eq!(
        current_box.get_computed_style(),
        Stylesheet::compute(tw_surface.stylesheet.find_styles_by_ids(vec!["style2"]))
      );
    });
  }
}

/// Create a stylesheet containing styles.
fn create_stylesheet() -> CommonResult<Stylesheet> {
  throws_with_return!({
    let mut stylesheet = Stylesheet::new();

    stylesheet.add_styles(vec![
      style! {
          id: style1
          attrib: [dim, bold]
          margin: 2
          color_fg: Color::Rgb { r: 255, g: 255, b: 0 } /* Yellow. */
          color_bg:  Color::Rgb { r: 128, g: 128, b: 128 } /* Grey. */
      },
      style! {
        id: style2
        attrib: [underline, strikethrough]
        margin: 3
        color_fg: Color::Rgb { r: 0, g: 0, b: 0 } /* Black. */
        color_bg: Color::Rgb { r: 255, g: 255, b: 255 } /* White. */
      },
    ])?;

    stylesheet
  })
}

/// Create a stylesheet containing styles using DSL.
fn dsl_stylesheet() -> CommonResult<Stylesheet> {
  throws_with_return!({
    stylesheet! {
      style! {
          id: style1
          attrib: [dim, bold]
          margin: 2
          color_fg: Color::Rgb { r: 255, g: 255, b: 0 } /* Yellow. */
          color_bg:  Color::Rgb { r: 128, g: 128, b: 128 } /* Grey. */
      },
      style! {
        id: style2
        attrib: [underline, strikethrough]
        margin: 3
        color_fg: Color::Rgb { r: 0, g: 0, b: 0 } /* Black. */
        color_bg: Color::Rgb { r: 255, g: 255, b: 255 } /* White. */
      }
  }
})
}