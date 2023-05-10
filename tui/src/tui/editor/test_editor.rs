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
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::*;

    #[test]
    fn test_multiline_true() {
        // multiline true.
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );

        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 1)
        );
    }

    #[test]
    fn test_multiline_false() {
        // multiline false.
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 6, row_index: 0)
        );

        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 6, row_index: 0)
        );
        let maybe_line_str: Option<UnicodeString> =
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine);
        assert_eq2!(maybe_line_str.unwrap().string, "abcaba");
    }
}

#[cfg(test)]
mod test_editor_ops {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::*;

    #[test]
    fn editor_delete() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::Delete,
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::Delete,
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::Delete,
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 1)
        );
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::Backspace],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertString("😃".into()),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert::line_at_caret(&buffer, &engine, "abcab");
    }

    #[test]
    fn editor_validate_caret_position_on_up() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 │😀        │
        // 1 ▸1         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("😀".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertChar('1'),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 0)
        );
    }

    #[test]
    fn editor_validate_caret_position_on_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "😀\n1".
        // R ┌──────────┐
        // 0 ▸1         │
        // 1 │😀        │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertChar('1'),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("😀".into()),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 2, row_index: 1)
        );
    }

    #[test]
    fn editor_move_caret_up_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "abc\nab\na".
        // `this` should look like:
        // R ┌──────────┐
        // 0 │abc       │
        // 1 │ab        │
        // 2 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("abc".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("ab".into()),
                EditorEvent::InsertNewLine,
                EditorEvent::InsertString("a".into()),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );

        // Move caret up.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 1)
        );

        // Move caret up.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Up)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 0)
        );

        // Move caret up a few times. Caret moves to position 0.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Up),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Down),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Down)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 2)
        );
    }

    #[test]
    fn editor_insert_new_line() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 │a         │
        // 1 ▸a         │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertNewLine],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertChar('b'),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );

        assert::none_is_at_caret(&buffer, &engine);
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine)
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::InsertNewLine,
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertNewLine,
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(buffer.get_lines().len(), 5);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 3)
        );
    }

    #[test]
    fn editor_move_caret_left_right() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸a         │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert::none_is_at_caret(&buffer, &engine);

        // Move caret left.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸a         │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left), // No-op.
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert::str_is_at_caret(&buffer, &engine, "a");

        // Insert "1".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸1a        │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('1')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine)
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Left)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert::str_is_at_caret(&buffer, &engine, "1");

        // Move caret right.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸1a        │
        //   └─▴────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert::str_is_at_caret(&buffer, &engine, "a");

        // Insert "2".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸12a       │
        //   └──▴───────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('2')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert::str_is_at_caret(&buffer, &engine, "a");
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(&buffer, &engine)
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right), // No-op.
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::MoveCaret(CaretDirection::Right),
                EditorEvent::InsertNewLine,
                EditorEvent::MoveCaret(CaretDirection::Left),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::MoveCaret(CaretDirection::Right)],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertNewLine,
                EditorEvent::MoveCaret(CaretDirection::Up),
                EditorEvent::MoveCaret(CaretDirection::Right),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 2)
        );
    }

    #[test]
    fn editor_empty_state() {
        let buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        assert_eq2!(buffer.get_lines().len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn editor_insertion() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Move caret to col: 0, row: 0. Insert "a".
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸a░        │
        //   └─▴────────┘
        //   C0123456789
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('a')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(*buffer.get_lines(), vec![UnicodeString::from("a")]);
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 0)
        );

        // Move caret to col: 0, row: 1. Insert "b".
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('b')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            *buffer.get_lines(),
            vec![UnicodeString::from("a"), UnicodeString::from("b")]
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 1, row_index: 1)
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertNewLine,
                EditorEvent::InsertNewLine,
                EditorEvent::InsertChar('😀'),
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertChar('d')],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
            position!(col_index: 3, row_index: 3)
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
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::InsertString("🙏🏽".into())],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
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
            position!(col_index: 7, row_index: 3)
        );
    }

    #[test]
    fn editor_move_caret_home_end() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello". Then press home.
        // `this` should look like:
        // R ┌──────────┐
        // 0 ▸hello     │
        //   └▴─────────┘
        //   C0123456789
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![
                EditorEvent::InsertString("hello".to_string()),
                EditorEvent::Home,
            ],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Press end.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::End],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 5, row_index: 0)
        );
    }

    #[test]
    fn editor_move_caret_page_up_page_down() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello" many times.
        let max_lines = 20;
        let mut count = max_lines;
        while count > 0 {
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![
                    EditorEvent::InsertString(format!("{count}: {}", "hello")),
                    EditorEvent::InsertNewLine,
                ],
                &mock_real_objects_for_editor::make_shared_global_data(None),
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
            );
            count -= 1;
        }
        assert_eq2!(buffer.len(), ch!(max_lines + 1)); /* One empty line after content */

        // Press page up.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 10)
        );

        // Press page up.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Press page up.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageUp],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 0)
        );

        // Press page down.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );

        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 10)
        );

        // Press page down.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 20)
        );

        // Press page down.
        EditorEvent::apply_editor_events(
            &mut engine,
            &mut buffer,
            vec![EditorEvent::PageDown],
            &mock_real_objects_for_editor::make_shared_global_data(None),
            &mut mock_real_objects_for_editor::make_component_registry(),
            0,
        );
        assert_eq2!(
            buffer.get_caret(CaretKind::ScrollAdjusted),
            position!(col_index: 0, row_index: 20)
        );
    }

    #[test]
    fn editor_scroll_vertical() {
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert "hello" many times.
        let max_lines = 20;
        for count in 1..=max_lines {
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![
                    EditorEvent::InsertString(format!("{count}: {}", "hello")),
                    EditorEvent::InsertNewLine,
                ],
                &mock_real_objects_for_editor::make_shared_global_data(None),
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
            );
        }
        assert_eq2!(buffer.len(), ch!(max_lines + 1)); /* One empty line after content */

        // Press up 12 times.
        for _ in 1..12 {
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Up)],
                &mock_real_objects_for_editor::make_shared_global_data(None),
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
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
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Down)],
                &mock_real_objects_for_editor::make_shared_global_data(None),
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
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
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Insert a long line of text.
        let max_cols = 15;
        for count in 1..=max_cols {
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::InsertString(format!("{count}"))],
                &mock_real_objects_for_editor::make_shared_global_data(None),
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
            );
        }
        assert_eq2!(buffer.len(), ch!(1));
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
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Left)],
                &mock_real_objects_for_editor::make_shared_global_data(None),
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
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
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &mock_real_objects_for_editor::make_shared_global_data(None),
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
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
        let viewport_width = ch!(65);
        let viewport_height = ch!(2);
        let window_size = size!(col_count: viewport_width, row_count: viewport_height);

        let shared_global_data =
            mock_real_objects_for_editor::make_shared_global_data(window_size.into());
        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT));
        let mut engine =
            mock_real_objects_for_editor::make_editor_engine_with_bounds(window_size);

        let long_line = "# Did he take those two new droids with him? They hit accelerator.🙏🏽😀░ We will deal with your Rebel friends. Commence primary ignition.🙏🏽😀░";
        buffer.set_lines(vec![long_line.to_string()]);

        // Setup assertions.
        {
            assert_eq2!(buffer.len(), ch!(1));
            assert_eq2!(buffer.get_lines()[0].string, long_line);
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
                EditorEvent::apply_editor_events(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &shared_global_data,
                    &mut mock_real_objects_for_editor::make_component_registry(),
                    0,
                );
            }
            assert_eq2!(
                buffer.get_scroll_offset(),
                position!(col_index: 6, row_index: 0)
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 66, row_index: 0)
            );
            // Right of viewport.
            let result = buffer.get_lines()[0]
                .clone()
                .get_string_at_display_col_index(
                    buffer.get_caret(CaretKind::ScrollAdjusted).col_index,
                );
            assert_eq2!(result.unwrap().unicode_string_seg.string, "🙏🏽");

            // Press right 1 more time. The caret should correctly jump the width of "😀" from 70 to
            // 72.
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &shared_global_data,
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 70, row_index: 0)
            );
            // Right of viewport.
            let result = buffer.get_lines()[0]
                .clone()
                .get_string_at_display_col_index(
                    buffer.get_caret(CaretKind::ScrollAdjusted).col_index,
                );
            assert_eq2!(result.unwrap().unicode_string_seg.string, "😀");
        }

        // Press right 60 more times. The **LEFT** side of the viewport should be at the jumbo
        // emoji.
        {
            for _ in 1..60 {
                EditorEvent::apply_editor_events(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &shared_global_data,
                    &mut mock_real_objects_for_editor::make_component_registry(),
                    0,
                );
            }
            assert_eq2!(
                buffer.get_caret(CaretKind::Raw),
                position!(col_index: 64, row_index: 0)
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 130, row_index: 0)
            );
            assert_eq2!(
                buffer.get_scroll_offset(),
                position!(col_index: 66, row_index: 0)
            );
            // Start of viewport.
            let result = buffer.get_lines()[0]
                .clone()
                .get_string_at_display_col_index(buffer.get_scroll_offset().col_index);
            assert_eq2!(result.unwrap().unicode_string_seg.string, "🙏🏽");
        }

        // Press right 1 more time. It should jump the jumbo emoji at the start of the line (and not
        // just 1 character width). This moves the caret and the scroll offset to make sure that the
        // emoji at the start of the line can be displayed properly.
        {
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &shared_global_data,
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::Raw),
                position!(col_index: 61, row_index: 0)
            );
            assert_eq2!(
                buffer.get_caret(CaretKind::ScrollAdjusted),
                position!(col_index: 131, row_index: 0)
            );
            assert_eq2!(
                buffer.get_scroll_offset(),
                position!(col_index: 70, row_index: 0)
            );
            // Start of viewport.
            let result = buffer.get_lines()[0]
                .clone()
                .get_string_at_display_col_index(buffer.get_scroll_offset().col_index);
            assert_eq2!(result.unwrap().unicode_string_seg.string, "😀");
        }

        // Press right 4 times. It should jump the emoji at the start of the line (and not
        // just 1 character width); this moves the scroll offset to make sure that the emoji can be
        // properly displayed & it moves the caret too.
        {
            for _ in 1..4 {
                EditorEvent::apply_editor_events(
                    &mut engine,
                    &mut buffer,
                    vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                    &shared_global_data,
                    &mut mock_real_objects_for_editor::make_component_registry(),
                    0,
                );
            }
            // Start of viewport.
            let result = buffer.get_lines()[0]
                .clone()
                .get_string_at_display_col_index(buffer.get_scroll_offset().col_index);
            assert_eq2!(result.unwrap().unicode_string_seg.string, "😀");
        }

        // Press right 1 more time. It should jump the emoji.
        {
            EditorEvent::apply_editor_events(
                &mut engine,
                &mut buffer,
                vec![EditorEvent::MoveCaret(CaretDirection::Right)],
                &shared_global_data,
                &mut mock_real_objects_for_editor::make_component_registry(),
                0,
            );
            // Start of viewport.
            let result = buffer.get_lines()[0]
                .clone()
                .get_string_at_display_col_index(buffer.get_scroll_offset().col_index);
            assert_eq2!(result.unwrap().unicode_string_seg.string, "░");
        }
    }
}

