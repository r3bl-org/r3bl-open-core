// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words desynchronization

//! [`VT-100`] Terminal Input Parsing Layer
//!
//! This module provides pure, reusable [`ANSI`] sequence parsing for terminal input. It
//! converts raw bytes (escape sequences, [`UTF-8`] text) into high-level input events,
//! handling both human keystrokes and terminal emulator responses.
//!
//! ## Bidirectional Communication: User Input vs. Terminal Responses
//!
//! In reality, terminal emulators ([`Ghostty`], [`Alacritty`], [`Kitty`], [`WezTerm`],
//! [`iTerm2`], VS Code Terminal, etc.) are bidirectional communication partners.
//!
//! While [`stdin`] is conventionally associated with human keystrokes, terminal emulators
//! synthesize and write control sequences directly into [`stdin`] in response to queries
//! sent by the application.
//!
//! A modern TUI app often needs information about the environment. For example:
//! - "Is the user running a dark theme or a light theme?"
//! - "What is the exact hex RGB of the terminal's default background?"
//! - "What is currently in the system clipboard?" (critical over SSH where X11/Wayland
//!   are not available).
//!
//! Because there is no OS syscall like `get_terminal_background_color()`, the TUI app
//! asks the terminal emulator directly via escape sequences written to `stdout`:
//!
//! ```text
//! ┌──────────────┐                                        ┌──────────────┐
//! │   TUI App    │ ─── stdout: "\x1b]11;?\x07" ─────────► │ Terminal     │
//! │              │     ("What is your background color?") │ Emulator     │
//! │              │                                        └──────┬───────┘
//! │              │                                               │
//! │              │ ◄── stdin:  "\x1b]11;rgb:1e1e/1e1e/1e1e\x07" ─┘
//! └──────────────┘     (Terminal writes its response into STDIN!)
//! ```
//!
//! Common sources of terminal-generated [`stdin`] sequences include:
//! - **Theme & Color Queries**: [`OSC`] 10 (foreground) and [`OSC`] 11 (background).
//! - **System Clipboard**: [`OSC`] 52 clipboard payload delivery over SSH.
//! - **Shell & Prompt Pre-fetches**: Tools like `starship` or `fzf` sending queries whose
//!   responses arrive just as our TUI initializes.
//! - **Terminal Multiplexers**: `tmux` or `zellij` forwarding state notifications.
//!
//! Because these responses arrive on [`stdin`] alongside user keystrokes, the input
//! parser must safely detect, frame, and absorb them without leaking payload bytes into
//! input buffers or falsely misinterpreting `ESC ]` as an `Alt+]` keypress.
//!
//! ## Primary Consumer
//!
//! The [`InputDevice`] enum provides a unified input API with multiple backends.
//! [`DirectToAnsiInputDevice`] is the only backend that uses this parser.
//!
//! - [`DirectToAnsiInputDevice`] manages a dedicated [`mio`] poller thread reading from
//!   non-blocking [`stdin`], accumulating bytes in [`StatefulInputParser`], and calling
//!   the main entry point function [`try_parse_input_event()`] in this module.
//! - This function inspects the accumulated sequence bytes and dispatches to the
//!   appropriate parser: keyboard, mouse, terminal events, or [`UTF-8`] text.
//! - The resulting events are converted to structured [`InputEvent`]s for the application
//!   by [`convert_input_event()`].
//!
//! Here's the data flow from the consumer's perspective:
//!
//! ```text
//! InputDevice (unified API for application)
//!    │
//!    │ InputDevice::DirectToAnsi contains backend (DirectToAnsiInputDevice instance)
//!    ▼
//! DirectToAnsiInputDevice (async I/O layer)
//!    │
//!    │ Dedicated mio-poller thread reads non-blocking stdin
//!    │ and accumulates bytes in StatefulInputParser
//!    ▼
//! Raw stdin bytes
//!    │
//!    │ StatefulInputParser calls try_parse_input_event() with accumulated byte slice
//!    ▼                       ┌──────────────────┐
//! try_parse_input_event() ◄──┤ **YOU ARE HERE** │
//!    │                       └──────────────────┘
//!    │ Code in this parser runs and returns Option<(VT100InputEventIR, ByteOffset)>
//!    ▼
//! convert_input_event() (protocol_conversion.rs)
//!    │
//!    │ Converts IR -> public API
//!    ▼
//! InputEvent (returned to application)
//! ```
//!
//! ## Architecture
//!
//! The [`VT-100`] terminal input parser uses a [`Sans-IO`] design - it parses [`ANSI`]
//! sequences independently of platform-specific I/O. This I/O-agnostic approach mirrors
//! the output architecture ([`generator`] + [`ansi_output`]) and enables:
//!
//! - **Testability**: Unit test parsers without I/O or async complexity
//! - **Reusability**: Multiple backends can use the same protocol parsers
//! - **Clarity**: [`ANSI`] protocol handling is centralized in `core/ansi/`
//! - **Separation of Concerns**: Protocol parsing ≠ async I/O ≠ buffering
//!
//! ### Comparison with Output Architecture
//!
//! The input parser is intentionally designed to parallel the output architecture:
//!
//! | Aspect         | Input                             | Output                       |
//! | :------------- | :-------------------------------- | :--------------------------- |
//! | Protocol layer | (this module)                     | [`generator`]                |
//! | Backend layer  | [`input`]                         | [`ansi_output`]              |
//! | Core API       | [`try_parse_input_event()`], etc. | [`SgrCode`], [`ansi_output`] |
//! | I/O device     | [`DirectToAnsiInputDevice`]       | [`OutputDevice`]             |
//!
//! Note: [`OutputDevice`] is shared across all backends (crossterm, `direct_to_ansi`),
//! unlike [`DirectToAnsiInputDevice`] which is backend-specific. The closest
//! `direct_to_ansi` specific type for output is [`RenderOpPaintImplDirectToAnsi`] which
//! uses the [`OutputDevice`].
//!
//! ### Escape Sequence Disambiguation
//!
//! Because we avoid fixed timer delays (such as Vim's 25-100ms `ttimeoutlen`),
//! disambiguating ambiguous prefixes (such as standalone [`ESC`] vs. `ESC [ A`, and
//! `Alt+]` vs. [`OSC`] responses) is handled through stream availability heuristics and
//! strict grammar scanning. See [`MaybeMore`] for the complete Disambiguation Matrix.
//!
//! ## Module Responsibilities
//!
//! Each submodule contains detailed documentation including supported sequences, edge
//! cases, and implementation notes. Click through to the module for full details.
//!
//! ### [`router`]
//! - Main entry point: [`try_parse_input_event()`]
//! - Route bytes to specialized parsers based on first byte
//! - Handle [`ESC`] key detection (single [`ESC`] vs escape sequence start)
//! - Coordinate between keyboard, mouse, terminal events, and [`UTF-8`] parsers
//!
//! ### [`keyboard`]
//! - Parse [`CSI`] sequences (`ESC [`) for arrow keys, function keys, special keys
//! - Parse `SS3` sequences (`ESC O`) for application mode keys (F1-F4, Home, End, arrows)
//! - Handle modifier combinations (Shift, Ctrl, Alt)
//! - Handle control characters and ambiguous key mappings
//!
//! ### [`mouse`]
//! - Parse [`SGR`] mouse protocol (modern standard): `CSI < Cb ; Cx ; Cy M/m`
//! - Parse [`X10`]/Legacy protocol (legacy): `CSI M Cb Cx Cy`
//! - Parse [`RXVT`] protocol (legacy): `CSI Cb ; Cx ; Cy M`
//! - Detect buttons, clicks, drags, motion, scrolling
//! - Extract modifier keys from mouse sequences
//!
//! ### [`terminal_events`]
//! - Parse window resize events: `CSI 8 ; rows ; cols t`
//! - Parse focus gained/lost: `CSI I` / `CSI O`
//! - Parse bracketed paste markers: `ESC [ 200 ~` / `ESC [ 201 ~`
//!
//! ### [`utf8`]
//! - Parse [`UTF-8`] text between [`ANSI`] sequences
//! - Generate character input events for typed text
//! - Handle multi-byte [`UTF-8`] sequences
//! - Buffer incomplete sequences for later completion
//!
//! ### [`maybe_more`]
//! - Evaluates stream availability heuristics ([`MaybeMore`]) without fixed timer delays
//! - Centralizes disambiguation for ambiguous prefixes:
//!   - `0x1B` ([`ESC`] vs. multi-byte escape sequences)
//!   - `0x1B 0x5D` (`Alt+]` vs. [`OSC`] responses)
//!   - `0x1B 0x5B` (`Alt+[` vs. [`CSI`] sequences)
//! - See [`MaybeMore`] for the complete architectural disambiguation matrix and packet
//!   fragmentation scenarios.
//!
//! ## Establishing Ground Truth Through Validation Testing
//!
//! The [`observe_terminal`] validation test is a critical tool for validating parser
//! accuracy against real terminal emulators.
//!
//! Run it with:
//! ```bash
//! cargo test observe_terminal -- --ignored --nocapture
//! ```
//!
//! ### One-Based Mouse Input Events
//!
//! Key findings from [`observe_terminal`] are incorporated into the [`mouse`] parser:
//! - [`VT-100`] mouse coordinates are 1-based (not 0-based), where (1, 1) is the top-left
//!   corner.
//! - Scroll wheel codes are **inverted on systems with natural scrolling enabled**:
//!   - On Linux with GNOME, check with: `gsettings get
//!     org.gnome.desktop.peripherals.mouse natural-scroll`
//! - [`SGR`] protocol uses codes (`XTerm` standard):
//!   - `64`=Wheel Down
//!   - `65`=Wheel Up
//! - Use [`TermRow`] and [`TermCol`] for type safety and explicit conversion to/from
//!   0-based buffer coordinates.
//!
//! ## Testing Strategy
//!
//! Testing a parser that talks to a generator creates an "oracle problem": if both share
//! the same misunderstanding of the [`VT-100`] protocol, tests pass but the code is
//! wrong.
//!
//! We solve this with two complementary approaches:
//!
//! - **Hardcoded sequences** (validation tests): Written by a human reading the
//!   [`VT-100`] spec, these provide ground truth independent of our generator. They catch
//!   systematic protocol misinterpretations.
//!
//! - **Generated sequences** (unit/integration tests): Created by our [`ansi_output`],
//!   these verify round-trip consistency - what we generate, we can parse. They're
//!   valuable for edge cases and keeping generator/parser synchronized.
//!
//! The [`generator`] module provides sequence builders shared between unit and
//! integration tests only - not validation tests, which maintain independence by using
//! hardcoded values.
//!
//! ```text
//!       ╱╲
//!      ╱  ╲  Integration (generated) - System testing
//!     ╱────╲
//!    ╱      ╲  Unit (generated) - Component testing
//!   ╱────────╲
//!  ╱          ╲  Validation (hardcoded) - Acceptance testing
//! ╱────────────╲
//! ```
//!
//! | Level         | Purpose                          | Sequences   | Catches                              |
//! | :------------ | :------------------------------- | :---------- | :----------------------------------- |
//! | Validation    | Spec compliance & ground truth   | Hardcoded   | Protocol misunderstandings           |
//! | Unit          | Component contracts              | Generated   | Generator/parser desynchronization   |
//! | Integration   | System behavior                  | Generated   | Real-world usage regressions         |
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`ansi_output`]: crate::ansi_output
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`convert_input_event()`]:
//!     crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`core::ansi`]: crate::core::ansi
//! [`CSI`]: crate::CsiSequence
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`ESC`]: crate::EscSequence
//! [`generator`]: mod@crate::generator
//! [`Ghostty`]: https://ghostty.org/
//! [`input`]: mod@crate::direct_to_ansi::input
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent`]: crate::InputEvent
//! [`iTerm2`]: https://iterm2.com/
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`maybe_more`]: mod@maybe_more
//! [`MaybeMore`]: MaybeMore
//! [`mio`]: mio
//! [`observe_terminal`]:
//!     crate::vt_100_terminal_input_parser::validation_tests::observe_real_interactive_terminal_input_events::observe_terminal
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`output`]: mod@crate::direct_to_ansi::output
//! [`OutputDevice`]: crate::OutputDevice
//! [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`Sans-IO`]: https://sans-io.readthedocs.io/
//! [`SGR`]: crate::SgrCode
//! [`SgrCode`]: crate::SgrCode
//! [`StatefulInputParser`]:
//!     crate::terminal_lib_backends::direct_to_ansi::input::stateful_parser::StatefulInputParser
//! [`stdin`]: std::io::stdin
//! [`TermCol`]: crate::vt_100_ansi_coords::TermCol
//! [`TermRow`]: crate::vt_100_ansi_coords::TermRow
//! [`try_parse_input_event()`]:
//!     crate::vt_100_terminal_input_parser::router::try_parse_input_event
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking

// Skip rustfmt for rest of file.
#![rustfmt::skip]

// Main entry point module (router/dispatcher)
// This is listed FIRST to emphasize it's the primary API surface
#[cfg(any(test, doc))]
pub mod router;
#[cfg(not(any(test, doc)))]
mod router;

// Conditionally public modules for documentation and testing.
// In test/doc builds: fully public (for rustdoc and test access)
// In release builds: private (encapsulated implementation details)
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
pub mod ir_event_types;
#[cfg(not(any(test, doc)))]
mod ir_event_types;

#[cfg(any(test, doc))]
pub mod maybe_more;
#[cfg(not(any(test, doc)))]
mod maybe_more;

// Re-export types for flat public API.
// Main entry point: try_parse_input_event().
pub use router::*;
// Specialized parsers.
pub use keyboard::*;
pub use mouse::*;
pub use terminal_events::*;
pub use utf8::*;
// Shared types.
pub use ir_event_types::*;
// Input stream availability heuristic enum.
pub use maybe_more::*;

// Three-tier test architecture.
#[cfg(any(test, doc))]
pub mod validation_tests;
#[cfg(any(test, doc))]
pub mod unit_tests;
#[cfg(any(test, doc))]
pub mod vt_100_parser_integration_tests;

// cspell:words ttimeoutlen Ghostty