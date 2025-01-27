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
mod test_config_options {
    use r3bl_core::{assert_eq2, position, UnicodeStringExt};

    use crate::{system_clipboard_service_provider::test_fixtures::TestClipboard,
                test_fixtures::mock_real_objects_for_editor,
                CaretDirection,
                CaretKind,
                EditorBuffer,
                EditorEngine,
                EditorEngineConfig,
                EditorEngineInternalApi,
                EditorEvent,
                LineMode,
                DEFAULT_SYN_HI_FILE_EXT};

    #[test]
    fn test_multiline_true() {
        // multiline true.
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::MultiLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );

        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 1)
        );
    }

    #[test]
    fn test_multiline_false() {
        // multiline false.
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine: EditorEngine = EditorEngine {
            config_options: EditorEngineConfig {
                multiline_mode: LineMode::SingleLine,
                ..Default::default()
            },
            ..mock_real_objects_for_editor::make_editor_engine()
        };

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸abcaba    │
        //   └──────▴───┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 6, row_index: 0)
        );

        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 6, row_index: 0)
        );
        let maybe_line_str =
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine);
        assert_eq2!(maybe_line_str.unwrap(), &"abcaba".unicode_string());
    }
}

#[cfg(test)]
mod test_editor_ops {
    use r3bl_core::{assert_eq2, ch, position, size, UnicodeString, UnicodeStringExt};
    use smallvec::smallvec;

    use crate::{editor::sizing::VecEditorContentLines,
                system_clipboard_service_provider::test_fixtures::TestClipboard,
                test_fixtures::{assert, mock_real_objects_for_editor},
                CaretDirection,
                CaretKind,
                EditorArgsMut,
                EditorBuffer,
                EditorEngineInternalApi,
                EditorEvent,
                DEFAULT_SYN_HI_FILE_EXT};

