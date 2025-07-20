/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

#[cfg(test)]
mod test_config_options {
    use crate::{assert_eq2, caret_scr_adj, col,
                editor::editor_test_fixtures::mock_real_objects_for_editor,
                editor_engine::engine_internal_api, row,
                system_clipboard_service_provider::clipboard_test_fixtures::TestClipboard,
                CaretDirection, EditorBuffer, EditorEngine, EditorEngineConfig,
                EditorEvent, GCStringExt, LineMode, DEFAULT_SYN_HI_FILE_EXT};

    #[test]
    fn test_multiline_true() {
        // multiline true.
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
        // 2 ❱a         │
        //   └─⮬────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));

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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(1)));
    }

    #[test]
    fn test_multiline_false() {
        // multiline false.
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
        // 0 ❱abcaba    │
        //   └──────⮬───┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(6) + row(0)));

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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(6) + row(0)));
        let maybe_line_str = engine_internal_api::line_at_caret_to_string(&buffer);
        assert_eq2!(maybe_line_str.unwrap(), &"abcaba".grapheme_string());
    }
}

#[cfg(test)]
mod test_editor_ops {
    use smallvec::smallvec;

    use crate::{assert_eq2, caret_raw, caret_scr_adj, col,
                editor::{editor_test_fixtures::{assert, mock_real_objects_for_editor},
                         sizing::VecEditorContentLines},
                editor_engine::engine_internal_api,
                height, row, scr_ofs,
                system_clipboard_service_provider::clipboard_test_fixtures::TestClipboard,
                width, CaretDirection, EditorArgsMut, EditorBuffer, EditorEvent,
                GCString, GCStringExt, DEFAULT_SYN_HI_FILE_EXT};

