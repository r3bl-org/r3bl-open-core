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

use std::fmt::Debug;

use r3bl_core::{call_if_true, string_storage, style_prompt, Size};

use crate::{editor_buffer::EditorBuffer,
            editor_buffer_clipboard_support::ClipboardService,
            history,
            DeleteSelectionWith,
            EditorArgsMut,
            EditorEngine,
            EditorEngineInternalApi,
            InputEvent,
            Key,
            KeyPress,
            KeyState,
            ModifierKeysMask,
            SelectMode,
            SpecialKey,
            DEBUG_TUI_COPY_PASTE};

/// Events that can be applied to the [EditorEngine] to modify an [EditorBuffer].
///
/// By providing a conversion from [InputEvent] to [EditorEvent] it becomes easier to write event
/// handlers that consume [InputEvent] and then execute [EditorEvent] on an [EditorBuffer].
#[derive(Clone, PartialEq, Eq)]
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
    Select(SelectionAction),
    Copy,
    Paste,
    Cut,
    Undo,
    Redo,
}

#[derive(Clone, PartialEq, Eq)]
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

#[derive(Clone, PartialEq, Eq)]
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
            let message = "🐥🐥🐥  EditorEvent::try_from";
            let details = string_storage!("{:?}", input_event);
            let details_fmt = style_prompt(&details);
            // % is Display, ? is Debug.
            tracing::debug! {
                message = %message,
                details = %details_fmt
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
        if editor_buffer.get_selection_list().is_empty() {
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
        engine: &mut EditorEngine,
        buffer: &mut EditorBuffer,
        editor_event: EditorEvent,
        clipboard_service_provider: &mut impl ClipboardService,
    ) {
        match editor_event {
            EditorEvent::Undo => {
                history::undo(buffer);
            }

            EditorEvent::Redo => {
                history::redo(buffer);
            }

            EditorEvent::InsertChar(character) => {
                Self::delete_text_if_selected(engine, buffer);
                EditorEngineInternalApi::insert_str_at_caret(
                    EditorArgsMut { buffer, engine },
                    &String::from(character),
                );
            }

            EditorEvent::InsertNewLine => {
                Self::delete_text_if_selected(engine, buffer);
                EditorEngineInternalApi::insert_new_line_at_caret(EditorArgsMut {
                    buffer,
                    engine,
                });
            }

            EditorEvent::Delete => {
                if buffer.get_selection_list().is_empty() {
                    // There is no selection and we want to delete a single character.
                    EditorEngineInternalApi::delete_at_caret(buffer, engine);
                } else {
                    // The text is selected and we want to delete the entire selected text.
                    EditorEngineInternalApi::delete_selected(
                        buffer,
                        engine,
                        DeleteSelectionWith::Delete,
                    );
                }
            }

            EditorEvent::Backspace => {
                if buffer.get_selection_list().is_empty() {
                    // There is no selection and we want to backspace a single character.
                    EditorEngineInternalApi::backspace_at_caret(buffer, engine);
                } else {
                    // The text is selected and we want to delete the entire selected text.
                    EditorEngineInternalApi::delete_selected(
                        buffer,
                        engine,
                        DeleteSelectionWith::Backspace,
                    );
                }
            }

            EditorEvent::MoveCaret(direction) => {
                match direction {
                    CaretDirection::Left => EditorEngineInternalApi::left(
                        buffer,
                        engine,
                        SelectMode::Disabled,
                    ),
                    CaretDirection::Right => EditorEngineInternalApi::right(
                        buffer,
                        engine,
                        SelectMode::Disabled,
                    ),
                    CaretDirection::Up => {
                        EditorEngineInternalApi::up(buffer, engine, SelectMode::Disabled)
                    }
                    CaretDirection::Down => EditorEngineInternalApi::down(
                        buffer,
                        engine,
                        SelectMode::Disabled,
                    ),
                };
            }

            EditorEvent::InsertString(chunk) => {
                Self::delete_text_if_selected(engine, buffer);
                EditorEngineInternalApi::insert_str_at_caret(
                    EditorArgsMut { buffer, engine },
                    &chunk,
                );
            }

            EditorEvent::Resize(_) => {
                // Check to see whether scroll is valid.
                EditorEngineInternalApi::validate_scroll(EditorArgsMut {
                    buffer,
                    engine,
                });
            }

            EditorEvent::Home => {
                EditorEngineInternalApi::home(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::End => {
                EditorEngineInternalApi::end(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::PageDown => {
                EditorEngineInternalApi::page_down(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::PageUp => {
                EditorEngineInternalApi::page_up(buffer, engine, SelectMode::Disabled);
            }

            EditorEvent::Select(selection_action) => match selection_action {
                SelectionAction::OneCharRight => {
                    EditorEngineInternalApi::right(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::OneCharLeft => {
                    EditorEngineInternalApi::left(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::OneLineDown => {
                    EditorEngineInternalApi::down(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::OneLineUp => {
                    EditorEngineInternalApi::up(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::PageUp => {
                    EditorEngineInternalApi::page_up(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::PageDown => {
                    EditorEngineInternalApi::page_down(
                        buffer,
                        engine,
                        SelectMode::Enabled,
                    );
                }
                SelectionAction::Home => {
                    EditorEngineInternalApi::home(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::End => {
                    EditorEngineInternalApi::end(buffer, engine, SelectMode::Enabled);
                }
                SelectionAction::All => {
                    EditorEngineInternalApi::select_all(buffer, SelectMode::Enabled);
                }
                SelectionAction::Esc => {
                    EditorEngineInternalApi::clear_selection(buffer);
                }
            },

            EditorEvent::Cut => {
                EditorEngineInternalApi::copy_editor_selection_to_clipboard(
                    buffer,
                    clipboard_service_provider,
                );
                Self::delete_text_if_selected(engine, buffer);
            }

            EditorEvent::Copy => {
                EditorEngineInternalApi::copy_editor_selection_to_clipboard(
                    buffer,
                    clipboard_service_provider,
                );
            }

            EditorEvent::Paste => {
                Self::delete_text_if_selected(engine, buffer);
                EditorEngineInternalApi::paste_clipboard_content_into_editor(
                    EditorArgsMut { buffer, engine },
                    clipboard_service_provider,
                );
            }
        };
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
