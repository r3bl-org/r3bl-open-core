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

use std::fmt::{Formatter, Result};

use r3bl_rs_utils_core::*;

use crate::*;

pub struct CrosstermDebugFormatRenderOp;

fn format_print_text(
    op_name: &str,
    text: &str,
    maybe_style: &Option<TuiStyle>,
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
