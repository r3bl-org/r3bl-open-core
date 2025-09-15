// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input event types and conversion logic for PTY communication.
//!
//! This module defines events that flow FROM the application TO the PTY child process:
//! - [`PtyInputEvent`] - Commands that can be sent to a PTY child process
//! - `KeyPress` to `PtyInputEvent` conversion using algorithmic approach
//! - Terminal control sequence generation for cross-platform compatibility

use portable_pty::PtySize;

use super::pty_output_events::{ControlSequence, CursorKeyMode};
use crate::tui::terminal_lib_backends::{FunctionKey, Key, KeyPress, KeyState,
                                        ModifierKeysMask, SpecialKey};

/// Input event types that can be sent to a child process through PTY.
///
/// # Summary
/// - Bidirectional communication API for sending commands to PTY child processes
/// - Event types: `Write` (raw data), `WriteLine` (text), `SendControl` (key sequences),
///   `Resize`, `Flush`, `Close`
/// - Supports terminal control sequences, window resizing, and process lifecycle
///   management
/// - Used with [`super::pty_sessions::PtyReadWriteSession`] for interactive terminal
///   applications
#[derive(Debug, Clone)]
pub enum PtyInputEvent {
    /// Send raw bytes to child's stdin.
    Write(Vec<u8>),
    /// Send text with automatic newline.
    WriteLine(String),
    /// Send control sequences with cursor mode awareness (Ctrl-C, Ctrl-D, arrow keys,
    /// etc.).
    SendControl(ControlSequence, CursorKeyMode),
    /// Resize the PTY window.
    Resize(PtySize),
    /// Explicit flush without writing data.
    /// Forces any buffered data to be sent to the child immediately.
    Flush,
    /// Close stdin (EOF) - graceful input termination only.
    ///
    /// **Important**: This event only stops the input writer loop and sends EOF to the
    /// child process's stdin. It does **not** kill the child process itself. The child
    /// process may continue running and prevent session termination.
    ///
    /// For forceful process termination, use the `child_process_terminate_handle` from
    /// [`crate::PtyReadWriteSession`] to call `kill()` on the child process directly.
    ///
    /// # Use Cases
    /// - Graceful shutdown: Send `Close` and wait for child to exit naturally
    /// - Forceful shutdown: Call `child_process_terminate_handle.kill()` then send
    ///   `Close`
    ///
    /// # See Also
    /// - [`crate::PtyReadWriteSession::child_process_terminate_handle`] for process
    ///   termination
    Close,
}

/// Clean modifier state representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModifierState {
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl ModifierState {
    fn from_mask(mask: ModifierKeysMask) -> Self {
        Self {
            ctrl: mask.ctrl_key_state == KeyState::Pressed,
            shift: mask.shift_key_state == KeyState::Pressed,
            alt: mask.alt_key_state == KeyState::Pressed,
        }
    }

    /// Convert to CSI modifier code:
    ///
    /// | Code | Modifiers        |
    /// |------|------------------|
    /// | 1    | none             |
    /// | 2    | shift            |
    /// | 3    | alt              |
    /// | 4    | alt+shift        |
    /// | 5    | ctrl             |
    /// | 6    | ctrl+shift       |
    /// | 7    | ctrl+alt         |
    /// | 8    | ctrl+alt+shift   |
    fn to_csi_modifier(self) -> u8 {
        1 + u8::from(self.shift)
            + (if self.alt { 2 } else { 0 })
            + (if self.ctrl { 4 } else { 0 })
    }
}

/// Elegant modifier-based keyboard event converter.
///
/// Instead of 300+ explicit pattern matches, this uses algorithmic generation
/// based on terminal standards and modifier encoding.
impl From<KeyPress> for Option<PtyInputEvent> {
    fn from(key: KeyPress) -> Self {
        match key {
            KeyPress::Plain { key } => convert_plain_key(key),
            KeyPress::WithModifiers { key, mask } => {
                convert_modified_key(key, ModifierState::from_mask(mask))
            }
        }
    }
}

