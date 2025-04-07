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

use r3bl_core::{fg_green, inline_string, Size};

use crate::{clipboard_support::ClipboardService,
            editor_buffer::EditorBuffer,
            editor_engine::engine_internal_api,
            terminal_lib_backends::KeyPress,
            validate_scroll_on_resize,
            DeleteSelectionWith,
            EditorArgsMut,
            EditorEngine,
            InputEvent,
            Key,
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
        engine_internal_api::delete_selected(
            editor_buffer,
            editor_engine,
            DeleteSelectionWith::AnyKey,
        );
    }

    pub fn apply_editor_event(
        engine: &mut EditorEngine,
        buffer: &mut EditorBuffer,
        event: EditorEvent,
        clipboard: &mut impl ClipboardService,
    ) {
        match event {
            EditorEvent::Undo => {
                buffer.undo();
            }

            EditorEvent::Redo => {
                buffer.redo();
            }

            EditorEvent::InsertChar(character) => {
                Self::delete_text_if_selected(engine, buffer);
                engine_internal_api::insert_str_at_caret(
                    EditorArgsMut { buffer, engine },
                    &String::from(character),
                );
            }

            EditorEvent::InsertNewLine => {
                Self::delete_text_if_selected(engine, buffer);
                engine_internal_api::insert_new_line_at_caret(EditorArgsMut {
                    buffer,
                    engine,
                });
            }

            EditorEvent::Delete => {
                if buffer.get_selection_list().is_empty() {
                    // There is no selection and we want to delete a single character.
                    engine_internal_api::delete_at_caret(buffer, engine);
                } else {
                    // The text is selected and we want to delete the entire selected text.
                    engine_internal_api::delete_selected(
                        buffer,
                        engine,
                        DeleteSelectionWith::Delete,
                    );
                }
            }

            EditorEvent::Backspace => {
                if buffer.get_selection_list().is_empty() {
                    // There is no selection and we want to backspace a single character.
                    engine_internal_api::backspace_at_caret(buffer, engine);
                } else {
                    // The text is selected and we want to delete the entire selected text.
                    engine_internal_api::delete_selected(
                        buffer,
                        engine,
                        DeleteSelectionWith::Backspace,
                    );
                }
            }

            EditorEvent::MoveCaret(direction) => {
                match direction {
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
                };
            }

            EditorEvent::InsertString(chunk) => {
                Self::delete_text_if_selected(engine, buffer);
                engine_internal_api::insert_str_at_caret(
                    EditorArgsMut { buffer, engine },
                    &chunk,
                );
            }

            EditorEvent::Resize(_) => {
                // Check to see whether scroll is valid.
                validate_scroll_on_resize(EditorArgsMut { buffer, engine });
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

            EditorEvent::Paste => {
                Self::delete_text_if_selected(engine, buffer);
                engine_internal_api::paste_clipboard_content_into_editor(
                    EditorArgsMut { buffer, engine },
                    clipboard,
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