    #[test]
    fn editor_delete() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );

        // Remove the "a" on the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸          │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::Delete,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 2)
        );

        // Move to the end of the 2nd line. Press delete.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ▸ab        │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::Delete,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 2);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 1)
        );

        // Move to the end of the 1st line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸abcab     │
        //   └───▴──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::Delete,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 1);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 3, row_index: 0)
        );
        assert::line_at_caret(&buffer, &engine, "abcab");
    }

    #[test]
    fn editor_backspace() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );

        // Remove the "a" on the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸          │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 2)
        );

        // Remove the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ▸ab        │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 1)
        );

        // Move caret to start of 2nd line. Then press backspace.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸abcab     │
        //   └───▴──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 1)
        );
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 1);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 3, row_index: 0)
        );
        assert::line_at_caret(&buffer, &engine, "abcab");

        // Move caret to end of line. Insert "😃". Then move caret to end of line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸abcab😃   │
        //   └───────▴──┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertString("😃".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 7, row_index: 0)
        );

        // Press backspace.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert::line_at_caret(&buffer, &engine, "abcab");
    }

    #[test]
    fn editor_validate_caret_position_on_up() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 │😀        │
        // 1 ▸1         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("😀".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertChar('1'),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 1)
        );

        // Move caret up. It should not be in the middle of the smiley face.
        // R ┌──────────┐
        // 0 ▸😀        │
        // 1 │1         │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 0)
        );
    }

    #[test]
    fn editor_validate_caret_position_on_down() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 ▸1         │
        // 1 │😀        │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertChar('1'),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("😀".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 1)
        );

        // Move caret up, and right. It should wrap around to the start of the next line and be to the
        // left of the smiley face.
        // R ┌──────────┐
        // 0 ▸1         │
        // 1 │😀        │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 1)
        );

        // Move caret down. It should move to the end of the last line.
        // R ┌──────────┐
        // 0 │1         │
        // 1 ▸😀        │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 1)
        );
    }

    #[test]
    fn editor_move_caret_up_down() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );

        // Move caret down. Goes to end of line 2 and stops.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );

        // Move caret up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 1)
        );

        // Move caret up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 0)
        );

        // Move caret up a few times. Caret moves to position 0.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Move right to end of line. Then down.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ▸ab        │
        // 2 │a         │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 1)
        );

        // Move caret down.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );
    }

    #[test]
    fn editor_insert_new_line() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

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
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer, &engine);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 0)
        );

        // Insert new line (at end of line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ▸          │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 2);
        assert::none_is_at_caret(&buffer, &engine);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 1)
        );

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ▸a         │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mut TestClipboard::default(),
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
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 3);
        assert::str_is_at_caret(&buffer, &engine, "a");
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 2)
        );

        // Move caret right, insert "b".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 ▸ab        │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertChar('b'),
            ],
            &mut TestClipboard::default(),
        );

        assert::none_is_at_caret(&buffer, &engine);
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine).unwrap(),
            &"ab".unicode_string()
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
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::InsertNewLine,
            ],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, &engine, "b");
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 3)
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
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertNewLine,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 5);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 3)
        );
    }

    #[test]
    fn editor_move_caret_left_right() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer, &engine);

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸a         │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left), // No-op.
            ],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, &engine, "a");

        // Insert "1".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸1a        │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('1')],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine).unwrap(),
            &"1a".unicode_string()
        );
        assert::str_is_at_caret(&buffer, &engine, "a");

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸1a        │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, &engine, "1");

        // Move caret right.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸1a        │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, &engine, "a");

        // Insert "2".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸12a       │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('2')],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, &engine, "a");
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine).unwrap(),
            &"12a".unicode_string()
        );

        // Move caret right. It should do nothing.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸12a       │
        //   └───▴──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right), // No-op.
            ],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer, &engine);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 3, row_index: 0)
        );

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸12a       │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Move caret to end of line, press enter, then move caret left (should be at end of prev line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸12a       │
        // 1 │          │
        //   └───▴──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertNewLine,
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 3, row_index: 0)
        );

        // Move caret right (should be at start of next line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │12a       │
        // 1 ▸          │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 1)
        );

        // Press enter. Press up. Press right (should be at start of next line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │12a       │
        // 1 │          │
        // 2 ▸          │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertNewLine,
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 2)
        );
    }

    #[test]
    fn editor_empty_state() {
        let buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        assert_eq2!(buffer.get_lines().len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn editor_insertion() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Move caret to col: FlexBoxId::from(0), row: 0. Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸a░        │
        //   └─▴────────┘
        //   C0123456789
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines = smallvec!["a".unicode_string()];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 0)
        );

        // Move caret to col: FlexBoxId::from(0), row: 1. Insert "b".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ▸b░        │
        //   └─▴────────┘
        //   C0123456789
        EditorEngineInternalApi::insert_new_line_at_caret(EditorArgsMut {
            editor_buffer: &mut buffer,
            editor_engine: &mut engine,
        });
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('b')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines =
            smallvec!["a".unicode_string(), "b".unicode_string()];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 1)
        );

        // Move caret to col: FlexBoxId::from(0), row: 3. Insert "😀" (unicode width = 2).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ▸😀░       │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertNewLine,
                EditorEvent::InsertNewLine,
                EditorEvent::InsertChar('😀'),
            ],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines = smallvec![
            "a".unicode_string(),
            "b".unicode_string(),
            "".unicode_string(),
            "😀".unicode_string()
        ];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 3)
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
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('d')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines = smallvec![
            "a".unicode_string(),
            "b".unicode_string(),
            "".unicode_string(),
            "😀d".unicode_string()
        ];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 3, row_index: 3)
        );

        // Insert "🙏🏽" (unicode width = 2).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ▸😀d🙏🏽░    │
        //   └─────▴────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("🙏🏽".into())],
            &mut TestClipboard::default(),
        );
        assert_eq2!(ch(2), UnicodeString::str_display_width("🙏🏽"));
        let expected: VecEditorContentLines = smallvec![
            "a".unicode_string(),
            "b".unicode_string(),
            "".unicode_string(),
            "😀d🙏🏽".unicode_string()
        ];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 5, row_index: 3)
        );
    }

    #[test]
    fn editor_move_caret_home_end() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello". Then press home.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸hello     │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("hello".to_string()),
                EditorEvent::Home,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Press end.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::End],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 5, row_index: 0)
        );
    }

    #[test]
    fn editor_move_caret_page_up_page_down() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello" many times.
        let max_lines = 20;
        let mut count = max_lines;
        while count > 0 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![
                    EditorEvent::InsertString(format!("{count}: {}", "hello")),
                    EditorEvent::InsertNewLine,
                ],
                &mut TestClipboard::default(),
            );
            count -= 1;
        }
        assert_eq2!(buffer.len(), ch(max_lines + 1)); /* One empty line after content */

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 10)
        );

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );

        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 10)
        );

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 20)
        );

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 20)
        );
    }

    #[test]
    fn editor_scroll_vertical() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello" many times.
        let max_lines = 20;
        for count in 1..=max_lines {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![
                    EditorEvent::InsertString(format!("{count}: {}", "hello")),
                    EditorEvent::InsertNewLine,
                ],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.len(), ch(max_lines + 1)); /* One empty line after content */

        // Press up 12 times.
        for _ in 1..12 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Up)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(
            buffer.get_caret(CaretKind::Raw),
            position!(col_index: 0, row_index: 0)
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 9)
        );
        assert_eq2!(
            buffer.get_scroll_offset(),
            position!(col_index: 0, row_index: 9)
        );

        // Press down 9 times.
        for _ in 1..9 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Down)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(
            buffer.get_caret(CaretKind::Raw),
            position!(col_index: 0, row_index: 8)
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 17)
        );
        assert_eq2!(
            buffer.get_scroll_offset(),
            position!(col_index: 0, row_index: 9)
        );
    }

    #[test]
    fn editor_scroll_horizontal() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert a long line of text.
        let max_cols = 15;
        for count in 1..=max_cols {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::InsertString(format!("{count}"))],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.len(), ch(1));
        assert_eq2!(
            buffer.get_caret(CaretKind::Raw),
            position!(col_index: 9, row_index: 0)
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 21, row_index: 0)
        );
        assert_eq2!(
            buffer.get_scroll_offset(),
            position!(col_index: 12, row_index: 0)
        );

        // Press left 5 times.
        for _ in 1..5 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Left)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(
            buffer.get_caret(CaretKind::Raw),
            position!(col_index: 5, row_index: 0)
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 17, row_index: 0)
        );
        assert_eq2!(
            buffer.get_scroll_offset(),
            position!(col_index: 12, row_index: 0)
        );

        // Press right 3 times.
        for _ in 1..3 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(
            buffer.get_caret(CaretKind::Raw),
            position!(col_index: 7, row_index: 0)
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 19, row_index: 0)
        );
        assert_eq2!(
            buffer.get_scroll_offset(),
            position!(col_index: 12, row_index: 0)
        );
    }

    /// A jumbo emoji is a combination of 2 emoji (each one of which has > 1 display width, or
    /// unicode width).
    /// 🙏🏽 = U+1F64F + U+1F3FD
    /// 1. https://unicodeplus.com/U+1F64F
    /// 2. https://unicodeplus.com/U+1F3FD
    #[test]
    fn editor_scroll_right_horizontal_long_line_with_jumbo_emoji() {
        // Setup.
        let viewport_width = ch(65);
        let viewport_height = ch(2);
        let window_size = size!(col_count: viewport_width, row_count: viewport_height);
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine =
            mock_real_objects_for_editor::make_editor_engine_with_bounds(window_size);

        let long_line = "# Did he take those two new droids with him? They hit accelerator.🙏🏽😀░ We will deal with your Rebel friends. Commence primary ignition.🙏🏽😀░";
        let long_line_us = long_line.unicode_string();
        buffer.set_lines([long_line]);

        // Setup assertions.
        {
            assert_eq2!(ch(2), UnicodeString::str_display_width("🙏🏽"));
            assert_eq2!(buffer.len(), ch(1));
            assert_eq2!(buffer.get_lines()[0], long_line.unicode_string());
            let us = &buffer.get_lines()[0];
            assert_eq2!(us, &long_line_us);
            assert_eq2!(
                buffer.get_caret(CaretKind::Raw),
                position!(col_index: 0, row_index: 0)
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 0, row_index: 0)
            );
            assert_eq2!(
                buffer.get_scroll_offset(),
                position!(col_index: 0, row_index: 0)
            );
        }

        // Press right 67 times. The caret should correctly jump the width of the jumbo emoji (🙏🏽)
        // on the **RIGHT** of viewport and select it.
        {
            let num_of_right = 67;
            for _ in 1..num_of_right {
                EditorEvent::apply_editor_events::<(), ()>(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &mut TestClipboard::default(),
                );
            }
            assert_eq2!(
                buffer.get_scroll_offset(),
                position!(col_index: 4, row_index: 0)
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 66, row_index: 0)
            );
            // Right of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_caret(CaretKind::ScrollAdjusted).col_index;
            let result = line.get_string_at_display_col_index(display_col_index);
            assert_eq2!(result.unwrap().unicode_string.string, "🙏🏽");

            // Press right 1 more time. The caret should correctly jump the width of "😀" from 68 to
            // 70.
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 68, row_index: 0)
            );
            // Right of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_caret(CaretKind::ScrollAdjusted).col_index;
            let result = line.get_string_at_display_col_index(display_col_index);
            assert_eq2!(result.unwrap().unicode_string.string, "😀");
        }

        // Press right 60 more times. The **LEFT** side of the viewport should be at the jumbo
        // emoji.
        {
            for _ in 1..60 {
                EditorEvent::apply_editor_events::<(), ()>(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &mut TestClipboard::default(),
                );
            }
            assert_eq2!(
                buffer.get_caret(CaretKind::Raw),
                position!(col_index: 64, row_index: 0)
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 128, row_index: 0)
            );
            assert_eq2!(
                buffer.get_scroll_offset(),
                position!(col_index: 64, row_index: 0)
            );
            // Start of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_scroll_offset().col_index;
            let result = line.get_string_at_display_col_index(display_col_index);
            assert_eq2!(result.unwrap().unicode_string.string, "r");
        }

        // Press right 1 more time. It should jump the jumbo emoji at the start of the line (and not
        // just 1 character width). This moves the caret and the scroll offset to make sure that the
        // emoji at the start of the line can be displayed properly.
        {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::Raw),
                position!(col_index: 64, row_index: 0)
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 129, row_index: 0)
            );
            assert_eq2!(
                buffer.get_scroll_offset(),
                position!(col_index: 65, row_index: 0)
            );
            // Start of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_scroll_offset().col_index;
            let result = line.get_string_at_display_col_index(display_col_index);
            assert_eq2!(result.unwrap().unicode_string.string, ".");
        }

        // Press right 4 times. It should jump the emoji at the start of the line (and not
        // just 1 character width); this moves the scroll offset to make sure that the emoji can be
        // properly displayed & it moves the caret too.
        {
            for _ in 1..4 {
                EditorEvent::apply_editor_events::<(), ()>(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &mut TestClipboard::default(),
                );
            }
            // Start of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_scroll_offset().col_index;
            let result = line.get_string_at_display_col_index(display_col_index);
            assert_eq2!(result.unwrap().unicode_string.string, "😀");
        }

        // Press right 1 more time. It should jump the emoji.
        {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
            // Start of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_scroll_offset().col_index;
            let result = line.get_string_at_display_col_index(display_col_index);
            assert_eq2!(result.unwrap().unicode_string.string, "░");
        }
    }
}

