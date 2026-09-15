// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Crossterm event conversion and event dispatch for async readline.

use crate::{Button, FunctionKey, InputEvent, Key, KeyPress, KeyState, LineState,
            ModifierKeysMask, MouseInput, MouseInputKind, ReadlineControlFlow,
            ReadlineError, ReadlineEvent, SafeHistory, SpecialKey, StdMutex, VPHeight,
            VPSize, VPWidth, key_press, vp_col, vp_row};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent,
                       MouseEventKind};
use std::{io::Write, sync::Arc};
use tokio::sync::broadcast;

/// Applies an input event to the line state and renders the updated state.
///
/// If a spinner is active and the event is `Ctrl+C` or `Ctrl+D`, it cancels the spinner
/// instead of passing the event to the line state.
///
/// # Panics
///
/// Panics if the internal mutex is poisoned.
///
/// # Poison Safety
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
/// in the crate root documentation for details.
///
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
pub fn apply_event_to_line_state_and_render(
    input_event: InputEvent,
    line_state: &mut LineState,
    term: &mut dyn Write,
    self_safe_history: &SafeHistory,
    self_safe_is_spinner_active: &Arc<StdMutex<Option<broadcast::Sender<()>>>>,
) -> ReadlineControlFlow<ReadlineEvent, ReadlineError> {
    // Check if this is Ctrl+C or Ctrl+D
    let is_ctrl_c_or_d = input_event.matches_any_of_these_keypresses(&[
        key_press!(@char ModifierKeysMask::new().with_ctrl(), 'c'),
        key_press!(@char ModifierKeysMask::new().with_ctrl(), 'd'),
    ]);

    // Intercept Ctrl+C or Ctrl+D here and send a signal to spinner (if it is
    // active). And early return!
    let is_spinner_active = self_safe_is_spinner_active.write(Option::take);

    if is_ctrl_c_or_d && let Some(spinner_shutdown_sender) = is_spinner_active {
        // Send signal to SharedWriter spinner shutdown channel.
        // We don't care about the result of this operation.
        spinner_shutdown_sender.send(()).ok();
        return ReadlineControlFlow::Continue;
    }

    // Regular readline event handling - use the canonical InputEvent directly
    line_state
        .apply_event_and_render(&input_event, term, self_safe_history)
        .into()
}

/// Converts crossterm `KeyCode` to canonical `Key`
#[must_use]
fn convert_key_code_to_key(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::Char(c) => Some(Key::Character(c)),
        KeyCode::F(n) => {
            let fn_key = match n {
                1 => FunctionKey::F1,
                2 => FunctionKey::F2,
                3 => FunctionKey::F3,
                4 => FunctionKey::F4,
                5 => FunctionKey::F5,
                6 => FunctionKey::F6,
                7 => FunctionKey::F7,
                8 => FunctionKey::F8,
                9 => FunctionKey::F9,
                10 => FunctionKey::F10,
                11 => FunctionKey::F11,
                12 => FunctionKey::F12,
                _ => return None,
            };
            Some(Key::FunctionKey(fn_key))
        }
        KeyCode::Up => Some(Key::SpecialKey(SpecialKey::Up)),
        KeyCode::Down => Some(Key::SpecialKey(SpecialKey::Down)),
        KeyCode::Left => Some(Key::SpecialKey(SpecialKey::Left)),
        KeyCode::Right => Some(Key::SpecialKey(SpecialKey::Right)),
        KeyCode::Home => Some(Key::SpecialKey(SpecialKey::Home)),
        KeyCode::End => Some(Key::SpecialKey(SpecialKey::End)),
        KeyCode::PageUp => Some(Key::SpecialKey(SpecialKey::PageUp)),
        KeyCode::PageDown => Some(Key::SpecialKey(SpecialKey::PageDown)),
        KeyCode::Tab => Some(Key::SpecialKey(SpecialKey::Tab)),
        KeyCode::BackTab => Some(Key::SpecialKey(SpecialKey::BackTab)),
        KeyCode::Delete => Some(Key::SpecialKey(SpecialKey::Delete)),
        KeyCode::Insert => Some(Key::SpecialKey(SpecialKey::Insert)),
        KeyCode::Enter => Some(Key::SpecialKey(SpecialKey::Enter)),
        KeyCode::Backspace => Some(Key::SpecialKey(SpecialKey::Backspace)),
        KeyCode::Esc => Some(Key::SpecialKey(SpecialKey::Esc)),
        _ => None,
    }
}

