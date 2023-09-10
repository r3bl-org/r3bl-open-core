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
use std::io::Result;

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub enum KeyPress {
    Up,
    Down,
    Enter,
    Esc,
    #[default]
    Noop,
}

pub fn read_key_press() -> Result<KeyPress> {
    if cfg!(windows) {
        // Windows.
        read_key_press_windows()
    } else {
        // Unix.
        read_key_press_unix()
    }
}

fn read_key_press_unix() -> Result<KeyPress> {
    let event = read()?;
    log_debug(format!("event: {:?}", event).to_string());

    match event {
        crossterm::event::Event::Key(KeyEvent { code, .. }) => {
            // Only trap the right code.
            match code {
                crossterm::event::KeyCode::Up => Ok(KeyPress::Up),
                crossterm::event::KeyCode::Down => Ok(KeyPress::Down),
                crossterm::event::KeyCode::Enter => Ok(KeyPress::Enter),
                crossterm::event::KeyCode::Esc => Ok(KeyPress::Esc),
                _ => Ok(KeyPress::Noop),
            }
        }
        _ => Ok(KeyPress::Noop),
    }
}

/// [KeyEvent::kind] only set if:
/// - Unix: [`KeyboardEnhancementFlags::REPORT_EVENT_TYPES`] has been enabled with
///   [`PushKeyboardEnhancementFlags`].
/// - Windows: always.
fn read_key_press_windows() -> Result<KeyPress> {
    let event = read()?;
    log_debug(format!("event: {:?}", event).to_string());

    match event {
        // Enter.
        Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press, // This is for Windows.
            state: KeyEventState::NONE,
        }) => Ok(KeyPress::Enter),

        // Down.
        Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press, // This is for Windows.
            state: KeyEventState::NONE,
        }) => Ok(KeyPress::Down),

        // Up.
        Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press, // This is for Windows.
            state: KeyEventState::NONE,
        }) => Ok(KeyPress::Up),

        // Esc.
        Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press, // This is for Windows.
            state: KeyEventState::NONE,
        }) => Ok(KeyPress::Esc),

        // Catchall.
        _ => Ok(KeyPress::Noop),
    }
}
