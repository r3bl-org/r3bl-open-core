/*
 *   Copyright (c) 2023 R3BL LLC
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

use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use r3bl_rs_utils_core::*;

use crate::DEVELOPMENT_MODE;

pub trait KeyPressReader {
    fn read_key_press(&mut self) -> KeyPress;
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub enum KeyPress {
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

pub struct CrosstermKeyPressReader {}
impl KeyPressReader for CrosstermKeyPressReader {
    fn read_key_press(&mut self) -> KeyPress {
        read_key_press()
    }
}

fn read_key_press() -> KeyPress {
    if cfg!(windows) {
        // Windows.
        read_key_press_windows()
    } else {
        // Unix.
        read_key_press_unix()
    }
}

fn read_key_press_unix() -> KeyPress {
    let result_event = read();
    match result_event {
        Ok(event) => {
            call_if_true!(DEVELOPMENT_MODE, {
                log_debug(format!("got event: {:?}", event).to_string());
            });

            match event {
                crossterm::event::Event::Resize(width, height) => KeyPress::Resize(Size {
                    col_count: ch!(width),
                    row_count: ch!(height),
                }),
                crossterm::event::Event::Key(KeyEvent {
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('c'),
                    ..
                }) => KeyPress::CtrlC,
                crossterm::event::Event::Key(KeyEvent { code, .. }) => {
                    // Only trap the right code.
                    match code {
                        crossterm::event::KeyCode::Up => KeyPress::Up,
                        crossterm::event::KeyCode::Down => KeyPress::Down,
                        crossterm::event::KeyCode::Enter => KeyPress::Enter,
                        crossterm::event::KeyCode::Esc => KeyPress::Esc,
                        crossterm::event::KeyCode::Char(' ') => KeyPress::Space,
                        _ => KeyPress::Noop,
                    }
                }
                _ => KeyPress::Noop,
            }
        }
        Err(err) => {
            log_error(format!("ERROR getting event: {:?}", err).to_string());
            KeyPress::Error
        }
    }
}

/// [KeyEvent::kind] only set if:
/// - Unix: [`KeyboardEnhancementFlags::REPORT_EVENT_TYPES`] has been enabled with
///   [`PushKeyboardEnhancementFlags`].
/// - Windows: always.
fn read_key_press_windows() -> KeyPress {
    let result_event = read();
    match result_event {
        Ok(event) => {
            call_if_true!(DEVELOPMENT_MODE, {
                log_debug(format!("got event: {:?}", event).to_string());
            });

            match event {
                // Enter.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => KeyPress::Enter,

                // Down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => KeyPress::Down,

                // Up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => KeyPress::Up,

                // Esc.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => KeyPress::Esc,

                // Space.
                Event::Key(KeyEvent {
                    code: KeyCode::Char(' '),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => KeyPress::Space,

                // Ctrl + c.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press, // This is for Windows.
                    state: KeyEventState::NONE,
                }) => KeyPress::CtrlC,

                // Resize.
                Event::Resize(width, height) => KeyPress::Resize(Size {
                    col_count: ch!(width),
                    row_count: ch!(height),
                }),

                // Catchall.
                _ => KeyPress::Noop,
            }
        }
        Err(err) => {
            log_error(format!("ERROR getting event: {:?}", err).to_string());
            KeyPress::Error
        }
    }
}
