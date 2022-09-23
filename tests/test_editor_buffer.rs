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
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::InsertString("abc".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("ab".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("a".into()),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 2));

  // Remove the "a" on the last line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸          │
  //   └▴─────────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Left),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::Delete,
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 0, row: 2));

  // Move to the end of the 2nd line. Press delete.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 ▸ab        │
  //   └──▴───────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Up),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::Delete,
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_lines().len(), 2);
  assert_eq2!(this.get_caret(), position!(col: 2, row: 1));

  // Move to the end of the 1st line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸abcab     │
  //   └───▴──────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Up),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::Delete,
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_lines().len(), 1);
  assert_eq2!(this.get_caret(), position!(col: 3, row: 0));
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
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::InsertString("abc".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("ab".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("a".into()),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 2));

  // Remove the "a" on the last line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸          │
  //   └▴─────────┘
  //   C0123456789
  this.backspace();
  assert_eq2!(this.get_caret(), position!(col: 0, row: 2));

  // Remove the last line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 ▸ab        │
  //   └──▴───────┘
  //   C0123456789
  this.backspace();
  assert_eq2!(this.get_caret(), position!(col: 2, row: 1));

  // Move caret to start of 2nd line. Then press backspace.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸abcab     │
  //   └───▴──────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Left),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Left),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 0, row: 1));
  this.backspace();
  assert_eq2!(this.get_lines().len(), 1);
  assert_eq2!(this.get_caret(), position!(col: 3, row: 0));
  assert::line_at_caret(&this, "abcab");

  // Move caret to end of line. Insert "😃". Then move caret to end of line.
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸abcab😃   │
  //   └───────▴──┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("😃".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 7, row: 0));

  // Press backspace.
  EditorBuffer::apply_editor_event(
    &mut this,
    EditorEvent::new(
      EditorBufferCommand::Backspace,
      Position::default(),
      Size::default(),
    ),
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
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
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::InsertString("😀".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertChar('1'),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 1));

  // Move caret up. It should not be in the middle of the smiley face.
  // R ┌──────────┐
  // 0 ▸😀        │
  // 1 │1         │
  //   └──▴───────┘
  //   C0123456789
  this.move_caret(CaretDirection::Up);
  assert_eq2!(this.get_caret(), position!(col: 2, row: 0));
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
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::InsertChar('1'),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("😀".into()),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 2, row: 1));

  // Move caret up, and 2 left.
  // R ┌──────────┐
  // 0 ▸1         │
  // 1 │😀        │
  //   └─▴────────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Up),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 0));

  // Move caret down. It should not be in the middle of the smiley face.
  // R ┌──────────┐
  // 0 │1         │
  // 1 ▸😀        │
  //   └──▴───────┘
  //   C0123456789
  this.move_caret(CaretDirection::Down);
  assert_eq2!(this.get_caret(), position!(col: 2, row: 1));
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
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::InsertString("abc".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("ab".into()),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertString("a".into()),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 2));

  // Move caret down. Noop.
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Down),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Down),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Down),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 2));

  // Move caret up.
  this.move_caret(CaretDirection::Up);
  assert_eq2!(this.get_caret(), position!(col: 1, row: 1));

  // Move caret up.
  this.move_caret(CaretDirection::Up);
  assert_eq2!(this.get_caret(), position!(col: 1, row: 0));

  // Move caret up a few times. Noop.
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Up),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Up),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Up),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 0));

  // Move right to end of line. Then down.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 ▸ab        │
  // 2 │a         │
  //   └──▴───────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Down),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_caret(), position!(col: 2, row: 1));

  // Move caret down.
  // `this` should look like:
  // R ┌──────────┐
  // 0 │abc       │
  // 1 │ab        │
  // 2 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.move_caret(CaretDirection::Down);
  assert_eq2!(this.get_caret(), position!(col: 1, row: 2));
}

