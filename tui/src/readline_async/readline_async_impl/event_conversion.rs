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
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section in the
/// crate root documentation for details.
///
/// [`InputEvent`]: crate::InputEvent
/// [`LineState`]: crate::LineState
/// [`ReadlineControlFlow`]: crate::ReadlineControlFlow
/// [`ReadlineError`]: crate::ReadlineError
/// [`ReadlineEvent`]: crate::ReadlineEvent
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
    if is_ctrl_c_or_d {
        let is_spinner_active = self_safe_is_spinner_active.write(Option::take);
        if let Some(spinner_shutdown_sender) = is_spinner_active {
            // Send signal to SharedWriter spinner shutdown channel.
            // We don't care about the result of this operation.
            spinner_shutdown_sender.send(()).ok();
            return ReadlineControlFlow::Continue;
        }
    }

    // Regular readline event handling - use the canonical InputEvent directly
    line_state
        .apply_event_and_render(&input_event, term, self_safe_history)
        .into()
}

/// Converts [`crossterm::event::KeyCode`] to canonical [`Key`].
///
/// [`Key`]: crate::Key
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

/// Converts [`crossterm::event::KeyModifiers`] to canonical [`ModifierKeysMask`].
///
/// [`ModifierKeysMask`]: crate::ModifierKeysMask
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

/// Converts [`crossterm::event::MouseButton`] to canonical [`Button`].
///
/// [`Button`]: crate::Button
#[must_use]
fn convert_mouse_button(button: MouseButton) -> Button {
    match button {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        MouseButton::Middle => Button::Middle,
    }
}

/// Converts [`crossterm::event::Event`] to canonical [`InputEvent`].
///
/// [`InputEvent`]: crate::InputEvent
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
mod tests {
    use super::*;
    use crate::{CrosstermEventResult, History, InlineVec,
                core::test_fixtures::StdoutMock, vp_height, vp_width};
    use smallvec::smallvec;

