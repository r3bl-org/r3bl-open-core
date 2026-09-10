// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Enhanced, KeyState, ModifierKeysMask};
use crate::{MediaKey, ModifierKeyEnum, SpecialKeyExt, try_convert_key_modifiers};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
                       ModifierKeyCode};
use std::fmt;
use std::str::FromStr;

/// Examples.
///
/// ```
/// use r3bl_tui::*;
///
/// fn make_keypress() {
///   let a = key_press!(@char 'a');
///   let a = KeyPress::Plain {
///     key: Key::Character('a'),
///   };
///
///   let alt_a = key_press!(@char ModifierKeysMask::new().with_alt(), 'a');
///   let alt_a = KeyPress::WithModifiers {
///     key: Key::Character('a'),
///     mask: ModifierKeysMask {
///         alt_key_state: KeyState::Pressed,
///         ..Default::default()
///     },
///   };
///
///   let enter = key_press!(@special SpecialKey::Enter);
///   let enter = KeyPress::Plain {
///     key: Key::SpecialKey(SpecialKey::Enter),
///   };
///
///   let alt_enter = key_press!(@special ModifierKeysMask::new().with_alt(), SpecialKey::Enter);
///   let alt_enter = KeyPress::WithModifiers {
///     key: Key::SpecialKey(SpecialKey::Enter),
///     mask: ModifierKeysMask {
///         alt_key_state: KeyState::Pressed,
///         ..Default::default()
///     }
///   };
/// }
/// ```
#[macro_export]
macro_rules! key_press {
    // @char
    (@char $arg_char : expr) => {
        $crate::KeyPress::Plain {
            key: $crate::Key::Character($arg_char),
        }
    };

    (@char $arg_modifiers : expr, $arg_char : expr) => {
        $crate::KeyPress::WithModifiers {
            mask: $arg_modifiers,
            key: $crate::Key::Character($arg_char),
        }
    };

    // @special
    (@special $arg_special : expr) => {
        $crate::KeyPress::Plain {
            key: $crate::Key::SpecialKey($arg_special),
        }
    };

    (@special $arg_modifiers : expr, $arg_special : expr) => {
        $crate::KeyPress::WithModifiers {
            mask: $arg_modifiers,
            key: $crate::Key::SpecialKey($arg_special),
        }
    };

    // @fn
    (@fn $arg_function : expr) => {
        $crate::KeyPress::Plain {
            key: $crate::Key::FunctionKey($arg_function),
        }
    };

    (@fn $arg_modifiers : expr, $arg_function : expr) => {
        $crate::KeyPress::WithModifiers {
            mask: $arg_modifiers,
            key: $crate::Key::FunctionKey($arg_function),
        }
    };
}

