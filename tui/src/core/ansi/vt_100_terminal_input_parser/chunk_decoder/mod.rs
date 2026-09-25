// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Stateless sequence decoder and prioritized sequence dispatcher.
//!
//! This module implements Layer 3 (Stateless Sequence Parser & Router) of the
//! [`Sans-IO Parser Architecture`][sans_io_arch]. It exports [`try_decode_input_event`],
//! the core stateless function decoding complete contiguous byte slices (framed by
//! [`chunk_framer`] in Layer 2) into strongly-typed intermediate representation
//! ([`VT100InputEventIR`]) events.
//!
//! For the overarching execution flow and bidirectional communication model, see the
//! [parent module documentation].
//!
//! ## Parser Dispatch Priority Pipeline
//!
//! When bytes are accumulated, [`try_decode_input_event`] dispatches them across
//! specialized parsers in a **predefined priority order**:
//!
//! ### 1. [`CSI`] Sequences (`ESC [`...)
//!
//! When the buffer begins with `ESC [`:
//! 1. **`parse_keyboard_sequence()`** ([`keyboard`]): Arrow keys, function keys, modified
//!    keys with [`CSI`] format (e.g., `ESC [ A` for Up, `ESC [ 1 ; 5 A` for Ctrl+Up, `ESC
//!    [ 1 5 ~` for F5).
//! 2. **`parse_mouse_sequence()`** ([`mouse`]): [`SGR`] mouse protocol for clicks, drags,
//!    scrolling (e.g., `ESC [ < 0 ; 10 ; 20 M` for left click, `ESC [ < 64 ; 10 ; 20 M`
//!    for scroll up).
//! 3. **`terminal_events::csi::parse()`** ([`terminal_events`]): Window resize, focus
//!    gained/lost, paste markers (e.g., `ESC [ 8 ; 24 ; 80 t` for resize, `ESC [ I` for
//!    focus gained).
//!
//! ### 2. SS3 Sequences (`ESC O`...)
//!
//! When the buffer begins with `ESC O`:
//! - **`parse_ss3_sequence()`** ([`keyboard`]): Application mode keys (F1-F4, Home, End,
//!   arrows, e.g., `ESC O P` for F1, `ESC O A` for Up in app mode).
//!
//! ### 3. [`OSC`] Sequences & `Alt+]` Disambiguation (`ESC ]`...)
//!
//! In [`VT-100`] terminals, both the `Alt+]` key combination and terminal-generated
//! [`OSC`] responses begin with `ESC ]` (`\x1b]`).
//!
//! Routing for `ESC ]` delegates directly to
//! [`terminal_events::osc::try_disambiguate_or_alt_bracket()`], which uses a dedicated
//! state machine implementing Rule 1 (the [`MaybeMore`] stream availability heuristic)
//! and Rule 2 (strict [`OSC`] syntax validation) to safely distinguish between user
//! keystrokes and terminal query responses.
//!
//! ### 4. [`ESC`] + Other Byte
//!
//! When the buffer begins with [`ESC`] + (a byte other than `[`, `O`, or `]`):
//! - **`parse_alt_letter()`** ([`keyboard`]): Alt+printable character combinations (e.g.,
//!   `ESC b` for Alt+B, `ESC 3` for Alt+3). If unrecognized, emits standalone [`ESC`] and
//!   leaves the trailing byte in the buffer for the next parse cycle.
//!
//! ### 5. Non-[`ESC`] Sequences (Regular Input)
//!
//! When the first byte is not [`ESC`]:
//! 1. **`parse_control_character()`** ([`keyboard`]): Ctrl+A through Ctrl+Z
//!    (`0x00`-`0x1F`, e.g., `0x01` for Ctrl+A, `0x04` for Ctrl+D).
//!    - **Must be tried before [`UTF-8`]**: Bytes `0x00`-`0x1F` are technically valid
//!      single-byte [`UTF-8`] but represent control characters. Without this priority,
//!      Ctrl+A would be misinterpreted as raw text.
//! 2. **`parse_utf8_text()`** ([`utf8`]): Regular text input and printable multi-byte
//!    characters (e.g., `a`, `ñ`, `日`).
//!
//! [`chunk_framer`]: mod@crate::core::ansi::vt_100_terminal_input_parser::chunk_framer
//! [`CSI`]: crate::CsiSequence
//! [`ESC`]: crate::EscSequence
//! [`keyboard`]: mod@keyboard
//! [`MaybeMore`]: crate::core::ansi::vt_100_terminal_input_parser::MaybeMore
//! [`mouse`]: mod@mouse
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`SGR`]: crate::SgrCode
//! [`terminal_events::osc::try_disambiguate_or_alt_bracket()`]: terminal_events::osc::try_disambiguate_or_alt_bracket
//! [`terminal_events`]: mod@terminal_events
//! [`try_decode_input_event`]: crate::core::ansi::vt_100_terminal_input_parser::chunk_decoder::try_decode_input_event
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`utf8`]: mod@utf8
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
//! [parent module documentation]: mod@crate::core::ansi::vt_100_terminal_input_parser#parser-architecture-and-mental-model
//! [sans_io_arch]: mod@crate::core::ansi::vt_100_terminal_input_parser#parser-architecture-sans-io

// Attach submodules with conditional visibility for documentation and testing.
#[cfg(any(test, doc))]
pub mod keyboard;
#[cfg(not(any(test, doc)))]
mod keyboard;

#[cfg(any(test, doc))]
pub mod mouse;
#[cfg(not(any(test, doc)))]
mod mouse;

#[cfg(any(test, doc))]
pub mod terminal_events;
#[cfg(not(any(test, doc)))]
mod terminal_events;

#[cfg(any(test, doc))]
pub mod utf8;
#[cfg(not(any(test, doc)))]
mod utf8;

#[cfg(any(test, doc))]
pub mod csi_scanner;
#[cfg(not(any(test, doc)))]
mod csi_scanner;

#[cfg(any(test, doc))]
pub mod osc_scanner;
#[cfg(not(any(test, doc)))]
mod osc_scanner;

#[cfg(any(test, doc))]
pub mod chunk_decoder_entry_point;
#[cfg(not(any(test, doc)))]
mod chunk_decoder_entry_point;

// Public re-exports (barrel export pattern).
pub use chunk_decoder_entry_point::*;
#[allow(unused_imports)]
pub use csi_scanner::*;
#[allow(unused_imports)]
pub use keyboard::*;
#[allow(unused_imports)]
pub use mouse::*;
#[allow(unused_imports)]
pub use osc_scanner::*;
#[allow(unused_imports)]
pub use terminal_events::*;
#[allow(unused_imports)]
pub use utf8::*;
