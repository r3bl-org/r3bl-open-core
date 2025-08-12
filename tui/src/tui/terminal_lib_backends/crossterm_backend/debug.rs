// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::{Formatter, Result};

use crate::{DebugFormatRenderOp, RenderOp,
            RenderOp::{ApplyColors, ClearScreen,
                       CompositorNoClipTruncPaintTextWithAttributes, EnterRawMode,
                       ExitRawMode, MoveCursorPositionAbs, MoveCursorPositionRelTo,
                       Noop, PaintTextWithAttributes, ResetColor, SetBgColor,
                       SetFgColor},
            TuiStyle};

#[derive(Debug)]
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
                format_print_text(f, "Compositor..PrintText...", text, *maybe_style)
            }
            PaintTextWithAttributes(text, maybe_style) => {
                format_print_text(f, "PrintTextWithAttributes", text, *maybe_style)
            }
        }
    }
}

fn format_print_text(
    f: &mut Formatter<'_>,
    op_name: &str,
    text: &str,
    maybe_style: Option<TuiStyle>,
) -> Result {
    match maybe_style {
        Some(style) => write!(f, "{op_name}({} bytes, {style:?})", text.len()),
        None => write!(f, "{op_name}({} bytes, None)", text.len()),
    }
}