/// This is equivalent to [`crossterm::event::KeyEvent`] except that it is cleaned up
/// semantically and impossible states are removed.
///
/// It enables the TUI framework to use a different backend other than [`crossterm`] in
/// the future. Apps written using this framework use [`KeyPress`] and not
/// [`crossterm::event::KeyEvent`]. See [`convert_key_event`] for more information on the
/// conversion.
///
/// Please use the [`key_press`] macro instead of directly constructing this struct.
///
/// # Architecture: Why Multiple Struct Layers?
///
/// The TUI framework uses a layered architecture to abstract terminal events:
///
/// 1. **[Crossterm Event]** (External dependency)
///    - Raw events from the terminal (keyboard, mouse, resize, etc.)
///    - Includes platform-specific quirks (Windows sends Press/Release, Unix only Press)
///    - API can change between crossterm versions
///    - Contains unnecessary complexity for most use cases
///
/// 2. **[`KeyPress`]** (Clean keyboard abstraction - this struct)
///    - Focuses only on keyboard input
///    - Filters out Release/Repeat events for cross-platform consistency
///    - Simplifies modifier handling (e.g., Shift+X becomes just 'X')
///    - Provides a stable API that won't break if crossterm changes
///    - Makes it easier to support other terminal backends in the future
///
/// 3. **[`InputEvent`]** (Unified input abstraction)
///    - Combines all input types: Keyboard, Mouse, Resize, Focus
///    - Provides a single type for the event loop to handle
///    - Each variant wraps the appropriate cleaned-up type ([`KeyPress`], [`MouseInput`],
///      etc.)
///
/// The conversion flow:
/// ```text
/// crossterm::Event::Key(KeyEvent)
///     → KeyPress (via TryFrom<KeyEvent>)
///     → InputEvent::Keyboard(KeyPress)
/// ```
///
/// This layered approach provides:
/// - **Abstraction**: Hide terminal backend implementation details
/// - **Stability**: Shield app code from crossterm API changes
/// - **Consistency**: Normalize behavior across platforms
/// - **Type safety**: Each layer handles specific concerns
/// - **Extensibility**: Easy to add new backends or event types
///
/// # [`Kitty`] keyboard protocol support limitations
///
/// 1. [`KeyPress`] explicitly matches on [`KeyEventKind::Press`] as of `crossterm
///    0.25.0`. It filters out [`Release`] and [`Repeat`] events on all platforms. This is
///    necessary because:
///    - Windows terminals send both [`Press`] and [`Release`] events for each key press
///    - Most Unix terminals only send [`Press`] events
///    - Terminals with [kitty keyboard protocol] support may send [`Press`], [`Release`],
///      and [`Repeat`] events
///
///    By filtering to only [`Press`] events, we ensure consistent behavior across all
///    platforms.
///
/// 2. Also, the [`KeyEvent`]'s `state` is totally ignored in the conversion to
///    [`KeyPress`]. The [`crossterm::event::KeyEventState`] isn't even considered in the
///    conversion code.
///
/// [`InputEvent`]: crate::InputEvent
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`MouseInput`]: crate::MouseInput
/// [`Press`]: KeyEventKind::Press
/// [`Release`]: KeyEventKind::Release
/// [`Repeat`]: KeyEventKind::Repeat
/// [Crossterm Event]: crossterm::event::Event
/// [kitty keyboard protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum KeyPress {
    Plain { key: Key },
    WithModifiers { key: Key, mask: ModifierKeysMask },
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum Key {
    /// [char] that can be printed to the console. Displayable characters are:
    /// - `a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z`
    /// - `A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z`
    /// - `1, 2, 3, 4, 5, 6, 7, 8, 9, 0`
    /// - `!, @, #, $, %, ^, &, *, (, ), _, +, -, =`
    /// - `[, ], {, }, |, \, ,, ., /, <, >, ?, ~`
    Character(char),
    SpecialKey(SpecialKey),
    FunctionKey(FunctionKey),
    /// See [`crossterm::event::PushKeyboardEnhancementFlags`] for more details on [kitty
    /// keyboard protocol] and the terminals on which this is currently supported:
    /// * [kitty terminal]
    /// * [foot terminal]
    /// * [WezTerm terminal]
    /// * [notcurses library]
    /// * [neovim text editor]
    /// * [kakoune text editor]
    /// * [dte text editor]
    ///
    /// Crossterm docs:
    /// - [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`]
    /// - [`PushKeyboardEnhancementFlags`]
    ///
    /// **Note:** [`MediaKey`] and [`SpecialKey`] can be read if:
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] has been enabled with
    /// [`PushKeyboardEnhancementFlags`].
    ///
    /// **Note:** [`ModifierKeyEnum`] can only be read if **both**
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`] and
    /// [`KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES`] have been enabled
    /// with [`PushKeyboardEnhancementFlags`].
    ///
    /// Here's how you can enable crossterm enhanced mode.
    ///
    /// ```
    /// use std::io::{Write, stdout};
    /// use crossterm::execute;
    /// use crossterm::event::{
    ///     KeyboardEnhancementFlags,
    ///     PushKeyboardEnhancementFlags,
    ///     PopKeyboardEnhancementFlags
    /// };
    ///
    /// let mut stdout = stdout();
    ///
    /// execute!(
    ///     stdout,
    ///     PushKeyboardEnhancementFlags(
    ///         KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
    ///     )
    /// );
    ///
    /// // Your code here.
    ///
    /// execute!(stdout, PopKeyboardEnhancementFlags);
    /// ```
    ///
    /// [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`]:
    ///     https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html
    /// [`KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES`]:
    ///     https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html
    /// [`PushKeyboardEnhancementFlags`]: crossterm::event::PushKeyboardEnhancementFlags
    /// [dte text editor]: https://gitlab.com/craigbarnes/dte/-/issues/138
    /// [foot terminal]: https://codeberg.org/dnkl/foot/issues/319
    /// [kakoune text editor]: https://github.com/mawww/kakoune/issues/4103
    /// [kitty keyboard protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
    /// [kitty terminal]: https://sw.kovidgoyal.net/kitty/
    /// [neovim text editor]: https://github.com/neovim/neovim/pull/18181
    /// [notcurses library]: https://github.com/dankamongmen/notcurses/issues/2131
    /// [WezTerm terminal]: https://wezterm.org/config/lua/config/enable_kitty_keyboard.html
    KittyKeyboardProtocol(Enhanced),
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum FunctionKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

