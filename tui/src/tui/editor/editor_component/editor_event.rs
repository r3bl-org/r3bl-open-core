// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::Debug;

use crate::{DEBUG_TUI_COPY_PASTE, DeleteSelectionWith, EditorArgsMut, EditorBuffer,
            EditorEngine, InputEvent, Key, KeyState, ModifierKeysMask, SelectMode, Size,
            SpecialKey, clipboard_support::ClipboardService,
            editor_engine::engine_internal_api, fg_green, inline_string,
            md_parser::constants::NEW_LINE_CHAR, terminal_lib_backends::KeyPress,
            validate_scroll_on_resize};

/// Events that can be applied to the [`EditorEngine`] to modify an [`EditorBuffer`].
///
/// By providing a conversion from [`InputEvent`] to [`EditorEvent`] it becomes easier to
/// write event handlers that consume [`InputEvent`] and then execute [`EditorEvent`] on
/// an [`EditorBuffer`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EditorEvent {
    InsertChar(char),
    /// Inserts a string directly into the editor buffer.
    ///
    /// This event is used in two scenarios:
    /// 1. **Bracketed paste**: When text is pasted via terminal (right-click,
    ///    middle-click, etc.), the terminal provides the text directly through
    ///    [`InputEvent::BracketedPaste`].
    /// 2. **Programmatic insertion**: When code needs to insert multi-line text.
    ///
    /// For clipboard paste via Ctrl+V, see [`EditorEvent::Paste`].
    InsertString(String),
    InsertNewLine,
    Delete,
    Backspace,
    Home,
    End,
    PageDown,
    PageUp,
    MoveCaret(CaretDirection),
    Resize(Size),
    Select(SelectionAction),
    Copy,
    /// Pastes text from the system clipboard (triggered by Ctrl+V).
    ///
    /// Unlike [`EditorEvent::InsertString`] which receives text directly,
    /// this event reads from the system clipboard using [`ClipboardService`].
    Paste,
    Cut,
    Undo,
    Redo,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectionAction {
    OneCharLeft,
    OneCharRight,
    OneLineUp,
    OneLineDown,
    PageUp,
    PageDown,
    Home,
    End,
    All,
    Esc,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaretDirection {
    Up,
    Down,
    Left,
    Right,
}

impl TryFrom<InputEvent> for EditorEvent {
    type Error = String;

    #[allow(clippy::too_many_lines)]
    fn try_from(input_event: InputEvent) -> Result<Self, Self::Error> {
        DEBUG_TUI_COPY_PASTE.then(|| {
            // % is Display, ? is Debug.
            tracing::debug! {
                message = "🐥🐥🐥  EditorEvent::try_from",
                details = %fg_green(&inline_string!("{:?}", input_event))
            };
        });

        match input_event {
            // Undo, redo events.
            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::Character('z'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Undo),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::Character('y'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Redo),

            // Selection events.
            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Right),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::OneCharRight)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Left),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::OneCharLeft)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Down),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::OneLineDown)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Up),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::OneLineUp)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::PageUp),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::PageUp)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::PageDown),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::PageDown)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Home),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::Home)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::End),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::End)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::Character('a'),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::NotPressed,
                        ctrl_key_state: KeyState::Pressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionAction::All)),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Esc),
            }) => Ok(EditorEvent::Select(SelectionAction::Esc)),

            // Clipboard events (Ctrl+C, Ctrl+X, Ctrl+V).
            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::Character('c'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Copy),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::Character('x'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Cut),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::Character('v'),
                mask:
                    ModifierKeysMask {
                        ctrl_key_state: KeyState::Pressed,
                        shift_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Paste),

            // Other events.
            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::PageDown),
            }) => Ok(EditorEvent::PageDown),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::PageUp),
            }) => Ok(EditorEvent::PageUp),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Home),
            }) => Ok(EditorEvent::Home),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::End),
            }) => Ok(EditorEvent::End),

            InputEvent::Resize(size) => Ok(EditorEvent::Resize(size)),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character(character),
            }) => Ok(Self::InsertChar(character)),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Enter),
            }) => Ok(Self::InsertNewLine),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Delete),
            }) => Ok(Self::Delete),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Backspace),
            }) => Ok(Self::Backspace),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Up),
            }) => Ok(Self::MoveCaret(CaretDirection::Up)),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Down),
            }) => Ok(Self::MoveCaret(CaretDirection::Down)),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Left),
            }) => Ok(Self::MoveCaret(CaretDirection::Left)),

            InputEvent::Keyboard(KeyPress::Plain {
                key: Key::SpecialKey(SpecialKey::Right),
            }) => Ok(Self::MoveCaret(CaretDirection::Right)),

            InputEvent::BracketedPaste(text) => Ok(EditorEvent::InsertString(text)),

            _ => Err(format!("Invalid input event: {input_event:?}")),
        }
    }
}

