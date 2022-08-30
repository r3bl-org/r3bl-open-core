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

use std::{collections::HashMap,
          fmt::Debug,
          io::{stderr, stdout, Write},
          ops::{Add, AddAssign}};

use crossterm::{cursor::*,
                event::*,
                style::*,
                terminal::{self, *},
                *};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::*;
const DEBUG: bool = false;

// ╭┄┄┄┄┄┄┄╮
// │ exec! │
// ╯       ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// Given a crossterm command, this will run it and [log!] the [Result] that is returned. If [log!]
/// fails, then it will print a message to stderr.
///
/// Paste docs: <https://github.com/dtolnay/paste>
#[macro_export]
macro_rules! exec {
  ($cmd: expr, $log_msg: expr) => {{
    // Generate a new function that returns [CommonResult]. This needs to be called. The only
    // purpose of this generated method is to handle errors that may result from calling log! macro
    // when there are issues accessing the log file for whatever reason.
    let _fn_wrap_for_logging_err = || -> CommonResult<()> {
      throws!({
        // Execute the command.
        if let Err(err) = $cmd {
          call_if_true!(
            DEBUG,
            log!(ERROR, "crossterm: ❌ Failed to {} due to {}", $log_msg, err)
          );
        } else {
          call_if_true! {
            DEBUG,
            log!(INFO, "crossterm: ✅ {} successfully", $log_msg)
          };
        }
      })
    };

    // Call this generated function. It will fail if there are problems w/ log!(). In this case, if
    // `DEBUG` is true, then it will dump the error to stderr.
    if let Err(logging_err) = _fn_wrap_for_logging_err() {
      let msg = format!(
        "❌ Failed to log exec output of {}, {}",
        stringify!($cmd),
        $log_msg
      );
      call_if_true! {
        DEBUG,
        debug!(ERROR_RAW &msg, logging_err)
      };
    }
  }};
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ tw_command_queue! │
// ╯                   ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This works together w/ [TWCommand] to enqueue commands, but not flush them. It will return a
/// [TWCommandQueue]. Here's an example.
///
/// ```ignore
/// let mut queue = tw_command_queue!(
///   TWCommand::ClearScreen,
///   TWCommand::ResetColor
/// ); // Returns the newly created queue.
/// ```
///
/// Another example.
///
/// ```ignore
/// let mut queue = tw_command_queue!(); // Returns the newly created queue.
/// tw_command_queue!(
///   queue push
///   TWCommand::ClearScreen,
///   TWCommand::ResetColor
/// ); // Returns nothing.
/// ```
///
/// Decl macro docs:
/// - <https://veykril.github.io/tlborm/decl-macros/macros-methodical.html#repetitions>
#[macro_export]
macro_rules! tw_command_queue {
  // Create a new queue & return it. If any ($element)* are passed, then add it to new queue.
  (
    $(                      /* Start a repetition. */
      $element:expr         /* Expression. */
    )                       /* End repetition. */
    ,                       /* Comma separated. */
    *                       /* Zero or more times. */
  ) => {
    /* Enclose the expansion in a block so that we can use multiple statements. */
    {
      let mut queue = TWCommandQueue::default();
      /* Start a repetition. */
      $(
        /* Each repeat will contain the following statement, with $element replaced. */
        queue.push($element);
      )*
      queue
    }
  };
  // Add a bunch of TWCommands $element* to the existing $queue & return nothing.
  ($queue:ident push $($element:expr),+) => {
    $(
      /* Each repeat will contain the following statement, with $element replaced. */
      $queue.push($element);
    )*
  };
}

// ╭┄┄┄┄┄┄┄┄┄┄┄╮
// │ TWCommand │
// ╯           ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum TWCommand {
  EnterRawMode,
  ExitRawMode,
  /// [Position] is the absolute column and row on the terminal screen.
  MoveCursorPositionAbs(Position),
  /// 1st [Position] is the origin column and row, and the 2nd [Position] is the offset column and
  /// row. They are added together to move the absolute position on the terminal screen.
  MoveCursorPositionRelTo(Position, Position),
  ClearScreen,
  /// Directly set the fg color for crossterm w/out using [Style].
  SetFgColor(TWColor),
  /// Directly set the bg color for crossterm w/out using [Style].
  SetBgColor(TWColor),
  ResetColor,
  /// Translate [Style] into fg and bg colors for crossterm.
  ApplyColors(Option<Style>),
  /// Translate [Style] into attributes [static@STYLE_TO_ATTRIBUTE_MAP] for crossterm (bold, underline,
  /// strikethrough, etc)
  PrintWithAttributes(String, Option<Style>),
  CursorShow,
  CursorHide,
}

mod command_helpers {
  use super::*;