/// Converts a function key into its numeric equivalent (`1`-`12`).
impl From<FunctionKey> for u8 {
    fn from(key: FunctionKey) -> u8 {
        match key {
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
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum SpecialKey {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab, /* Shift + Tab */
    Delete,
    Insert,
    Esc,
}

/// Typecast / convert [`KeyEvent`] to [`KeyPress`].
///
/// There is special handling of displayable characters in this conversion. This occurs if
/// the [`KeyEvent`] is a [`KeyCode::Char`].
///
/// An example is typing "X" by pressing "Shift + X" on the keyboard, which shows up in
/// crossterm as "Shift + X". In this case, the [`KeyModifiers`] `SHIFT` and `NONE` are
/// ignored when converted into a [`KeyPress`]. This means the following:
///
/// ```text
/// ╔════════════════════╦════════════════════════════════════════════════════════════════╗
/// ║ User action        ║ Result                                                         ║
/// ╠════════════════════╬════════════════════════════════════════════════════════════════╣
/// ║ Type "x"           ║ InputEvent::Key(keypress! {@char 'x'})                         ║
/// ╠════════════════════╬════════════════════════════════════════════════════════════════╣
/// ║ Type "X"           ║ InputEvent::Key(keypress! {@char 'X'}) and not                 ║
/// ║ (On keyboard press ║ InputEvent::Key(keypress! {@char ModifierKeysMask::SHIFT, 'X'})║
/// ║ Shift+X)           ║ ie, the "SHIFT" is ignored                                     ║
/// ╠════════════════════╬════════════════════════════════════════════════════════════════╣
/// ║ Type "Shift + x"   ║ same as above                                                  ║
/// ╚════════════════════╩════════════════════════════════════════════════════════════════╝
/// ```
///
/// See [Crossterm KeyCode::Char] for more details.
///
/// [Crossterm KeyCode::Char]: https://docs.rs/crossterm/latest/crossterm/event/enum.KeyCode.html#variant.Char
pub mod convert_key_event {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl TryFrom<KeyEvent> for KeyPress {
        type Error = ();
        /// Converts a [`KeyEvent`] into a [`KeyPress`], filtering out non-[`Press`]
        /// events.
        ///
        /// Only [`Press`] events are processed. [`Release`] and [`Repeat`] events return
        /// [`Err(())`], which is an expected "error" that signals [`InputDevice::next()`]
        /// to skip the event and continue reading the next one.
        ///
        /// This ensures consistent behavior across different terminals:
        ///
        /// - Most Unix terminals only send [`Press`] events.
        /// - Windows terminals send both [`Press`] and [`Release`] events for each key
        ///   press.
        /// - Some terminals with [kitty keyboard protocol] support may send [`Repeat`]
        ///   events.
        ///
        /// [`Err(())`]: Result::Err
        /// [`InputDevice::next()`]: crate::InputDevice::next
        /// [`Press`]: KeyEventKind::Press
        /// [`Release`]: KeyEventKind::Release
        /// [`Repeat`]: KeyEventKind::Repeat
        /// [kitty keyboard protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
        fn try_from(key_event: KeyEvent) -> Result<Self, Self::Error> {
            match key_event {
                    KeyEvent {
                        kind: KeyEventKind::Press,
                        .. /* ignore everything else: code, modifiers, etc */
                    } => {
                        impl_special_handling::process_key_event(key_event)
                    }
                    _ => {
                        // Filter out Release and Repeat events.
                        Err(())
                    }
                }
        }
    }

    pub mod impl_special_handling {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        /// Processes a [`KeyEvent`] known to be a [`Press`] event into a [`KeyPress`].
        ///
        /// # Errors
        ///
        /// Returns `Err(())` when the key event does not map to a recognized [`KeyPress`]
        /// variant.
        ///
        /// [`Press`]: KeyEventKind::Press
        #[allow(clippy::result_unit_err)]
        pub fn process_key_event(key_event: KeyEvent) -> Result<KeyPress, ()> {
            if let KeyEvent {
                    code: KeyCode::Char(character),
                    modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT, // Ignore SHIFT.
                    .. // Ignore `state` and `kind`.
                } = key_event {
                Ok(generate_character_key(character))
            } else {
                let maybe_modifiers_keys_mask = try_convert_key_modifiers(&key_event.modifiers);
                let maybe_key: Option<Key> = copy_code_from_key_event(&key_event);
                if let Some(key) = maybe_key {
                    if let Some(mask) = maybe_modifiers_keys_mask {
                        Ok(generate_non_character_key_with_modifiers(key, mask))
                    } else {
                        Ok(generate_non_character_key_without_modifiers(key))
                    }
                } else {
                    Err(())
                }
            }
        }

        fn generate_character_key(character: char) -> KeyPress {
            key_press! { @char character }
        }

        fn generate_non_character_key_without_modifiers(key: Key) -> KeyPress {
            KeyPress::Plain { key }
        }

        fn generate_non_character_key_with_modifiers(
            key: Key,
            mask: ModifierKeysMask,
        ) -> KeyPress {
            KeyPress::WithModifiers { mask, key }
        }
    }

    /// Macro to insulate this library from changes in crossterm
    /// [`crossterm::event::KeyEvent`] constructor & fields.
    #[macro_export]
    macro_rules! crossterm_keyevent {
        (
            code: $arg_key_code: expr,
            modifiers: $arg_key_modifiers: expr
        ) => {
            crossterm::event::KeyEvent::new($arg_key_code, $arg_key_modifiers)
        };
    }

    fn match_fn_key(fn_key: u8) -> Option<Key> {
        match fn_key {
            1 => Key::FunctionKey(FunctionKey::F1).into(),
            2 => Key::FunctionKey(FunctionKey::F2).into(),
            3 => Key::FunctionKey(FunctionKey::F3).into(),
            4 => Key::FunctionKey(FunctionKey::F4).into(),
            5 => Key::FunctionKey(FunctionKey::F5).into(),
            6 => Key::FunctionKey(FunctionKey::F6).into(),
            7 => Key::FunctionKey(FunctionKey::F7).into(),
            8 => Key::FunctionKey(FunctionKey::F8).into(),
            9 => Key::FunctionKey(FunctionKey::F9).into(),
            10 => Key::FunctionKey(FunctionKey::F10).into(),
            11 => Key::FunctionKey(FunctionKey::F11).into(),
            12 => Key::FunctionKey(FunctionKey::F12).into(),
            _ => None,
        }
    }

    #[must_use]
    pub fn copy_code_from_key_event(key_event: &KeyEvent) -> Option<Key> {
        // Make the code easier to read below using this alias.
        type KC = KeyCode;
        match key_event.code {
            KC::Null => None,
            KC::Backspace => Key::SpecialKey(SpecialKey::Backspace).into(),
            KC::Enter => Key::SpecialKey(SpecialKey::Enter).into(),
            KC::Left => Key::SpecialKey(SpecialKey::Left).into(),
            KC::Right => Key::SpecialKey(SpecialKey::Right).into(),
            KC::Up => Key::SpecialKey(SpecialKey::Up).into(),
            KC::Down => Key::SpecialKey(SpecialKey::Down).into(),
            KC::Home => Key::SpecialKey(SpecialKey::Home).into(),
            KC::End => Key::SpecialKey(SpecialKey::End).into(),
            KC::PageUp => Key::SpecialKey(SpecialKey::PageUp).into(),
            KC::PageDown => Key::SpecialKey(SpecialKey::PageDown).into(),
            KC::Tab => Key::SpecialKey(SpecialKey::Tab).into(),
            KC::BackTab => Key::SpecialKey(SpecialKey::BackTab).into(),
            KC::Delete => Key::SpecialKey(SpecialKey::Delete).into(),
            KC::Insert => Key::SpecialKey(SpecialKey::Insert).into(),
            KC::Esc => Key::SpecialKey(SpecialKey::Esc).into(),
            KC::F(fn_key) => match_fn_key(fn_key),
            KC::Char(character) => Key::Character(character).into(),
            // New "enhanced" keys since crossterm 0.25.0.
            KC::CapsLock => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::CapsLock,
            ))
            .into(),
            KC::ScrollLock => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::ScrollLock,
            ))
            .into(),
            KC::NumLock => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::NumLock,
            ))
            .into(),
            KC::PrintScreen => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::PrintScreen,
            ))
            .into(),
            KC::Pause => {
                Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(SpecialKeyExt::Pause))
                    .into()
            }
            KC::Menu => {
                Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(SpecialKeyExt::Menu))
                    .into()
            }
            KC::KeypadBegin => Key::KittyKeyboardProtocol(Enhanced::SpecialKeyExt(
                SpecialKeyExt::KeypadBegin,
            ))
            .into(),
            KC::Media(media_key) => match_enhanced_media_key(media_key).into(),
            KC::Modifier(modifier_key_code) => {
                match_enhanced_modifier_key_code(modifier_key_code).into()
            }
        }
    }

    fn match_enhanced_media_key(media_key: MediaKeyCode) -> Key {
        // Make the code easier to read below using this alias.
        type KC = MediaKeyCode;
        match media_key {
            KC::Play => Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Play)),
            KC::Pause => Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Pause)),
            KC::Stop => Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Stop)),
            KC::PlayPause => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::PlayPause))
            }
            KC::Reverse => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Reverse))
            }
            KC::FastForward => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::FastForward))
            }
            KC::Rewind => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Rewind))
            }
            KC::TrackNext => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::TrackNext))
            }
            KC::TrackPrevious => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::TrackPrevious))
            }
            KC::Record => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::Record))
            }
            KC::LowerVolume => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::LowerVolume))
            }
            KC::RaiseVolume => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::RaiseVolume))
            }
            KC::MuteVolume => {
                Key::KittyKeyboardProtocol(Enhanced::MediaKey(MediaKey::MuteVolume))
            }
        }
    }

    fn match_enhanced_modifier_key_code(modifier_key_code: ModifierKeyCode) -> Key {
        // Make the code easier to read below using this alias.
        type KC = ModifierKeyCode;
        match modifier_key_code {
            KC::LeftShift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftShift,
            )),
            KC::LeftControl => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftControl,
            )),
            KC::LeftAlt => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftAlt,
            )),
            KC::LeftSuper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftSuper,
            )),
            KC::LeftHyper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftHyper,
            )),
            KC::LeftMeta => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::LeftMeta,
            )),
            KC::RightShift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightShift,
            )),
            KC::RightControl => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightControl,
            )),
            KC::RightAlt => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightAlt,
            )),
            KC::RightSuper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightSuper,
            )),
            KC::RightHyper => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightHyper,
            )),
            KC::RightMeta => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::RightMeta,
            )),
            KC::IsoLevel3Shift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::IsoLevel3Shift,
            )),
            KC::IsoLevel5Shift => Key::KittyKeyboardProtocol(Enhanced::ModifierKeyEnum(
                ModifierKeyEnum::IsoLevel5Shift,
            )),
        }
    }
}

