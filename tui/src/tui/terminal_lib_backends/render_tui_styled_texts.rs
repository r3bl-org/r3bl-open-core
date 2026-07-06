// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{RenderOpCommon, RenderOpIR, RenderOpIRVec, TuiStyledTexts};

pub fn render_tui_styled_texts_into(
    texts: &TuiStyledTexts,
    mut render_ops: &mut RenderOpIRVec,
) {
    for styled_text in &texts.inner {
        let style = styled_text.get_style();
        render_ops += RenderOpCommon::ApplyColors(Some(*style));
        render_ops += RenderOpIR::PaintTextWithAttributes(
            styled_text.get_text().into(),
            Some(*style),
        );
        render_ops += RenderOpCommon::ResetColor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommonResult, TuiStylesheet, ZOrder, assert_eq2, console_log,
                render_pipeline, throws};

    #[test]
    fn test_styled_text_renders_correctly() -> CommonResult {
        throws!({
            let texts = test_helper::create_styled_text()?;
            let mut render_ops = RenderOpIRVec::new();
            render_tui_styled_texts_into(&texts, &mut render_ops);

            let mut pipeline = render_pipeline!();
            pipeline.push(ZOrder::Normal, render_ops);

            console_log!(pipeline);
            let _set: &RenderOpIRVec = pipeline.get(&ZOrder::Normal);

            // 3 RenderOpIR each for "Hello" & "World".
            assert_eq2!(pipeline.get_all_render_op_in(ZOrder::Normal), 6);
        })
    }

    mod test_helper {
        use super::*;
        use crate::{new_style, throws_with_return, tui_color, tui_styled_text,
                    tui_styled_texts, tui_stylesheet};

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
                    new_style! {
                        id: {1}
                        padding: {1}
                        color_bg: {tui_color!(55, 55, 100)}
                    },
                    new_style! {
                        id: {2}
                        padding: {1}
                        color_bg: {tui_color!(55, 55, 248)}
                    }
                }
            })
        }
    }
}
