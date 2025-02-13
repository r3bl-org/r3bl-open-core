/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use r3bl_core::TuiStyledTexts;

use crate::{RenderOp, RenderOps};

pub fn render_tui_styled_texts_into(texts: &TuiStyledTexts, render_ops: &mut RenderOps) {
    for styled_text in texts.inner.iter() {
        let style = styled_text.get_style();
        render_ops.push(RenderOp::ApplyColors(Some(*style)));
        render_ops.push(RenderOp::PaintTextWithAttributes(
            // PERF: [ ] perf
            styled_text.get_text().into(),
            Some(*style),
        ));
        render_ops.push(RenderOp::ResetColor);
    }
}

#[cfg(test)]
mod tests {
    use r3bl_core::{assert_eq2,
                    console_log,
                    throws,
                    throws_with_return,
                    tui_styled_text,
                    tui_styled_texts,
                    tui_stylesheet,
                    CommonResult,
                    RgbValue,
                    TuiColor,
                    TuiStylesheet,
                    VecArray};
    use r3bl_macro::tui_style;

    use super::*;
    use crate::{render_ops, render_pipeline, ZOrder};

    #[test]
    fn test_styled_text_renders_correctly() -> CommonResult<()> {
        throws!({
            let texts = helpers::create_styled_text()?;
            let mut render_ops = render_ops!();
            render_tui_styled_texts_into(&texts, &mut render_ops);

            let mut pipeline = render_pipeline!();
            pipeline.push(ZOrder::Normal, render_ops);

            console_log!(pipeline);
            assert_eq2!(pipeline.len(), 1);

            let set: &VecArray<RenderOps> = pipeline.get(&ZOrder::Normal).unwrap();

            // "Hello" and "World" together.
            assert_eq2!(set.len(), 1);

            // 3 RenderOp each for "Hello" & "World".
            assert_eq2!(
                pipeline.get_all_render_op_in(ZOrder::Normal).unwrap().len(),
                6
            );
        })
    }

    mod helpers {
        use super::*;

        pub fn create_styled_text() -> CommonResult<TuiStyledTexts> {
            throws_with_return!({
                let stylesheet = create_stylesheet()?;
                let maybe_style1 = stylesheet.find_style_by_id(1);
                let maybe_style2 = stylesheet.find_style_by_id(2);

                tui_styled_texts! {
                    tui_styled_text! {
                        @style: maybe_style1.unwrap(),
                        @text: "Hello",
                    },
                    tui_styled_text! {
                        @style: maybe_style2.unwrap(),
                        @text: "World",
                    }
                }
            })
        }

        pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
            throws_with_return!({
                tui_stylesheet! {
                  tui_style! {
                    id: 1
                    padding: 1
                    color_bg: TuiColor::Rgb(RgbValue{ red: 55, green: 55, blue: 100 })
                  },
                  tui_style! {
                    id: 2
                    padding: 1
                    color_bg: TuiColor::Rgb(RgbValue{ red: 55, green: 55, blue: 248 })
                  }
                }
            })
        }
    }
}