#[test]
fn test_insert_new_line() {
  // Starts w/ an empty line.
  let mut this = EditorBuffer::default();
  assert_eq2!(this.get_lines().len(), 1);

  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸          │
  //   └▴─────────┘
  //   C0123456789
  assert_eq2!(this.get_lines().len(), 1);
  assert::none_is_at_caret(&this);

  // Insert "a".
  // `this` should look like:
  // R ┌──────────┐
  // 0 ▸a         │
  //   └─▴────────┘
  //   C0123456789
  this.insert_char('a');
  assert::none_is_at_caret(&this);
  assert_eq2!(this.get_caret(), position!(col: 1, row: 0));

  // Insert new line (at end of line).
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸          │
  //   └▴─────────┘
  //   C0123456789
  this.insert_new_line();
  assert_eq2!(this.get_lines().len(), 2);
  assert::none_is_at_caret(&this);
  assert_eq2!(this.get_caret(), position!(col: 0, row: 1));

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
  assert_eq2!(this.get_lines().len(), 3);
  assert::str_is_at_caret(&this, "a");
  assert_eq2!(this.get_caret(), position!(col: 0, row: 2));

  // Move caret right, insert "b".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │          │
  // 2 ▸ab        │
  //   └──▴───────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertChar('b'),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );

  assert::none_is_at_caret(&this);
  assert_eq2!(
    line_buffer_content::line_at_caret_to_string(&this)
      .unwrap()
      .string,
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
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Left),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert::str_is_at_caret(&this, "b");
  assert_eq2!(this.get_caret(), position!(col: 0, row: 3));
  assert_eq2!(this.get_lines().len(), 4);

  // Move caret to end of prev line. Press enter. `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │          │
  // 2 │a         │
  // 3 ▸          │
  // 4 │b         │
  //   └▴─────────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Up),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::MoveCaret(CaretDirection::Right),
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(this.get_lines().len(), 5);
  assert_eq2!(this.get_caret(), position!(col: 0, row: 3));
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
    line_buffer_content::line_at_caret_to_string(&this)
      .unwrap()
      .string,
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
    line_buffer_content::line_at_caret_to_string(&this)
      .unwrap()
      .string,
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
  let editor_buffer = EditorBuffer::default();
  assert_eq2!(editor_buffer.get_lines().len(), 1);
  assert!(!editor_buffer.is_empty());
}

fn make_shared_tw_data() -> SharedTWData {
  use std::sync::Arc;

  use tokio::sync::RwLock;

  let shared_tw_data: SharedTWData = Arc::new(RwLock::new(TWData::default()));
  shared_tw_data
}

fn make_component_registry() -> ComponentRegistry<String, String> {
  let component_registry: ComponentRegistry<String, String> = ComponentRegistry::default();
  component_registry
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
  assert_eq2!(this.get_caret(), position!(col: 0, row: 0));
  this.insert_char('a');
  assert_eq2!(*this.get_lines(), vec![UnicodeString::from("a")]);
  assert_eq2!(this.get_caret(), position!(col: 1, row: 0));

  // Move caret to col: 0, row: 1. Insert "b".
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 ▸b░        │
  //   └─▴────────┘
  //   C0123456789
  line_buffer_content_mut::insert_new_line_at_caret(&mut this);
  this.insert_char('b');
  assert_eq2!(
    *this.get_lines(),
    vec![UnicodeString::from("a"), UnicodeString::from("b")]
  );
  assert_eq2!(this.get_caret(), position!(col: 1, row: 1));

  // Move caret to col: 0, row: 3. Insert "😀" (unicode width = 2).
  // `this` should look like:
  // R ┌──────────┐
  // 0 │a         │
  // 1 │b         │
  // 2 │          │
  // 3 ▸😀░       │
  //   └──▴───────┘
  //   C0123456789
  EditorBuffer::apply_editor_events(
    &mut this,
    vec![
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertNewLine,
        Position::default(),
        Size::default(),
      ),
      EditorEvent::new(
        EditorBufferCommand::InsertChar('😀'),
        Position::default(),
        Size::default(),
      ),
    ],
    &make_shared_tw_data(),
    &mut make_component_registry(),
    "",
  );
  assert_eq2!(
    *this.get_lines(),
    vec![
      UnicodeString::from("a"),
      UnicodeString::from("b"),
      UnicodeString::from(""),
      UnicodeString::from("😀")
    ]
  );
  assert_eq2!(this.get_caret(), position!(col: 2, row: 3));

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
  assert_eq2!(
    *this.get_lines(),
    vec![
      UnicodeString::from("a"),
      UnicodeString::from("b"),
      UnicodeString::from(""),
      UnicodeString::from("😀d")
    ]
  );
  assert_eq2!(this.get_caret(), position!(col: 3, row: 3));

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
  assert_eq2!(
    *this.get_lines(),
    vec![
      UnicodeString::from("a"),
      UnicodeString::from("b"),
      UnicodeString::from(""),
      UnicodeString::from("😀d🙏🏽")
    ]
  );
  assert_eq2!(this.get_caret(), position!(col: 7, row: 3));
}

pub mod assert {
  use super::*;

  pub fn none_is_at_caret(editor_buffer: &EditorBuffer) {
    assert_eq2!(line_buffer_content::string_at_caret(editor_buffer), None);
  }

  pub fn str_is_at_caret(editor_buffer: &EditorBuffer, expected: &str) {
    match line_buffer_content::string_at_caret(editor_buffer) {
      Some(UnicodeStringSegmentSliceResult {
        unicode_string_seg: s,
        ..
      }) => assert_eq2!(s.string, expected),
      None => panic!("Expected string at caret, but got None."),
    }
  }

  pub fn line_at_caret(editor_buffer: &EditorBuffer, expected: &str) {
    assert_eq2!(
      line_buffer_content::line_at_caret_to_string(editor_buffer)
        .unwrap()
        .string,
      expected
    );
  }
}
