/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use crossterm::event::{read,
                       Event,
                       KeyCode,
                       KeyEvent,
                       KeyEventKind,
                       KeyEventState,
                       KeyModifiers};
use miette::IntoDiagnostic;
use r3bl_core::{height, width, InputDevice, Size};

use crate::DEVELOPMENT_MODE;

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ChooseKeyPress {
    Up,
    Down,
    Enter,
    Esc,
    #[default]
    Noop,
    Error,
    Space,
    Resize(Size),
    CtrlC,
}

pub async fn get_key_press_from_input_device(
    input_device: &mut InputDevice,
) -> ChooseKeyPress {
    let result_event = input_device.next().await;
    read_key_press(result_event)
}

pub trait KeyPressReader {
    fn read_key_press(&mut self) -> ChooseKeyPress;
}

pub struct CrosstermKeyPressReader {}
impl KeyPressReader for CrosstermKeyPressReader {
    fn read_key_press(&mut self) -> ChooseKeyPress {
        let result_event = read();
        read_key_press(result_event.into_diagnostic())
    }
}

fn read_key_press(result_event: miette::Result<Event>) -> ChooseKeyPress {
    if cfg!(windows) {
        // Windows.
        read_key_press_windows(result_event)
    } else {
        // Unix.
        read_key_press_unix(result_event)
    }
}

fn read_key_press_unix(result_event: miette::Result<Event>) -> ChooseKeyPress {
    match result_event {
        Ok(event) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "got event",
                    event = ?event,
                );
            });

            match event {
                crossterm::event::Event::Resize(width_u16, height_u16) => {
                    ChooseKeyPress::Resize(width(width_u16) + height(height_u16))
                }
                crossterm::event::Event::Key(KeyEvent {
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('c'),
                    ..
                }) => ChooseKeyPress::CtrlC,
                crossterm::event::Event::Key(KeyEvent { code, .. }) => {
                    // Only trap the right code.
                    match code {
                        crossterm::event::KeyCode::Up => ChooseKeyPress::Up,
                        crossterm::event::KeyCode::Down => ChooseKeyPress::Down,
                        crossterm::event::KeyCode::Enter => ChooseKeyPress::Enter,
                        crossterm::event::KeyCode::Esc => ChooseKeyPress::Esc,
                        crossterm::event::KeyCode::Char(' ') => ChooseKeyPress::Space,
                        _ => ChooseKeyPress::Noop,
                    }
                }
                _ => ChooseKeyPress::Noop,
            }
        }
        Err(err) => {
            // % is Display, ? is Debug.
            tracing::error!(
                message = "ERROR getting event",
                err = ?err,
            );
            ChooseKeyPress::Error
        }
    }
}

/// [KeyEvent::kind] only set if:
/// - Unix: [`KeyboardEnhancementFlags::REPORT_EVENT_TYPES`] has been enabled with
///   [`PushKeyboardEnhancementFlags`].
/// - Windows: always.
fn read_key_press_windows(result_event: miette::Result<Event>) -> ChooseKeyPress {
    match result_event {
        Ok(event) => {
            DEVELOPMENT_MODE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "got event",
                    event = ?event,
                );
            });

            match event {
                // Enter.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => ChooseKeyPress::Enter,

                // Down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => ChooseKeyPress::Down,

                // Up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => ChooseKeyPress::Up,

                // Esc.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => ChooseKeyPress::Esc,

                // Space.
                Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => ChooseKeyPress::Space,

                // Ctrl + c.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => ChooseKeyPress::CtrlC,

                // Resize.
                Event::Resize(width_u16, height_u16) => {
                    ChooseKeyPress::Resize(width(width_u16) + height(height_u16))
                }

                // Catchall.
                _ => ChooseKeyPress::Noop,
            }
        }
        Err(err) => {
            // % is Display, ? is Debug.
            tracing::error!(
                message = "ERROR getting event",
                err = ?err,
            );
            ChooseKeyPress::Error
        }
    }
}
