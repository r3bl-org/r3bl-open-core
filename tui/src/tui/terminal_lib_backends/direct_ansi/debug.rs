// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DebugFormatRenderOp, RenderOpCommon,
            RenderOpCommon::{ApplyColors, ClearCurrentLine, ClearScreen, ClearToEndOfLine,
                             ClearToStartOfLine,
                             DisableBracketedPaste, DisableMouseTracking,
                             EnableBracketedPaste, EnableMouseTracking, EnterAlternateScreen,
                             EnterRawMode, ExitAlternateScreen, ExitRawMode, HideCursor,
                             MoveCursorPositionAbs, MoveCursorPositionRelTo,
                             MoveCursorToColumn, MoveCursorToNextLine,
                             MoveCursorToPreviousLine, Noop, PrintStyledText, ResetColor,
                             RestoreCursorPosition, SaveCursorPosition, SetBgColor, SetFgColor,
                             ShowCursor}};
use std::fmt::{Formatter, Result};

#[derive(Debug)]
pub struct DirectAnsiDebugFormatRenderOp;

impl DebugFormatRenderOp for DirectAnsiDebugFormatRenderOp {
    fn fmt_debug(&self, this: &RenderOpCommon, f: &mut Formatter<'_>) -> Result {
        match this {
            Noop => f.write_str("Noop"),
            EnterRawMode => f.write_str("EnterRawMode"),
            ExitRawMode => f.write_str("ExitRawMode"),
            ClearScreen => f.write_str("ClearScreen"),
            ResetColor => f.write_str("ResetColor"),
            SetFgColor(fg_color) => {
                write!(f, "SetFgColor({fg_color:?})")
            }
            SetBgColor(bg_color) => {
                write!(f, "SetBgColor({bg_color:?})")
            }
            ApplyColors(maybe_style) => match maybe_style {
                Some(style) => write!(f, "ApplyColors({style:?})"),
                None => f.write_str("ApplyColors(None)"),
            },
            MoveCursorPositionAbs(pos) => {
                write!(f, "MoveCursorPositionAbs({pos:?})")
            }
            MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => write!(
                f,
                "MoveCursorPositionRelTo({box_origin_pos:?}, {content_rel_pos:?})"
            ),
            PrintStyledText(text) => {
                write!(f, "PrintStyledText({} bytes)", text.len())
            }
            // ===== Incremental Rendering Operations (Phase 1) =====
            MoveCursorToColumn(col_index) => {
                write!(f, "MoveCursorToColumn({col_index:?})")
            }
            MoveCursorToNextLine(row_height) => {
                write!(f, "MoveCursorToNextLine({row_height:?})")
            }
            MoveCursorToPreviousLine(row_height) => {
                write!(f, "MoveCursorToPreviousLine({row_height:?})")
            }
            ClearCurrentLine => f.write_str("ClearCurrentLine"),
            ClearToEndOfLine => f.write_str("ClearToEndOfLine"),
            ClearToStartOfLine => f.write_str("ClearToStartOfLine"),
            ShowCursor => f.write_str("ShowCursor"),
            HideCursor => f.write_str("HideCursor"),
            SaveCursorPosition => f.write_str("SaveCursorPosition"),
            RestoreCursorPosition => f.write_str("RestoreCursorPosition"),
            // ===== Terminal Mode Operations =====
            EnterAlternateScreen => f.write_str("EnterAlternateScreen"),
            ExitAlternateScreen => f.write_str("ExitAlternateScreen"),
            EnableMouseTracking => f.write_str("EnableMouseTracking"),
            DisableMouseTracking => f.write_str("DisableMouseTracking"),
            EnableBracketedPaste => f.write_str("EnableBracketedPaste"),
            DisableBracketedPaste => f.write_str("DisableBracketedPaste"),
        }
    }
}