/// Convert plain (unmodified) keys
fn convert_plain_key(key: Key) -> Option<PtyInputEvent> {
    match key {
        Key::Character(ch) => Some(PtyInputEvent::Write(ch.to_string().into_bytes())),

        Key::SpecialKey(special) => convert_special_key(
            special,
            ModifierState {
                ctrl: false,
                shift: false,
                alt: false,
            },
        ),

        Key::FunctionKey(func) => convert_function_key(
            func,
            ModifierState {
                ctrl: false,
                shift: false,
                alt: false,
            },
        ),

        Key::KittyKeyboardProtocol(_) => None,
    }
}

/// Convert modified keys using algorithmic approach
fn convert_modified_key(key: Key, modifiers: ModifierState) -> Option<PtyInputEvent> {
    match key {
        Key::Character(ch) => Some(convert_character_with_modifiers(ch, modifiers)),
        Key::SpecialKey(special) => convert_special_key(special, modifiers),
        Key::FunctionKey(func) => convert_function_key(func, modifiers),
        Key::KittyKeyboardProtocol(_) => None,
    }
}

/// Algorithmically convert characters with modifiers
fn convert_character_with_modifiers(ch: char, modifiers: ModifierState) -> PtyInputEvent {
    match modifiers {
        // Ctrl-only combinations.
        ModifierState {
            ctrl: true,
            shift: false,
            alt: false,
        } => convert_ctrl_character(ch),

        // Alt-only combinations (simple meta sequences)
        ModifierState {
            ctrl: false,
            shift: false,
            alt: true,
        } => {
            // For ASCII characters, use ESC + char.
            if ch.is_ascii() {
                PtyInputEvent::SendControl(
                    ControlSequence::RawSequence(vec![0x1B, ch as u8]),
                    CursorKeyMode::default(),
                )
            } else {
                // For non-ASCII, use CSI u sequences.
                generate_csi_u_sequence(ch as u32, modifiers.to_csi_modifier())
            }
        }

        // Shift-only for space (important for reverse direction in some apps)
        ModifierState {
            ctrl: false,
            shift: true,
            alt: false,
        } if ch == ' ' => {
            // Shift+Space typically sends regular space but some apps detect it via CSI
            // u.
            generate_csi_u_sequence(' ' as u32, modifiers.to_csi_modifier())
        }

        // Alt+Shift combinations.
        ModifierState {
            ctrl: false,
            shift: true,
            alt: true,
        } => {
            // For letters, send ESC + uppercase.
            if ch.is_ascii_alphabetic() {
                PtyInputEvent::SendControl(
                    ControlSequence::RawSequence(vec![
                        0x1B,
                        ch.to_ascii_uppercase() as u8,
                    ]),
                    CursorKeyMode::default(),
                )
            } else if ch.is_ascii() {
                // For other ASCII chars, send ESC + char (shift is handled by the char.
                // itself)
                PtyInputEvent::SendControl(
                    ControlSequence::RawSequence(vec![0x1B, ch as u8]),
                    CursorKeyMode::default(),
                )
            } else {
                generate_csi_u_sequence(ch as u32, modifiers.to_csi_modifier())
            }
        }

        // Ctrl+Alt combinations.
        ModifierState {
            ctrl: true,
            shift: false,
            alt: true,
        } => {
            if let Some(ctrl_code) = get_ctrl_code_extended(ch) {
                PtyInputEvent::SendControl(
                    ControlSequence::RawSequence(vec![0x1B, ctrl_code]),
                    CursorKeyMode::default(),
                )
            } else {
                // Fallback to CSI u for unsupported combinations.
                generate_csi_u_sequence(ch as u32, modifiers.to_csi_modifier())
            }
        }

        // Ctrl+Shift combinations (important for terminal tabs, Ctrl+Shift+T, etc.)
        ModifierState {
            ctrl: true,
            shift: true,
            alt: false,
        } => {
            // Handle uppercase letters specially.
            let target_char = if ch.is_ascii_lowercase() {
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            // Use CSI u sequences for Ctrl+Shift combinations.
            generate_csi_u_sequence(target_char as u32, modifiers.to_csi_modifier())
        }

        // Other combinations default to CSI u sequences.
        _ => generate_csi_u_sequence(ch as u32, modifiers.to_csi_modifier()),
    }
}

/// Convert Ctrl+letter combinations (a-z, A-Z)
fn convert_ctrl_letter(ch: char) -> PtyInputEvent {
    match ch {
        'a' | 'A' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlA, CursorKeyMode::default())
        }
        'c' | 'C' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlC, CursorKeyMode::default())
        }
        'd' | 'D' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlD, CursorKeyMode::default())
        }
        'e' | 'E' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlE, CursorKeyMode::default())
        }
        'k' | 'K' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlK, CursorKeyMode::default())
        }
        'l' | 'L' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlL, CursorKeyMode::default())
        }
        'u' | 'U' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlU, CursorKeyMode::default())
        }
        'z' | 'Z' => {
            PtyInputEvent::SendControl(ControlSequence::CtrlZ, CursorKeyMode::default())
        }
        _ => {
            if let Some(ctrl_code) = get_ctrl_code_extended(ch) {
                PtyInputEvent::SendControl(
                    ControlSequence::RawSequence(vec![ctrl_code]),
                    CursorKeyMode::default(),
                )
            } else {
                generate_csi_u_sequence(ch as u32, 5)
            }
        }
    }
}

