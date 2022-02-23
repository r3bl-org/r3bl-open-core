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

pub mod tty {
  use std::io::stdin;

  use crate::utils::style_error;

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
        println!(
          "{}",
          style_error("Something went wrong when reading input from terminal.")
        );
        (0, "".to_string())
      }
    }
  }
}
