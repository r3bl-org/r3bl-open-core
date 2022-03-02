/*
 * Copyright (c) 2022 R3BL LLC. All rights reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use ansi_term::Colour::Purple;

/// ANSI colorized text:
/// - <https://github.com/ogham/rust-ansi-term>
///
/// Equivalent for template string literal. One way to do this using `format!`
/// 1. <https://doc.rust-lang.org/std/fmt/>
/// 2. <https://internals.rust-lang.org/t/string-interpolation-template-literals-like-js/9082/3>
pub fn print_header(msg: &str) {
  let hamburger = "☰";
  let msg = format!("{0} {1} {0}", hamburger, msg);
  println!("{}", Purple.paint(&msg));
}

/// Equivalent for template string literal. Another way to do this using `+=` and `insert_str`.
pub fn print_header2(arg: &str) {
  let hamburger = "☰";
  let mut msg = String::from(hamburger);
  msg += " "; // msg.insert_str(msg.len(), " ");
  msg += arg; // msg.insert_str(msg.len(), arg);
  msg.insert_str(msg.len(), " ");
  msg.insert_str(msg.len(), hamburger);
  println!("{}", Purple.paint(&msg))
}

pub mod styles {
  use ansi_term::ANSIGenericString;
  use ansi_term::Colour::{Blue, Green, Red, White};

  pub fn style_primary(text: &str) -> ANSIGenericString<str> {
    return Green.bold().paint(text);
  }

  pub fn style_prompt(text: &str) -> ANSIGenericString<str> {
    return Blue.bold().paint(text);
  }

  pub fn style_error(text: &str) -> ANSIGenericString<str> {
    return Red.bold().paint(text);
  }

  pub fn style_dimmed(text: &str) -> ANSIGenericString<str> {
    return White.underline().paint(text);
  }
}
