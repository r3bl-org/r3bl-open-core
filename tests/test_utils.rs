/*
*   Copyright (c) 2022 R3BL LLC
*   All rights reserved.

*   Licensed under the Apache License, Version 2.0 (the "License");
*   you may not use this file except in compliance with the License.
*   You may obtain a copy of the License at

*   http://www.apache.org/licenses/LICENSE-2.0

*   Unless required by applicable law or agreed to in writing, software
*   distributed under the License is distributed on an "AS IS" BASIS,
*   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*   See the License for the specific language governing permissions and
*   limitations under the License.
*/

//! Integration tests for the `utils`
/// Rust book: https://doc.rust-lang.org/book/ch11-03-test-organization.html#the-tests-directory
use ansi_term::Colour::Green;
use r3bl_rs_utils::utils::type_of;
use r3bl_rs_utils_core::style_primary;

#[test]
fn test_color_styles_work() {
  let text = "foo";
  let styled_text = style_primary(text);
  assert_eq!(Green.bold().paint(text), styled_text);
}

#[test]
fn test_type_of_works() {
  let text = "foo".to_string();
  let type_of_text = type_of(&text);
  assert_eq!(type_of_text, "alloc::string::String");
}