// ---------------------------------------------------------------------------------------
// Human-readable string (de)serialization, e.g. for parsing keybindings from a config
// file. The grammar is `[ctrl+][alt+][shift+]<key>` (modifiers are case-insensitive and
// may appear in any order) where `<key>` is a single character, a named special key
// (`tab`, `esc`, `pageup`, ...), `space`, or a function key `f1`..`f12`.
// ---------------------------------------------------------------------------------------

impl fmt::Display for KeyPress {
    /// Renders a [`KeyPress`] in the canonical `ctrl+alt+shift+<key>` form (lowercase
    /// modifiers, fixed order). The round-trip `s.parse::<KeyPress>()?.to_string()` is
    /// stable for every value this crate delivers from the terminal.
    ///
    /// [`Key::KittyKeyboardProtocol`] values are not representable in the config grammar;
    /// they are rendered lossily (and [`FromStr`] rejects them), which is fine because the
    /// app never writes those into a config file.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (key, mask) = match self {
            KeyPress::Plain { key } => (key, None),
            KeyPress::WithModifiers { key, mask } => (key, Some(mask)),
        };
        if let Some(mask) = mask {
            if mask.ctrl_key_state == KeyState::Pressed {
                f.write_str("ctrl+")?;
            }
            if mask.alt_key_state == KeyState::Pressed {
                f.write_str("alt+")?;
            }
            if mask.shift_key_state == KeyState::Pressed {
                f.write_str("shift+")?;
            }
        }
        match key {
            Key::Character(' ') => f.write_str("space"),
            Key::Character(c) => write!(f, "{c}"),
            Key::SpecialKey(sk) => f.write_str(special_key_name(*sk)),
            Key::FunctionKey(fk) => write!(f, "f{}", u8::from(*fk)),
            Key::KittyKeyboardProtocol(enhanced) => write!(f, "kitty:{enhanced:?}"),
        }
    }
}