/// Converts crossterm modifiers to canonical modifier mask
#[must_use]
fn convert_modifier_keys(modifiers: KeyModifiers) -> ModifierKeysMask {
    ModifierKeysMask {
        shift_key_state: if modifiers.contains(KeyModifiers::SHIFT) {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        ctrl_key_state: if modifiers.contains(KeyModifiers::CONTROL) {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
        alt_key_state: if modifiers.contains(KeyModifiers::ALT) {
            KeyState::Pressed
        } else {
            KeyState::NotPressed
        },
    }
}

/// Converts crossterm mouse button to canonical button
#[must_use]
fn convert_mouse_button(button: MouseButton) -> Button {
    match button {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        MouseButton::Middle => Button::Middle,
    }
}

/// Converts `crossterm::event::Event` to canonical `InputEvent`
#[must_use]
pub fn convert_crossterm_event_to_input_event(event: Event) -> Option<InputEvent> {
    match event {
        Event::Key(KeyEvent {
            code, modifiers, ..
        }) => {
            let key = convert_key_code_to_key(code)?;

            let mask = convert_modifier_keys(modifiers);
            let keypress = if mask.shift_key_state == KeyState::NotPressed
                && mask.ctrl_key_state == KeyState::NotPressed
                && mask.alt_key_state == KeyState::NotPressed
            {
                KeyPress::Plain { key }
            } else {
                KeyPress::WithModifiers { key, mask }
            };

            Some(InputEvent::Keyboard(keypress))
        }
        Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers,
        }) => {
            let modifiers_mask = convert_modifier_keys(modifiers);
            let mouse_input = MouseInput {
                pos: vp_col(column) + vp_row(row),
                kind: match kind {
                    MouseEventKind::Down(btn) => {
                        MouseInputKind::MouseDown(convert_mouse_button(btn))
                    }
                    MouseEventKind::Up(btn) => {
                        MouseInputKind::MouseUp(convert_mouse_button(btn))
                    }
                    MouseEventKind::Drag(btn) => {
                        MouseInputKind::MouseDrag(convert_mouse_button(btn))
                    }
                    MouseEventKind::Moved => MouseInputKind::MouseMove,
                    MouseEventKind::ScrollUp => MouseInputKind::ScrollUp,
                    MouseEventKind::ScrollDown => MouseInputKind::ScrollDown,
                    MouseEventKind::ScrollLeft => MouseInputKind::ScrollLeft,
                    MouseEventKind::ScrollRight => MouseInputKind::ScrollRight,
                },
                maybe_modifier_keys: if modifiers_mask.shift_key_state
                    == KeyState::NotPressed
                    && modifiers_mask.ctrl_key_state == KeyState::NotPressed
                    && modifiers_mask.alt_key_state == KeyState::NotPressed
                {
                    None
                } else {
                    Some(modifiers_mask)
                },
            };
            Some(InputEvent::Mouse(mouse_input))
        }
        Event::Resize(width, height) => Some(InputEvent::Resize(VPSize {
            col_width: VPWidth::from(width),
            row_height: VPHeight::from(height),
        })),
        _ => None,
    }
}

#[cfg(test)]
pub mod readline_test_fixtures {
    use crate::{CrosstermEventResult, InlineVec};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use smallvec::smallvec;

    pub(super) fn get_input_vec() -> InlineVec<CrosstermEventResult> {
        smallvec![
            // a
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
            ))),
            // b
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('b'),
                KeyModifiers::NONE,
            ))),
            // c
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::NONE,
            ))),
            // enter
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
        ]
    }
}

#[cfg(test)]
mod test_streams {
    use super::*;
    use crate::core::test_fixtures::gen_input_stream;
    use test_streams::readline_test_fixtures::get_input_vec;

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_generate_event_stream_pinned() {
        use futures_util::StreamExt;

        let mut count = 0;
        let mut it = gen_input_stream(get_input_vec());
        while let Some(event) = it.next().await {
            let lhs = event.expect("conversion error");
            let rhs = get_input_vec()[count]
                .as_ref()
                .expect("conversion error")
                .clone();
            assert_eq!(lhs, rhs);
            count += 1;
        }
    }
}