    #[test]
    fn editor_delete() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));

        // Remove the "a" on the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱          │
        //   └⮬─────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));

        // Move to the end of the 2nd line. Press delete.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ❱ab        │
        //   └──⮬───────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move to the end of the 1st line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱abcab     │
        //   └───⮬──────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));
        assert::line_at_caret(&buffer, "abcab");
    }

    #[test]
    fn editor_backspace() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));

        // Remove the "a" on the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));

        // Remove the last line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ❱ab        │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move caret to start of 2nd line. Then press backspace.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱abcab     │
        //   └───⮬──────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 1);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));
        assert::line_at_caret(&buffer, "abcab");

        // Move caret to end of line. Insert "😃". Then move caret to end of line.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱abcab😃   │
        //   └───────⮬──┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(7) + row(0)));

        // Press backspace.
        EditorEvent::apply_editor_event(
            &mut engine,
            &mut buffer,
            EditorEvent::Backspace,
            &mut TestClipboard::default(),
        );
        assert::line_at_caret(&buffer, "abcab");
    }

    #[test]
    fn editor_validate_caret_pos_on_up() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 │😀        │
        // 1 ❱1         │
        //   └─⮬────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(1)));

        // Move caret up. It should not be in the middle of the smiley face.
        // R ┌──────────┐
        // 0 ❱😀        │
        // 1 │1         │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(0)));
    }

    #[test]
    fn editor_validate_caret_pos_on_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 ❱1         │
        // 1 │😀        │
        //   └──⮬───────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move caret up, and right. It should wrap around to the start of the next line
        // and be to the left of the smiley face.
        // R ┌──────────┐
        // 0 ❱1         │
        // 1 │😀        │
        //   └⮬─────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));

        // Move caret down. It should move to the end of the last line.
        // R ┌──────────┐
        // 0 │1         │
        // 1 ❱😀        │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));
    }

    #[test]
    fn editor_move_caret_up_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));

        // Move caret down. Goes to end of line 2 and stops.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));

        // Move caret up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(1)));

        // Move caret up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(0)));

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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Move right to end of line. Then down.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 ❱ab        │
        // 2 │a         │
        //   └──⮬───────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(1)));

        // Move caret down.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(2)));
    }

    #[test]
    fn editor_insert_new_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Starts w/ an empty line.
        assert_eq2!(buffer.get_lines().len(), 1);

        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        assert_eq2!(buffer.get_lines().len(), 1);
        assert::none_is_at_caret(&buffer);

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(0)));

        // Insert new line (at end of line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 2);
        assert::none_is_at_caret(&buffer);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ❱a         │
        //   └─⮬────────┘
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
        // 1 ❱a         │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");

        // Insert new line (at start of line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 ❱a         │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_lines().len(), 3);
        assert::str_is_at_caret(&buffer, "a");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));

        // Move caret right, insert "b".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 ❱ab        │
        //   └──⮬───────┘
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

        assert::none_is_at_caret(&buffer);
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(&buffer,).unwrap(),
            &"ab".grapheme_string()
        );

        // Move caret left, insert new line (at middle of line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 │a         │
        // 3 ❱b         │
        //   └⮬─────────┘
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
        assert::str_is_at_caret(&buffer, "b");
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(3)));
        assert_eq2!(buffer.get_lines().len(), 4);

        // Move caret to end of prev line. Press enter. `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │          │
        // 2 │a         │
        // 3 ❱          │
        // 4 │b         │
        //   └⮬─────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(3)));
    }

    #[test]
    fn editor_move_caret_left_right() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱a         │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        assert::none_is_at_caret(&buffer);

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱a         │
        //   └⮬─────────┘
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
        assert::str_is_at_caret(&buffer, "a");

        // Insert "1".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱1a        │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('1')],
            &mut TestClipboard::default(),
        );
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(&buffer,).unwrap(),
            &"1a".grapheme_string()
        );
        assert::str_is_at_caret(&buffer, "a");

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱1a        │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "1");

        // Move caret right.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱1a        │
        //   └─⮬────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");

        // Insert "2".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        //   └──⮬───────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('2')],
            &mut TestClipboard::default(),
        );
        assert::str_is_at_caret(&buffer, "a");
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(&buffer,).unwrap(),
            &"12a".grapheme_string()
        );

        // Move caret right. It should do nothing.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        //   └───⮬──────┘
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
        assert::none_is_at_caret(&buffer);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        //   └⮬─────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Move caret to end of line, press enter, then move caret left (should be at end
        // of prev line). `this` should look like:
        // R ┌──────────┐
        // 0 ❱12a       │
        // 1 │          │
        //   └───⮬──────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(0)));

        // Move caret right (should be at start of next line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │12a       │
        // 1 ❱          │
        //   └⮬─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(1)));

        // Press enter. Press up. Press right (should be at start of next line).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │12a       │
        // 1 │          │
        // 2 ❱          │
        //   └⮬─────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(2)));
    }

    #[test]
    fn editor_empty_state() {
        let buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        assert_eq2!(buffer.get_lines().len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn editor_insertion() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Move caret to col: FlexBoxId::from(0), row: 0. Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱a░        │
        //   └─⮬────────┘
        //   C0123456789
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines = smallvec!["a".grapheme_string()];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(0)));

        // Move caret to col: FlexBoxId::from(0), row: 1. Insert "b".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ❱b░        │
        //   └─⮬────────┘
        //   C0123456789
        engine_internal_api::insert_new_line_at_caret(EditorArgsMut {
            buffer: &mut buffer,
            engine: &mut engine,
        });
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('b')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines =
            smallvec!["a".grapheme_string(), "b".grapheme_string()];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(1) + row(1)));

        // Move caret to col: FlexBoxId::from(0), row: 3. Insert "😀" (unicode width = 2).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ❱😀░       │
        //   └──⮬───────┘
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
            "a".grapheme_string(),
            "b".grapheme_string(),
            "".grapheme_string(),
            "😀".grapheme_string()
        ];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(2) + row(3)));

        // Insert "d".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ❱😀d░      │
        //   └───⮬──────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('d')],
            &mut TestClipboard::default(),
        );
        let expected: VecEditorContentLines = smallvec![
            "a".grapheme_string(),
            "b".grapheme_string(),
            "".grapheme_string(),
            "😀d".grapheme_string()
        ];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(3) + row(3)));

        // Insert "🙏🏽" (unicode width = 2).
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 │b         │
        // 2 │          │
        // 3 ❱😀d🙏🏽░    │
        //   └─────⮬────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("🙏🏽".into())],
            &mut TestClipboard::default(),
        );
        assert_eq2!(width(2), GCString::width("🙏🏽"));
        let expected: VecEditorContentLines = smallvec![
            "a".grapheme_string(),
            "b".grapheme_string(),
            "".grapheme_string(),
            "😀d🙏🏽".grapheme_string()
        ];
        assert_eq2!(*buffer.get_lines(), expected);
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(3)));
    }

    #[test]
    fn editor_move_caret_home_end() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello". Then press home.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ❱hello     │
        //   └⮬─────────┘
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
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Press end.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::End],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(0)));
    }

    #[test]
    fn editor_move_caret_home_end_overflow_viewport() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // console_log!(OK_RAW "press hello");

        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("hello".to_string())],
            &mut TestClipboard::default(),
        );

        // console_log!(OK_RAW "press helloHello + END");

        // Insert "hello". Then press home.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸helloHELLO│
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("HELLOhello".to_string()),
                EditorEvent::Home,
            ],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // console_log!(OK_RAW "press end");

        // Press end.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::End],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_raw(), caret_raw(col(9) + row(0)));
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(15) + row(0)));
    }

    #[test]
    fn editor_move_caret_page_up_page_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
        assert_eq2!(buffer.len(), height(max_lines + 1)); /* One empty line after content */

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(10)));

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Press page up.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );

        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(10)));

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(20)));

        // Press page down.
        EditorEvent::apply_editor_events::<(), ()>(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mut TestClipboard::default(),
        );
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(20)));
    }

    #[test]
    fn editor_scroll_vertical() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
        assert_eq2!(buffer.len(), height(max_lines + 1)); /* One empty line after content */

        // Press up 12 times.
        for _ in 1..12 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Up)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_caret_raw(), caret_raw(col(0) + row(0)));
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(9)));
        assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(0) + row(9)));

        // Press down 9 times.
        for _ in 1..9 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Down)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_caret_raw(), caret_raw(col(0) + row(8)));
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(17)));
        assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(0) + row(9)));
    }

    #[test]
    fn editor_scroll_horizontal() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
        assert_eq2!(buffer.len(), height(1));
        assert_eq2!(buffer.get_caret_raw(), caret_raw(col(9) + row(0)));
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(21) + row(0)));
        assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(12) + row(0)));

        // Press left 5 times.
        for _ in 1..5 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Left)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_caret_raw(), caret_raw(col(5) + row(0)));
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(17) + row(0)));
        assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(12) + row(0)));

        // Press right 3 times.
        for _ in 1..3 {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
        }
        assert_eq2!(buffer.get_caret_raw(), caret_raw(col(7) + row(0)));
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(19) + row(0)));
        assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(12) + row(0)));
    }

    /// A jumbo emoji is a combination of 2 emoji (each one of which has > 1 display
    /// width, or unicode width).
    ///
    /// 🙏🏽 = U+1F64F + U+1F3FD
    /// 1. <https://unicodeplus.com/U+1F64F>
    /// 2. <https://unicodeplus.com/U+1F3FD>
    #[test]
    fn editor_scroll_right_horizontal_long_line_with_jumbo_emoji() {
        // Setup.
        let viewport_width = width(65);
        let viewport_height = height(2);
        let window_size = viewport_width + viewport_height;
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine =
            mock_real_objects_for_editor::make_editor_engine_with_bounds(window_size);

        let long_line = "# Did he take those two new droids with him? They hit accelerator.🙏🏽😀░ We will deal with your Rebel friends. Commence primary ignition.🙏🏽😀░";
        let long_line_gcs = long_line.grapheme_string();
        buffer.set_lines([long_line]);

        // Setup assertions.
        {
            assert_eq2!(width(2), GCString::width("🙏🏽"));
            assert_eq2!(buffer.len(), height(1));
            assert_eq2!(buffer.get_lines()[0], long_line.grapheme_string());
            let us = &buffer.get_lines()[0];
            assert_eq2!(us, &long_line_gcs);
            assert_eq2!(buffer.get_caret_raw(), caret_raw(col(0) + row(0)));
            assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));
            assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(0) + row(0)));
        }

        // Press right 67 times. The caret should correctly jump the width of the jumbo
        // emoji (🙏🏽) on the **RIGHT** of viewport and select it.
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
            assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(4) + row(0)));
            assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(66) + row(0)));
            // Right of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_caret_scr_adj().col_index;
            let result = line.get_string_at(display_col_index);
            assert_eq2!(result.unwrap().string.string, "🙏🏽");

            // Press right 1 more time. The caret should correctly jump the width of "😀"
            // from 68 to 70.
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
            assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(68) + row(0)));
            // Right of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_caret_scr_adj().col_index;
            let result = line.get_string_at(display_col_index);
            assert_eq2!(result.unwrap().string.string, "😀");
        }

        // Press right 60 more times. The **LEFT** side of the viewport should be at the
        // jumbo emoji.
        {
            for _ in 1..60 {
                EditorEvent::apply_editor_events::<(), ()>(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &mut TestClipboard::default(),
                );
            }
            assert_eq2!(buffer.get_caret_raw(), caret_raw(col(64) + row(0)));
            assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(128) + row(0)));
            assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(64) + row(0)));
            // Start of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_scr_ofs().col_index;
            let result = line.get_string_at(display_col_index);
            assert_eq2!(result.unwrap().string.string, "r");
        }

        // Press right 1 more time. It should jump the jumbo emoji at the start of the
        // line (and not just 1 character width). This moves the caret and the scroll
        // offset to make sure that the emoji at the start of the line can be displayed
        // properly.
        {
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mut TestClipboard::default(),
            );
            assert_eq2!(buffer.get_caret_raw(), caret_raw(col(64) + row(0)));
            assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(129) + row(0)));
            assert_eq2!(buffer.get_scr_ofs(), scr_ofs(col(65) + row(0)));
            // Start of viewport.
            let line = &buffer.get_lines()[0];
            let display_col_index = buffer.get_scr_ofs().col_index;
            let result = line.get_string_at(display_col_index);
            assert_eq2!(result.unwrap().string.string, ".");
        }

        // Press right 4 times. It should jump the emoji at the start of the line (and not
        // just 1 character width); this moves the scroll offset to make sure that the
        // emoji can be properly displayed & it moves the caret too.
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
            let display_col_index = buffer.get_scr_ofs().col_index;
            let result = line.get_string_at(display_col_index);
            assert_eq2!(result.unwrap().string.string, "😀");
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
            let display_col_index = buffer.get_scr_ofs().col_index;
            let result = line.get_string_at(display_col_index);
            assert_eq2!(result.unwrap().string.string, "░");
        }
    }
}

