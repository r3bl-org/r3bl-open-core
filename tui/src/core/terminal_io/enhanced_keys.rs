// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Crossterm docs:
/// - [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
/// - [`PushKeyboardEnhancementFlags`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum Enhanced {
    /// **Note:** this key can only be read if
    /// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` has been enabled with
    /// `PushKeyboardEnhancementFlags`.
    MediaKey(MediaKey),
    /// **Note:** this key can only be read if
    /// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` has been enabled with
    /// `PushKeyboardEnhancementFlags`.
    SpecialKeyExt(SpecialKeyExt),
    /// **Note:** these keys can only be read if **both**
    /// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` and
    /// `KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES` have been enabled
    /// with `PushKeyboardEnhancementFlags`.
    ModifierKeyEnum(ModifierKeyEnum),
}

/// Crossterm docs:
/// - [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
/// - [`PushKeyboardEnhancementFlags`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
///
/// **Note:** these keys can only be read if **both**
/// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` and
/// `KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES` have been enabled with
/// `PushKeyboardEnhancementFlags`.
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum ModifierKeyEnum {
    /// Left Shift key.
    LeftShift,
    /// Left Control key.
    LeftControl,
    /// Left Alt key.
    LeftAlt,
    /// Left Super key.
    LeftSuper,
    /// Left Hyper key.
    LeftHyper,
    /// Left Meta key.
    LeftMeta,
    /// Right Shift key.
    RightShift,
    /// Right Control key.
    RightControl,
    /// Right Alt key.
    RightAlt,
    /// Right Super key.
    RightSuper,
    /// Right Hyper key.
    RightHyper,
    /// Right Meta key.
    RightMeta,
    /// Iso Level3 Shift key.
    IsoLevel3Shift,
    /// Iso Level5 Shift key.
    IsoLevel5Shift,
}

/// Crossterm docs:
/// - [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
/// - [`PushKeyboardEnhancementFlags`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
///
/// **Note:** this key can only be read if
/// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` has been enabled with
/// `PushKeyboardEnhancementFlags`.
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum SpecialKeyExt {
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    Menu,
    KeypadBegin,
}

/// Crossterm docs:
/// - [`KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
/// - [`PushKeyboardEnhancementFlags`](https://docs.rs/crossterm/0.25.0/crossterm/event/struct.KeyboardEnhancementFlags.html)
///
/// **Note:** this key can only be read if
/// `KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES` has been enabled with
/// `PushKeyboardEnhancementFlags`.
#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum MediaKey {
    Play,
    Pause,
    PlayPause,
    Reverse,
    Stop,
    FastForward,
    Rewind,
    TrackNext,
    TrackPrevious,
    Record,
    LowerVolume,
    RaiseVolume,
    MuteVolume,
}