#[cfg(test)]
mod selection_tests {
    use r3bl_core::{assert_eq2, ch, SelectionRange, VecArray};
    use smallvec::smallvec;

    type SelectionList = VecArray<(RowIndex, SelectionRange)>;

    use crate::{system_clipboard_service_provider::test_fixtures::TestClipboard,
                test_fixtures::mock_real_objects_for_editor,
                CaretDirection,
                EditorBuffer,
                EditorEvent,
                RowIndex,
                SelectionAction,
                DEFAULT_SYN_HI_FILE_EXT};

    #[test]
    fn test_text_selection() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.set_lines(["abc r3bl xyz", "pqr rust uvw"]);

        {
            // Current Caret Position : [row : 0, col : 0]
            // Selecting up to the end of the first line.

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::End)],
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 0, col : 12]

            // Selection Map : {{0, SelectionRange {start: 0, end: 12}}}
            let selection_list: SelectionList = smallvec! {
                (ch(0), SelectionRange::new(ch(0), ch(12)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 0, col : 12]
            // Reverse selection up to the start of the line.

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 5], // Move caret to right for 5 times
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 4]

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::Home)], // Select text up to starting
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 0]

            // Selection Map : {{1, SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (ch(1), SelectionRange::new(ch(0), ch(4)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 0]
            // De-Select one character to right

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneCharRight)], // Move Selection to Right
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 1]

            // Selection Map : {{1, SelectionRange {start: 1, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (ch(1), SelectionRange::new(ch(1), ch(4)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 1]
            // Select one character to left

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneCharLeft)], // Move Selection to Left
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 0]

            // Selection Map : {{1, SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (ch(1), SelectionRange::new(ch(0), ch(4)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 0]
            // Move Selection Caret to one line upwards

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneLineUp)], // Select one line up
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 0, col : 0]

            // Selection Map : {{0, SelectionRange {start: 0, end: 12}}, {1, SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (ch(0), SelectionRange::new(ch(0), ch(12))),
                (ch(1), SelectionRange::new(ch(0), ch(4)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 0, col : 0]
            // Move Selection Caret to one line downwards

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneLineDown)], // De-Select one line down
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 0]

            // Selection Map : {{1, SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (ch(1), SelectionRange::new(ch(0), ch(4)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 0]
            // Move Caret to one char right and drop down selection
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)], // Move caret to right
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 1]

            // Selection Map : {}
            let selection_list: SelectionList = smallvec![];
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 1]
            // Select by pressing PageUp
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageUp)], // Select by pressing PageUp
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 0, col : 1]

            // Selection Map : {{0, SelectionRange {start: 1, end: 12}}, {1, SelectionRange {start: 0, end: 1}}}
            let selection_list: SelectionList = smallvec! {
                (ch(0), SelectionRange::new(ch(1), ch(12))),
                (ch(1), SelectionRange::new(ch(0), ch(1)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 0, col : 1]
            // Select by pressing PageDown
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)], // Move caret one char right
                &mut TestClipboard::default(),
            );
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageDown)], // Select by pressing PageDown
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 2]

            // Selection Map : {{0, SelectionRange {start: 2, end: 12}},{1, SelectionRange {start: 0, end: 2}}}
            let selection_list: SelectionList = smallvec! {
                (ch(0), SelectionRange::new(ch(2), ch(12))),
                (ch(1), SelectionRange::new(ch(0), ch(2)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 2]
            // Select by pressing All
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::All)], // Select by pressing All
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 2]

            // Selection Map : {{0, SelectionRange {start: 0, end: 12}},{1, SelectionRange {start: 0, end: 2}}}
            let selection_list: SelectionList = smallvec! {
                (ch(0), SelectionRange::new(ch(0), ch(12))),
                (ch(1), SelectionRange::new(ch(0), ch(12)))
            };
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 2]
            // Select by pressing Esc
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::Esc)], // Select by pressing Esc
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 2]

            // Selection Map : {}
            let selection_list: SelectionList = smallvec![];
            assert_eq2!(
                buffer.get_selection_map().get_ordered_list(),
                &selection_list
            );
        }
    }
}

