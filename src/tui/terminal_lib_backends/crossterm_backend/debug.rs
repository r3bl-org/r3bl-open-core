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

use std::fmt;

use crate::*;

pub struct CrosstermDebugFormatCommand;

impl DebugFormatCommand for CrosstermDebugFormatCommand {
  fn debug_fmt(&self, this: &TWCommand, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "{}",
      match this {
        TWCommand::EnterRawMode => "EnterRawMode".into(),
        TWCommand::ExitRawMode => "ExitRawMode".into(),
        TWCommand::MoveCursorPositionAbs(pos) => format!("MoveCursorPositionAbs({:?})", pos),
        TWCommand::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => format!(
          "MoveCursorPositionRelTo({:?}, {:?})",
          box_origin_pos, content_rel_pos
        ),
        TWCommand::ClearScreen => "ClearScreen".into(),
        TWCommand::SetFgColor(fg_color) => format!("SetFgColor({:?})", fg_color),
        TWCommand::SetBgColor(bg_color) => format!("SetBgColor({:?})", bg_color),
        TWCommand::ResetColor => "ResetColor".into(),
        TWCommand::ApplyColors(maybe_style) => match maybe_style {
          Some(style) => format!("ApplyColors({:?})", style),
          None => "ApplyColors(None)".into(),
        },
        TWCommand::PrintPlainTextWithAttributes(text, maybe_style)
        | TWCommand::PrintANSITextWithAttributes(text, maybe_style) => {
          match try_strip_ansi(text) {
            Some(plain_text) => {
              // Successfully stripped ANSI escape codes.
              match maybe_style {
                Some(style) => format!("PrintWithAttributes(\"{}\", {:?})", plain_text, style),
                None => format!("PrintWithAttributes(\"{}\", None)", plain_text),
              }
            }
            None => {
              // Couldn't strip ANSI, so just print the text.
              match maybe_style {
                Some(style) => format!("PrintWithAttributes({} bytes, {:?})", text.len(), style),
                None => format!("PrintWithAttributes({} bytes, None)", text.len()),
              }
            }
          }
        }
        TWCommand::CursorShow => "CursorShow".into(),
        TWCommand::CursorHide => "CursorHide".into(),
        TWCommand::RequestShowCursorAtPositionAbs(pos) =>
          format!("ShowCursorAtPosition({:?})", pos),
        TWCommand::RequestShowCursorAtPositionRelTo(box_origin_pos, content_rel_pos) => format!(
          "ShowCursorAtPosition({:?}, {:?})",
          box_origin_pos, content_rel_pos
        ),
      }
    )
  }
}