pub mod mock_real_objects_for_editor {
    use r3bl_rs_utils_core::*;

    use crate::{test_dialog::mock_real_objects_for_dialog::State, *};

    pub fn make_shared_global_data(window_size: Option<Size>) -> SharedGlobalData {
        use std::sync::Arc;

        use tokio::sync::RwLock;

        let mut global_data = GlobalData::default();
        if let Some(window_size) = window_size {
            global_data.window_size = window_size;
        }

        let shared_global_data: SharedGlobalData = Arc::new(RwLock::new(global_data));
        shared_global_data
    }

    pub fn make_component_registry() -> ComponentRegistry<State, String> {
        let component_registry: ComponentRegistry<_, _> = ComponentRegistry::default();
        component_registry
    }

    pub fn make_editor_engine_with_bounds(size: Size) -> EditorEngine {
        let flex_box = FlexBox {
            style_adjusted_bounds_size: size,
            style_adjusted_origin_pos: position!( col_index: 0, row_index: 0 ),
            ..Default::default()
        };
        let current_box: PartialFlexBox = (&flex_box).into();
        EditorEngine {
            current_box,
            ..Default::default()
        }
    }

    pub fn make_editor_engine() -> EditorEngine {
        let flex_box = FlexBox {
            style_adjusted_bounds_size: size!( col_count: 10, row_count: 10 ),
            style_adjusted_origin_pos: position!( col_index: 0, row_index: 0 ),
            ..Default::default()
        };
        let current_box: PartialFlexBox = (&flex_box).into();
        EditorEngine {
            current_box,
            ..Default::default()
        }
    }
}

pub mod assert {
    use r3bl_rs_utils_core::*;

    use crate::*;

    pub fn none_is_at_caret(buffer: &EditorBuffer, engine: &EditorEngine) {
        assert_eq2!(
            EditorEngineInternalApi::string_at_caret(buffer, engine),
            None
        );
    }

    pub fn str_is_at_caret(
        editor_buffer: &EditorBuffer,
        engine: &EditorEngine,
        expected: &str,
    ) {
        match EditorEngineInternalApi::string_at_caret(editor_buffer, engine) {
            Some(UnicodeStringSegmentSliceResult {
                unicode_string_seg: s,
                ..
            }) => assert_eq2!(s.string, expected),
            None => panic!("Expected string at caret, but got None."),
        }
    }

    pub fn line_at_caret(
        editor_buffer: &EditorBuffer,
        engine: &EditorEngine,
        expected: &str,
    ) {
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(editor_buffer, engine)
                .unwrap()
                .string,
            expected
        );
    }
}