fn special_key_name(sk: SpecialKey) -> &'static str {
    match sk {
        SpecialKey::Backspace => "backspace",
        SpecialKey::Enter => "enter",
        SpecialKey::Left => "left",
        SpecialKey::Right => "right",
        SpecialKey::Up => "up",
        SpecialKey::Down => "down",
        SpecialKey::Home => "home",
        SpecialKey::End => "end",
        SpecialKey::PageUp => "pageup",
        SpecialKey::PageDown => "pagedown",
        SpecialKey::Tab => "tab",
        SpecialKey::BackTab => "backtab",
        SpecialKey::Delete => "delete",
        SpecialKey::Insert => "insert",
        SpecialKey::Esc => "esc",
    }
}

impl FromStr for KeyPress {
    type Err = String;

    /// Parses a [`KeyPress`] from the `[ctrl+][alt+][shift+]<key>` grammar.
    ///
    /// The result mirrors exactly what the terminal delivers, so a parsed binding compares
    /// equal to a real key event:
    /// - A character key never carries a shift modifier; `shift` folds into the character
    ///   (uppercasing ASCII letters), matching the crossterm → [`KeyPress`] conversion.
    /// - `shift+tab` and `backtab` both yield [`SpecialKey::BackTab`].
    /// - With no modifiers left, the result is [`KeyPress::Plain`]; otherwise
    ///   [`KeyPress::WithModifiers`] (there is no empty-mask representation).
    ///
    /// # Errors
    ///
    /// Returns a human-readable message for an empty input, a missing key after modifiers,
    /// or an unrecognized key token.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut rest = s.trim();
        if rest.is_empty() {
            return Err("empty key specification".to_string());
        }

