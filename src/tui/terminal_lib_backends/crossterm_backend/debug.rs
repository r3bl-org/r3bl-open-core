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

use crate::*;

pub struct CrosstermDebugFormatRenderOp;

impl DebugFormatRenderOp for CrosstermDebugFormatRenderOp {
  fn debug_format(&self, this: &RenderOp, f: &mut Formatter<'_>) -> Result {
    write!(
      f,
      "{}",
      match this {
        RenderOp::Noop => "Noop".into(),
        RenderOp::EnterRawMode => "EnterRawMode".into(),
        RenderOp::ExitRawMode => "ExitRawMode".into(),
        RenderOp::MoveCursorPositionAbs(pos) => format!("MoveCursorPositionAbs({:?})", pos),
        RenderOp::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => format!(
          "MoveCursorPositionRelTo({:?}, {:?})",
          box_origin_pos, content_rel_pos
        ),
        RenderOp::ClearScreen => "ClearScreen".into(),
        RenderOp::SetFgColor(fg_color) => format!("SetFgColor({:?})", fg_color),
        RenderOp::SetBgColor(bg_color) => format!("SetBgColor({:?})", bg_color),
        RenderOp::ResetColor => "ResetColor".into(),
        RenderOp::ApplyColors(maybe_style) => match maybe_style {
          Some(style) => format!("ApplyColors({:?})", style),
          None => "ApplyColors(None)".into(),
        },
        RenderOp::PrintTextWithAttributes(text, maybe_style) => {
          match try_strip_ansi(text) {
            Some(plain_text) => {
              // Successfully stripped ANSI escape codes.
              match maybe_style {
                Some(style) => format!("PrintTextWithAttributes(\"{}\", {:?})", plain_text, style),
                None => format!("PrintTextWithAttributes(\"{}\", None)", plain_text),
              }
            }
            None => {
              // Couldn't strip ANSI, so just print the text.
              match maybe_style {
                Some(style) => {
                  format!("PrintTextWithAttributes({} bytes, {:?})", text.len(), style)
                }
                None => format!("PrintTextWithAttributes({} bytes, None)", text.len()),
              }
            }
          }
        }
        RenderOp::CursorShow => "CursorShow".into(),
        RenderOp::CursorHide => "CursorHide".into(),
        RenderOp::RequestShowCaretAtPositionAbs(pos) => format!("ShowCursorAtPosition({:?})", pos),
        RenderOp::RequestShowCaretAtPositionRelTo(box_origin_pos, content_rel_pos) => format!(
          "ShowCursorAtPosition({:?}, {:?})",
          box_origin_pos, content_rel_pos
        ),
      }
    )
  }
}
