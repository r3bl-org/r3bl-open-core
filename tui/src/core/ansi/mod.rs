// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`] Terminal Abstraction Layer
//!
//! This module provides bidirectional [`ANSI`] sequence handling for terminal emulation:
//!
//! ## Key Subsystems
//!
//! - **Output Parser** ([`VTE`]-based): Parse incoming [`PTY`] output ([`ANSI`] sequences from child
//!   processes) → terminal state updates → [`OffscreenBuffer`] storage (via
//!   [`vt_100_pty_output_parser`] and [`AnsiToOfsBufPerformer`])
//! - **Input Parser** (custom): Parse terminal input (keyboard/mouse) → structured
//!   [`VT100InputEventIR`] → application logic (via [`vt_100_terminal_input_parser`])
//! - **Generator**: Convert application styling → outgoing [`ANSI`] sequences → real terminal
//!   display (via [`RenderOpOutput`], [`SgrCode`], [`CliTextInline`])
//! - **Constants & Color**: Shared [`ANSI`] specifications - color types (`RGB` ↔ ANSI256),
//!   escape sequence definitions, used by all subsystems
//!
//! ## Architecture Overview
//!
//! ```text
//!   PTY Output (child process)       User Input (keyboard/mouse)
//!            │                              │
//!            │                              │
//! ┌──────────▼───────────────┐   ┌──────────▼───────────────────┐
//! │ VTE Output Parser        │   │ Custom Input Parser          │
//! │ vt_100_pty_output_parser │   │ vt_100_terminal_input_parser │
//! └──────────┬───────────────┘   └──────────┬───────────────────┘
//!            │                              │
//!            ▼                              ▼
//!  Terminal State Updates          VT100InputEventIR
//!  (cursor/color/text changes)     (keyboard/mouse/terminal)
//!            │                              │
//!   ┌────────▼─────────┐             ┌──────▼───────┐
//!   │ OffscreenBuffer  │             │ Application  │
//!   │ (emulator state) │             │ Logic        │
//!   └──────────────────┘             └──────┬───────┘
//!                                           │
//!                                    ┌──────▼────────────┐
//!                                    │  Generator        │
//!                                    │  • RenderOpOutput │
//!                                    │  • SgrCode        │
//!                                    │  • CliText        │
//!                                    └──────┬────────────┘
//!                                           │
//!                                           ▼
//!                                    ANSI Sequences
//!                                           │
//!                                           ▼
//!                                    Real Terminal
//!                                    (stdout display)
//!
//! ┌──────────────────────────────────┐
//! │  Constants & Color (ANSI specs)  │ ← Shared by all components
//! │  • Color types (RGB ↔ ANSI256)   │
//! │  • Escape sequence definitions   │
//! └──────────────────────────────────┘
//! ```
//!
//! ## Terminal Input Modes: Raw vs Cooked
//!
//! To understand why this module exists, you need to know how terminals handle input.
//!
//! ### Cooked Mode (Default)
//!
//! This is the **default terminal mode** when you open a shell:
//!
//! ```text
//! You type:        "hello^H^H"  (^H = backspace key)
//!                      ↓
//! OS processes:    character buffering, line editing, special key handling
//!                      ↓
//! Program gets:    "hel" (only after Enter, with backspace processed)
//! ```
//!
//! The OS handles input processing: backspace deletes, Ctrl+C terminates the program,
//! Enter sends the line. The program only receives complete lines.
//!
//! ### Raw Mode (Interactive TUI)
//!
//! Interactive applications (vim, less, this R3BL TUI crate) need
//! **character-by-character input**:
//!
//! ```text
//! You press:       [individual keystroke]
//!                      ↓
//! OS processing:   [NONE - raw bytes sent immediately]
//!                      ↓
//! Program gets:    raw keystroke immediately
//!                  (including escape sequences for arrow keys, Ctrl+C, etc.)
//! ```
//!
//! **Why raw mode?** The program needs to:
//! - Capture every keystroke immediately (no line buffering)
//! - Distinguish between Ctrl+C (user interrupt) vs. Ctrl+C keypress the user wants
//! - Detect special keys (arrows, function keys) sent as **escape sequences**
//! - Control the cursor, colors, and screen layout
//!
//! ### Escape Sequences in Raw Mode
//!
//! When a user presses a special key in raw mode, the terminal sends an **escape
//! sequence**. For example:
//!
//! ```text
//! User presses:    Up arrow
//! Terminal sends:  ESC [ A    (3 bytes: 0x1B 0x5B 0x41)
//! Displayed as:    ^[[A       (when using cat -v to visualize)
//! ```
//!
//! Use `cat -v` to see raw escape sequences:
//!
//! ```text
//! $ cat -v          # cat with visualization of control characters
//! # [user types: "hello" then Up arrow then Left arrow]
//! hello^[[A^[[D
//! # ^[ is the Escape character (ESC, 0x1B)
//! # [A is "cursor up"
//! # [D is "cursor left"
//! ```
//!
//! **Common escape sequences:**
//! - `^[[A` = Up arrow
//! - `^[[B` = Down arrow
//! - `^[[C` = Right arrow
//! - `^[[D` = Left arrow
//! - `^[[3~` = Delete key
//! - `^[OP` = F1 key
//!
//! ## Two Separate Parsers: Why?
//!
//! This module contains **two distinct parsers** that handle different data streams:
//!
//! ### Output Parser: VTE-based ([`vt_100_pty_output_parser`])
//!
//! **What it does**: Parses [`ANSI`] escape sequences sent TO the terminal by child
//! processes (via the [`PTY`] controller).
//!
//! **Architecture**: Uses the [`VTE`] crate - a battle-tested state machine from the
//! [`Alacritty`] terminal emulator project.
//!
//! **Why stateful parsing?** [`PTY`] output is **non-contiguous**. Child processes can write
//! partial sequences that span multiple buffer reads:
//! ```text
//! PTY Read 1: [0x1B, 0x5B, 0x31]        // ESC [ 1
//! PTY Read 2: [0x3B, 0x35, 0x41]        // ; 5 A
//! Complete:   ESC [ 1 ; 5 A (Ctrl+Up Arrow)
//! ```
//!
//! [`VTE`] handles this by maintaining parse state across `advance()` calls, buffering
//! incomplete parameters until the final sequence byte arrives.
//!
//! **Benefits**:
//! - ✅ Robust state machine for split sequences and edge cases
//! - ✅ Battle-tested in production ([`Alacritty`] uses it)
//! - ✅ Proper [`ANSI`]/[`VT-100`] spec compliance
//! - ✅ Low maintenance (bug fixes come from upstream)
//!
//! ### Input Parser: Custom Implementation ([`vt_100_terminal_input_parser`])
//!
//! **What it does**: Parses terminal input events (keyboard, mouse, terminal
//! resize/focus) sent FROM the user TO the application.
//!
//! **Architecture**: Custom Rust implementation using stateless pattern matching.
//!
//! **Why NOT use [`VTE`]?** Terminal input has fundamentally different characteristics:
//!
//! 1. **Complete sequences**: Terminal emulators send input sequences **in a single
//!    burst** in single writes:
//!    ```text
//!    User presses:   Up Arrow
//!    Terminal sends: "ESC [ A" (3 bytes in one syscall)
//!    stdin read():   [0x1B, 0x5B, 0x41] (always complete)
//!    ```
//!
//! 2. **Different event types**: [`VTE`] cannot parse keyboard/mouse events - it's
//!    designed for output sequences only. Input events require custom parsing logic:
//!    - `ESC [ A` = User pressed Up Arrow (not "move cursor up")
//!    - `ESC [ < 0 ; 10 ; 20 M` = Mouse click at (10,20)
//!    - `ESC [ ? 1049 h` = Terminal entered alternate buffer mode
//!
//! 3. **Simpler logic**: Input patterns are predictable - no need for full state
//!    machine overhead
//!
//! **Benefits**:
//! - ✅ Zero-latency [`ESC`] key detection (instant emit when buffer = `[0x1B]`)
//! - ✅ Optimal for complete sequences (no buffering overhead)
//! - ✅ Full control over parsing logic
//! - ✅ Can optimize for specific terminal features ([`SGR`] mouse, [`Kitty`] etc.)
//!
//! **Key insight**: The architectural split ([`VTE`] for output, custom for input) is
//! **not a limitation** - it's the correct design because output and input are
//! fundamentally different problems requiring different solutions.
//!
//! ## Key Types and Public API
//!
//! **Color System:**
//! - `TuiColor` - Terminal color with `RGB` and ANSI256 support
//! - [`RgbValue`], [`AnsiValue`] - Color value types
//!
//! **Text Styling:**
//! - [`SgrCode`] - [`SGR`] (Select Graphic Rendition) styling codes
//! - [`CliTextInline`] - Styled inline text for output
//!
//! **Output Parsing** ([`PTY`] escape sequences):
//! - [`AnsiToOfsBufPerformer`] - [`VTE`] [`Perform`] trait implementation for [`PTY`] parsing
//! - [`CsiSequence`] - [`CSI`] escape sequence types
//!
//! **Input Parsing** (keyboard/mouse events):
//! - `VT100InputEventIR` - Keyboard, mouse, and terminal events (see [`vt_100_terminal_input_parser`])
//! - `VT100KeyCodeIR` - Keyboard event key codes
//!
//! **Terminal I/O:**
//! - Color detection and support queries
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`CliTextInline`]: crate::CliTextInline
//! [`CSI`]: crate::CsiSequence
//! [`ESC`]: crate::EscSequence
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`Perform`]: vte::Perform
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`RenderOpOutput`]: crate::RenderOpOutput
//! [`SGR`]: crate::SgrCode
//! [`SgrCode`]: crate::SgrCode
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VT100InputEventIR`]: crate::vt_100_terminal_input_parser::VT100InputEventIR
//! [`vt_100_pty_output_parser`]: mod@crate::vt_100_pty_output_parser
//! [`vt_100_terminal_input_parser`]: mod@crate::vt_100_terminal_input_parser
//! [`VTE`]: mod@vte

#![rustfmt::skip]

// Public modules.
pub mod constants;

// Private modules.
mod color;
mod detect_color_support;

// XMARK: conditional visibility for docs and test only

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[macro_use]
#[cfg(any(test, doc))]
pub mod generator;
#[macro_use]
#[cfg(not(any(test, doc)))]
mod generator;

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod terminal_raw_mode;
// This module is private in non-test, non-doc builds.
#[cfg(not(any(test, doc)))]
mod terminal_raw_mode;

// XMARK: Example for how to conditionally expose private modules for testing and documentation.

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod vt_100_pty_output_parser;
// This module is private in non-test, non-doc builds.
#[cfg(not(any(test, doc)))]
mod vt_100_pty_output_parser;

// Input parsing module - public for protocol access
pub mod vt_100_terminal_input_parser;

// Re-export flat public API.
pub use color::*;
pub use constants::*;
pub use detect_color_support::*;
pub use generator::*;
pub use vt_100_pty_output_parser::*;
pub use terminal_raw_mode::*;

// Re-export test fixtures for testing purposes only.
#[cfg(test)]
pub use vt_100_pty_output_parser::vt_100_pty_output_conformance_tests;
