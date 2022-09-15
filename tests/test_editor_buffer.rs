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
fn test_delete() {
  let mut this = EditorBuffer::default();

  // Insert "abc\nab\na".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::InsertString("abc".into()),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertString("ab".into()),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertString("a".into()),
  ]);
  assert_eq2!(this.caret, position!(col: 1, row: 2));

  // Remove the "a" on the last line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸          │
  //   └▴─────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Left),
    EditorBufferCommand::Delete,
  ]);
  assert_eq2!(this.caret, position!(col: 0, row: 2));

  // Move to the end of the 2nd line. Press delete.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 ▸ab        │
  //   └──▴───────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Up),
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
    EditorBufferCommand::Delete,
  ]);
  assert_eq2!(this.vec_lines.len(), 2);
  assert_eq2!(this.caret, position!(col: 2, row: 1));

  // Move to the end of the 1st line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸abcab     │
  //   └───▴──────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Up),
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
    EditorBufferCommand::Delete,
  ]);
  assert_eq2!(this.vec_lines.len(), 1);
  assert_eq2!(this.caret, position!(col: 3, row: 0));
  assert::line_at_caret(&this, "abcab");
}

#[test]
fn test_backspace() {
  let mut this = EditorBuffer::default();

  // Insert "abc\nab\na".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::InsertString("abc".into()),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertString("ab".into()),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertString("a".into()),
  ]);
  assert_eq2!(this.caret, position!(col: 1, row: 2));

  // Remove the "a" on the last line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸          │
  //   └▴─────────┘
  //   C0123456789
  this.backspace();
  assert_eq2!(this.caret, position!(col: 0, row: 2));

  // Remove the last line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 ▸ab        │
  //   └──▴───────┘
  //   C0123456789
  this.backspace();
  assert_eq2!(this.caret, position!(col: 2, row: 1));

  // Move caret to start of 2nd line. Then press backspace.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸abcab     │
  //   └───▴──────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Left),
    EditorBufferCommand::MoveCaret(CaretDirection::Left),
  ]);
  assert_eq2!(this.caret, position!(col: 0, row: 1));
  this.backspace();
  assert_eq2!(this.vec_lines.len(), 1);
  assert_eq2!(this.caret, position!(col: 3, row: 0));
  assert::line_at_caret(&this, "abcab");
}

#[test]
fn test_validate_caret_position_on_up() {
  let mut this = EditorBuffer::default();

  // Insert "😀\n1".
  // R ┌──────────┐
  // 0 │😀        │
  // 1 ▸1         │
  //   └─▴────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::InsertString("😀".into()),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertChar('1'),
  ]);
  assert_eq2!(this.caret, position!(col: 1, row: 1));

  // Move caret up. It should not be in the middle of the smiley face.
  // R ┌──────────┐
  // 0 ▸😀        │
  // 1 │1         │
  //   └──▴───────┘
  //   C0123456789
  this.move_caret(CaretDirection::Up);
  assert_eq2!(this.caret, position!(col: 2, row: 0));
}

#[test]
fn test_validate_caret_position_on_down() {
  let mut this = EditorBuffer::default();

  // Insert "😀\n1".
  // R ┌──────────┐
  // 0 ▸1         │
  // 1 │😀        │
  //   └──▴───────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::InsertChar('1'),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertString("😀".into()),
  ]);
  assert_eq2!(this.caret, position!(col: 2, row: 1));

  // Move caret up, and 2 left.
  // R ┌──────────┐
  // 0 ▸1         │
  // 1 │😀        │
  //   └─▴────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Up),
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
  ]);
  assert_eq2!(this.caret, position!(col: 1, row: 0));

  // Move caret down. It should not be in the middle of the smiley face.
  // R ┌──────────┐
  // 0 │1         │
  // 1 ▸😀        │
  //   └──▴───────┘
  //   C0123456789
  this.move_caret(CaretDirection::Down);
  assert_eq2!(this.caret, position!(col: 2, row: 1));
}

