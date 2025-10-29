// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! VT-100 Terminal Input Parsing Layer
//!
//! This module provides pure, reusable ANSI sequence parsing for terminal user input.
//! It converts raw bytes (escape sequences, UTF-8 text) into high-level input events.
//!
//! ## Architecture
//!
//! The VT-100 terminal input parser is a **protocol-agnostic layer** that parses ANSI
//! sequences independently of platform-specific I/O. This design mirrors the output
//! architecture (generator + renderer) and enables:
//!
//! - **Testability**: Unit test parsers without I/O or async complexity
//! - **Reusability**: Multiple backends can use the same protocol parsers
//! - **Clarity**: ANSI protocol handling is centralized in `core/ansi/`
//! - **Separation of Concerns**: Protocol parsing ≠ async I/O ≠ buffering
//!
//! ## Layered Architecture
//!
//! ```text
//! Raw Terminal Input (stdin)
//!    │
//! ┌──▼───────────────────────────────────────┐
//! │  DirectToAnsiInputDevice (async I/O)     │  ← tui/src/tui/terminal_lib_backends/
//! │  • Read from tokio::io::stdin()          │     direct_to_ansi/input/
//! │  • Manage buffers (4KB, 150ms timeout)   │
//! │  • Dispatch to protocol parsers          │
//! └──────────────────────────────────────────┘
//!    │ (delegate parsing)
//! ┌──▼───────────────────────────────────────┐
//! │  vt_100_terminal_input_parser (pure)     │  ← tui/src/core/ansi/
//! │  • parse_keyboard_sequence()             │     vt_100_terminal_input_parser/
//! │  • parse_mouse_sequence()                │
//! │  • parse_terminal_event()                │
//! │  • parse_utf8_text()                     │
//! └──────────────────────────────────────────┘
//!    │
//!    ▼
//! InputEvent (keyboard, mouse, resize, focus, paste)
//! ```
//!
//! ## Comparison with Output Architecture
//!
//! The input parser is intentionally designed to parallel the output architecture:
//!
//! | Aspect            | Output                                         | Input                                         |
//! |-------------------|----------------------------------------------- |-----------------------------------------------|
//! | Protocol layer    | `core/ansi/generator/`                         | `core/ansi/vt_100_terminal_input_parser/`     |
//! | Backend layer     | `terminal_lib_backends/direct_to_ansi/output/` | `terminal_lib_backends/direct_to_ansi/input/` |
//! | Styling/Rendering | `SgrCode`, `ansi_sequence_generator`           | `parse_keyboard_sequence`, etc.               |
//! | I/O handling      | `paint_render_op_impl`, `pixel_char_renderer`  | `DirectToAnsiInputDevice`                     |
//!
//! ## Module Responsibilities
//!
//! ### keyboard.rs
//! - Parse CSI sequences for arrow keys, function keys, special keys
//! - Handle modifier combinations (Shift, Ctrl, Alt)
//! - Support Kitty keyboard protocol for extended functionality
//!
//! ### mouse.rs
//! - Parse SGR mouse protocol (modern standard): `CSI < Cb ; Cx ; Cy M/m`
//! - Parse X10/Normal protocol (legacy)
//! - Detect buttons, clicks, drags, motion, scrolling
//! - Extract modifier keys from mouse sequences
//!
//! ### terminal_events.rs
//! - Parse window resize events: `CSI 8 ; rows ; cols t`
//! - Parse focus gained/lost: `CSI I` / `CSI O`
//! - Parse bracketed paste markers: `ESC [ 200 ~` / `ESC [ 201 ~`
//!
//! ### utf8.rs
//! - Parse UTF-8 text between ANSI sequences
//! - Generate character input events for typed text
//! - Handle multi-byte UTF-8 sequences
//! - Buffer incomplete sequences for later completion
//!
//! ## Usage (By Backend I/O Layer)
//!
//! ```ignore
//! // In DirectToAnsiInputDevice::read_event():
//! use crate::core::ansi::vt_100_terminal_input_parser::{
//!     keyboard, mouse, terminal_events, utf8
//! };
//!
//! match self.buffer[0] {
//!     b'\x1b' => {
//!         // Try keyboard/mouse/terminal event parsers in order
//!         if let Some(event) = keyboard::parse_keyboard_sequence(&self.buffer) {
//!             return Some(event);
//!         }
//!         if let Some(event) = mouse::parse_mouse_sequence(&self.buffer) {
//!             return Some(event);
//!         }
//!         if let Some(event) = terminal_events::parse_terminal_event(&self.buffer) {
//!             return Some(event);
//!         }
//!     }
//!     _ => {
//!         // Regular UTF-8 text
//!         let events = utf8::parse_utf8_text(&self.buffer);
//!         return events.into_iter().next();
//!     }
//! }
//! ```
//!
//! # Establishing Ground Truth with Integration Tests
//!
//! The `observe_real_terminal_input_events.rs` integration test is a critical tool for
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
//! - **SGR mouse protocol**: Button codes, scroll wheel interpretation, press/release
//! - **OS platform quirks**: Natural scrolling inversion on GNOME/Ubuntu/macOS
//! - **Terminal-specific behaviors**: Different emulators may have subtle variations
//!
//! Key findings incorporated into this parser:
//! - VT-100 mouse coordinates are **1-based** (not 0-based)
//! - Scroll wheel codes are **inverted on systems with natural scrolling enabled**:
//!   - Check with: `gsettings get org.gnome.desktop.peripherals.mouse natural-scroll`
//! - SGR protocol uses codes: 64=Wheel Down, 65=Wheel Up (XTerm standard)
//!
//! # One based mouse input events
//!
//! **Confirmed by `observe_real_terminal_input_events.rs` test**: VT-100 mouse
//! coordinates are 1-based, where (1, 1) is the top-left corner. Uses [`TermRow`] and
//! [`TermCol`] for type safety and explicit conversion to/from 0-based buffer
//! coordinates.
//!
//! [`TermRow`]: crate::core::coordinates::vt_100_ansi_coords::term_units::TermRow
//! [`TermCol`]: crate::core::coordinates::vt_100_ansi_coords::term_units::TermCol

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

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
pub mod types;
#[cfg(not(any(test, doc)))]
mod types;

// Re-export types for flat public API.
pub use keyboard::*;
pub use mouse::*;
pub use terminal_events::*;
pub use utf8::*;
pub use types::*;

// Integration tests for terminal input parsing.
#[cfg(test)]
mod integration_tests;