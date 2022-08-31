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

use std::{fmt::Display,
          io::{stdout, Write},
          thread::sleep,
          time::Duration};

use rand::{thread_rng, Rng};
use serde::*;
use get_size::GetSize;

use crate::*;

/// Given a mutable [Lolcat], colorize the token tree that follows.
///
/// ```ignore
/// pub lolcat = Lolcat::default();
/// pub content = "Hello, world!";
/// let colored_content = colorize_using_lolcat!(
///   &mut lolcat, "{}", content
/// );
/// ```
///
/// See [my_print!] for more information on how this macro is written.
#[macro_export]
macro_rules! colorize_using_lolcat {
  ($lolcat: expr, $($arg:tt)*) => {
    format!("{}", std::format_args!($($arg)*)).color_with($lolcat);
  };
}

pub type OutputCollectorType = Vec<String>;

#[derive(Debug, Clone)]
pub struct OutputCollector {
  pub output_vec: OutputCollectorType,
}

impl OutputCollector {
  pub fn from(output_vec: OutputCollectorType) -> OutputCollector { OutputCollector { output_vec } }
}

impl Display for OutputCollector {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", String::from_iter(self.output_vec.clone()))
  }
}

pub trait LolcatStringExt {
  fn color_with(&self, lolcat: &mut Lolcat) -> String;
}

impl LolcatStringExt for String {
  fn color_with(&self, lolcat: &mut Lolcat) -> String { lolcat.format_str(self).to_string() }
}

pub trait SupportsColor {
  fn get_lolcat(&mut self) -> &mut Lolcat;
}

/// Docs: <https://doc.rust-lang.org/stable/std/fmt/struct.Arguments.html>
#[macro_export]
macro_rules! my_print {
  ($output_collector: expr, $($arg:tt)*) => {
      _print($output_collector, std::format_args!($($arg)*))
  };
}

/// Docs: <https://doc.rust-lang.org/stable/std/fmt/struct.Arguments.html>
fn _print(output_vec: &mut OutputCollectorType, args: std::fmt::Arguments) {
  let content = format!("{}", args);
  output_vec.push(content);
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, GetSize)]
pub struct Lolcat {
  pub color_wheel_control: ColorWheelControl,
}

impl Lolcat {
  pub fn new() -> Self {
    let control = ColorWheelControl::default();
    Self {
      color_wheel_control: control,
    }
  }

  pub fn format_str(&mut self, input_str: &str) -> OutputCollector {
    let chars_iter: std::str::Chars = input_str.chars();
    self.format_iter(chars_iter, true)
  }

  pub fn to_string(output_vec: OutputCollectorType) -> String { String::from_iter(output_vec) }

