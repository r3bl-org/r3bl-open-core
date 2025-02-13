/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use r3bl_core::TuiStyle;

use crate::{DebugFormatRenderOp, RenderOp, RenderOp::*};

pub struct CrosstermDebugFormatRenderOp;

impl DebugFormatRenderOp for CrosstermDebugFormatRenderOp {
    fn fmt_debug(&self, this: &RenderOp, f: &mut Formatter<'_>) -> Result {
        match this {
            Noop => {
                write!(f, "Noop")
            }
            EnterRawMode => {
                write!(f, "EnterRawMode")
            }
            ExitRawMode => {
                write!(f, "ExitRawMode")
            }
            ClearScreen => {
                write!(f, "ClearScreen")
            }
            ResetColor => {
                write!(f, "ResetColor")
            }
            SetFgColor(fg_color) => {
                write!(f, "SetFgColor({fg_color:?})")
            }
            SetBgColor(bg_color) => {
                write!(f, "SetBgColor({bg_color:?})")
            }
            ApplyColors(maybe_style) => match maybe_style {
                Some(style) => write!(f, "ApplyColors({style:?})"),
                None => write!(f, "ApplyColors(None)"),
            },
            MoveCursorPositionAbs(pos) => {
                write!(f, "MoveCursorPositionAbs({pos:?})")
            }
            MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => write!(
                f,
                "MoveCursorPositionRelTo({box_origin_pos:?}, {content_rel_pos:?})"
            ),
            CompositorNoClipTruncPaintTextWithAttributes(text, maybe_style) => {
                format_print_text(f, "Compositor..PrintText...", text, maybe_style)
            }
            PaintTextWithAttributes(text, maybe_style) => {
                format_print_text(f, "PrintTextWithAttributes", text, maybe_style)
            }
        }
    }
}

fn format_print_text(
    f: &mut Formatter<'_>,
    op_name: &str,
    text: &str,
    maybe_style: &Option<TuiStyle>,
) -> Result {
    match maybe_style {
        Some(style) => write!(f, "{op_name}({} bytes, {style:?})", text.len()),
        None => write!(f, "{op_name}({} bytes, None)", text.len()),
    }
}
