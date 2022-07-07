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

//! This module contains a set of functions to make it easier to work with
//! terminals.

use std::{env::{args, Args},
          error::Error,
          io::{stdin, stdout, Write}};

use r3bl_rs_utils_core::{style_error, style_prompt};

use super::with;

/// Return String not &str due to "struct lifetime"
/// - <https://stackoverflow.com/a/29026565/2085356>
pub fn readline() -> (usize, String) {
  let mut temp_string_buffer: String = String::new();
  // <https://learning-rust.github.io/docs/e4.unwrap_and_expect.html>
  match stdin().read_line(&mut temp_string_buffer) {
    Ok(bytes_read) => {
      let guess: String = temp_string_buffer.trim().to_string(); // Remove any whitespace (including \n).
      (bytes_read, guess)
    }
    Err(_) => {
      println!("{}", style_error("Something went wrong when reading input from terminal."));
      (0, "".to_string())
    }
  }
}

/// Prints a prompt to the terminal (no buffering / immediately) without a
/// newline.
pub fn print_prompt(prompt: &str) -> Result<(), Box<dyn Error>> {
  print!("{}", style_prompt(prompt));
  stdout().lock().flush()?;
  Ok(())
}

/// Prints and prompt and then waits for input from the terminal.
pub fn readline_with_prompt(prompt: &str) -> Result<String, Box<dyn Error>> {
  print_prompt(prompt)?;
  Ok(readline().1)
}

/// If you run `echo "test" | cargo run` the following will return true.
pub fn is_stdin_piped() -> bool { atty::isnt(atty::Stream::Stdin) }

/// If you run `cargo run | grep foo` the following will return true.
pub fn is_stdout_piped() -> bool { atty::isnt(atty::Stream::Stdout) }

/// If you run `cargo run` the following will return true.
pub fn is_tty() -> bool { atty::is(atty::Stream::Stdin) && atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stderr) }

/// Helper trait and impl to convert [std::env::Args][Args] to a `Vec<String>`
/// after removing the first item (which is the path to the executable)..
pub trait ArgsToStrings {
  fn filter_and_convert_to_strings(&self) -> Vec<String>;
}

impl ArgsToStrings for Args {
  fn filter_and_convert_to_strings(&self) -> Vec<String> {
    with(args().collect::<Vec<String>>(), |mut list| {
      if !list.is_empty() {
        list.remove(0);
      }
      list
    })
  }
}
