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
//! For architectural diagrams and details on how streaming byte accumulation, chunk
//! framing, and runaway/circuit-breaker sequence protection are orchestrated before
//! dispatching to this parser, see [`input_byte_stream_to_ir`].
//!
//! ## Primary Consumer
//!
//! The [`InputDevice`] enum provides a unified input API with multiple backends.
//! [`DirectToAnsiInputDevice`] is the only backend that uses this parser.
//!
//! - [`DirectToAnsiInputDevice`] manages a dedicated [`mio`] poller thread reading from
//!   non-blocking [`stdin`], accumulating bytes in [`InputByteStreamToIrParser`]
//!   (in [`input_byte_stream_to_ir`]), and calling the main entry point function
//!   [`try_parse_input_event()`] in this module.
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
//!    │ and accumulates bytes in InputByteStreamToIrParser
//!    ▼
//! Raw stdin bytes
//!    │
//!    │ InputByteStreamToIrParser calls try_parse_input_event() with accumulated byte slice
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
//! In terminal input streams, different inputs often share identical prefix bytes. For
//! example, pressing <kbd>Esc</kbd> emits `1B`, which is identical to the first byte of
//! multi-byte escape sequences (like arrow keys). Traditional terminal programs (such as
//! Vim via [`ttimeoutlen`]) pause and wait 25-100ms on a timer to see if more bytes
//! arrive before deciding what was pressed. To eliminate this latency and keep keystrokes
//! instant (0ms latency), [`try_parse_input_event()`] and its submodules disambiguate
//! collisions through targeted architectural strategies.
//!
//! Here is a brief summary demonstrating these collisions (this is not exhaustive):
//!
//! | Keystroke                   | Colliding Escape Sequence                      | Prefix (Hex) |
//! | :-------------------------- | :--------------------------------------------- | :----------- |
//! | <kbd>Esc</kbd>              | Multi-byte sequences (e.g. Up Arrow `ESC [ A`) | `1B`         |
//! | <kbd>Alt</kbd>+<kbd>]</kbd> | [`OSC` spec] responses (`ESC ] ...`)           | `1B 5D`      |
//! | <kbd>Alt</kbd>+<kbd>[</kbd> | [`CSI` spec] sequences (`ESC [ ...`)           | `1B 5B`      |
//!
//! #### The 3 Primary Collisions Explained
//!
//! 1. **Standalone [`ESC`] Key (`1B` in hex) vs. Multi-Byte Sequences**:
//!    - *Collision*: A physical [`ESC`] keypress emits the single byte `1B` in hex. Every
//!      multi-byte escape sequence (such as Up Arrow `ESC [ A`) also begins with `1B` in
//!      hex.
//!    - *Resolution*: Handled by stream availability heuristics via [`MaybeMore`] in
//!      [`maybe_more`]. When the userspace buffer is not full ([`MaybeMore::KernelDrained`]),
//!      the kernel queue was drained, allowing a lone [`ESC`] to be emitted immediately
//!      with 0ms latency. If full ([`MaybeMore::KernelMayHaveMore`]), the parser waits
//!      for pending bytes.
//!
//! 2. **Standalone `Alt+]` Key (`ESC ]`, `1B 5D` in hex) vs. [`OSC`] Responses**:
//!    - *Collision*: The keystroke `Alt+]` emits `ESC ]`. Operating System Command
//!      ([`OSC`]) responses written by the terminal (such as color queries `ESC ] 11 ;
//!      rgb:... BEL`) also begin with `ESC ]`.
//!    - *Resolution*: Handled in [`terminal_events`] via
//!      [`try_disambiguate_osc_or_alt_bracket()`]. Terminal [`OSC`] responses strictly
//!      conform to [`OSC` spec] (command digits followed by `;` or `?`). Non-digits
//!      immediately identify human input (`Alt+]`). If command digits arrive but the
//!      stream is [`MaybeMore::KernelDrained`] before the delimiter, `Alt+]` is emitted. Valid
//!      [`OSC`] payloads wait across reads for the terminator.
//!
//! 3. **Standalone `Alt+[` Key (`ESC [`, `1B 5B` in hex) vs. [`CSI`] Sequences**:
//!    - *Collision*: In legacy [`VT-100`] / [`xterm`], `Alt+[` emits `ESC [`. This is the
//!      Control Sequence Introducer ([`CSI` spec]) prefix used by all arrow keys,
//!      function keys, mouse tracking, and bracketed paste.
//!    - *Resolution*: Protocol negotiation in [`keyboard`]. In legacy [`VT-100`], bare
//!      `Alt+[` is indistinguishable from [`CSI`] and cannot be resolved without a timer.
//!      It is resolved by negotiating the [Kitty Keyboard Protocol], which encodes
//!      keystrokes unambiguously (`ESC [ 91 ; 3 u`).
//!
//! ## Terminal Input Capability Matrix: Legacy [`VT-100`] vs. [`Kitty`] Keyboard Protocol
//!
//! | Keystroke / Protocol Event                            | Legacy [`VT-100`] / [`xterm`] (Default)       | [`Kitty`] Keyboard Protocol (`CSI u`)      | Technical Reason & Ambiguity                                      |
//! | :---------------------------------------------------- | :-------------------------------------------- | :----------------------------------------- | :---------------------------------------------------------------- |
//! | **Standard Characters (`a-z`, `0-9`, [`UTF-8`])**     | ✅ Supported ([`UTF-8`] bytes)                | ✅ Supported ([`UTF-8`] bytes)             | Unambiguous in both modes.                                        |
//! | **Basic Control Keys (`Ctrl+A` .. `Ctrl+Z`)**         | ✅ Supported (`0x01` .. `0x1A`)               | ✅ Supported                               | Standard [`ASCII`] control characters.                            |
//! | **Enter / Return**                                    | ✅ Supported (`\r`, `0x0D`)                   | ✅ Supported (`\r` or `CSI 13 u`)          | Standard carriage return.                                         |
//! | **`Shift + Enter`**                                   | ❌ **Collides with Enter** (`\r`)             | ✅ **Supported** (`ESC [ 13 ; 2 u`)        | Legacy terminals send identical `0x0D` for both.                  |
//! | **Tab**                                               | ✅ Supported (`\t`, `0x09`)                   | ✅ Supported (`\t` or `CSI 9 u`)           | Standard horizontal tab.                                          |
//! | **`Shift + Tab` (`BackTab`)**                         | ✅ Supported (`ESC [ Z`)                      | ✅ Supported (`ESC [ 9 ; 2 u`)             | Legacy terminals have standard `CSI Z`.                           |
//! | **`Ctrl + Tab`**                                      | ❌ **Collides with Tab** (`\t`)               | ✅ **Supported** (`ESC [ 9 ; 5 u`)         | Legacy terminals send identical `0x09` for both.                  |
//! | **Distinct `Ctrl+I` vs. `Tab`**                       | ❌ **Indistinguishable** (`0x09`)             | ✅ **Supported** (distinct codepoints)     | [`ASCII`] `Ctrl+I` is literally `0x09` (`Tab`).                   |
//! | **Distinct `Ctrl+M` vs. `Enter`**                     | ❌ **Indistinguishable** (`0x0D`)             | ✅ **Supported** (distinct codepoints)     | [`ASCII`] `Ctrl+M` is literally `0x0D` (`Enter`).                 |
//! | **Standalone `Escape` Key**                           | ✅ Supported (0ms latency)                    | ✅ Supported (`ESC [ 27 u`)                | Legacy uses `MaybeMore::KernelDrained`; [`Kitty`] is unambiguous. |
//! | **Navigation Keys (Arrows, Home, End, PageUp/Dn)**    | ✅ Supported ([`CSI`] / `SS3`)                | ✅ Supported ([`CSI`] / `CSI u`)           | Standard [`xterm`] / VT220 sequences.                             |
//! | **Modified Navigation (`Shift+Home`, `Ctrl+Up`)**     | ✅ Supported (`ESC [ 1 ; <m> <final>`)        | ✅ Supported                               | Standard [`xterm`] parameter encoding.                            |
//! | **Function Keys (`F1` .. `F12`)**                     | ✅ Supported (VT220 `~` / `SS3`)              | ✅ Supported                               | Standard escape encodings.                                        |
//! | **`Alt + Key` (Letters & Digits)**                    | ✅ Supported (`ESC <char>`)                   | ✅ Supported (`ESC [ <codepoint> ; 3 u`)   | Legacy prefixes with `0x1B`.                                      |
//! | **`Alt + ]` ([`OSC`] Prefix Collision)**              | ✅ **Solved in Step 8** (`scan_osc_sequence`) | ✅ **Supported** (`ESC [ 93 ; 3 u`)        | Step 8 rejects non-digits / uses `MaybeMore::KernelDrained`.      |
//! | **`Alt + [` ([`CSI`] Prefix Collision)**              | ❌ **Unresolvable in Legacy**                 | ✅ **Solved in Step 9** (`ESC [ 91 ; 3 u`) | Legacy `Alt+[` is byte-for-byte identical to [`CSI`] (`\x1b[`).   |
//! | **Terminal [`OSC`] Query Replies (Theme, Clipboard)** | ✅ **Solved in Step 8** (Framed & Absorbed)   | ✅ Supported (Framed & Absorbed)           | Step 8 frames with `scan_osc_sequence`, prevents text leakage.    |
//! | **Key Release & Repeat Events**                       | ❌ Unsupported by [`VT-100`]                  | ✅ Supported (via [`Kitty`] Flag 2)        | Legacy terminals only report key press down events.               |
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
//! - Detect, frame, and discard unhandled [`OSC`] responses: `ESC ] ... (BEL | ST)`
//! - Disambiguate lone `Alt+]` from terminal-generated [`OSC`] responses
//! - Provide [`scan_osc_sequence()`], [`try_disambiguate_osc_or_alt_bracket()`], and
//!   [`OscScanResult`]
//!
//! ### [`utf8`]
//! - Parse [`UTF-8`] text between [`ANSI`] sequences
//! - Generate character input events for typed text
//! - Handle multi-byte [`UTF-8`] sequences
//! - Buffer incomplete sequences for later completion
//!
//! ### [`maybe_more`]
//! - Evaluates stream availability heuristics ([`MaybeMore`]) without fixed timer delays
//! - Disambiguates standalone `1B` in hex ([`ESC`] vs. multi-byte escape sequences)
//! - Supplies availability hints for [`terminal_events`] during `Alt+]` disambiguation
//! - See [`MaybeMore`] for packet fragmentation and buffer boundary scenarios
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
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`convert_input_event()`]:
//!     crate::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`core::ansi`]: crate::core::ansi
//! [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
//! [`CSI`]: crate::CsiSequence
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`ESC`]: crate::EscSequence
//! [`generator`]: mod@crate::generator
//! [`Ghostty`]: https://ghostty.org/
//! [`input_byte_stream_to_ir`]: mod@input_byte_stream_to_ir
//! [`input`]: mod@crate::direct_to_ansi::input
//! [`InputByteStreamToIrParser`]: InputByteStreamToIrParser
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent`]: crate::InputEvent
//! [`iTerm2`]: https://iterm2.com/
//! [`keyboard`]: mod@keyboard
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`maybe_more`]: mod@maybe_more
//! [`MaybeMore`]: MaybeMore
//! [`mio`]: mio
//! [`observe_terminal`]:
//!     crate::vt_100_terminal_input_parser::validation_tests::observe_real_interactive_terminal_input_events::observe_terminal
//! [`OSC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`OscScanResult`]: crate::vt_100_terminal_input_parser::terminal_events::OscScanResult
//! [`output`]: mod@crate::direct_to_ansi::output
//! [`OutputDevice`]: crate::OutputDevice
//! [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
//! [`router`]: mod@router
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`Sans-IO`]: https://sans-io.readthedocs.io/
//! [`scan_osc_sequence()`]:
//!     crate::vt_100_terminal_input_parser::terminal_events::scan_osc_sequence
//! [`SGR`]: crate::SgrCode
//! [`SgrCode`]: crate::SgrCode
//! [`stdin`]: std::io::stdin
//! [`TermCol`]: crate::vt_100_ansi_coords::TermCol
//! [`terminal_events`]: mod@terminal_events
//! [`TermRow`]: crate::vt_100_ansi_coords::TermRow
//! [`try_disambiguate_osc_or_alt_bracket()`]:
//!     crate::vt_100_terminal_input_parser::terminal_events::try_disambiguate_osc_or_alt_bracket
//! [`try_parse_input_event()`]:
//!     crate::vt_100_terminal_input_parser::router::try_parse_input_event
//! [`ttimeoutlen`]:
//!     https://vi.stackexchange.com/questions/24925/usage-of-timeoutlen-and-ttimeoutlen
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`WezTerm`]: https://wezfurlong.org/wezterm/
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
//! [`xterm`]: https://en.wikipedia.org/wiki/Xterm
//! [Kitty Keyboard Protocol]: https://sw.kovidgoyal.net/kitty/keyboard-protocol/

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

#[cfg(any(test, doc))]
pub mod input_byte_stream_to_ir;
#[cfg(not(any(test, doc)))]
mod input_byte_stream_to_ir;

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
// Stateful stream accumulator parser.
pub use input_byte_stream_to_ir::*;

// Three-tier test architecture.
#[cfg(any(test, doc))]
pub mod validation_tests;
#[cfg(any(test, doc))]
pub mod unit_tests;
#[cfg(any(test, doc))]
pub mod vt_100_parser_integration_tests;

// cspell:words ttimeoutlen Ghostty