        let (mut ctrl, mut alt, mut shift) = (false, false, false);
        loop {
            if let Some(r) = strip_prefix_ci(rest, "ctrl+") {
                ctrl = true;
                rest = r;
            } else if let Some(r) = strip_prefix_ci(rest, "alt+") {
                alt = true;
                rest = r;
            } else if let Some(r) = strip_prefix_ci(rest, "shift+") {
                shift = true;
                rest = r;
            } else {
                break;
            }
        }

        if rest.is_empty() {
            return Err(format!("missing key after modifiers in '{s}'"));
        }

        let mut key = parse_key_token(rest)?;

        // Fold shift to match terminal delivery (see doc comment above).
        if shift {
            match key {
                Key::Character(c) => {
                    if c.is_ascii_alphabetic() {
                        key = Key::Character(c.to_ascii_uppercase());
                    }
                    shift = false;
                }
                Key::SpecialKey(SpecialKey::Tab) => {
                    key = Key::SpecialKey(SpecialKey::BackTab);
                    shift = false;
                }
                _ => {}
            }
        }

        if !(ctrl || alt || shift) {
            Ok(KeyPress::Plain { key })
        } else {
            let mut mask = ModifierKeysMask::new();
            if ctrl {
                mask = mask.with_ctrl();
            }
            if alt {
                mask = mask.with_alt();
            }
            if shift {
                mask = mask.with_shift();
            }
            Ok(KeyPress::WithModifiers { key, mask })
        }
    }
}

