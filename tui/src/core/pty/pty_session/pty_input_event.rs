// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CSI_START, ControlSequence, CursorKeyMode, FunctionKey, Key, KeyPress,
            KeyState, ModifierKeysMask, Size, SpecialKey};

/// Input events that can be sent to an interactive [`PTY`] session.
///
/// These events allow your program to communicate with the child process running in
/// the [`PTY`], from basic text input to terminal control sequences and window resizing.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone)]
pub enum PtyInputEvent {
    /// Send raw bytes to child process's stdin.
    Write(Vec<u8>),

    /// Send text with an automatic newline.
    WriteLine(String),

    /// Send a terminal control sequence (Ctrl-C, Arrow keys, Function keys, etc.).
    /// Takes a [`ControlSequence`] and the current [`CursorKeyMode`].
    SendControl(ControlSequence, CursorKeyMode),

    /// Request a terminal window resize.
    Resize(Size),

    /// Explicit flush without writing new data.
    ///
    /// Forces any previously buffered data to be sent to the child process immediately.
    Flush,

    /// Close the input stream (EOF).
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

    /// Converts to [`CSI`] modifier code.
    ///
    /// | Code   | Modifiers          |
    /// | ------ | ------------------ |
    /// | 1      | none               |
    /// | 2      | shift              |
    /// | 3      | alt                |
    /// | 4      | alt+shift          |
    /// | 5      | ctrl               |
    /// | 6      | ctrl+shift         |
    /// | 7      | ctrl+alt           |
    /// | 8      | ctrl+alt+shift     |
    ///
    /// [`CSI`]: crate::CsiSequence
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

/// Converts plain (unmodified) keys.
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

/// Converts modified keys using algorithmic approach.
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

/// Converts special keys with modifiers using [`CSI`] sequences.
///
/// [`CSI`]: crate::CsiSequence
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

/// Converts function keys with modifiers.
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

/// Generate [`CSI`] sequence: [`ESC`][key;modifier;letter or [`ESC`][key;modifier~
///
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
fn generate_csi_sequence(key_code: u32, modifier: u8, suffix: &str) -> PtyInputEvent {
    if modifier == 1 {
        // No modifier - use simple sequence.
        let seq = if suffix == "~" {
            format!("{CSI_START}{key_code}~")
        } else {
            format!("{CSI_START}{suffix}")
        };
        PtyInputEvent::SendControl(
            ControlSequence::RawSequence(seq.into_bytes()),
            CursorKeyMode::default(),
        )
    } else {
        // With modifier
        let seq = if suffix == "~" {
            format!("{CSI_START}{key_code};{modifier}~")
        } else {
            format!("{CSI_START}1;{modifier}{suffix}")
        };
        PtyInputEvent::SendControl(
            ControlSequence::RawSequence(seq.into_bytes()),
            CursorKeyMode::default(),
        )
    }
}

/// Generate [`CSI`] u sequence: [`ESC`][unicode;modifier;u
///
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
fn generate_csi_u_sequence(unicode: u32, modifier: u8) -> PtyInputEvent {
    let seq = if modifier == 1 {
        format!("{CSI_START}{unicode}u")
    } else {
        format!("{CSI_START}{unicode};{modifier}u")
    };
    PtyInputEvent::SendControl(
        ControlSequence::RawSequence(seq.into_bytes()),
        CursorKeyMode::default(),
    )
}

#[cfg(test)]
mod tests {
    use crate::{CursorKeyMode, Key, KeyPress, KeyState, ModifierKeysMask, PtyInputEvent};

    #[test]
    fn test_plain_char_conversion() {
        let key = KeyPress::Plain {
            key: Key::Character('a'),
        };
        let event: Option<PtyInputEvent> = key.into();
        if let Some(PtyInputEvent::Write(bytes)) = event {
            assert_eq!(bytes, b"a");
        } else {
            panic!("Expected Write event");
        }
    }

    #[test]
    fn test_ctrl_char_conversion() {
        // Ctrl+C
        let key = KeyPress::WithModifiers {
            key: Key::Character('c'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                ..Default::default()
            },
        };
        let event: Option<PtyInputEvent> = key.into();
        match event {
            Some(PtyInputEvent::SendControl(ctrl, mode)) => {
                assert_eq!(ctrl.to_bytes(mode).as_ref(), &[0x03]);
            }
            _ => panic!("Expected SendControl event"),
        }

        // Ctrl+A
        let key = KeyPress::WithModifiers {
            key: Key::Character('a'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                ..Default::default()
            },
        };
        let event: Option<PtyInputEvent> = key.into();
        match event {
            Some(PtyInputEvent::SendControl(ctrl, mode)) => {
                assert_eq!(ctrl.to_bytes(mode).as_ref(), &[0x01]);
            }
            _ => panic!("Expected SendControl event"),
        }
    }

    #[test]
    fn test_arrow_key_conversion() {
        // Up arrow - Normal mode
        let key = KeyPress::Plain {
            key: Key::SpecialKey(crate::SpecialKey::Up),
        };
        let event: Option<PtyInputEvent> = key.into();
        match event {
            Some(PtyInputEvent::SendControl(ctrl, _)) => {
                assert_eq!(ctrl.to_bytes(CursorKeyMode::Normal).as_ref(), b"\x1b[A");
                assert_eq!(
                    ctrl.to_bytes(CursorKeyMode::Application).as_ref(),
                    b"\x1bOA"
                );
            }
            _ => panic!("Expected SendControl event"),
        }
    }

    #[test]
    fn test_complex_modifiers() {
        // Ctrl+Alt+A
        let key = KeyPress::WithModifiers {
            key: Key::Character('a'),
            mask: ModifierKeysMask {
                ctrl_key_state: KeyState::Pressed,
                alt_key_state: KeyState::Pressed,
                ..Default::default()
            },
        };
        let event: Option<PtyInputEvent> = key.into();
        match event {
            Some(PtyInputEvent::SendControl(ctrl, mode)) => {
                // Should be ESC + Ctrl+A -> \x1b\x01
                assert_eq!(ctrl.to_bytes(mode).as_ref(), &[0x1b, 0x01]);
            }
            _ => panic!("Expected SendControl event"),
        }
    }

    #[test]
    fn test_function_keys() {
        // F1
        let key = KeyPress::Plain {
            key: Key::FunctionKey(crate::FunctionKey::F1),
        };
        let event: Option<PtyInputEvent> = key.into();
        match event {
            Some(PtyInputEvent::SendControl(ctrl, mode)) => {
                assert_eq!(ctrl.to_bytes(mode).as_ref(), b"\x1bOP");
            }
            _ => panic!("Expected SendControl event"),
        }

        // Shift+F1
        let key = KeyPress::WithModifiers {
            key: Key::FunctionKey(crate::FunctionKey::F1),
            mask: ModifierKeysMask {
                shift_key_state: KeyState::Pressed,
                ..Default::default()
            },
        };
        let event: Option<PtyInputEvent> = key.into();
        match event {
            Some(PtyInputEvent::SendControl(ctrl, mode)) => {
                // Should be CSI 1;2P
                assert_eq!(ctrl.to_bytes(mode).as_ref(), b"\x1b[1;2P");
            }
            _ => panic!("Expected SendControl event"),
        }
    }
}
