// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line editing state machine for async readline.
//!
//! This module implements the core line editing functionality that powers
//! [`Readline`]. It handles Unicode text input, cursor movement, terminal rendering,
//! and keyboard event processing with pause/resume support.
//!
//! # Architecture
//!
//! The module is organized by functional responsibility:
//!
//! | Module          | Responsibility                                             |
//! |-----------------|------------------------------------------------------------|
//! | `core`          | [`LineState`] struct, [`LineStateLiveness`] enum, state   |
//! | `cursor`        | Cursor movement, grapheme navigation, terminal positioning |
//! | `event_handlers`| Keyboard event dispatch (Ctrl, Alt, regular keys)          |
//! | `output`        | Data printing, prompt updates, exit handling               |
//! | `render`        | Terminal clear/render operations with ANSI sequences       |
//!
//! # Type Safety
//!
//! This module uses the [`bounds_check`] system for type-safe cursor positioning:
//!
//! - [`SegIndex`]: 0-based grapheme cursor position within the line
//! - [`ColIndex`]: 0-based terminal column position
//! - [`ColWidth`]: Display width for terminal cursor movement
//!
//! The [`CursorBoundsCheck`] trait is used (not [`ArrayBoundsCheck`]) because text
//! cursors can validly be positioned at the end-of-line (index == length), unlike
//! array access where this would be out of bounds.
//!
//! # Pause/Resume Support
//!
//! [`LineState`] can be paused to allow spinners or other UI elements to take over
//! the terminal. While paused, keyboard events are ignored and rendering is
//! suppressed. See [`LineStateLiveness`] and [`LineState::set_paused`].
//!
//! [`Readline`]: crate::Readline
//! [`bounds_check`]: crate::core::coordinates::bounds_check
//! [`SegIndex`]: crate::SegIndex
//! [`ColIndex`]: crate::ColIndex
//! [`ColWidth`]: crate::ColWidth
//! [`CursorBoundsCheck`]: crate::CursorBoundsCheck
//! [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules organized by functional responsibility.
mod core;
mod cursor;
mod event_handlers;
mod output;
mod render;

// Public re-exports (expose stable API).
pub use core::*;