impl EditorEvent {
    fn delete_text_if_selected(
        editor_engine: &mut EditorEngine,
        editor_buffer: &mut EditorBuffer,
    ) {
        if editor_buffer.get_selection_list().is_empty() {
            return;
        }

        engine_internal_api::delete_selected(
            editor_buffer,
            editor_engine,
            DeleteSelectionWith::AnyKey,
        );
    }

    /// Inserts text into the editor, normalizing line endings and handling multi-line
    /// content.
    ///
    /// # Text Sources
    /// The `text` parameter can come from various sources:
    /// - **Bracketed Paste** (Ctrl+Shift+V, right-click, middle-click): Terminal sends
    ///   raw text directly via [`InputEvent::BracketedPaste`]
    /// - **Clipboard Paste** (Ctrl+V): Text read from system clipboard via
    ///   [`ClipboardService`]
    /// - **Programmatic insertion**: Text inserted by code (e.g., autocomplete, snippets)
    ///
    /// # Why Line Ending Normalization is Critical
    /// Text can originate from different operating systems and terminal emulators, each
    /// with their own line ending conventions:
    /// - **Windows**: Uses CRLF (`\r\n`)
    /// - **Unix/Linux/macOS (modern)**: Uses LF (`\n`)
    /// - **Classic Mac OS**: Used CR (`\r`)
    ///
    /// Additionally, different terminal emulators may preserve or transform these line
    /// endings differently when handling bracketed paste. Some terminals on Windows might
    /// send `\r` instead of `\n`, while others preserve the original format. This
    /// function ensures consistent behavior regardless of the source.
    ///
    /// # Processing Steps
    /// 1. All line endings (`\r\n`, `\n`, `\r`) are normalized to `\n`
    /// 2. Text is split into lines at each `\n`
    /// 3. Lines are inserted individually with explicit newline characters between them
    ///
    /// This approach is required because the editor's internal APIs need lines to be
    /// inserted separately for proper rendering, cursor positioning, and undo/redo
    /// tracking.
    ///
    /// # Arguments
    /// * `engine` - The editor engine for cursor and viewport management
    /// * `buffer` - The editor buffer to insert text into
    /// * `text` - The text to insert (may contain any line ending format)
    /// * `is_paste` - Whether this text comes from a paste operation (for debug logging)
    fn insert_text_with_normalized_line_endings(
        engine: &mut EditorEngine,
        buffer: &mut EditorBuffer,
        text: &str,
        is_paste: bool,
    ) {
        // Normalize line endings: handle \r\n, \n, and \r as line separators
        let normalized_text = text
            .replace("\r\n", "\n") // Windows CRLF -> LF
            .replace('\r', "\n"); // Old Mac CR -> LF

        if normalized_text.contains(NEW_LINE_CHAR) {
            let lines: Vec<&str> = normalized_text.split(NEW_LINE_CHAR).collect();

            // For multi-line operations, use the batched insert to avoid multiple
            // validations
            engine_internal_api::insert_str_batch_at_caret(
                EditorArgsMut { engine, buffer },
                &lines,
            );
        } else {
            // Single line - insert directly
            engine_internal_api::insert_str_at_caret(
                EditorArgsMut { engine, buffer },
                &normalized_text,
            );
        }

        // Log paste operations for debugging
        if is_paste {
            DEBUG_TUI_COPY_PASTE.then(|| {
                tracing::debug! {
                    message = "📋📋📋 Text was pasted from clipboard",
                    clipboard_text = %text
                };
            });
        }
    }

