// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Terminal I/O abstractions for input devices, output devices, and events.
//!
//! This module provides a unified interface for terminal input/output operations,
//! abstracting over different backends ([`CrosstermInputDevice`],
//! [`DirectToAnsiInputDevice`]).
//!
//! # Key Types
//!
//! - [`InputDevice`] - Trait for async input event streams
//! - [`InputEvent`] - Unified event type (keyboard, mouse, resize, etc.)
//! - [`KeyPress`] - Keyboard input with key code and modifiers
//! - [`ModifierKeysMask`] - Bitflags for Shift, Ctrl, Alt, etc.
//! - [`MouseInput`] - Mouse events (click, scroll, move)
//!
//! # Testing
//!
//! Backend compatibility tests live in `backend_compat_tests` (test-only module). These
//! PTY-based tests verify that [`DirectToAnsiInputDevice`] and [`CrosstermInputDevice`]
//! produce identical [`InputEvent`] values for the same ANSI byte sequences.
//!
//! Run the tests with:
//!
//! ```bash
//! cargo test -p r3bl_tui --lib test_pty_backend -- --nocapture
//! ```
//!
//! [`CrosstermInputDevice`]: crate::CrosstermInputDevice
//! [`DirectToAnsiInputDevice`]: crate::direct_to_ansi::DirectToAnsiInputDevice
//! [`InputDevice`]: crate::InputDevice
//! [`InputEvent`]: crate::InputEvent
//! [`KeyPress`]: crate::KeyPress
//! [`ModifierKeysMask`]: crate::ModifierKeysMask
//! [`MouseInput`]: crate::MouseInput

// Private modules (hide internal structure).
mod enhanced_keys;
mod input_device;
mod input_event;
mod key_press;
mod modifier_keys_mask;
mod mouse_input;
mod output_device;
mod shared_writer;
mod terminal_io_type_aliases;

// Re-exports for flat public API.
pub use enhanced_keys::*;
pub use input_device::*;
pub use input_event::*;
pub use key_press::*;
pub use modifier_keys_mask::*;
pub use mouse_input::*;
pub use output_device::*;
pub use shared_writer::*;
pub use terminal_io_type_aliases::*;

// Backend compatibility tests (Linux-only PTY tests).
// Public for docs and tests so intra-doc links resolve.
#[cfg(all(any(test, doc), target_os = "linux"))]
pub mod backend_compat_tests;