  impl Debug for TWCommand {
    /// When [TWCommandQueue] is printed as debug, each [TWCommand] is printed using this method.
    /// Also [exec!] does not use this; it has its own way of logging output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(
        f,
        "{}",
        match self {
          TWCommand::EnterRawMode => "EnterRawMode".into(),
          TWCommand::ExitRawMode => "ExitRawMode".into(),
          TWCommand::MoveCursorPositionAbs(pos) => format!("MoveCursorPositionAbs({:?})", pos),
          TWCommand::MoveCursorPositionRelTo(orig_pos, rel_pos) =>
            format!("MoveCursorPositionRelTo({:?}, {:?})", orig_pos, rel_pos),
          TWCommand::ClearScreen => "ClearScreen".into(),
          TWCommand::SetFgColor(fg_color) => format!("SetFgColor({:?})", fg_color),
          TWCommand::SetBgColor(bg_color) => format!("SetBgColor({:?})", bg_color),
          TWCommand::ResetColor => "ResetColor".into(),
          TWCommand::ApplyColors(maybe_style) => match maybe_style {
            Some(style) => format!("ApplyColors({:?})", style),
            None => "ApplyColors(None)".into(),
          },
          TWCommand::PrintWithAttributes(text, maybe_style) => {
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
        }
      )
    }
  }
}

impl TWCommand {
  pub fn flush() {
    exec!(stdout().flush(), "flush() -> stdout");
    exec!(stderr().flush(), "flush() -> stderr");
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ TWCommandQueue │
// ╯                ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// This works w/ [TWCommand] items. It allows them to be added in sequence, and then flushed at the
/// end. Here's an example.
///
/// ```ignore
/// let mut queue = CommandQueue::default();
/// queue.add(TWCommand::ClearScreen);
/// queue.add(TWCommand::CursorShow);
/// queue.flush();
/// ```
#[derive(Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TWCommandQueue {
  pub queue: Vec<TWCommand>,
}

mod queue_helpers {
  use super::*;

  impl Debug for TWCommandQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      let mut temp_vec: Vec<String> = vec![];
      for command in &self.queue {
        let line: String = format!("{:?}", command);
        temp_vec.push(line);
      }
      write!(f, "\n    - {}", temp_vec.join("\n    - "))
    }
  }

  impl AddAssign for TWCommandQueue {
    fn add_assign(&mut self, other: TWCommandQueue) { self.queue.extend(other.queue); }
  }

  impl Add<TWCommand> for TWCommandQueue {
    type Output = TWCommandQueue;
    fn add(mut self, other: TWCommand) -> TWCommandQueue {
      self.queue.push(other);
      self
    }
  }

  impl AddAssign<TWCommand> for TWCommandQueue {
    fn add_assign(&mut self, other: TWCommand) { self.queue.push(other); }
  }
}

impl TWCommandQueue {
  pub fn push(&mut self, cmd_wrapper: TWCommand) -> &mut Self {
    self.queue.push(cmd_wrapper);
    self
  }

  pub fn push_all(&mut self, cmd_wrapper_vec: Vec<TWCommand>) -> &mut Self {
    self.queue.extend(cmd_wrapper_vec);
    self
  }

  pub fn push_another(&mut self, other: TWCommandQueue) -> &mut Self {
    self.queue.extend(other.queue);
    self
  }

