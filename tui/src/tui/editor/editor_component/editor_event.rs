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

use std::fmt::Debug;

use crossterm::style::Stylize;
use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::{editor_buffer_clipboard_support::clipboard_provider_mock::EditorClipboard,
            *};

/// Events that can be applied to the [EditorEngine] to modify an [EditorBuffer].
///
/// By providing a conversion from [InputEvent] to [EditorEvent] it becomes easier to write event
/// handlers that consume [InputEvent] and then execute [EditorEvent] on an [EditorBuffer].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorEvent {
    InsertChar(char),
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
    Select(SelectionScope),
    Copy,
    Paste,
    Cut,
    Undo,
    Redo,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionScope {
    OneCharLeft,
    OneCharRight,
    OneLineUp,
    OneLineDown,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, GetSize)]
pub enum CaretDirection {
    Up,
    Down,
    Left,
    Right,
}

impl TryFrom<InputEvent> for EditorEvent {
    type Error = String;

    fn try_from(input_event: InputEvent) -> Result<Self, Self::Error> {
        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            log_debug(format!(
                "\n🐥🐥🐥  EditorEvent::try_from: {}",
                format!("{}", input_event).red().on_white()
            ));
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
            }) => Ok(EditorEvent::Select(SelectionScope::OneCharRight)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Left),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionScope::OneCharLeft)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Down),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionScope::OneLineDown)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Up),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionScope::OneLineUp)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::PageUp),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionScope::PageUp)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::PageDown),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionScope::PageDown)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::Home),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionScope::Home)),

            InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::SpecialKey(SpecialKey::End),
                mask:
                    ModifierKeysMask {
                        shift_key_state: KeyState::Pressed,
                        ctrl_key_state: KeyState::NotPressed,
                        alt_key_state: KeyState::NotPressed,
                    },
            }) => Ok(EditorEvent::Select(SelectionScope::End)),

            //  Clipboard events.
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

            _ => Err(format!("Invalid input event: {input_event:?}")),
        }
    }
}

impl EditorEvent {
    fn delete_text_if_selected(
        editor_engine: &mut EditorEngine,
        editor_buffer: &mut EditorBuffer,
    ) {
        if editor_buffer.get_selection_map().is_empty() {
            return;
        }

        // The text is selected and we want to delete the entire selected text.
        EditorEngineInternalApi::delete_selected(
            editor_buffer,
            editor_engine,
            DeleteSelectionWith::AnyKey,
        );
    }

