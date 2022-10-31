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
  fn test_create_styled_text_with_dsl() -> CommonResult<()> {
    throws!({
      let st_vec = helpers::create_styled_text()?;
      assert_eq2!(st_vec.is_empty(), false);
      assert_eq2!(st_vec.len(), 2);
    })
  }

  #[test]
  fn test_styled_text_renders_correctly() -> CommonResult<()> {
    throws!({
      let st_vec = helpers::create_styled_text()?;
      let mut render_ops = render_ops!();
      st_vec.render_into(&mut render_ops);

      let mut pipeline = render_pipeline!();
      pipeline.push(ZOrder::Normal, render_ops);

      debug!(pipeline);
      assert_eq2!(pipeline.len(), 1);

      let set: &Vec<RenderOps> = pipeline.get(&ZOrder::Normal).unwrap();

      // "Hello" and "World" together.
      assert_eq2!(set.len(), 1);

      // 3 RenderOp each for "Hello" & "World".
      assert_eq2!(pipeline.get_all_render_op_in(ZOrder::Normal).unwrap().len(), 6);
    })
  }

  mod helpers {
    use super::*;

    pub fn create_styled_text() -> CommonResult<Vec<StyledText>> {
      throws_with_return!({
        let stylesheet = create_stylesheet()?;
        let maybe_style1 = stylesheet.find_style_by_id("style1");
        let maybe_style2 = stylesheet.find_style_by_id("style2");

        styled_texts! {
          styled_text! {
            "Hello".to_string(),
            maybe_style1.unwrap()
          },
          styled_text! {
            "World".to_string(),
            maybe_style2.unwrap()
          }
        }
      })
    }

    pub fn create_stylesheet() -> CommonResult<Stylesheet> {
      throws_with_return!({
        stylesheet! {
          style! {
            id: "style1"
            padding: 1
            color_bg: TWColor::Rgb { r: 55, g: 55, b: 100 }
          },
          style! {
            id: "style2"
            padding: 1
            color_bg: TWColor::Rgb { r: 55, g: 55, b: 248 }
          }
        }
      })
    }
  }
}