  /// - Takes in an iterator over characters.
  /// - Duplicates escape sequences, otherwise prints printable characters with
  ///   colored_print.
  /// - Print newlines correctly, resetting background.
  /// - If constantly_flush is on, it won't wait till a newline to flush stdout.
  fn format_iter<I: Iterator<Item = char>>(
    &mut self, mut iter: I, constantly_flush: bool,
  ) -> OutputCollector {
    let mut original_seed = self.color_wheel_control.seed;
    let mut ignore_whitespace = self.color_wheel_control.background_mode;
    let mut output_vec: OutputCollectorType = vec![];

    if !self.color_wheel_control.print_color {
      for character in iter {
        my_print!(&mut output_vec, "{}", character);
      }
      return OutputCollector::from(output_vec);
    }

    while let Some(character) = iter.next() {
      match character {
        // Consume escape sequences
        '\x1b' => {
          // Escape sequences seem to be one of many different categories: https://en.wikipedia.org/wiki/ANSI_escape_code
          // CSI sequences are \e \[ [bytes in 0x30-0x3F] [bytes in 0x20-0x2F] [final byte
          // in 0x40-0x7E] nF Escape seq are \e [bytes in 0x20-0x2F] [byte in
          // 0x30-0x7E] Fp Escape seq are \e [byte in 0x30-0x3F] [I have no
          // clue, but `sl` creates one where the next byte is the end of the escape
          // sequence, so assume that] Fe Escape seq are \e [byte in 0x40-0x5F]
          // [I have no idea, '' though sl doesn't make one] Fs Escape seq are
          // \e [byte in 0x60-0x7E] [I have no idea, '' though sl doesn't make one]
          // Otherwise the next byte is the whole escape sequence (maybe? I can't exactly
          // tell, but I will go with it) We will consume up to, but not
          // through, the next printable character In addition, we my_print
          // everything in the escape sequence, even if it is a color (that will be
          // overridden)
          my_print!(&mut output_vec, "\x1b");
          let mut escape_sequence_character = iter
            .next()
            .expect("Escape character with no escape sequence after it");
          my_print!(&mut output_vec, "{}", escape_sequence_character);
          match escape_sequence_character {
            '[' => loop {
              escape_sequence_character =
                iter.next().expect("CSI escape sequence did not terminate");
              my_print!(&mut output_vec, "{}", escape_sequence_character);
              match escape_sequence_character {
                '\x30'..='\x3F' => continue,
                '\x20'..='\x2F' => {
                  loop {
                    escape_sequence_character =
                      iter.next().expect("CSI escape sequence did not terminate");
                    my_print!(&mut output_vec, "{}", escape_sequence_character);
                    match escape_sequence_character {
                      '\x20'..='\x2F' => continue,
                      '\x40'..='\x7E' => break,
                      _ => {
                        panic!("CSI escape sequence terminated with an incorrect value")
                      }
                    }
                  }
                  break;
                }
                '\x40'..='\x7E' => break,
                _ => panic!("CSI escape sequence terminated with an incorrect value"),
              }
            },
            '\x20'..='\x2F' => loop {
              escape_sequence_character =
                iter.next().expect("nF escape sequence did not terminate");
              my_print!(&mut output_vec, "{}", escape_sequence_character);
              match escape_sequence_character {
                '\x20'..='\x2F' => continue,
                '\x30'..='\x7E' => break,
                _ => panic!("nF escape sequence terminated with an incorrect value"),
              }
            },
            //            '\x30' ..= '\x3F' => panic!("Fp escape sequences are not supported"),
            //            '\x40' ..= '\x5F' => panic!("Fe escape sequences are not supported"),
            //            '\x60' ..= '\x7E' => panic!("Fs escape sequences are not supported"),
            // be lazy and assume in all other cases we consume exactly 1 byte
            _ => (),
          }
        }
        // Newlines my_print escape sequences to end background prints, and in dialup mode sleep,
        // and reset the seed of the coloring and the value of ignore_whitespace.
        '\n' => {
          if self.color_wheel_control.print_color {
            // Reset the background color only, as we don't have to reset the foreground
            // till the end of the program.
            // We reset the background here because otherwise it bleeds all the way to the
            // next line.
            if self.color_wheel_control.background_mode {
              my_print!(&mut output_vec, "\x1b[49m");
            }
          }
          println!();
          if self.color_wheel_control.dialup_mode {
            let mut rng = thread_rng();
            let rand_value: u64 = rng.gen_range(30..700);
            let stall = Duration::from_millis(rand_value);
            sleep(stall);
          }

          original_seed += f64::from(self.color_wheel_control.color_change_speed);
          self.color_wheel_control.seed = original_seed; // Reset the seed, but bump it a bit
          ignore_whitespace = self.color_wheel_control.background_mode;
        }
        // If not an escape sequence or a newline, my_print a colorful escape sequence and then the
        // character.
        _ => {
          // In background mode, don't my_print colorful whitespace until the first
          // printable character.
          if ignore_whitespace && character.is_whitespace() {
            my_print!(&mut output_vec, "{}", character);
            continue;
          } else {
            ignore_whitespace = false;
          }

          self.colored_print(&mut output_vec, character);
          self.color_wheel_control.seed += f64::from(self.color_wheel_control.color_change_speed);
        }
      }

      // If we should constantly flush, flush after each completed sequence, and also
      // reset colors because otherwise weird things happen.
      if constantly_flush {
        self.reset_colors(&mut output_vec);
        stdout().flush().unwrap();
      }
    }

    OutputCollector::from(output_vec)
  }

  fn reset_colors(&self, output_vec: &mut OutputCollectorType) {
    if self.color_wheel_control.print_color {
      // Reset the background color.
      if self.color_wheel_control.background_mode {
        my_print!(output_vec, "\x1b[49m");
      }

      // Reset the foreground color.
      my_print!(output_vec, "\x1b[39m");
    }
  }

  fn colored_print(&self, output_vec: &mut OutputCollectorType, character: char) {
    if self.color_wheel_control.background_mode {
      let bg = ColorUtils::get_color_tuple(&self.color_wheel_control);
      let fg = ColorUtils::calc_fg_color(bg);
      my_print!(
        output_vec,
        "\x1b[38;2;{};{};{};48;2;{};{};{}m{}",
        fg.0,
        fg.1,
        fg.2,
        bg.0,
        bg.1,
        bg.2,
        character
      );
    } else {
      let fg = ColorUtils::get_color_tuple(&self.color_wheel_control);
      my_print!(
        output_vec,
        "\x1b[38;2;{};{};{}m{}",
        fg.0,
        fg.1,
        fg.2,
        character
      );
    }
  }
}