#[cfg(test)]
mod clipboard_tests {
    use r3bl_core::{assert_eq2, UnicodeStringExt as _};
    use smallvec::smallvec;

    use crate::{editor::sizing::VecEditorContentLines,
                system_clipboard_service_provider::test_fixtures::TestClipboard,
                test_fixtures::mock_real_objects_for_editor,
                CaretDirection,
                EditorBuffer,
                EditorEvent,
                SelectionAction,
                DEFAULT_SYN_HI_FILE_EXT};

    #[test]
    fn test_copy() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.set_lines(["abc r3bl xyz", "pqr rust uvw"]);
        let mut test_clipboard = TestClipboard::default();
        // Single Line copying
        {
            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::End)],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 12]

            // Copying the contents from Selection
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Copy],
                &mut test_clipboard,
            );
            let content = test_clipboard.content.clone();
            assert_eq2!(content, "abc r3bl xyz".to_string());
        }

        // Multi-line Copying
        {
            // Current Caret Position : [row : 0, col : 12]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageDown)],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 1, col : 12]

            // Copying the contents from Selection
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Copy],
                &mut test_clipboard,
            );

            let content = test_clipboard.content;
            /* cspell:disable-next-line */
            assert_eq2!(content, "abc r3bl xyz\npqr rust uvw".to_string());
        }
    }

    #[test]
    fn test_paste() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.set_lines(["abc r3bl xyz", "pqr rust uvw"]);

        // Single Line Pasting
        {
            let mut test_clipboard = TestClipboard {
                content: "copied text ".to_string(),
            };

            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 4], // Move caret by 4 positions
                &mut test_clipboard,
            );

            // Current Caret Position : [row : 0, col : 4]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Paste],
                &mut test_clipboard,
            );

            let new_lines: VecEditorContentLines = smallvec![
                "abc copied text r3bl xyz".unicode_string(),
                "pqr rust uvw".unicode_string()
            ];
            assert_eq2!(buffer.get_lines(), &new_lines);
        }

        // Multi-line Pasting
        {
            // Current Caret Position : [row : 0, col : 4]
            let mut test_clipboard = TestClipboard {
                content: "old line\nnew line ".to_string(),
            };

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Paste],
                &mut test_clipboard,
            );

            let new_lines: VecEditorContentLines = smallvec![
                "abc copied text old line".unicode_string(),
                "new line r3bl xyz".unicode_string(),
                "pqr rust uvw".unicode_string()
            ];
            assert_eq2!(buffer.get_lines(), &new_lines);
        }
    }

    #[test]
    fn test_cut() {
        let mut buffer = EditorBuffer::new_empty(&Some(DEFAULT_SYN_HI_FILE_EXT), &None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.set_lines(["abc r3bl xyz", "pqr rust uvw"]);

        // Single Line cutting
        {
            let mut test_clipboard = TestClipboard::default();

            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::End)],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 12]

            // Cutting the contents from Selection and pasting to clipboard
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Cut],
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 0]

            let content = test_clipboard.content.clone();
            assert_eq2!(content, "abc r3bl xyz".to_string()); // copied to clipboard

            let new_lines: VecEditorContentLines = smallvec![
                "pqr rust uvw".unicode_string(), // First line 'abc r3bl xyz' is cut
            ];
            assert_eq2!(buffer.get_lines(), &new_lines);
        }

        // Multi-line Cutting
        {
            let mut test_clipboard = TestClipboard::default();

            buffer.set_lines(["abc r3bl xyz", "pqr rust uvw"]);
            // Current Caret Position : [row : 0, col : 0]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Down)],
                &mut test_clipboard,
            );
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 4], // Move caret by 4 positions
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 1, col : 4]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageUp)], // Select by pressing PageUp
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 0, col : 4]

            // Cutting the contents from Selection and pasting to clipboard
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Cut],
                &mut test_clipboard,
            );

            let content = test_clipboard.content;
            /* cspell:disable-next-line */
            assert_eq2!(content, "r3bl xyz\npqr ".to_string()); // copied to clipboard
            let new_lines: VecEditorContentLines =
                smallvec!["abc ".unicode_string(), "rust uvw".unicode_string()];
            assert_eq2!(buffer.get_lines(), &new_lines);
        }
    }
}