/// Case-insensitively strips `prefix` from the start of `s`, returning the remainder.
/// Uses [`str::get`] so a non-ASCII leading character can never cause a byte-boundary
/// panic.
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

/// Parses the key token (the part after any modifiers): a named special key, `space`, a
/// function key `f1`..`f12`, or a single character.
fn parse_key_token(token: &str) -> Result<Key, String> {
    let lower = token.to_ascii_lowercase();

    let special = match lower.as_str() {
        "backspace" => Some(SpecialKey::Backspace),
        "enter" | "return" => Some(SpecialKey::Enter),
        "left" => Some(SpecialKey::Left),
        "right" => Some(SpecialKey::Right),
        "up" => Some(SpecialKey::Up),
        "down" => Some(SpecialKey::Down),
        "home" => Some(SpecialKey::Home),
        "end" => Some(SpecialKey::End),
        "pageup" | "pgup" => Some(SpecialKey::PageUp),
        "pagedown" | "pgdn" => Some(SpecialKey::PageDown),
        "tab" => Some(SpecialKey::Tab),
        "backtab" => Some(SpecialKey::BackTab),
        "delete" | "del" => Some(SpecialKey::Delete),
        "insert" | "ins" => Some(SpecialKey::Insert),
        "esc" | "escape" => Some(SpecialKey::Esc),
        _ => None,
    };
    if let Some(sk) = special {
        return Ok(Key::SpecialKey(sk));
    }

    if lower == "space" {
        return Ok(Key::Character(' '));
    }

    // Function keys f1..f12 (but not a bare "f", which is the character key).
    if let Some(num) = lower.strip_prefix('f')
        && let Ok(n) = num.parse::<u8>()
    {
        let fk = match n {
            1 => Some(FunctionKey::F1),
            2 => Some(FunctionKey::F2),
            3 => Some(FunctionKey::F3),
            4 => Some(FunctionKey::F4),
            5 => Some(FunctionKey::F5),
            6 => Some(FunctionKey::F6),
            7 => Some(FunctionKey::F7),
            8 => Some(FunctionKey::F8),
            9 => Some(FunctionKey::F9),
            10 => Some(FunctionKey::F10),
            11 => Some(FunctionKey::F11),
            12 => Some(FunctionKey::F12),
            _ => None,
        };
        if let Some(fk) = fk {
            return Ok(Key::FunctionKey(fk));
        }
    }

    // Single character (letters, digits, symbols like `+` or `` ` ``).
    let mut chars = token.chars();
    if let Some(c) = chars.next()
        && chars.next().is_none()
    {
        return Ok(Key::Character(c));
    }

    Err(format!("unrecognized key: '{token}'"))
}

#[cfg(test)]
mod parse_display_tests {
    use super::*;

    fn ctrl() -> ModifierKeysMask {
        ModifierKeysMask::new().with_ctrl()
    }
    fn alt() -> ModifierKeysMask {
        ModifierKeysMask::new().with_alt()
    }

    #[test]
    fn parses_plain_characters() {
        assert_eq!("q".parse::<KeyPress>().unwrap(), key_press! { @char 'q' });
        assert_eq!("A".parse::<KeyPress>().unwrap(), key_press! { @char 'A' });
        assert_eq!("`".parse::<KeyPress>().unwrap(), key_press! { @char '`' });
        assert_eq!("+".parse::<KeyPress>().unwrap(), key_press! { @char '+' });
        assert_eq!("space".parse::<KeyPress>().unwrap(), key_press! { @char ' ' });
    }

    #[test]
    fn shift_folds_into_the_character() {
        // Shift+letter is delivered by the terminal as the uppercase char, no modifier.
        assert_eq!("shift+a".parse::<KeyPress>().unwrap(), key_press! { @char 'A' });
    }

    #[test]
    fn parses_modified_characters() {
        assert_eq!(
            "ctrl+a".parse::<KeyPress>().unwrap(),
            key_press! { @char ctrl(), 'a' }
        );
        assert_eq!(
            "alt+`".parse::<KeyPress>().unwrap(),
            key_press! { @char alt(), '`' }
        );
    }