/// Convert Ctrl+symbol combinations (space, punctuation, etc.)
fn convert_ctrl_symbol(ch: char) -> PtyInputEvent {
    match ch {
        // Special cases for important symbols - multiple ways to send NUL.
        ' ' | '`' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x00]),
            CursorKeyMode::default(),
        ), // Ctrl+Space/Ctrl+` -> NUL (autocomplete/alternative)
        '[' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x1B]),
            CursorKeyMode::default(),
        ), // Ctrl+[ -> ESC
        '\\' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![28]),
            CursorKeyMode::default(),
        ), // Ctrl+\ -> FS
        ']' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![29]),
            CursorKeyMode::default(),
        ), // Ctrl+] -> GS
        '^' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![30]),
            CursorKeyMode::default(),
        ), // Ctrl+^ -> RS
        '_' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![31]),
            CursorKeyMode::default(),
        ), // Ctrl+_ -> US

        // Additional important symbols.
        '-' => generate_csi_u_sequence('-' as u32, 5), // Ctrl+- (zoom out)
        '=' => generate_csi_u_sequence('=' as u32, 5), // Ctrl+= (zoom in)
        '+' => generate_csi_u_sequence('+' as u32, 5), // Ctrl++ (also zoom in)
        ';' => generate_csi_u_sequence(';' as u32, 5), // Ctrl+; (editor shortcut)
        '\'' => generate_csi_u_sequence('\'' as u32, 5), // Ctrl+' (editor shortcut)
        ',' => generate_csi_u_sequence(',' as u32, 5), // Ctrl+, (settings)
        '.' => generate_csi_u_sequence('.' as u32, 5), // Ctrl+. (context menu)
        '/' => generate_csi_u_sequence('/' as u32, 5), // Ctrl+/ (comment)
        _ => generate_csi_u_sequence(ch as u32, 5),    // Fallback for other symbols
    }
}

/// Convert Ctrl+number combinations (0-9)
fn convert_ctrl_number(ch: char) -> PtyInputEvent {
    match ch {
        '2' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x00]),
            CursorKeyMode::default(),
        ), // Ctrl+2 -> NUL
        '3' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x1B]),
            CursorKeyMode::default(),
        ), // Ctrl+3 -> ESC
        '4' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x1C]),
            CursorKeyMode::default(),
        ), // Ctrl+4 -> FS
        '5' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x1D]),
            CursorKeyMode::default(),
        ), // Ctrl+5 -> GS
        '6' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x1E]),
            CursorKeyMode::default(),
        ), // Ctrl+6 -> RS
        '7' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x1F]),
            CursorKeyMode::default(),
        ), // Ctrl+7 -> US
        '8' => PtyInputEvent::SendControl(
            ControlSequence::RawSequence(vec![0x7F]),
            CursorKeyMode::default(),
        ), // Ctrl+8 -> DEL
        // For 0, 1, 9 use CSI u sequences as they don't have traditional control codes.
        _ => generate_csi_u_sequence(ch as u32, 5),
    }
}

/// Convert Ctrl+character combinations
fn convert_ctrl_character(ch: char) -> PtyInputEvent {
    match ch {
        'a'..='z' | 'A'..='Z' => convert_ctrl_letter(ch),
        '0'..='9' => convert_ctrl_number(ch),
        _ => convert_ctrl_symbol(ch),
    }
}