#[cfg(test)]
mod selection_tests {
    use smallvec::smallvec;

    use crate::{assert_eq2, caret_scr_adj, col, row, CaretScrAdj, InlineVec, RowIndex,
                SelectionRange};

    type SelectionList = InlineVec<(RowIndex, SelectionRange)>;

    use crate::{editor::editor_test_fixtures::mock_real_objects_for_editor,
                system_clipboard_service_provider::clipboard_test_fixtures::TestClipboard,
                CaretDirection, EditorBuffer, EditorEvent, SelectionAction,
                DEFAULT_SYN_HI_FILE_EXT};

    fn csa(col_index: usize, row_index: usize) -> CaretScrAdj {
        caret_scr_adj(col(col_index) + row(row_index))
    }

    #[test]
    fn test_text_selection() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
                (row(0), (csa(0, 0), csa(12, 0)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 0, col : 12]
            // Reverse selection up to the start of the line.

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 5], /* Move caret
                                                                         * to right for
                                                                         * 5 times */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 4]

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::Home)], /* Select text up to
                                                                   * starting */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 0]

            // Selection Map : {{1, SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (row(1), (csa(0, 1), csa(4, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 0]
            // De-Select one character to right

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneCharRight)], /* Move Selection to Right */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 1]

            // Selection Map : {{1, SelectionRange {start: 1, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (row(1), (csa(1, 1), csa(4, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 1]
            // Select one character to left

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneCharLeft)], /* Move Selection to Left */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 0]

            // Selection Map : {{1, SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (row(1), (csa(0, 1), csa(4, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 0]
            // Move Selection Caret to one line upwards

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneLineUp)], /* Select one
                                                                        * line up */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 0, col : 0]

            // Selection Map : {{0, SelectionRange {start: 0, end: 12}}, {1,
            // SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (row(0), (csa(0, 0), csa(12, 0)).into()),
                (row(1), (csa(0, 1), csa(4, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 0, col : 0]
            // Move Selection Caret to one line downwards

            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::OneLineDown)], /* De-Select one line down */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 0]

            // Selection Map : {{1, SelectionRange {start: 0, end: 4}}}
            let selection_list: SelectionList = smallvec! {
                (row(1), (csa(0, 1), csa(4, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 0]
            // Move Caret to one char right and drop down selection
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)], /* Move caret to
                                                                      * right */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 1]

            // Selection Map : {}
            let selection_list: SelectionList = smallvec![];
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 1]
            // Select by pressing PageUp
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageUp)], /* Select by pressing PageUp */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 0, col : 1]

            // Selection Map : {{0, SelectionRange {start: 1, end: 12}}, {1,
            // SelectionRange {start: 0, end: 1}}}
            let selection_list: SelectionList = smallvec! {
                (row(0), (csa(1, 0), csa(12, 0)).into()),
                (row(1), (csa(0, 1), csa(1, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 0, col : 1]
            // Select by pressing PageDown
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)], /* Move caret one
                                                                      * char right */
                &mut TestClipboard::default(),
            );
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageDown)], /* Select by pressing PageDown */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 2]

            // Selection Map : {{0, SelectionRange {start: 2, end: 12}},{1, SelectionRange
            // {start: 0, end: 2}}}
            let selection_list: SelectionList = smallvec! {
                (row(0), (csa(2, 0), csa(12, 0)).into()),
                (row(1), (csa(0, 1), csa(2, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 2]
            // Select by pressing All
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::All)], /* Select by pressing
                                                                  * All */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 2]

            // Selection Map : {{0, SelectionRange {start: 0, end: 12}},{1, SelectionRange
            // {start: 0, end: 2}}}
            let selection_list: SelectionList = smallvec! {
                (row(0), (csa(0, 0), csa(12, 0)).into()),
                (row(1), (csa(0, 1), csa(12, 1)).into())
            };
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }

        {
            // Current Caret Position : [row : 1, col : 2]
            // Select by pressing Esc
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::Esc)], /* Select by pressing
                                                                  * Esc */
                &mut TestClipboard::default(),
            );
            // Current Caret Position : [row : 1, col : 2]

            // Selection Map : {}
            let selection_list: SelectionList = smallvec![];
            assert_eq2!(
                buffer.get_selection_list().get_ordered_list(),
                &selection_list
            );
        }
    }
}