#[test]
fn test_move_caret_up_down() {
  let mut this = EditorBuffer::default();

  // Insert "abc\nab\na".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::InsertString("abc".into()),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertString("ab".into()),
    EditorBufferCommand::InsertNewLine,
    EditorBufferCommand::InsertString("a".into()),
  ]);
  assert_eq2!(this.caret, position!(col: 1, row: 2));

  // Move caret down. Noop.
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Down),
    EditorBufferCommand::MoveCaret(CaretDirection::Down),
    EditorBufferCommand::MoveCaret(CaretDirection::Down),
  ]);
  assert_eq2!(this.caret, position!(col: 1, row: 2));

  // Move caret up.
  this.move_caret(CaretDirection::Up);
  assert_eq2!(this.caret, position!(col: 1, row: 1));

  // Move caret up.
  this.move_caret(CaretDirection::Up);
  assert_eq2!(this.caret, position!(col: 1, row: 0));

  // Move caret up a few times. Noop.
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Up),
    EditorBufferCommand::MoveCaret(CaretDirection::Up),
    EditorBufferCommand::MoveCaret(CaretDirection::Up),
  ]);
  assert_eq2!(this.caret, position!(col: 1, row: 0));

  // Move right to end of line. Then down.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 ▸ab        │
  // 2 │a         │
  //   └──▴───────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
    EditorBufferCommand::MoveCaret(CaretDirection::Down),
  ]);
  assert_eq2!(this.caret, position!(col: 2, row: 1));

  // Move caret down.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.move_caret(CaretDirection::Down);
  assert_eq2!(this.caret, position!(col: 1, row: 2));
}

#[test]
fn test_insert_new_line() {
  let mut this = EditorBuffer::default();

  // Insert new line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸          │
  //   └▴─────────┘
  //   C0123456789
  this.insert_new_line();
  assert_eq2!(this.vec_lines.len(), 1);
  assert::none_is_at_caret(&this);

  // Insert "a".
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.insert_char('a');
  assert::none_is_at_caret(&this);
  assert_eq2!(this.caret, position!(col: 1, row: 0));

  // Insert new line (at end of line).
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸          │
  //   └▴─────────┘
  //   C0123456789
  this.insert_new_line();
  assert_eq2!(this.vec_lines.len(), 2);
  assert::none_is_at_caret(&this);
  assert_eq2!(this.caret, position!(col: 0, row: 1));

  // Insert "a".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.insert_char('a');

  // Move caret left.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸a         │
  //   └▴─────────┘
  //   C0123456789
  this.move_caret(CaretDirection::Left);
  assert::str_is_at_caret(&this, "a");

  // Insert new line (at start of line).
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │          │
  // 2 ▸a         │
  //   └▴─────────┘
  //   C0123456789
  this.insert_new_line();
  assert_eq2!(this.vec_lines.len(), 3);
  assert::str_is_at_caret(&this, "a");
  assert_eq2!(this.caret, position!(col: 0, row: 2));

  // Move caret right, insert "b".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │          │
  // 2 ▸ab        │
  //   └──▴───────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
    EditorBufferCommand::InsertChar('b'),
  ]);

  assert::none_is_at_caret(&this);
  assert_eq2!(
    line_buffer_get_content::line_as_string(&this).unwrap(),
    "ab"
  );

  // Move caret left, insert new line (at middle of line).
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │          │
  // 2 │a         │
  // 3 ▸b         │
  //   └▴─────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Left),
    EditorBufferCommand::InsertNewLine,
  ]);
  assert::str_is_at_caret(&this, "b");
  assert_eq2!(this.caret, position!(col: 0, row: 3));
  assert_eq2!(this.vec_lines.len(), 4);

  // Move caret to end of prev line. Press enter. `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │          │
  // 2 │a         │
  // 3 ▸          │
  // 4 │b         │
  //   └▴─────────┘
  //   C0123456789
  this.apply_commands(vec![
    EditorBufferCommand::MoveCaret(CaretDirection::Up),
    EditorBufferCommand::MoveCaret(CaretDirection::Right),
    EditorBufferCommand::InsertNewLine,
  ]);
  assert_eq2!(this.vec_lines.len(), 5);
  assert_eq2!(this.caret, position!(col: 0, row: 3));
}