/// Extended control code getter that handles more cases
fn get_ctrl_code_extended(ch: char) -> Option<u8> {
    match ch {
        // Lowercase letters.
        c @ 'a'..='z' => Some((c as u8) - b'a' + 1),
        // Uppercase letters (same control codes as lowercase)
        c @ 'A'..='Z' => Some((c as u8) - b'A' + 1),
        _ => None,
    }
}

/// Convert special keys with modifiers using CSI sequences
fn convert_special_key(
    special: SpecialKey,
    modifiers: ModifierState,
) -> Option<PtyInputEvent> {
    // Plain special keys (no modifiers)
    if modifiers
        == (ModifierState {
            ctrl: false,
            shift: false,
            alt: false,
        })
    {
        return match special {
            SpecialKey::Enter => Some(PtyInputEvent::SendControl(
                ControlSequence::Enter,
                CursorKeyMode::default(),
            )),
            SpecialKey::Tab => Some(PtyInputEvent::SendControl(
                ControlSequence::Tab,
                CursorKeyMode::default(),
            )),
            SpecialKey::Backspace => Some(PtyInputEvent::SendControl(
                ControlSequence::Backspace,
                CursorKeyMode::default(),
            )),
            SpecialKey::Esc => Some(PtyInputEvent::SendControl(
                ControlSequence::Escape,
                CursorKeyMode::default(),
            )),
            SpecialKey::Delete => Some(PtyInputEvent::SendControl(
                ControlSequence::Delete,
                CursorKeyMode::default(),
            )),
            SpecialKey::Up => Some(PtyInputEvent::SendControl(
                ControlSequence::ArrowUp,
                CursorKeyMode::default(),
            )),
            SpecialKey::Down => Some(PtyInputEvent::SendControl(
                ControlSequence::ArrowDown,
                CursorKeyMode::default(),
            )),
            SpecialKey::Right => Some(PtyInputEvent::SendControl(
                ControlSequence::ArrowRight,
                CursorKeyMode::default(),
            )),
            SpecialKey::Left => Some(PtyInputEvent::SendControl(
                ControlSequence::ArrowLeft,
                CursorKeyMode::default(),
            )),
            SpecialKey::Home => Some(PtyInputEvent::SendControl(
                ControlSequence::Home,
                CursorKeyMode::default(),
            )),
            SpecialKey::End => Some(PtyInputEvent::SendControl(
                ControlSequence::End,
                CursorKeyMode::default(),
            )),
            SpecialKey::PageUp => Some(PtyInputEvent::SendControl(
                ControlSequence::PageUp,
                CursorKeyMode::default(),
            )),
            SpecialKey::PageDown => Some(PtyInputEvent::SendControl(
                ControlSequence::PageDown,
                CursorKeyMode::default(),
            )),
            SpecialKey::Insert => Some(PtyInputEvent::SendControl(
                ControlSequence::RawSequence(vec![0x1B, 0x5B, 0x32, 0x7E]),
                CursorKeyMode::default(),
            )),
            SpecialKey::BackTab => Some(PtyInputEvent::SendControl(
                ControlSequence::RawSequence(vec![0x1B, 0x5B, 0x5A]),
                CursorKeyMode::default(),
            )),
        };
    }

    // Modified special keys - use CSI sequences algorithmically.
    let (base_seq, key_code) = match special {
        SpecialKey::Up => ("A", 'A' as u32),
        SpecialKey::Down => ("B", 'B' as u32),
        SpecialKey::Right => ("C", 'C' as u32),
        SpecialKey::Left => ("D", 'D' as u32),
        SpecialKey::Home => ("H", 'H' as u32),
        SpecialKey::End => ("F", 'F' as u32),
        SpecialKey::Tab => ("I", 'I' as u32),
        SpecialKey::Enter => ("M", 'M' as u32),
        SpecialKey::PageUp => ("~", 5),
        SpecialKey::PageDown => ("~", 6),
        SpecialKey::Insert => ("~", 2),
        SpecialKey::Delete => ("~", 3),
        _ => return None,
    };

    Some(generate_csi_sequence(
        key_code,
        modifiers.to_csi_modifier(),
        base_seq,
    ))
}