#[cfg(test)]
mod clipboard_tests {
    use smallvec::smallvec;

    use crate::{assert_eq2,
                editor::{editor_test_fixtures::mock_real_objects_for_editor,
                         sizing::VecEditorContentLines},
                system_clipboard_service_provider::clipboard_test_fixtures::TestClipboard,
                CaretDirection, EditorBuffer, EditorEvent, GCStringExt,
                SelectionAction, DEFAULT_SYN_HI_FILE_EXT};

    #[test]
    fn test_copy() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 4], /* Move caret by 4 positions */
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
                "abc copied text r3bl xyz".grapheme_string(),
                "pqr rust uvw".grapheme_string()
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
                "abc copied text old line".grapheme_string(),
                "new line r3bl xyz".grapheme_string(),
                "pqr rust uvw".grapheme_string()
            ];
            assert_eq2!(buffer.get_lines(), &new_lines);
        }
    }

    #[test]
    fn test_cut() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
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
                "pqr rust uvw".grapheme_string(), // First line 'abc r3bl xyz' is cut
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
                vec![EditorEvent::MoveCaret(CaretDirection::Right); 4], /* Move caret by 4 positions */
                &mut test_clipboard,
            );
            // Current Caret Position : [row : 1, col : 4]
            EditorEvent::apply_editor_events::<(), ()>(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::Select(SelectionAction::PageUp)], /* Select by pressing PageUp */
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
                smallvec!["abc ".grapheme_string(), "rust uvw".grapheme_string()];
            assert_eq2!(buffer.get_lines(), &new_lines);
        }
    }
}

