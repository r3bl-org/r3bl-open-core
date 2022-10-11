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

#[cfg(test)]
mod tests {
  use r3bl_rs_utils_core::*;

  use crate::*;

  #[test]
  fn editor_delete() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "abc\nab\na".
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 │ab        │
    // 2 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::InsertString("abc".into()),
        EditorOp::InsertNewLine,
        EditorOp::InsertString("ab".into()),
        EditorOp::InsertNewLine,
        EditorOp::InsertString("a".into()),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 2)
    );

    // Remove the "a" on the last line.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 │ab        │
    // 2 ▸          │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Left), EditorOp::Delete],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 2)
    );

    // Move to the end of the 2nd line. Press delete.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 ▸ab        │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Up),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::Delete,
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(buffer.get_lines().len(), 2);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 2, row: 1)
    );

    // Move to the end of the 1st line.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸abcab     │
    //   └───▴──────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Up),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::Delete,
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(buffer.get_lines().len(), 1);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 3, row: 0)
    );
    assert::line_at_caret(&buffer, &engine, "abcab");
  }

  #[test]
  fn editor_backspace() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "abc\nab\na".
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 │ab        │
    // 2 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::InsertString("abc".into()),
        EditorOp::InsertNewLine,
        EditorOp::InsertString("ab".into()),
        EditorOp::InsertNewLine,
        EditorOp::InsertString("a".into()),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 2)
    );

    // Remove the "a" on the last line.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 │ab        │
    // 2 ▸          │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::Backspace],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 2)
    );

    // Remove the last line.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 ▸ab        │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::Backspace],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 2, row: 1)
    );

    // Move caret to start of 2nd line. Then press backspace.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸abcab     │
    //   └───▴──────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Left),
        EditorOp::MoveCaret(CaretDirection::Left),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 1)
    );
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::Backspace],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(buffer.get_lines().len(), 1);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 3, row: 0)
    );
    assert::line_at_caret(&buffer, &engine, "abcab");

    // Move caret to end of line. Insert "😃". Then move caret to end of line.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸abcab😃   │
    //   └───────▴──┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::InsertString("😃".into()),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 7, row: 0)
    );

    // Press backspace.
    EditorOp::apply_editor_event(
      &mut engine,
      &mut buffer,
      EditorOp::Backspace,
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::line_at_caret(&buffer, &engine, "abcab");
  }

  #[test]
  fn editor_validate_caret_position_on_up() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "😀\n1".
    // R ┌──────────┐
    // 0 │😀        │
    // 1 ▸1         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::InsertString("😀".into()),
        EditorOp::InsertNewLine,
        EditorOp::InsertChar('1'),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 1)
    );

    // Move caret up. It should not be in the middle of the smiley face.
    // R ┌──────────┐
    // 0 ▸😀        │
    // 1 │1         │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Up)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 2, row: 0)
    );
  }

  #[test]
  fn editor_validate_caret_position_on_down() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "😀\n1".
    // R ┌──────────┐
    // 0 ▸1         │
    // 1 │😀        │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::InsertChar('1'),
        EditorOp::InsertNewLine,
        EditorOp::InsertString("😀".into()),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 2, row: 1)
    );

    // Move caret up, and right. It should wrap around to the start of the next line and be to the
    // left of the smiley face.
    // R ┌──────────┐
    // 0 ▸1         │
    // 1 │😀        │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Up),
        EditorOp::MoveCaret(CaretDirection::Right),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 1)
    );

    // Move caret down. It should move to the end of the last line.
    // R ┌──────────┐
    // 0 │1         │
    // 1 ▸😀        │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Down)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 2, row: 1)
    );
  }

  #[test]
  fn editor_move_caret_up_down() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "abc\nab\na".
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 │ab        │
    // 2 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::InsertString("abc".into()),
        EditorOp::InsertNewLine,
        EditorOp::InsertString("ab".into()),
        EditorOp::InsertNewLine,
        EditorOp::InsertString("a".into()),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 2)
    );

    // Move caret down. Goes to end of line 2 and stops.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 │ab        │
    // 2 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Down),
        EditorOp::MoveCaret(CaretDirection::Down),
        EditorOp::MoveCaret(CaretDirection::Down),
        EditorOp::MoveCaret(CaretDirection::Down),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 2)
    );

    // Move caret up.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Up)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 1)
    );

    // Move caret up.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Up)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 0)
    );

    // Move caret up a few times. Caret moves to position 0.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Up),
        EditorOp::MoveCaret(CaretDirection::Up),
        EditorOp::MoveCaret(CaretDirection::Up),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 0)
    );

    // Move right to end of line. Then down.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 ▸ab        │
    // 2 │a         │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::MoveCaret(CaretDirection::Down),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 2, row: 1)
    );

    // Move caret down.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │abc       │
    // 1 │ab        │
    // 2 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Down)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 2)
    );
  }

  #[test]
  fn editor_insert_new_line() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Starts w/ an empty line.
    assert_eq2!(buffer.get_lines().len(), 1);

    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸          │
    //   └▴─────────┘
    //   C0123456789
    assert_eq2!(buffer.get_lines().len(), 1);
    assert::none_is_at_caret(&buffer, &engine);

    // Insert "a".
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('a')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::none_is_at_caret(&buffer, &engine);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 0)
    );

    // Insert new line (at end of line).
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 ▸          │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertNewLine],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(buffer.get_lines().len(), 2);
    assert::none_is_at_caret(&buffer, &engine);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 1)
    );

    // Insert "a".
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('a')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );

    // Move caret left.
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 ▸a         │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Left)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::str_is_at_caret(&buffer, &engine, "a");

    // Insert new line (at start of line).
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 │          │
    // 2 ▸a         │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertNewLine],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(buffer.get_lines().len(), 3);
    assert::str_is_at_caret(&buffer, &engine, "a");
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 2)
    );

    // Move caret right, insert "b".
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 │          │
    // 2 ▸ab        │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::InsertChar('b'),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );

    assert::none_is_at_caret(&buffer, &engine);
    assert_eq2!(
      get_content::line_at_caret_to_string(&buffer, &engine)
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
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Left),
        EditorOp::InsertNewLine,
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::str_is_at_caret(&buffer, &engine, "b");
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 3)
    );
    assert_eq2!(buffer.get_lines().len(), 4);

    // Move caret to end of prev line. Press enter. `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 │          │
    // 2 │a         │
    // 3 ▸          │
    // 4 │b         │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Up),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::InsertNewLine,
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(buffer.get_lines().len(), 5);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 3)
    );
  }

  #[test]
  fn editor_move_caret_left_right() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "a".
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸a         │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('a')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::none_is_at_caret(&buffer, &engine);

    // Move caret left.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸a         │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Left),
        EditorOp::MoveCaret(CaretDirection::Left), // No-op.
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::str_is_at_caret(&buffer, &engine, "a");

    // Insert "1".
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸1a        │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('1')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      get_content::line_at_caret_to_string(&buffer, &engine)
        .unwrap()
        .string,
      "1a"
    );
    assert::str_is_at_caret(&buffer, &engine, "a");

    // Move caret left.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸1a        │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Left)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::str_is_at_caret(&buffer, &engine, "1");

    // Move caret right.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸1a        │
    //   └─▴────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Right)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::str_is_at_caret(&buffer, &engine, "a");

    // Insert "2".
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸12a       │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('2')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::str_is_at_caret(&buffer, &engine, "a");
    assert_eq2!(
      get_content::line_at_caret_to_string(&buffer, &engine)
        .unwrap()
        .string,
      "12a"
    );

    // Move caret right. It should do nothing.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸12a       │
    //   └───▴──────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::MoveCaret(CaretDirection::Right), // No-op.
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert::none_is_at_caret(&buffer, &engine);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 3, row: 0)
    );

    // Move caret left.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸12a       │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Left),
        EditorOp::MoveCaret(CaretDirection::Left),
        EditorOp::MoveCaret(CaretDirection::Left),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 0)
    );

    // Move caret to end of line, press enter, then move caret left (should be at end of prev line).
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸12a       │
    // 1 │          │
    //   └───▴──────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::MoveCaret(CaretDirection::Right),
        EditorOp::InsertNewLine,
        EditorOp::MoveCaret(CaretDirection::Left),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 3, row: 0)
    );

    // Move caret right (should be at start of next line).
    // `this` should look like:
    // R ┌──────────┐
    // 0 │12a       │
    // 1 ▸          │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::MoveCaret(CaretDirection::Right)],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 1)
    );
  }

  #[test]
  fn editor_empty_state() {
    let buffer = EditorBuffer::default();
    assert_eq2!(buffer.get_lines().len(), 1);
    assert!(!buffer.is_empty());
  }

  #[test]
  fn editor_insertion() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Move caret to col: 0, row: 0. Insert "a".
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸a░        │
    //   └─▴────────┘
    //   C0123456789
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 0)
    );
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('a')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(*buffer.get_lines(), vec![UnicodeString::from("a")]);
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 0)
    );

    // Move caret to col: 0, row: 1. Insert "b".
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 ▸b░        │
    //   └─▴────────┘
    //   C0123456789
    mut_content::insert_new_line_at_caret(EditorArgsMut {
      buffer: &mut buffer,
      engine: &mut engine,
    });
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('b')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      *buffer.get_lines(),
      vec![UnicodeString::from("a"), UnicodeString::from("b")]
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 1, row: 1)
    );

    // Move caret to col: 0, row: 3. Insert "😀" (unicode width = 2).
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 │b         │
    // 2 │          │
    // 3 ▸😀░       │
    //   └──▴───────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![
        EditorOp::InsertNewLine,
        EditorOp::InsertNewLine,
        EditorOp::InsertChar('😀'),
      ],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      *buffer.get_lines(),
      vec![
        UnicodeString::from("a"),
        UnicodeString::from("b"),
        UnicodeString::from(""),
        UnicodeString::from("😀")
      ]
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 2, row: 3)
    );

    // Insert "d".
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 │b         │
    // 2 │          │
    // 3 ▸😀d░      │
    //   └───▴──────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertChar('d')],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      *buffer.get_lines(),
      vec![
        UnicodeString::from("a"),
        UnicodeString::from("b"),
        UnicodeString::from(""),
        UnicodeString::from("😀d")
      ]
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 3, row: 3)
    );

    // Insert "🙏🏽" (unicode width = 4).
    // `this` should look like:
    // R ┌──────────┐
    // 0 │a         │
    // 1 │b         │
    // 2 │          │
    // 3 ▸😀d🙏🏽  ░  │
    //   └───────▴──┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertString("🙏🏽".into())],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      *buffer.get_lines(),
      vec![
        UnicodeString::from("a"),
        UnicodeString::from("b"),
        UnicodeString::from(""),
        UnicodeString::from("😀d🙏🏽")
      ]
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 7, row: 3)
    );
  }

  #[test]
  fn editor_move_caret_home_end() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "hello". Then press home.
    // `this` should look like:
    // R ┌──────────┐
    // 0 ▸hello     │
    //   └▴─────────┘
    //   C0123456789
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::InsertString("hello".to_string()), EditorOp::Home],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 0)
    );

    // Press end.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::End],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 5, row: 0)
    );
  }

  #[test]
  fn editor_move_caret_page_up_page_down() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "hello" many times.
    let max_lines = 20;
    let mut count = max_lines;
    while count > 0 {
      EditorOp::apply_editor_events(
        &mut engine,
        &mut buffer,
        vec![
          EditorOp::InsertString(format!("{}: {}", count, "hello")),
          EditorOp::InsertNewLine,
        ],
        &make_shared_tw_data(),
        &mut make_component_registry(),
        "",
      );
      count -= 1;
    }
    assert_eq2!(buffer.len(), ch!(max_lines + 1)); /* One empty line after content */

    // Press page up.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::PageUp],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 10)
    );

    // Press page up.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::PageUp],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 0)
    );

    // Press page up.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::PageUp],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 0)
    );

    // Press page down.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::PageDown],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );

    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 10)
    );

    // Press page down.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::PageDown],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 20)
    );

    // Press page down.
    EditorOp::apply_editor_events(
      &mut engine,
      &mut buffer,
      vec![EditorOp::PageDown],
      &make_shared_tw_data(),
      &mut make_component_registry(),
      "",
    );
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 20)
    );
  }

  #[test]
  fn editor_scroll_vertical() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert "hello" many times.
    let max_lines = 20;
    for count in 1..=max_lines {
      EditorOp::apply_editor_events(
        &mut engine,
        &mut buffer,
        vec![
          EditorOp::InsertString(format!("{}: {}", count, "hello")),
          EditorOp::InsertNewLine,
        ],
        &make_shared_tw_data(),
        &mut make_component_registry(),
        "",
      );
    }
    assert_eq2!(buffer.len(), ch!(max_lines + 1)); /* One empty line after content */

    // Press up 12 times.
    for _ in 1..12 {
      EditorOp::apply_editor_events(
        &mut engine,
        &mut buffer,
        vec![EditorOp::MoveCaret(CaretDirection::Up)],
        &make_shared_tw_data(),
        &mut make_component_registry(),
        "",
      );
    }
    assert_eq2!(buffer.get_caret(CaretKind::Raw), position!(col: 0, row: 0));
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 9)
    );
    assert_eq2!(buffer.get_scroll_offset(), position!(col: 0, row: 9));

    // Press down 9 times.
    for _ in 1..9 {
      EditorOp::apply_editor_events(
        &mut engine,
        &mut buffer,
        vec![EditorOp::MoveCaret(CaretDirection::Down)],
        &make_shared_tw_data(),
        &mut make_component_registry(),
        "",
      );
    }
    assert_eq2!(buffer.get_caret(CaretKind::Raw), position!(col: 0, row: 8));
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 0, row: 17)
    );
    assert_eq2!(buffer.get_scroll_offset(), position!(col: 0, row: 9));
  }

  #[test]
  fn editor_scroll_horizontal() {
    let mut buffer = EditorBuffer::default();
    let mut engine = make_editor_engine();

    // Insert a long line of text.
    let max_cols = 15;
    for count in 1..=max_cols {
      EditorOp::apply_editor_events(
        &mut engine,
        &mut buffer,
        vec![EditorOp::InsertString(format!("{}", count))],
        &make_shared_tw_data(),
        &mut make_component_registry(),
        "",
      );
    }
    assert_eq2!(buffer.len(), ch!(1));
    assert_eq2!(buffer.get_caret(CaretKind::Raw), position!(col: 9, row: 0));
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 21, row: 0)
    );
    assert_eq2!(buffer.get_scroll_offset(), position!(col: 12, row: 0));

    // Press left 5 times.
    for _ in 1..5 {
      EditorOp::apply_editor_events(
        &mut engine,
        &mut buffer,
        vec![EditorOp::MoveCaret(CaretDirection::Left)],
        &make_shared_tw_data(),
        &mut make_component_registry(),
        "",
      );
    }
    assert_eq2!(buffer.get_caret(CaretKind::Raw), position!(col: 5, row: 0));
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 17, row: 0)
    );
    assert_eq2!(buffer.get_scroll_offset(), position!(col: 12, row: 0));

    // Press right 3 times.
    for _ in 1..3 {
      EditorOp::apply_editor_events(
        &mut engine,
        &mut buffer,
        vec![EditorOp::MoveCaret(CaretDirection::Right)],
        &make_shared_tw_data(),
        &mut make_component_registry(),
        "",
      );
    }
    assert_eq2!(buffer.get_caret(CaretKind::Raw), position!(col: 7, row: 0));
    assert_eq2!(
      buffer.get_caret(CaretKind::ScrollAdjusted),
      position!(col: 19, row: 0)
    );
    assert_eq2!(buffer.get_scroll_offset(), position!(col: 12, row: 0));
  }

  mod mock_real_objects {
    use super::*;

    pub fn make_shared_tw_data() -> SharedTWData {
      use std::sync::Arc;

      use tokio::sync::RwLock;

      let shared_tw_data: SharedTWData = Arc::new(RwLock::new(TWData::default()));
      shared_tw_data
    }

    pub fn make_component_registry() -> ComponentRegistry<String, String> {
      let component_registry: ComponentRegistry<String, String> = ComponentRegistry::default();
      component_registry
    }

    pub fn make_editor_engine() -> EditorEngine {
      EditorEngine {
        current_box: FlexBox {
          style_adjusted_bounds_size: size!( cols: 10, rows: 10 ),
          style_adjusted_origin_pos: position!( col: 0, row: 0 ),
          ..Default::default()
        },
        ..Default::default()
      }
    }
  }
  use mock_real_objects::*;

  mod assert {
    use super::*;

    pub fn none_is_at_caret(buffer: &EditorBuffer, engine: &EditorEngine) {
      assert_eq2!(get_content::string_at_caret(buffer, engine), None);
    }

    pub fn str_is_at_caret(editor_buffer: &EditorBuffer, engine: &EditorEngine, expected: &str) {
      match get_content::string_at_caret(editor_buffer, engine) {
        Some(UnicodeStringSegmentSliceResult {
          unicode_string_seg: s,
          ..
        }) => assert_eq2!(s.string, expected),
        None => panic!("Expected string at caret, but got None."),
      }
    }

    pub fn line_at_caret(editor_buffer: &EditorBuffer, engine: &EditorEngine, expected: &str) {
      assert_eq2!(
        get_content::line_at_caret_to_string(editor_buffer, engine)
          .unwrap()
          .string,
        expected
      );
    }
  }
}