/// Convert function keys with modifiers
fn convert_function_key(
    func: FunctionKey,
    modifiers: ModifierState,
) -> Option<PtyInputEvent> {
    let func_num = match func {
        FunctionKey::F1 => 1,
        FunctionKey::F2 => 2,
        FunctionKey::F3 => 3,
        FunctionKey::F4 => 4,
        FunctionKey::F5 => 5,
        FunctionKey::F6 => 6,
        FunctionKey::F7 => 7,
        FunctionKey::F8 => 8,
        FunctionKey::F9 => 9,
        FunctionKey::F10 => 10,
        FunctionKey::F11 => 11,
        FunctionKey::F12 => 12,
    };

    // Plain function keys.
    if modifiers
        == (ModifierState {
            ctrl: false,
            shift: false,
            alt: false,
        })
    {
        return Some(PtyInputEvent::SendControl(
            ControlSequence::F(func_num),
            CursorKeyMode::default(),
        ));
    }

    // Modified function keys - use CSI sequences.
    let (base_seq, key_code) = match func_num {
        1..=4 => {
            // F1-F4 use single letter sequences.
            let letter = match func_num {
                1 => 'P',
                2 => 'Q',
                3 => 'R',
                4 => 'S',
                _ => unreachable!(),
            };
            (letter.to_string(), letter as u32)
        }
        5..=12 => {
            // F5-F12 use numeric sequences.
            let seq_num = match func_num {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => return None,
            };
            ("~".to_string(), seq_num)
        }
        _ => return None,
    };

    Some(generate_csi_sequence(
        key_code,
        modifiers.to_csi_modifier(),
        &base_seq,
    ))
}

/// Generate CSI sequence: ESC[key;modifier;letter or ESC[key;modifier~
fn generate_csi_sequence(key_code: u32, modifier: u8, suffix: &str) -> PtyInputEvent {
    if modifier == 1 {
        // No modifier - use simple sequence.
        let seq = if suffix == "~" {
            format!("\x1B[{key_code}~")
        } else {
            format!("\x1B[{suffix}")
        };
        PtyInputEvent::SendControl(
            ControlSequence::RawSequence(seq.into_bytes()),
            CursorKeyMode::default(),
        )
    } else {
        // With modifier
        let seq = if suffix == "~" {
            format!("\x1B[{key_code};{modifier}~")
        } else {
            format!("\x1B[1;{modifier}{suffix}")
        };
        PtyInputEvent::SendControl(
            ControlSequence::RawSequence(seq.into_bytes()),
            CursorKeyMode::default(),
        )
    }
}