#[test]
fn test_move_caret_left_right() {
  let mut this = EditorBuffer::default();

  // Insert "a".
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.insert_char('a');
  assert::none_is_at_caret(&this);

  // Move caret left.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸a         │
  //   └▴─────────┘
  //   C0123456789
  this.move_caret(CaretDirection::Left);
  this.move_caret(CaretDirection::Left); // Noop.
  assert::str_is_at_caret(&this, "a");

  // Insert "1".
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸1a        │
  //   └─▴────────┘
  //   C0123456789
  this.insert_char('1');
  assert_eq2!(
    line_buffer_get_content::line_as_string(&this).unwrap(),
    "1a"
  );
  assert::str_is_at_caret(&this, "a");

  // Move caret left.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸1a        │
  //   └▴─────────┘
  //   C0123456789
  this.move_caret(CaretDirection::Left);
  assert::str_is_at_caret(&this, "1");

  // Move caret right.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸1a        │
  //   └─▴────────┘
  //   C0123456789
  this.move_caret(CaretDirection::Right);
  assert::str_is_at_caret(&this, "a");

  // Insert "2".
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸12a       │
  //   └──▴───────┘
  //   C0123456789
  this.insert_char('2');
  assert::str_is_at_caret(&this, "a");
  assert_eq2!(
    line_buffer_get_content::line_as_string(&this).unwrap(),
    "12a"
  );

  // Move caret right.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸12a       │
  //   └───▴──────┘
  //   C0123456789
  this.move_caret(CaretDirection::Right);
  this.move_caret(CaretDirection::Right); // Noop.
  assert::none_is_at_caret(&this);
}

#[test]
fn test_empty_state() {
  let mut editor_buffer = EditorBuffer::default();
  assert!(editor_buffer.is_empty());
  editor_buffer.insert_char('a');
  assert!(!editor_buffer.is_empty());
}

#[test]
fn test_insertion() {
  let mut this = EditorBuffer::default();

  // Move caret to col: 0, row: 0. Insert "a".
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸a░        │
  //   └─▴────────┘
  //   C0123456789
  assert_eq2!(this.caret, position!(col: 0, row: 0));
  this.insert_char('a');
  assert_eq2!(this.vec_lines, vec!["a"]);
  assert_eq2!(this.caret, position!(col: 1, row: 0));

  // Move caret to col: 0, row: 1. Insert "b".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸b░        │
  //   └─▴────────┘
  //   C0123456789
  this.caret = position!(col: 0, row: 1);
  this.insert_char('b');
  assert_eq2!(this.vec_lines, vec!["a", "b"]);
  assert_eq2!(this.caret, position!(col: 1, row: 1));

  // Move caret to col: 0, row: 3. Insert "😀" (unicode width = 2).
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │b         │
  // 2 │          │
  // 3 ▸😀░       │
  //   └──▴───────┘
  //   C0123456789
  this.caret = position!(col: 0, row: 3);
  this.insert_char('😀');
  assert_eq2!(this.vec_lines, vec!["a", "b", "", "😀"]);
  assert_eq2!(this.caret, position!(col: 2, row: 3));

  // Insert "d".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │b         │
  // 2 │          │
  // 3 ▸😀d░      │
  //   └───▴──────┘
  //   C0123456789
  this.insert_char('d');
  assert_eq2!(this.vec_lines, vec!["a", "b", "", "😀d"]);
  assert_eq2!(this.caret, position!(col: 3, row: 3));

  // Insert "🙏🏽" (unicode width = 4).
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │b         │
  // 2 │          │
  // 3 ▸😀d🙏🏽  ░  │
  //   └───────▴──┘
  //   C0123456789
  this.insert_str("🙏🏽");
  assert_eq2!(this.vec_lines, vec!["a", "b", "", "😀d🙏🏽"]);
  assert_eq2!(this.caret, position!(col: 7, row: 3));
}

pub mod assert {
  use super::*;

  pub fn none_is_at_caret(editor_buffer: &EditorBuffer) {
    assert_eq2!(
      line_buffer_get_content::string_at_caret(editor_buffer),
      None
    );
  }

  pub fn str_is_at_caret(editor_buffer: &EditorBuffer, expected: &str) {
    match line_buffer_get_content::string_at_caret(editor_buffer) {
      Some((s, _)) => assert_eq2!(s, expected),
      None => panic!("Expected string at caret, but got None."),
    }
  }

  pub fn line_at_caret(editor_buffer: &EditorBuffer, expected: &str) {
    assert_eq2!(
      line_buffer_get_content::line_as_string(editor_buffer).unwrap(),
      expected
    );
  }
}
