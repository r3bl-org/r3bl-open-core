// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! VT-100 Terminal Input Parsing Layer
//!
//! This module provides pure, reusable ANSI sequence parsing for terminal user input.
//! It converts raw bytes (escape sequences, UTF-8 text) into high-level input events.
//!
//! ## Primary Consumer
//!
//! The [`InputDevice`] enum provides a unified input API with multiple backends.
//! [`DirectToAnsiInputDevice`] is the only backend that uses this parser.
//!
//! - [`DirectToAnsiInputDevice`] reads from stdin, and calls the main entry point
//!   function [`try_parse_input_event()`] in this module.
//! - This function checks if the buffer starts with escape sequences (`ESC`, `0x1b`), and
//!   then dispatches to the appropriate parser: keyboard, mouse, terminal events, or UTF-8
//!   text.
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
//!    │ Reads from tokio::io::stdin()
//!    ▼
//! Raw stdin bytes
//!    │
//!    │ Calls try_parse_input_event()
//!    ▼
//! This parser (vt_100_terminal_input_parser)
//!    │
//!    │ Returns VT100InputEventIR
//!    ▼
//! convert_input_event() (protocol_conversion.rs)
//!    │
//!    │ Converts IR → public API
//!    ▼
//! InputEvent (returned to application)
//! ```
//!
//! ## Architecture
//!
//! The VT-100 terminal input parser is a **protocol-agnostic layer** that parses ANSI
//! sequences independently of platform-specific I/O. This design mirrors the output
//! architecture ([`generator`] + [`output`]) and enables:
//!
//! - **Testability**: Unit test parsers without I/O or async complexity
//! - **Reusability**: Multiple backends can use the same protocol parsers
//! - **Clarity**: ANSI protocol handling is centralized in `core/ansi/`
//! - **Separation of Concerns**: Protocol parsing ≠ async I/O ≠ buffering
//!
//! ### Comparison with Output Architecture
//!
//! The input parser is intentionally designed to parallel the output architecture:
//!
//! | Aspect         | Output                                                   | Input                               |
//! | -------------- | -------------------------------------------------------- | ----------------------------------- |
//! | Protocol layer | [`generator`]                                            | (this module)                       |
//! | Backend layer  | [`output`]                                               | [`input`]                           |
//! | Core types     | [`SgrCode`], [`AnsiSequenceGenerator`]                   | [`try_parse_input_event()`], etc.   |
//! | I/O device     | [`RenderOpPaintImplDirectToAnsi`], [`PixelCharRenderer`] | [`DirectToAnsiInputDevice`]         |
//!
//! ## Module Responsibilities
//!
//! ### [`parser`]
//! - Main entry point: [`try_parse_input_event()`]
//! - Route bytes to specialized parsers based on first byte
//! - Handle `ESC` key detection (single `ESC` vs escape sequence start)
//! - Coordinate between keyboard, mouse, terminal events, and UTF-8 parsers
//!
//! ### [`keyboard`]
//! - Parse `CSI` sequences (`ESC [`) for arrow keys, function keys, special keys
//! - Parse `SS3` sequences (`ESC O`) for application mode keys (F1-F4, Home, End, arrows)
//! - Handle modifier combinations (Shift, Ctrl, Alt)
//! - Handle control characters and ambiguous key mappings
//! - Support `Kitty` keyboard protocol for extended functionality
//!
//! ### [`mouse`]
//! - Parse `SGR` mouse protocol (modern standard): `CSI < Cb ; Cx ; Cy M/m`
//! - Parse `X10`/Normal protocol (legacy): `CSI M Cb Cx Cy`
//! - Parse `RXVT` protocol (legacy): `CSI Cb ; Cx ; Cy M`
//! - Detect buttons, clicks, drags, motion, scrolling
//! - Extract modifier keys from mouse sequences
//!
//! ### [`terminal_events`]
//! - Parse window resize events: `CSI 8 ; rows ; cols t`
//! - Parse focus gained/lost: `CSI I` / `CSI O`
//! - Parse bracketed paste markers: `ESC [ 200 ~` / `ESC [ 201 ~`
//!
//! ### [`utf8`]
//! - Parse UTF-8 text between ANSI sequences
//! - Generate character input events for typed text
//! - Handle multi-byte UTF-8 sequences
//! - Buffer incomplete sequences for later completion
//!
//! ## One-Based Mouse Input Events
//!
//! **Confirmed by [`observe_terminal`] test**: VT-100 mouse
//! coordinates are 1-based, where (1, 1) is the top-left corner. Uses [`TermRow`] and
//! [`TermCol`] for type safety and explicit conversion to/from 0-based buffer
//! coordinates.
//!
//! ## Establishing Ground Truth with Validation Tests
//!
//! The [`observe_terminal`] validation test is a critical tool for
//! validating parser accuracy against real terminal emulators.
//!
//! Run it with:
//! ```bash
//! cargo test observe_terminal -- --ignored --nocapture
//! ```
//!
//! This test captures raw bytes from actual terminal interactions and helps establish
//! ground truth for:
//! - **Coordinate systems**: Confirms 1-based (col, row) with (1, 1) at top-left
//! - **`SGR` mouse protocol**: Button codes, scroll wheel interpretation, press/release
//! - **OS platform quirks**: Natural scrolling inversion on GNOME/Ubuntu/macOS
//! - **Terminal-specific behaviors**: Different emulators may have subtle variations
//!
//! Key findings incorporated into this parser:
//! - VT-100 mouse coordinates are **1-based** (not 0-based)
//! - Scroll wheel codes are **inverted on systems with natural scrolling enabled**:
//!   - Check with: `gsettings get org.gnome.desktop.peripherals.mouse natural-scroll`
//! - `SGR` protocol uses codes: 64=Wheel Down, 65=Wheel Up (`XTerm` standard)
//!
//! ## Testing Strategy
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
//! | Level         | Purpose                          | Sequences   | Why                                       |
//! | ------------- | -------------------------------- | ----------- | ----------------------------------------- |
//! | Validation    | Spec compliance & Ground truth   | Hardcoded   | Independent reference (VT-100 protocol)   |
//! | Unit          | Component contracts              | Generated   | Round-trip (generator ↔ parser)           |
//! | Integration   | System behavior                  | Generated   | Real-world usage pattern                  |
//!
//! The [`test_fixtures`] module is shared between the unit, and integration tests only.
//!
//! [`TermCol`]: crate::core::coordinates::vt_100_ansi_coords::TermCol
//! [`TermRow`]: crate::core::coordinates::vt_100_ansi_coords::TermRow
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent`]: crate::InputEvent
//! [`generator`]: mod@crate::core::ansi::generator
//! [`output`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi::output
//! [`input`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi::input
//! [`SgrCode`]: crate::SgrCode
//! [`AnsiSequenceGenerator`]: crate::AnsiSequenceGenerator
//! [`try_parse_input_event()`]: crate::core::ansi::vt_100_terminal_input_parser::parser::try_parse_input_event
//! [`RenderOpPaintImplDirectToAnsi`]: crate::RenderOpPaintImplDirectToAnsi
//! [`PixelCharRenderer`]: crate::PixelCharRenderer
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`keyboard`]: mod@crate::core::ansi::vt_100_terminal_input_parser::keyboard
//! [`mouse`]: mod@crate::core::ansi::vt_100_terminal_input_parser::mouse
//! [`terminal_events`]: mod@crate::core::ansi::vt_100_terminal_input_parser::terminal_events
//! [`utf8`]: mod@crate::core::ansi::vt_100_terminal_input_parser::utf8
//! [`observe_terminal`]: crate::core::ansi::vt_100_terminal_input_parser::validation_tests::observe_real_interactive_terminal_input_events::observe_terminal
//! [`test_fixtures`]: mod@crate::core::ansi::vt_100_terminal_input_parser::test_fixtures
//! [`convert_input_event()`]: crate::tui::terminal_lib_backends::direct_to_ansi::input::protocol_conversion::convert_input_event
//! [`core::ansi`]: crate::core::ansi

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Main entry point module (router/dispatcher)
// This is listed FIRST to emphasize it's the primary API surface
#[cfg(any(test, doc))]
pub mod parser;
#[cfg(not(any(test, doc)))]
mod parser;

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

// Re-export types for flat public API.
// parser is listed FIRST as it's the main entry point
pub use parser::*; // Main entry point: try_parse_input_event()
pub use keyboard::*; // Specialized parsers
pub use mouse::*;
pub use terminal_events::*;
pub use utf8::*;
pub use ir_event_types::*; // Shared types

// Three-tier test architecture.
#[cfg(any(test, doc))]
pub mod validation_tests;
#[cfg(any(test, doc))]
pub mod test_fixtures;
#[cfg(any(test, doc))]
pub mod unit_tests;
#[cfg(any(test, doc))]
pub mod integration_tests;