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

use std::borrow::Cow;

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Convert to UnicodeString │
// ╯                          ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
impl From<&str> for UnicodeString {
  fn from(s: &str) -> Self { make_unicode_string_from(s) }
}

impl From<String> for UnicodeString {
  fn from(s: String) -> Self { make_unicode_string_from(&s) }
}

impl From<Cow<'_, str>> for UnicodeString {
  fn from(s: Cow<'_, str>) -> Self { make_unicode_string_from(&s) }
}

impl From<&String> for UnicodeString {
  fn from(s: &String) -> Self { make_unicode_string_from(s) }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ Convert to String │
// ╯                   ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
impl From<UnicodeString> for String {
  fn from(s: UnicodeString) -> Self { s.string }
}
