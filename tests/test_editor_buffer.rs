/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE─2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use r3bl_rs_utils::*;

#[test]
fn test_buffer_insert_into_new_line() {
  let mut editor_buffer = EditorBuffer::default();

  // Move caret to col: 0, row: 0. Insert "a".
  // `editor_buffer` should look like:
  // R ┌──────────┐
  // 0 ▸a░        │
  //   └─▴────────┘
  //   C0123456789
  assert_eq2!(editor_buffer.caret, position!(col: 0, row: 0));
  editor_buffer.insert_char_into_current_line('a');
  assert_eq2!(editor_buffer.vec_lines, vec!["a"]);
  assert_eq2!(editor_buffer.caret, position!(col: 1, row: 0));

  // Move caret to col: 0, row: 1. Insert "b".
  // `editor_buffer` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸b░        │
  //   └─▴────────┘
  //   C0123456789
  editor_buffer.caret = position!(col: 0, row: 1);
  editor_buffer.insert_char_into_current_line('b');
  assert_eq2!(editor_buffer.vec_lines, vec!["a", "b"]);
  assert_eq2!(editor_buffer.caret, position!(col: 1, row: 1));

  // Move caret to col: 0, row: 3. Insert "😀" (unicode width = 2).
  // `editor_buffer` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │b         │
  // 2 │          │
  // 3 ▸😀░       │
  //   └──▴───────┘
  //   C0123456789
  editor_buffer.caret = position!(col: 0, row: 3);
  editor_buffer.insert_char_into_current_line('😀');
  assert_eq2!(editor_buffer.vec_lines, vec!["a", "b", "", "😀"]);
  assert_eq2!(editor_buffer.caret, position!(col: 2, row: 3));

  // Insert "d".
  // `editor_buffer` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │b         │
  // 2 │          │
  // 3 ▸😀d░      │
  //   └───▴──────┘
  //   C0123456789
  editor_buffer.insert_char_into_current_line('d');
  assert_eq2!(editor_buffer.vec_lines, vec!["a", "b", "", "😀d"]);
  assert_eq2!(editor_buffer.caret, position!(col: 3, row: 3));

  // Insert "🙏🏽" (unicode width = 4).
  // `editor_buffer` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │b         │
  // 2 │          │
  // 3 ▸😀d🙏🏽  ░  │
  //   └───────▴──┘
  //   C0123456789
  editor_buffer.insert_str_into_current_line("🙏🏽");
  assert_eq2!(editor_buffer.vec_lines, vec!["a", "b", "", "😀d🙏🏽"]);
  assert_eq2!(editor_buffer.caret, position!(col: 7, row: 3));
}