    pub fn apply_editor_event(
        editor_engine: &mut EditorEngine,
        editor_buffer: &mut EditorBuffer,
        editor_event: EditorEvent,
        clipboard: &mut impl EditorClipboard,
    ) {
        match editor_event {
            EditorEvent::Undo => {
                history::undo(editor_buffer);
            }

            EditorEvent::Redo => {
                history::redo(editor_buffer);
            }

            EditorEvent::InsertChar(character) => {
                Self::delete_text_if_selected(editor_engine, editor_buffer);
                EditorEngineInternalApi::insert_str_at_caret(
                    EditorArgsMut {
                        editor_buffer,
                        editor_engine,
                    },
                    &String::from(character),
                )
            }

            EditorEvent::InsertNewLine => {
                Self::delete_text_if_selected(editor_engine, editor_buffer);
                EditorEngineInternalApi::insert_new_line_at_caret(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }

            EditorEvent::Delete => {
                if editor_buffer.get_selection_map().is_empty() {
                    // There is no selection and we want to delete a single character.
                    EditorEngineInternalApi::delete_at_caret(
                        editor_buffer,
                        editor_engine,
                    );
                } else {
                    // The text is selected and we want to delete the entire selected text.
                    EditorEngineInternalApi::delete_selected(
                        editor_buffer,
                        editor_engine,
                        DeleteSelectionWith::Delete,
                    );
                }
            }

            EditorEvent::Backspace => {
                if editor_buffer.get_selection_map().is_empty() {
                    // There is no selection and we want to backspace a single character.
                    EditorEngineInternalApi::backspace_at_caret(
                        editor_buffer,
                        editor_engine,
                    );
                } else {
                    // The text is selected and we want to delete the entire selected text.
                    EditorEngineInternalApi::delete_selected(
                        editor_buffer,
                        editor_engine,
                        DeleteSelectionWith::Backspace,
                    );
                }
            }

            EditorEvent::MoveCaret(direction) => {
                match direction {
                    CaretDirection::Left => EditorEngineInternalApi::left(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Disabled,
                    ),
                    CaretDirection::Right => EditorEngineInternalApi::right(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Disabled,
                    ),
                    CaretDirection::Up => EditorEngineInternalApi::up(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Disabled,
                    ),
                    CaretDirection::Down => EditorEngineInternalApi::down(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Disabled,
                    ),
                };
            }

            EditorEvent::InsertString(chunk) => {
                Self::delete_text_if_selected(editor_engine, editor_buffer);
                EditorEngineInternalApi::insert_str_at_caret(
                    EditorArgsMut {
                        editor_buffer,
                        editor_engine,
                    },
                    &chunk,
                )
            }

            EditorEvent::Resize(_) => {
                // Check to see whether scroll is valid.
                EditorEngineInternalApi::validate_scroll(EditorArgsMut {
                    editor_buffer,
                    editor_engine,
                });
            }

            EditorEvent::Home => {
                EditorEngineInternalApi::home(
                    editor_buffer,
                    editor_engine,
                    SelectMode::Disabled,
                );
            }

            EditorEvent::End => {
                EditorEngineInternalApi::end(
                    editor_buffer,
                    editor_engine,
                    SelectMode::Disabled,
                );
            }

            EditorEvent::PageDown => {
                EditorEngineInternalApi::page_down(
                    editor_buffer,
                    editor_engine,
                    SelectMode::Disabled,
                );
            }

            EditorEvent::PageUp => {
                EditorEngineInternalApi::page_up(
                    editor_buffer,
                    editor_engine,
                    SelectMode::Disabled,
                );
            }

            EditorEvent::Select(selection_scope) => match selection_scope {
                SelectionScope::OneCharRight => {
                    EditorEngineInternalApi::right(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionScope::OneCharLeft => {
                    EditorEngineInternalApi::left(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionScope::OneLineDown => {
                    EditorEngineInternalApi::down(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionScope::OneLineUp => {
                    EditorEngineInternalApi::up(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionScope::PageUp => {
                    EditorEngineInternalApi::page_up(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionScope::PageDown => {
                    EditorEngineInternalApi::page_down(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionScope::Home => {
                    EditorEngineInternalApi::home(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionScope::End => {
                    EditorEngineInternalApi::end(
                        editor_buffer,
                        editor_engine,
                        SelectMode::Enabled,
                    );
                }
            },

            EditorEvent::Cut => {
                EditorEngineInternalApi::copy_editor_selection_to_clipboard(
                    editor_buffer,
                    clipboard,
                );
                Self::delete_text_if_selected(editor_engine, editor_buffer);
            }

            EditorEvent::Copy => {
                EditorEngineInternalApi::copy_editor_selection_to_clipboard(
                    editor_buffer,
                    clipboard,
                );
            }

            EditorEvent::Paste => {
                Self::delete_text_if_selected(editor_engine, editor_buffer);
                EditorEngineInternalApi::paste_clipboard_content_into_editor(
                    EditorArgsMut {
                        editor_buffer,
                        editor_engine,
                    },
                    clipboard,
                )
            }
        };
    }

    pub fn apply_editor_events<S, A>(
        editor_engine: &mut EditorEngine,
        editor_buffer: &mut EditorBuffer,
        editor_event_vec: Vec<EditorEvent>,
        clipboard: &mut impl EditorClipboard,
    ) where
        S: Debug + Default + Clone + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
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