  // TODO: support termion, along w/ crossterm, by providing another impl of this fn
  #[allow(unreachable_patterns)]
  pub fn flush(&self, clear_before_flush: bool) {
    let mut skip_flush = false;

    if clear_before_flush {
      exec! {
        queue!(stdout(),
          ResetColor,
          Clear(ClearType::All),
        ),
      "flush() -> after ResetColor, Clear"
      }
    }

    self.queue.iter().for_each(|cmd_wrapper| match cmd_wrapper {
      TWCommand::EnterRawMode => {
        exec! {
          terminal::enable_raw_mode(),
          "EnterRawMode -> enable_raw_mode()"
        };
        exec! {
          queue!(stdout(),
            EnableMouseCapture,
            EnterAlternateScreen,
            MoveTo(0,0),
            Clear(ClearType::All),
            Hide,
          ),
        "EnterRawMode -> EnableMouseCapture, EnterAlternateScreen, MoveTo(0,0), Clear(ClearType::All), Hide"
        }
        TWCommand::flush();
        skip_flush = true;
      }
      TWCommand::ExitRawMode => {
        exec! {
          queue!(stdout(),
            Show,
            LeaveAlternateScreen,
            DisableMouseCapture
          ),
          "ExitRawMode -> Show, LeaveAlternateScreen, DisableMouseCapture"
        };
        TWCommand::flush();
        exec! {terminal::disable_raw_mode(), "ExitRawMode -> disable_raw_mode()"}
        skip_flush = true;
      }
      TWCommand::MoveCursorPositionAbs(Position { col, row }) => {
        exec!(
          queue!(stdout(), MoveTo(*col, *row)),
          format!("MoveCursorPosition(col: {}, row: {})", *col, *row)
        )
      }
      TWCommand::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
        let Position { col, row } = *box_origin_pos + *content_rel_pos;
        exec!(
          queue!(stdout(), MoveTo(col, row)),
          format!("MoveCursorPosition(col: {}, row: {})", col, row)
        )
      }
      TWCommand::ClearScreen => {
        exec!(queue!(stdout(), Clear(ClearType::All)), "ClearScreen")
      }
      TWCommand::SetFgColor(color) => {
        let color = color_converter::to_crossterm_color(*color);
        exec!(
          queue!(stdout(), SetForegroundColor(color)),
          format!("SetFgColor({:?})", color)
        )
      }
      TWCommand::SetBgColor(color) => {
        let color: crossterm::style::Color = color_converter::to_crossterm_color(*color);
        exec!(
          queue!(stdout(), SetBackgroundColor(color)),
          format!("SetBgColor({:?})", color)
        )
      }
      TWCommand::ResetColor => {
        exec!(queue!(stdout(), ResetColor), "ResetColor")
      }
      TWCommand::CursorShow => {
        exec!(queue!(stdout(), Show), "CursorShow")
      }
      TWCommand::CursorHide => {
        exec!(queue!(stdout(), Hide), "CursorHide")
      }
      TWCommand::ApplyColors(style) => {
        if style.is_some() {
          // Use Style to set crossterm Colors.
          // Docs: https://docs.rs/crossterm/latest/crossterm/style/index.html#colors
          let mut style = style.clone().unwrap();
          let mask = style.get_bitflags();
          if mask.contains(StyleFlag::COLOR_BG_SET) {
            let color_bg = style.color_bg.unwrap();
            let color_bg: crossterm::style::Color = color_converter::to_crossterm_color(color_bg);
            exec!(
              queue!(stdout(), SetBackgroundColor(color_bg)),
              format!("ApplyColors -> SetBackgroundColor({:?})", color_bg)
            )
          }
          if mask.contains(StyleFlag::COLOR_FG_SET) {
            let color_fg = style.color_fg.unwrap();
            let color_fg: crossterm::style::Color = color_converter::to_crossterm_color(color_fg);
            exec!(
              queue!(stdout(), SetForegroundColor(color_fg)),
              format!("ApplyColors -> SetForegroundColor({:?})", color_fg)
            )
          }
        }
      }
      TWCommand::PrintWithAttributes(text, maybe_style) => {
        let log_msg: String = match try_strip_ansi(text) {
          Some(text_plain) => format!("\"{}\"", text_plain),
          None => format!("bytes {}", text.len())
        };

        match maybe_style {
          Some(style) => {
            // Use Style to set crossterm Attributes.
            // Docs: https://docs.rs/crossterm/latest/crossterm/style/index.html#attributes
            let mask = style.clone().get_bitflags();
            let mut needs_reset = false;

            STYLE_TO_ATTRIBUTE_MAP.iter().for_each(|(flag, attr)| {
              if mask.contains(*flag) {
                exec!(
                  queue!(stdout(), SetAttribute(*attr)),
                  format!("PrintWithAttributes -> SetAttribute({:?})", attr)
                );
                needs_reset = true;
              }
            });

            exec!(
              queue!(stdout(), Print(text.clone())),
              format!("PrintWithAttributes -> Style + Print({})", log_msg)
            );

            if needs_reset {
              exec!(
                queue!(stdout(), SetAttribute(Attribute::Reset)),
                format!("PrintWithAttributes -> SetAttribute(Reset))")
              );
            }
          }
          None =>{
            exec!(
              queue!(stdout(), Print(text.clone())),
              format!("PrintWithAttributes -> None + Print({})", log_msg)
            )
          }
        }
      }
      _ => {
        log_no_err!{
          ERROR,
          target: "TWCommandQueue::flush()", 
          "🧨 {:?} not implemented", cmd_wrapper
        };
      }
    });

    // Flush all the commands that were added via calls to `queue!` above.
    if !skip_flush {
      TWCommand::flush()
    };
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Style to attribute map │
// ╯                        ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
pub static STYLE_TO_ATTRIBUTE_MAP: Lazy<HashMap<StyleFlag, Attribute>> = Lazy::new(|| {
  let mut map = HashMap::new();
  map.insert(StyleFlag::BOLD_SET, Attribute::Bold);
  map.insert(StyleFlag::DIM_SET, Attribute::Dim);
  map.insert(StyleFlag::UNDERLINE_SET, Attribute::Underlined);
  map.insert(StyleFlag::REVERSE_SET, Attribute::Reverse);
  map.insert(StyleFlag::HIDDEN_SET, Attribute::Hidden);
  map.insert(StyleFlag::STRIKETHROUGH_SET, Attribute::Fraktur);
  map
});