#[cfg(test)]
mod test_batch_operations {
    use crate::{assert_eq2, caret_scr_adj, col,
                editor::editor_test_fixtures::mock_real_objects_for_editor,
                editor_engine::engine_internal_api,
                row, EditorArgsMut, EditorBuffer, DEFAULT_SYN_HI_FILE_EXT, 
                width, height, GCStringExt};

    #[test]
    fn test_insert_lines_batch_at_caret_basic() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        let lines = vec!["line1", "line2", "line3"];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut { engine: &mut engine, buffer: &mut buffer },
            lines,
        );
        
        assert_eq2!(buffer.get_lines().len(), 3);
        assert_eq2!(buffer.get_lines()[0], "line1".grapheme_string());
        assert_eq2!(buffer.get_lines()[1], "line2".grapheme_string());
        assert_eq2!(buffer.get_lines()[2], "line3".grapheme_string());
        
        // Caret should be at the end of the last line
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(5) + row(2)));
    }

    #[test]
    fn test_insert_lines_batch_with_empty_lines() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        let lines = vec!["line1", "", "line3"];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut { engine: &mut engine, buffer: &mut buffer },
            lines,
        );
        
        assert_eq2!(buffer.get_lines().len(), 3);
        assert_eq2!(buffer.get_lines()[0], "line1".grapheme_string());
        assert_eq2!(buffer.get_lines()[1], "".grapheme_string());
        assert_eq2!(buffer.get_lines()[2], "line3".grapheme_string());
    }

    #[test]
    fn test_insert_lines_batch_at_middle_of_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        // First insert some initial content
        buffer.set_lines(vec!["existing content".to_string()]);
        
        // Move caret to middle of line (after "existing")
        let buffer_mut = buffer.get_mut(engine.viewport());
        buffer_mut.inner.caret_raw.col_index = col(8); // Position after "existing"
        drop(buffer_mut);
        
        // Insert new lines
        let lines = vec!["NEW1", "NEW2"];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut { engine: &mut engine, buffer: &mut buffer },
            lines,
        );
        
        // The batch insert behavior when inserting in the middle of a line:
        // 1. "NEW1" is inserted at position 8, making "existingNEW1"
        // 2. A newline is inserted (caret is now at end of "existingNEW1", so new line is created)
        // 3. "NEW2" is inserted on the new line
        // 4. The remaining " content" stays with "NEW2" since there's no trailing newline
        // This is the expected behavior for batch insert
        assert_eq2!(buffer.get_lines().len(), 2);
        assert_eq2!(buffer.get_lines()[0], "existingNEW1".grapheme_string());
        assert_eq2!(buffer.get_lines()[1], "NEW2 content".grapheme_string());
    }

    #[test]
    fn test_batch_vs_individual_insert_result_equivalence() {
        // Test that batch insert produces the same result as individual inserts
        
        // Setup for batch insert
        let mut buffer_batch = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine_batch = mock_real_objects_for_editor::make_editor_engine();
        
        // Setup for individual inserts
        let mut buffer_individual = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine_individual = mock_real_objects_for_editor::make_editor_engine();
        
        let lines = vec!["line1", "line2", "line3"];
        
        // Batch insert
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut { engine: &mut engine_batch, buffer: &mut buffer_batch },
            lines.clone(),
        );
        
        // Individual inserts
        for (i, line) in lines.iter().enumerate() {
            engine_internal_api::insert_str_at_caret(
                EditorArgsMut { engine: &mut engine_individual, buffer: &mut buffer_individual },
                line,
            );
            if i < lines.len() - 1 {
                engine_internal_api::insert_new_line_at_caret(
                    EditorArgsMut { engine: &mut engine_individual, buffer: &mut buffer_individual },
                );
            }
        }
        
        // Both should produce the same result
        assert_eq2!(buffer_batch.get_lines().len(), buffer_individual.get_lines().len());
        for i in 0..buffer_batch.get_lines().len() {
            assert_eq2!(
                buffer_batch.get_lines()[i],
                buffer_individual.get_lines()[i]
            );
        }
        assert_eq2!(buffer_batch.get_caret_scr_adj(), buffer_individual.get_caret_scr_adj());
    }

    #[test]
    fn test_insert_lines_batch_empty_vector() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        let lines: Vec<&str> = vec![];
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut { engine: &mut engine, buffer: &mut buffer },
            lines,
        );
        
        // Should remain with the default single empty line
        assert_eq2!(buffer.get_lines().len(), 1);
        assert_eq2!(buffer.get_lines()[0], "".grapheme_string());
        assert_eq2!(buffer.get_caret_scr_adj(), caret_scr_adj(col(0) + row(0)));
    }

    #[test]
    fn test_insert_lines_batch_large_content() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine_with_bounds(
            width(80) + height(50)
        );
        
        // Create a large set of lines
        let line_strings: Vec<String> = (0..100)
            .map(|i| format!("Line number {}", i))
            .collect();
        let lines: Vec<&str> = line_strings.iter().map(|s| s.as_str()).collect();
        
        engine_internal_api::insert_str_batch_at_caret(
            EditorArgsMut { engine: &mut engine, buffer: &mut buffer },
            lines.clone(),
        );
        
        assert_eq2!(buffer.get_lines().len(), 100);
        for i in 0..100 {
            assert_eq2!(buffer.get_lines()[i], format!("Line number {}", i).grapheme_string());
        }
    }
}