    #[test]
    fn modifier_order_is_irrelevant_and_case_insensitive() {
        let a = "ctrl+alt+x".parse::<KeyPress>().unwrap();
        let b = "ALT+Ctrl+x".parse::<KeyPress>().unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a,
            key_press! { @char ModifierKeysMask::new().with_ctrl().with_alt(), 'x' }
        );
    }

    #[test]
    fn parses_special_keys() {
        assert_eq!(
            "tab".parse::<KeyPress>().unwrap(),
            key_press! { @special SpecialKey::Tab }
        );
        assert_eq!(
            "esc".parse::<KeyPress>().unwrap(),
            key_press! { @special SpecialKey::Esc }
        );
        // Both spellings of Shift+Tab collapse to BackTab.
        let backtab = key_press! { @special SpecialKey::BackTab };
        assert_eq!("backtab".parse::<KeyPress>().unwrap(), backtab);
        assert_eq!("shift+tab".parse::<KeyPress>().unwrap(), backtab);
    }

    #[test]
    fn parses_ctrl_arrows() {
        for (spec, sk) in [
            ("ctrl+up", SpecialKey::Up),
            ("ctrl+down", SpecialKey::Down),
            ("ctrl+left", SpecialKey::Left),
            ("ctrl+right", SpecialKey::Right),
        ] {
            assert_eq!(
                spec.parse::<KeyPress>().unwrap(),
                key_press! { @special ctrl(), sk }
            );
        }
    }

    #[test]
    fn parses_function_keys() {
        assert_eq!(
            "f1".parse::<KeyPress>().unwrap(),
            key_press! { @fn FunctionKey::F1 }
        );
        assert_eq!(
            "f12".parse::<KeyPress>().unwrap(),
            key_press! { @fn FunctionKey::F12 }
        );
        // A bare `f` is the character key, not a function key.
        assert_eq!("f".parse::<KeyPress>().unwrap(), key_press! { @char 'f' });
    }

    #[test]
    fn rejects_invalid_specs() {
        assert!("".parse::<KeyPress>().is_err());
        assert!("   ".parse::<KeyPress>().is_err());
        assert!("ctrl+".parse::<KeyPress>().is_err());
        assert!("nonsense".parse::<KeyPress>().is_err());
        assert!("f13".parse::<KeyPress>().is_err());
    }

    #[test]
    fn displays_in_canonical_form() {
        assert_eq!(key_press! { @char 'q' }.to_string(), "q");
        assert_eq!(key_press! { @char 'A' }.to_string(), "A");
        assert_eq!(key_press! { @char ' ' }.to_string(), "space");
        assert_eq!(key_press! { @char ctrl(), 'a' }.to_string(), "ctrl+a");
        assert_eq!(key_press! { @char alt(), '`' }.to_string(), "alt+`");
        assert_eq!(
            key_press! { @special ctrl(), SpecialKey::Down }.to_string(),
            "ctrl+down"
        );
        assert_eq!(
            key_press! { @special SpecialKey::BackTab }.to_string(),
            "backtab"
        );
        assert_eq!(key_press! { @fn FunctionKey::F1 }.to_string(), "f1");
    }

    #[test]
    fn round_trips_every_representable_value() {
        let cases = [
            key_press! { @char 'q' },
            key_press! { @char 'A' },
            key_press! { @char ' ' },
            key_press! { @char '`' },
            key_press! { @char ctrl(), 'a' },
            key_press! { @char alt(), '`' },
            key_press! { @special SpecialKey::Tab },
            key_press! { @special SpecialKey::BackTab },
            key_press! { @special SpecialKey::Esc },
            key_press! { @special ctrl(), SpecialKey::Up },
            key_press! { @special ctrl(), SpecialKey::Down },
            key_press! { @special ctrl(), SpecialKey::Left },
            key_press! { @special ctrl(), SpecialKey::Right },
            key_press! { @fn FunctionKey::F5 },
        ];
        for kp in cases {
            let rendered = kp.to_string();
            let reparsed = rendered.parse::<KeyPress>().unwrap();
            assert_eq!(kp, reparsed, "round-trip failed for {rendered:?}");
        }
    }
}