/// Generate CSI u sequence: ESC[unicode;modifier;u
fn generate_csi_u_sequence(unicode: u32, modifier: u8) -> PtyInputEvent {
    let seq = if modifier == 1 {
        format!("\x1B[{unicode}u")
    } else {
        format!("\x1B[{unicode};{modifier}u")
    };
    PtyInputEvent::SendControl(
        ControlSequence::RawSequence(seq.into_bytes()),
        CursorKeyMode::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elegant_ctrl_combinations() {
        // Test Ctrl+A
        let key = KeyPress::WithModifiers {
            key: Key::Character('a'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(matches!(
            event,
            Some(PtyInputEvent::SendControl(ControlSequence::CtrlA, _))
        ));

        // Test Ctrl+Space
        let key = KeyPress::WithModifiers {
            key: Key::Character(' '),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _)) if bytes == &[0x00])
        );
    }

    #[test]
    fn test_elegant_alt_combinations() {
        // Test Alt+X (should be ESC + 'x')
        let key = KeyPress::WithModifiers {
            key: Key::Character('x'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _)) if bytes == &[0x1B, b'x'])
        );
    }

    #[test]
    fn test_modifier_state_conversion() {
        let mask = ModifierKeysMask {
            ctrl_key_state: KeyState::Pressed,
            shift_key_state: KeyState::Pressed,
            alt_key_state: KeyState::NotPressed,
        };
        let state = ModifierState::from_mask(mask);
        assert_eq!(state.to_csi_modifier(), 6); // Ctrl+Shift = 4+1+1 = 6
    }

    #[test]
    fn test_pty_input_debug_and_clone() {
        let input = PtyInputEvent::Write(b"test".to_vec());
        let cloned = input.clone();
        assert_eq!(format!("{input:?}"), format!("{cloned:?}"));
    }

    #[test]
    fn test_keypress_to_pty_input_event() {
        use crate::tui::terminal_lib_backends::{Key, KeyPress, KeyState,
                                                ModifierKeysMask, SpecialKey};

        // Test regular character.
        let key = KeyPress::Plain {
            key: Key::Character('a'),
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(matches!(event, Some(PtyInputEvent::Write(bytes)) if bytes == b"a"));

        // Test special keys.
        let key = KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Enter),
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(matches!(
            event,
            Some(PtyInputEvent::SendControl(ControlSequence::Enter, _))
        ));

        // Test arrow keys
        let key = KeyPress::Plain {
            key: Key::SpecialKey(SpecialKey::Up),
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(matches!(
            event,
            Some(PtyInputEvent::SendControl(ControlSequence::ArrowUp, _))
        ));

        // Test function keys.
        let key = KeyPress::Plain {
            key: Key::FunctionKey(crate::tui::terminal_lib_backends::FunctionKey::F2),
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(matches!(
            event,
            Some(PtyInputEvent::SendControl(ControlSequence::F(2), _))
        ));

        let key = KeyPress::Plain {
            key: Key::FunctionKey(crate::tui::terminal_lib_backends::FunctionKey::F10),
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(matches!(
            event,
            Some(PtyInputEvent::SendControl(ControlSequence::F(10), _))
        ));

        // Test Ctrl+C
        let key = KeyPress::WithModifiers {
            key: Key::Character('c'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(matches!(
            event,
            Some(PtyInputEvent::SendControl(ControlSequence::CtrlC, _))
        ));

        // Test other Ctrl combinations.
        let key = KeyPress::WithModifiers {
            key: Key::Character('x'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(bytes), _)) if bytes == vec![24])
        ); // Ctrl+X = 24
    }

    #[test]
    fn test_comprehensive_ctrl_numbers() {
        // Test all Ctrl+Number combinations (0-9)
        let test_cases = [
            ('0', None, "48;5u"),        // CSI u sequence - '0' = ASCII 48
            ('1', None, "49;5u"),        // CSI u sequence - '1' = ASCII 49
            ('2', Some(vec![0x00]), ""), // Special: NUL
            ('3', Some(vec![0x1B]), ""), // Special: ESC
            ('4', Some(vec![0x1C]), ""), // Special: FS
            ('5', Some(vec![0x1D]), ""), // Special: GS
            ('6', Some(vec![0x1E]), ""), // Special: RS
            ('7', Some(vec![0x1F]), ""), // Special: US
            ('8', Some(vec![0x7F]), ""), // Special: DEL
            ('9', None, "57;5u"),        // CSI u sequence - '9' = ASCII 57
        ];

        for (digit, expected_raw, expected_csi) in test_cases {
            let key = KeyPress::WithModifiers {
                key: Key::Character(digit),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::Pressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::NotPressed,
                },
            };
            let event = Option::<PtyInputEvent>::from(key);

            if let Some(raw_bytes) = expected_raw {
                // Should be raw sequence.
                assert!(
                    matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _)) if bytes == &raw_bytes),
                    "Ctrl+{digit} should produce raw sequence {raw_bytes:?}, got {event:?}"
                );
            } else {
                // Should be CSI u sequence.
                assert!(
                    matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
                    if String::from_utf8_lossy(bytes).contains(expected_csi)),
                    "Ctrl+{digit} should produce CSI u sequence containing '{expected_csi}', got {event:?}"
                );
            }
        }
    }

    #[test]
    fn test_comprehensive_ctrl_symbols() {
        // Test all important Ctrl+Symbol combinations.
        let test_cases = [
            (' ', vec![0x00], ""), // Ctrl+Space -> NUL (autocomplete)
            ('[', vec![0x1B], ""), // Ctrl+[ -> ESC
            ('\\', vec![28], ""),  // Ctrl+\ -> FS
            (']', vec![29], ""),   // Ctrl+] -> GS
            ('^', vec![30], ""),   // Ctrl+^ -> RS
            ('_', vec![31], ""),   // Ctrl+_ -> US
            ('`', vec![0x00], ""), // Ctrl+` -> NUL (alternative)
        ];

        let csi_test_cases = [
            ('-', "45;5u"),  // Ctrl+- (zoom out)
            ('=', "61;5u"),  // Ctrl+= (zoom in)
            ('+', "43;5u"),  // Ctrl++ (also zoom in)
            (';', "59;5u"),  // Ctrl+; (editor shortcut)
            ('\'', "39;5u"), // Ctrl+' (editor shortcut)
            (',', "44;5u"),  // Ctrl+, (settings)
            ('.', "46;5u"),  // Ctrl+. (context menu)
            ('/', "47;5u"),  // Ctrl+/ (comment)
        ];

        // Test raw sequence symbols.
        for (symbol, expected_bytes, _) in test_cases {
            let key = KeyPress::WithModifiers {
                key: Key::Character(symbol),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::Pressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::NotPressed,
                },
            };
            let event = Option::<PtyInputEvent>::from(key);
            assert!(
                matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _)) if bytes == &expected_bytes),
                "Ctrl+{symbol} should produce raw sequence {expected_bytes:?}, got {event:?}"
            );
        }

        // Test CSI u sequence symbols.
        for (symbol, expected_csi) in csi_test_cases {
            let key = KeyPress::WithModifiers {
                key: Key::Character(symbol),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::Pressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::NotPressed,
                },
            };
            let event = Option::<PtyInputEvent>::from(key);
            assert!(
                matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
                if String::from_utf8_lossy(bytes).contains(expected_csi)),
                "Ctrl+{symbol} should produce CSI u sequence containing '{expected_csi}', got {event:?}"
            );
        }
    }

    #[test]
    fn test_comprehensive_alt_combinations() {
        // Test Alt+Numbers.
        for digit in '0'..='9' {
            let key = KeyPress::WithModifiers {
                key: Key::Character(digit),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::NotPressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::Pressed,
                },
            };
            let event = Option::<PtyInputEvent>::from(key);
            let expected = vec![0x1B, digit as u8];
            assert!(
                matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _)) if bytes == &expected),
                "Alt+{digit} should produce ESC+digit {expected:?}, got {event:?}"
            );
        }

        // Test Alt+Letters.
        for letter in ['a', 'z', 'A', 'Z'] {
            let key = KeyPress::WithModifiers {
                key: Key::Character(letter),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::NotPressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::Pressed,
                },
            };
            let event = Option::<PtyInputEvent>::from(key);
            let expected = vec![0x1B, letter as u8];
            assert!(
                matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _)) if bytes == &expected),
                "Alt+{letter} should produce ESC+char {expected:?}, got {event:?}"
            );
        }

        // Test Alt+Symbols.
        for symbol in ['-', '=', '[', ']', ';', '\'', ',', '.', '/'] {
            let key = KeyPress::WithModifiers {
                key: Key::Character(symbol),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::NotPressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::Pressed,
                },
            };
            let event = Option::<PtyInputEvent>::from(key);
            let expected = vec![0x1B, symbol as u8];
            assert!(
                matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _)) if bytes == &expected),
                "Alt+{symbol} should produce ESC+symbol {expected:?}, got {event:?}"
            );
        }
    }

    #[test]
    fn test_multi_modifier_combinations() {
        // Test Ctrl+Shift combinations.
        let key = KeyPress::WithModifiers {
            key: Key::Character('t'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // Should generate CSI u sequence with modifier 6 (Ctrl+Shift = 4+1+1 = 6)
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("84;6u")), // 'T' = 84
            "Ctrl+Shift+T should produce CSI u sequence, got {event:?}"
        );

        // Test Ctrl+Alt combinations.
        let key = KeyPress::WithModifiers {
            key: Key::Character('a'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // Should generate ESC + Ctrl+A.
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if bytes == &[0x1B, 1]), // ESC + Ctrl+A (1)
            "Ctrl+Alt+A should produce ESC+CtrlA, got {event:?}"
        );

        // Test Alt+Shift combinations.
        let key = KeyPress::WithModifiers {
            key: Key::Character('a'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::Pressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // Should generate ESC + uppercase 'A'.
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if bytes == &[0x1B, b'A']),
            "Alt+Shift+A should produce ESC+A, got {event:?}"
        );

        // Test Ctrl+Alt+Shift (all three)
        let key = KeyPress::WithModifiers {
            key: Key::Character('x'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::Pressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // Should generate CSI u sequence with modifier 8 (all modifiers = 4+2+1+1 = 8)
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("120;8u")), /* 'x' = 120,
                                                                    * modifier = 8 */
            "Ctrl+Alt+Shift+x should produce CSI u sequence, got {event:?}"
        );
    }

    #[test]
    fn test_modified_special_keys() {
        // Test Ctrl+Arrow keys.
        let key = KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Up),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // Should generate CSI sequence like ESC[1;5A (modifier 5 = Ctrl)
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("1;5A")),
            "Ctrl+Up should produce CSI sequence, got {event:?}"
        );

        // Test Shift+Tab
        let key = KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Tab),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // Should generate CSI sequence with modifier 2 (Shift)
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("1;2I")),
            "Shift+Tab should produce CSI sequence, got {event:?}"
        );

        // Test Alt+Home
        let key = KeyPress::WithModifiers {
            key: Key::SpecialKey(SpecialKey::Home),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // Should generate CSI sequence with modifier 3 (Alt)
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("1;3H")),
            "Alt+Home should produce CSI sequence, got {event:?}"
        );
    }

    #[test]
    fn test_modified_function_keys() {
        // Test Ctrl+F1
        let key = KeyPress::WithModifiers {
            key: Key::FunctionKey(crate::tui::terminal_lib_backends::FunctionKey::F1),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // F1 with Ctrl should generate ESC[1;5P.
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("1;5P")),
            "Ctrl+F1 should produce CSI sequence, got {event:?}"
        );

        // Test Shift+F5
        let key = KeyPress::WithModifiers {
            key: Key::FunctionKey(crate::tui::terminal_lib_backends::FunctionKey::F5),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // F5 with Shift should generate ESC[15;2~.
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("15;2~")),
            "Shift+F5 should produce CSI sequence, got {event:?}"
        );

        // Test Alt+F12
        let key = KeyPress::WithModifiers {
            key: Key::FunctionKey(crate::tui::terminal_lib_backends::FunctionKey::F12),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::NotPressed,
                alt_key_state: KeyState::Pressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        // F12 with Alt should generate ESC[24;3~.
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("24;3~")),
            "Alt+F12 should produce CSI sequence, got {event:?}"
        );
    }

    #[test]
    fn test_edge_cases_and_csi_validation() {
        // Test shift-only for space.
        let key = KeyPress::WithModifiers {
            key: Key::Character(' '),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes) == "\x1B[32;2u"),
            "Shift+Space should produce CSI u sequence, got {event:?}"
        );

        // Test shift-only for non-space character (should fall through to CSI u)
        let key = KeyPress::WithModifiers {
            key: Key::Character('x'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::NotPressed,
                shift_key_state: KeyState::Pressed,
                alt_key_state: KeyState::NotPressed,
            },
        };
        let event = Option::<PtyInputEvent>::from(key);
        assert!(
            matches!(event, Some(PtyInputEvent::SendControl(ControlSequence::RawSequence(ref bytes), _))
            if String::from_utf8_lossy(bytes).contains("120;2u")), // 'x' = 120
            "Shift+X should produce CSI u sequence, got {event:?}"
        );

        // Test uppercase letters with Ctrl.
        for (lower, upper) in [('a', 'A'), ('z', 'Z')] {
            let key_lower = KeyPress::WithModifiers {
                key: Key::Character(lower),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::Pressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::NotPressed,
                },
            };
            let key_upper = KeyPress::WithModifiers {
                key: Key::Character(upper),
                mask: ModifierKeysMask {
                    ctrl_key_state: KeyState::Pressed,
                    shift_key_state: KeyState::NotPressed,
                    alt_key_state: KeyState::NotPressed,
                },
            };

            let event_lower = Option::<PtyInputEvent>::from(key_lower);
            let event_upper = Option::<PtyInputEvent>::from(key_upper);

            // Both should produce the same result.
            assert_eq!(
                format!("{event_lower:?}"),
                format!("{event_upper:?}"),
                "Ctrl+{lower} and Ctrl+{upper} should produce the same result"
            );
        }

        // Test modifier calculation correctness.
        let state = ModifierState {
            ctrl: true,
            shift: true,
            alt: true,
        };
        assert_eq!(state.to_csi_modifier(), 8); // 1 + 1 + 2 + 4 = 8

        let state = ModifierState {
            ctrl: false,
            shift: true,
            alt: true,
        };
        assert_eq!(state.to_csi_modifier(), 4); // 1 + 1 + 2 = 4

        let state = ModifierState {
            ctrl: true,
            shift: false,
            alt: false,
        };
        assert_eq!(state.to_csi_modifier(), 5); // 1 + 4 = 5
    }
}
