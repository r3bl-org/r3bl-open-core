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

//! ANSI colorized text <https://github.com/ogham/rust-ansi-term> helper methods.

use ansi_term::Colour::Purple;

#[macro_export]
macro_rules! print_header {
  (error $msg: expr) => {
    let hamburger = "☰";
    let msg = format!("{0} {1} {0}", hamburger, $msg);
    eprintln!("{}", Purple.paint(&msg));
  };
  (normal $msg: expr) => {
    let hamburger = "☰";
    let msg = format!("{0} {1} {0}", hamburger, $msg);
    println!("{}", Purple.paint(&msg));
  };
}

///
/// Equivalent for template string literal. One way to do this using `format!`
/// 1. <https://doc.rust-lang.org/std/fmt/>
/// 2. <https://internals.rust-lang.org/t/string-interpolation-template-literals-like-js/9082/3>
pub fn print_header(msg: &str) {
  print_header!(normal msg);
}

pub fn eprint_header(msg: &str) {
  print_header!(error msg);
}

pub mod styles {
  use ansi_term::{ANSIGenericString,
                  Colour::{Blue, Green, Red, White}};

  pub fn style_primary(text: &str) -> ANSIGenericString<str> { Green.bold().paint(text) }

  pub fn style_prompt(text: &str) -> ANSIGenericString<str> { Blue.bold().paint(text) }

  pub fn style_error(text: &str) -> ANSIGenericString<str> { Red.bold().paint(text) }

  pub fn style_dimmed(text: &str) -> ANSIGenericString<str> { White.underline().paint(text) }
}