#[cfg(test)]
mod test_engine_internal_api {
    use crate::{assert_eq2, col, row, caret_raw,
                editor::editor_test_fixtures::mock_real_objects_for_editor,
                editor_engine::engine_internal_api,
                system_clipboard_service_provider::clipboard_test_fixtures::TestClipboard,
                EditorBuffer, SelectMode, DeleteSelectionWith,
                DEFAULT_SYN_HI_FILE_EXT, GCStringExt};

    #[test]
    fn test_select_all() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let _engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add some content
        buffer.set_lines(vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);
        
        // Select all
        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);
        
        // Check that all lines are selected
        let selection_list = buffer.get_selection_list();
        assert_eq2!(selection_list.len(), 3);
        assert!(selection_list.get(row(0)).is_some());
        assert!(selection_list.get(row(1)).is_some());
        assert!(selection_list.get(row(2)).is_some());
    }

    #[test]
    fn test_clear_selection() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let _engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add some content and select all
        buffer.set_lines(vec![
            "line 1".to_string(),
            "line 2".to_string(),
        ]);
        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);
        
        // Verify selection exists
        assert_eq2!(buffer.get_selection_list().len(), 2);
        
        // Clear selection
        engine_internal_api::clear_selection(&mut buffer);
        
        // Verify selection is cleared
        assert_eq2!(buffer.get_selection_list().len(), 0);
    }

    #[test]
    fn test_delete_selected() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add some content
        buffer.set_lines(vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);
        
        // Select line 2
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(1));
        drop(buffer_mut);
        
        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);
        
        // Delete selected content
        engine_internal_api::delete_selected(
            &mut buffer,
            &mut engine,
            DeleteSelectionWith::Delete,
        );
        
        // Should have no lines left after deleting all
        assert_eq2!(buffer.get_lines().len(), 0);
    }

    #[test]
    fn test_copy_editor_selection_to_clipboard() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut test_clipboard = TestClipboard::default();
        
        // Add some content
        buffer.set_lines(vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ]);
        
        // Select all
        engine_internal_api::select_all(&mut buffer, SelectMode::Enabled);
        
        // Copy to clipboard
        engine_internal_api::copy_editor_selection_to_clipboard(
            &buffer,
            &mut test_clipboard,
        );
        
        // Check clipboard content
        assert_eq2!(test_clipboard.content, "line 1\nline 2\nline 3");
    }

    #[test]
    fn test_delete_selected_with_partial_selection() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add some content
        buffer.set_lines(vec![
            "hello world".to_string(),
            "second line".to_string(),
        ]);
        
        // Move caret to position (5, 0) - after "hello"
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(5) + row(0));
        drop(buffer_mut);
        
        // Select from current position to end of line
        engine_internal_api::end(&mut buffer, &mut engine, SelectMode::Enabled);
        
        // Delete selected content
        engine_internal_api::delete_selected(
            &mut buffer,
            &mut engine,
            DeleteSelectionWith::Delete,
        );
        
        // Should have "hello" on first line and "second line" on second
        assert_eq2!(buffer.get_lines().len(), 2);
        assert_eq2!(buffer.get_lines()[0], "hello".grapheme_string());
        assert_eq2!(buffer.get_lines()[1], "second line".grapheme_string());
    }

    #[test]
    fn test_navigation_with_selection() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add content
        buffer.set_lines(vec![
            "first line".to_string(),
            "second line".to_string(),
            "third line".to_string(),
        ]);
        
        // Start at beginning
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(0));
        drop(buffer_mut);
        
        // Select right 5 characters
        for _ in 0..5 {
            engine_internal_api::right(&mut buffer, &mut engine, SelectMode::Enabled);
        }
        
        // Should have selected "first"
        let selection_list = buffer.get_selection_list();
        assert_eq2!(selection_list.len(), 1);
        assert!(selection_list.get(row(0)).is_some());
        
        // Move down with selection
        engine_internal_api::down(&mut buffer, &mut engine, SelectMode::Enabled);
        
        // Should now have selection on two lines
        let selection_list = buffer.get_selection_list();
        assert_eq2!(selection_list.len(), 2);
        assert!(selection_list.get(row(0)).is_some());
        assert!(selection_list.get(row(1)).is_some());
    }

    #[test]
    fn test_line_at_caret_to_string() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add content
        buffer.set_lines(vec![
            "first line".to_string(),
            "second line".to_string(),
        ]);
        
        // Test at first line
        let line = engine_internal_api::line_at_caret_to_string(&buffer);
        assert_eq2!(line.unwrap(), &"first line".grapheme_string());
        
        // Move to second line
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(1));
        drop(buffer_mut);
        
        let line = engine_internal_api::line_at_caret_to_string(&buffer);
        assert_eq2!(line.unwrap(), &"second line".grapheme_string());
    }

    #[test]
    fn test_page_navigation() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add many lines
        let lines: Vec<String> = (0..50).map(|i| format!("Line {}", i)).collect();
        buffer.set_lines(lines);
        
        // Start at top
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(0) + row(0));
        drop(buffer_mut);
        
        // Page down
        engine_internal_api::page_down(&mut buffer, &mut engine, SelectMode::Disabled);
        
        // Should have moved down (exact amount depends on viewport height)
        let caret_pos = buffer.get_caret_scr_adj();
        assert!(caret_pos.row_index > row(0));
        
        // Page up
        engine_internal_api::page_up(&mut buffer, &mut engine, SelectMode::Disabled);
        
        // Should be back near the top
        let caret_pos = buffer.get_caret_scr_adj();
        assert_eq2!(caret_pos.row_index, row(0));
    }

    #[test]
    fn test_home_end_navigation() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();
        
        // Add content
        buffer.set_lines(vec!["Hello, World!".to_string()]);
        
        // Move to middle of line
        let buffer_mut = buffer.get_mut(engine.viewport());
        *buffer_mut.inner.caret_raw = caret_raw(col(7) + row(0));
        drop(buffer_mut);
        
        // Test home
        engine_internal_api::home(&mut buffer, &mut engine, SelectMode::Disabled);
        assert_eq2!(buffer.get_caret_scr_adj().col_index, col(0));
        
        // Test end
        engine_internal_api::end(&mut buffer, &mut engine, SelectMode::Disabled);
        assert_eq2!(buffer.get_caret_scr_adj().col_index, col(13)); // Length of "Hello, World!"
    }
}