    fn get_input_vec() -> InlineVec<CrosstermEventResult> {
        smallvec![
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
            ))),
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('b'),
                KeyModifiers::NONE,
            ))),
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::NONE,
            ))),
            Ok(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
        ]
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_convert_key_code_to_key() {
        assert_eq!(
            convert_key_code_to_key(KeyCode::Char('z')),
            Some(Key::Character('z'))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::F(1)),
            Some(Key::FunctionKey(FunctionKey::F1))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::F(12)),
            Some(Key::FunctionKey(FunctionKey::F12))
        );
        assert_eq!(convert_key_code_to_key(KeyCode::F(13)), None);
        assert_eq!(
            convert_key_code_to_key(KeyCode::Up),
            Some(Key::SpecialKey(SpecialKey::Up))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Down),
            Some(Key::SpecialKey(SpecialKey::Down))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Left),
            Some(Key::SpecialKey(SpecialKey::Left))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Right),
            Some(Key::SpecialKey(SpecialKey::Right))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Home),
            Some(Key::SpecialKey(SpecialKey::Home))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::End),
            Some(Key::SpecialKey(SpecialKey::End))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::PageUp),
            Some(Key::SpecialKey(SpecialKey::PageUp))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::PageDown),
            Some(Key::SpecialKey(SpecialKey::PageDown))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Tab),
            Some(Key::SpecialKey(SpecialKey::Tab))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::BackTab),
            Some(Key::SpecialKey(SpecialKey::BackTab))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Delete),
            Some(Key::SpecialKey(SpecialKey::Delete))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Insert),
            Some(Key::SpecialKey(SpecialKey::Insert))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Enter),
            Some(Key::SpecialKey(SpecialKey::Enter))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Backspace),
            Some(Key::SpecialKey(SpecialKey::Backspace))
        );
        assert_eq!(
            convert_key_code_to_key(KeyCode::Esc),
            Some(Key::SpecialKey(SpecialKey::Esc))
        );
        assert_eq!(convert_key_code_to_key(KeyCode::Null), None);
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_convert_modifier_keys() {
        let none = convert_modifier_keys(KeyModifiers::NONE);
        assert_eq!(none.shift_key_state, KeyState::NotPressed);
        assert_eq!(none.ctrl_key_state, KeyState::NotPressed);
        assert_eq!(none.alt_key_state, KeyState::NotPressed);

        let all = convert_modifier_keys(
            KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_eq!(all.shift_key_state, KeyState::Pressed);
        assert_eq!(all.ctrl_key_state, KeyState::Pressed);
        assert_eq!(all.alt_key_state, KeyState::Pressed);
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_convert_mouse_button() {
        assert_eq!(convert_mouse_button(MouseButton::Left), Button::Left);
        assert_eq!(convert_mouse_button(MouseButton::Right), Button::Right);
        assert_eq!(convert_mouse_button(MouseButton::Middle), Button::Middle);
    }

    #[test]
    #[allow(clippy::needless_return)]
    fn test_convert_crossterm_event_to_input_event() {
        // Plain keyboard event.
        let key_event = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(
            convert_crossterm_event_to_input_event(key_event),
            Some(InputEvent::Keyboard(KeyPress::Plain {
                key: Key::Character('a')
            }))
        );

        // Modified keyboard event.
        let ctrl_c = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(
            convert_crossterm_event_to_input_event(ctrl_c),
            Some(InputEvent::Keyboard(KeyPress::WithModifiers {
                key: Key::Character('c'),
                mask: ModifierKeysMask::new().with_ctrl(),
            }))
        );

        // Mouse event.
        let mouse_event = Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            convert_crossterm_event_to_input_event(mouse_event),
            Some(InputEvent::Mouse(MouseInput {
                pos: vp_col(10) + vp_row(5),
                kind: MouseInputKind::MouseDown(Button::Left),
                maybe_modifier_keys: None,
            }))
        );

        // Resize event.
        let resize_event = Event::Resize(120, 40);
        assert_eq!(
            convert_crossterm_event_to_input_event(resize_event),
            Some(InputEvent::Resize(VPSize {
                col_width: vp_width(120),
                row_height: vp_height(40),
            }))
        );

        // Non-supported event.
        assert_eq!(
            convert_crossterm_event_to_input_event(Event::FocusGained),
            None
        );
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_apply_event_spinner_interception() {
        let mut line_state =
            LineState::new("> ".to_string(), vp_width(100) + vp_height(100));
        let mut stdout_mock = StdoutMock::default();
        let safe_history: SafeHistory = Arc::new(StdMutex::new(History::new()));
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
        let safe_is_spinner_active = Arc::new(StdMutex::new(Some(shutdown_tx)));

        // 1. Normal character 'x' while spinner active does NOT intercept spinner.
        let char_event = InputEvent::Keyboard(KeyPress::Plain {
            key: Key::Character('x'),
        });
        let result = apply_event_to_line_state_and_render(
            char_event,
            &mut line_state,
            &mut stdout_mock,
            &safe_history,
            &safe_is_spinner_active,
        );
        assert!(matches!(result, ReadlineControlFlow::Continue));
        assert!(safe_is_spinner_active.read(Option::is_some));
        assert_eq!(line_state.line.to_string(), "x");

        // 2. Ctrl+C while spinner active DOES intercept spinner and cancel it.
        let ctrl_c = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('c'),
            mask: ModifierKeysMask::new().with_ctrl(),
        });
        let result = apply_event_to_line_state_and_render(
            ctrl_c,
            &mut line_state,
            &mut stdout_mock,
            &safe_history,
            &safe_is_spinner_active,
        );
        assert!(matches!(result, ReadlineControlFlow::Continue));
        // Spinner was taken and canceled.
        assert!(safe_is_spinner_active.read(Option::is_none));
        assert!(shutdown_rx.recv().await.is_ok());

        // 3. Ctrl+C when spinner is NOT active triggers normal readline break.
        let ctrl_c = InputEvent::Keyboard(KeyPress::WithModifiers {
            key: Key::Character('c'),
            mask: ModifierKeysMask::new().with_ctrl(),
        });
        let result = apply_event_to_line_state_and_render(
            ctrl_c,
            &mut line_state,
            &mut stdout_mock,
            &safe_history,
            &safe_is_spinner_active,
        );
        assert!(matches!(
            result,
            ReadlineControlFlow::ReturnOk(ReadlineEvent::Interrupted)
        ));
    }

    #[tokio::test]
    #[allow(clippy::needless_return)]
    async fn test_generate_event_stream_pinned() {
        use crate::core::test_fixtures::gen_input_stream;
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
