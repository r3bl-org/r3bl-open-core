// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! System-level [`PTY`] tests: end-to-end validation in real pseudoterminals.
//!
//! These tests validate the complete input handling stack in a real [`PTY`] environment:
//! - [`DirectToAnsiInputDevice`] async I/O and buffer management
//! - Zero-latency [`ESC`] key detection
//! - Full parsing from raw bytes to [`InputEvent`]
//!
//! All tests use **generated sequences** to verify the system can handle its own output.
//!
//! Run with:
//! ```bash
//! cargo test test_pty -- --nocapture
//! ```
//!
//! # Testing Philosophy
//!
//! **[`PTY`] tests use generated sequences** because:
//! - **Real-world testing**: In production, our generator creates sequences and our
//!   parser consumes them; we test this integration
//! - **System compatibility**: If both generator and parser have matching bugs, this is
//!   still OK; the system works end-to-end
//! - **Scalability**: Easy to test many input combinations without hardcoding hundreds of
//!   test cases
//!
//! **Protocol conformance** is tested separately in [`validation_tests`], which use
//! hardcoded sequences to validate against the [`VT-100`] spec.
//!
//! See the [parent module documentation] for the overall testing strategy.
//!
//! # Architecture and Process Roles
//!
//! **Simulating a Terminal Emulator over [`PTY`]**
//!
//! Rather than spawning an external terminal emulator application (such as [`Kitty`], Ghostty, or
//! [`Alacritty`]), our [`PTY`] integration tests split responsibilities across the OS pseudoterminal
//! boundary:
//!
//! ```text
//! ┌──────────────────────────────────────────┐  OS PTY Device   ┌────────────────────────────────────┐
//! │ PTY Controller / Host Process            │ ───────────────> │ PTY Controlled / System Under Test │
//! │ (Simulated Terminal Emulator)            │                  │ (TUI Application & Input Engine)   │
//! │ Synthesizes CSI u, OSC, mouse, focus,    │                  │ Runs DirectToAnsiInputDevice on    │
//! │ paste, queries over controller PTY fd.   │ <─────────────── │ stdin with Tokio async event loop. │
//! └──────────────────────────────────────────┘    stdout / IPC  └────────────────────────────────────┘
//! ```
//!
//! **Why simulate a terminal emulator in the controller?**
//! - **Hermetic and Headless**: Runs reliably in headless CI environments without requiring
//!   a windowing system (X11/Wayland) or external desktop terminal binaries.
//! - **Deterministic Wire Control**: Directly controls byte streams, chunk boundaries,
//!   packet fragmentation, and interleaving to test edge cases (such as protocol collisions
//!   and runaway payloads).
//! - **Fast Execution**: Eliminates the overhead of terminal GUI rendering, running full
//!   end-to-end integration suites in milliseconds.
//!
//! **Process Roles Breakdown**:
//!
//! 1. **Controller Process (Host Terminal Emulator Simulation)**:
//!    - Connected to the controller side of the OS [`PTY`].
//!    - Simulates the external terminal emulator host (such as [`Kitty`], Ghostty, or [`XTerm`]).
//!    - Synthesizes raw escape sequences down the wire directly into child [`stdin`] (e.g.,
//!      `CSI u` keyboard protocol, [`SGR`] mouse coordinates, bracketed paste framing,
//!      and responses to application queries).
//!    - Can act as a fault injector, sending runaway or malformed payloads to test circuit
//!      breakers.
//!    - Asserts that decoded event reports echoed over child [`stdout`] match expected domain
//!      types ([`InputEvent`]).
//!
//! 2. **Controlled Process (System Under Test / TUI Input Engine)**:
//!    - Runs in a child subprocess connected to the controlled side of the OS [`PTY`].
//!    - Initializes [`DirectToAnsiInputDevice`].
//!    - Behind the scenes, the background [`mio`] polling thread reads child [`stdin`],
//!      feeding raw bytes into [`ChunkFramer`] and the Sans-IO [`vt_100`] parser.
//!    - Runs an asynchronous Tokio event loop consuming `input_device.next().await`.
//!    - Maps intermediate parser IR into strongly typed domain events
//!      ([`InputEvent::Keyboard`], [`InputEvent::Mouse`], [`InputEvent::TerminalColorReport`]).
//!    - Echoes confirmation lines over [`stdout`] for controller verification.
//!
//! [`Alacritty`]: https://alacritty.org/
//! [`ChunkFramer`]: crate::core::ansi::vt_100_terminal_input_parser::ChunkFramer
//! [`DirectToAnsiInputDevice`]: crate::DirectToAnsiInputDevice
//! [`ESC`]: crate::EscSequence
//! [`InputEvent::Keyboard`]: crate::InputEvent::Keyboard
//! [`InputEvent::Mouse`]: crate::InputEvent::Mouse
//! [`InputEvent::TerminalColorReport`]: crate::InputEvent::TerminalColorReport
//! [`InputEvent`]: crate::InputEvent
//! [`Kitty`]: https://sw.kovidgoyal.net/kitty/
//! [`mio`]: mio
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`SGR`]: crate::SgrCode
//! [`stdin`]: std::io::stdin
//! [`stdout`]: std::io::stdout
//! [`validation_tests`]: mod@crate::vt_100_terminal_input_parser::validation_tests
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`vt_100`]: crate::core::ansi::vt_100_terminal_input_parser
//! [`XTerm`]: https://invisible-island.net/xterm/
//! [parent module documentation]: mod@super#testing-strategy

#![rustfmt::skip]

// These PTY tests use DirectToAnsiInputDevice which is Linux-only.
// On macOS/Windows, Crossterm backend is used instead and these tests are skipped.
// Non-Linux platforms (macOS/Windows) exclude these modules across all builds.
#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_bracketed_paste_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_input_device_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_keyboard_modifiers_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_mouse_events_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_terminal_events_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_utf8_text_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_keypad_and_special_keys_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_csi_u_keyboard_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_sigwinch_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_mio_poller_thread_lifecycle_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_mio_poller_thread_reuse_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_mio_poller_subscribe_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_mio_poller_singleton_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_osc_color_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_osc_sender_test;

#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod pty_osc_safety_test;