    /// Applies an editor event to modify the buffer.
    ///
    /// Note: Text insertion has two paths:
    /// - `InsertString`: Direct text insertion (e.g., from bracketed paste)
    /// - `Paste`: Reads from system clipboard (requires `ClipboardService`)
    #[allow(clippy::too_many_lines)]
    pub fn apply_editor_event(
        engine: &mut EditorEngine,
        buffer: &mut EditorBuffer,
        event: EditorEvent,
        clipboard: &mut impl ClipboardService,
    ) {
        match event {
            EditorEvent::Undo => {
                engine.clear_ast_cache();
                buffer.undo();
            }

            EditorEvent::Redo => {
                engine.clear_ast_cache();
                buffer.redo();
            }

            EditorEvent::InsertChar(character) => {
                Self::delete_text_if_selected(engine, buffer);
                engine_internal_api::insert_str_at_caret(
                    EditorArgsMut { engine, buffer },
                    &String::from(character),
                );
            }

            EditorEvent::InsertNewLine => {
                Self::delete_text_if_selected(engine, buffer);
                engine_internal_api::insert_new_line_at_caret(EditorArgsMut {
                    engine,
                    buffer,
                });
            }

            EditorEvent::Delete => {
                if buffer.get_selection_list().is_empty() {
                    engine_internal_api::delete_at_caret(buffer, engine);
                } else {
                    engine_internal_api::delete_selected(
                        buffer,
                        engine,
                        DeleteSelectionWith::Delete,
                    );
                }
            }

            EditorEvent::Backspace => {
                if buffer.get_selection_list().is_empty() {
                    engine_internal_api::backspace_at_caret(buffer, engine);
                } else {
                    engine_internal_api::delete_selected(
                        buffer,
                        engine,
                        DeleteSelectionWith::Backspace,
                    );
                }
            }

            EditorEvent::MoveCaret(direction) => match direction {
                CaretDirection::Left => {
                    engine_internal_api::left(buffer, engine, SelectMode::Disabled);
                }
                CaretDirection::Right => {
                    engine_internal_api::right(buffer, engine, SelectMode::Disabled);
                }
                CaretDirection::Up => {
                    engine_internal_api::up(buffer, engine, SelectMode::Disabled);
                }
                CaretDirection::Down => {
                    engine_internal_api::down(buffer, engine, SelectMode::Disabled);
                }
            },

            EditorEvent::Resize(_) => {
                validate_scroll_on_resize(EditorArgsMut { engine, buffer });
            }

            EditorEvent::Home => {
                engine_internal_api::home(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::End => {
                engine_internal_api::end(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::PageDown => {
                engine_internal_api::page_down(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::PageUp => {
                engine_internal_api::page_up(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::Select(selection_action) => match selection_action {
                SelectionAction::OneCharRight => {
                    engine_internal_api::right(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::OneCharLeft => {
                    engine_internal_api::left(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::OneLineDown => {
                    engine_internal_api::down(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::OneLineUp => {
                    engine_internal_api::up(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::PageUp => {
                    engine_internal_api::page_up(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::PageDown => {
                    engine_internal_api::page_down(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::Home => {
                    engine_internal_api::home(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::End => {
                    engine_internal_api::end(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::All => {
                    engine_internal_api::select_all(buffer, SelectMode::Enabled);
                }
                SelectionAction::Esc => {
                    engine_internal_api::clear_selection(buffer);
                }
            },

            EditorEvent::Cut => {
                engine_internal_api::copy_editor_selection_to_clipboard(
                    buffer, clipboard,
                );
                Self::delete_text_if_selected(engine, buffer);
            }

            EditorEvent::Copy => {
                engine_internal_api::copy_editor_selection_to_clipboard(
                    buffer, clipboard,
                );
            }

            EditorEvent::InsertString(chunk) => {
                Self::delete_text_if_selected(engine, buffer);
                Self::insert_text_with_normalized_line_endings(
                    engine, buffer, &chunk, false,
                );
            }

            EditorEvent::Paste => {
                Self::delete_text_if_selected(engine, buffer);

                match clipboard.try_to_get_content_from_clipboard() {
                    Ok(clipboard_text) => {
                        Self::insert_text_with_normalized_line_endings(
                            engine,
                            buffer,
                            &clipboard_text,
                            true,
                        );
                    }
                    Err(error) => {
                        DEBUG_TUI_COPY_PASTE.then(|| {
                            tracing::debug! {
                                message = "📋📋📋 Failed to paste the text from clipboard",
                                error = ?error
                            };
                        });
                    }
                }
            }
        }
    }

    pub fn apply_editor_events<S, AS>(
        editor_engine: &mut EditorEngine,
        editor_buffer: &mut EditorBuffer,
        editor_event_vec: Vec<EditorEvent>,
        clipboard: &mut impl ClipboardService,
    ) where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        for editor_event in editor_event_vec {
            EditorEvent::apply_editor_event(
                editor_engine,
                editor_buffer,
                editor_event,
                clipboard,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{CaretDirection, CaretScrAdj, DEFAULT_SYN_HI_FILE_EXT, EditorBuffer,
                EditorEngine, EditorEngineConfig, EditorEvent, LineMode,
                SelectionAction, assert_eq2, caret_scr_adj,
                clipboard_service::clipboard_test_fixtures::TestClipboard, col,
                editor::editor_test_fixtures::mock_real_objects_for_editor,
                editor_engine::engine_internal_api, row};

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
        assert_eq2!(maybe_line_str.unwrap().content(), "abcaba");
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn test_text_selection() {
        use smallvec::smallvec;

        use crate::{InlineVec, RowIndex, SelectionRange};

        type SelectionList = InlineVec<(RowIndex, SelectionRange)>;

        fn csa(col_index: usize, row_index: usize) -> CaretScrAdj {
            caret_scr_adj(col(col_index) + row(row_index))
        }

        let mut buffer = EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        let mut engine = mock_real_objects_for_editor::make_editor_engine();

        // Buffer has two lines.
        // Row Index : 0 , Column Length : 12
        // Row Index : 1 , Column Length : 12
        buffer.init_with(["abc r3bl xyz", "pqr rust uvw"]);

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
