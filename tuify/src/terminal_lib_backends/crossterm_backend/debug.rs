use std::fmt::{Formatter, Result};
use r3bl_rs_utils_core::*;
use crate::*;
pub struct CrosstermDebugFormatRenderOp;

fn format_print_text(
    op_name: &str,
    text: &String,
    maybe_style: &Option<Style>,
) -> String {
    match maybe_style {
        Some(style) => {
            format!("{op_name}({} bytes, {style:?})", text.len())
        }
        None => format!("{op_name}({} bytes, None)", text.len()),
    }
}

impl DebugFormatRenderOp for CrosstermDebugFormatRenderOp {
    fn debug_format(&self, this: &RenderOp, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}",
            match this {
                RenderOp::Noop => "Noop".into(),
                RenderOp::EnterRawMode => "EnterRawMode".into(),
                RenderOp::ExitRawMode => "ExitRawMode".into(),
                RenderOp::MoveCursorPositionAbs(pos) =>
                    format!("MoveCursorPositionAbs({pos:?})"),
                RenderOp::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) =>
                    format!(
                        "MoveCursorPositionRelTo({box_origin_pos:?}, {content_rel_pos:?})"
                    ),
                RenderOp::ClearScreen => "ClearScreen".into(),
                RenderOp::SetFgColor(fg_color) => format!("SetFgColor({fg_color:?})"),
                RenderOp::SetBgColor(bg_color) => format!("SetBgColor({bg_color:?})"),
                RenderOp::ResetColor => "ResetColor".into(),
                RenderOp::ApplyColors(maybe_style) => match maybe_style {
                    Some(style) => format!("ApplyColors({style:?})"),
                    None => "ApplyColors(None)".into(),
                },
                RenderOp::CompositorNoClipTruncPaintTextWithAttributes(
                    text,
                    maybe_style,
                ) => {
                    format_print_text("Compositor..PrintText...", text, maybe_style)
                }
                RenderOp::PaintTextWithAttributes(text, maybe_style) => {
                    format_print_text("PrintTextWithAttributes", text, maybe_style)
                }
            }
        )
    }
}
