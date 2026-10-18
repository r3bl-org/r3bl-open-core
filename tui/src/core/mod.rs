// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod decl_macros;
#[macro_use]
pub(crate) mod stack_alloc_types; // Cross-folder rustdoc module link (heap_alloc_types).
#[macro_use]
pub(crate) mod test_fixtures; // Cross-folder rustdoc module link (lib.rs -> pty_test_fixtures).
#[macro_use]
pub mod common;
#[macro_use]
pub mod tui_style;
#[macro_use]
pub mod tui_styled_text;
#[macro_use]
pub mod coordinates;
#[macro_use]
pub mod terminal_io;
#[macro_use]
mod misc;
#[macro_use]
mod heap_alloc_types;

// Attach.
pub mod ansi;
pub mod log;
pub mod script;
pub mod notification;
pub mod color_wheel;
pub mod glyphs;
pub mod graphemes;
pub mod pty;
pub mod resilient_reactor_thread;
pub mod storage;

// Re-export.
pub use color_wheel::*;
pub use coordinates::*;
pub use glyphs::*;
pub use graphemes::*;
pub use heap_alloc_types::*;
pub use log::*;
pub use misc::*;
pub use notification::*;
pub use pty::*;
pub use script::*;
pub use stack_alloc_types::*;
pub use storage::*;
pub use terminal_io::*;
pub use test_fixtures::*;
pub use tui_style::*;
pub use tui_styled_text::*;
pub use resilient_reactor_thread::*;
pub use ansi::*;
pub use common::*;
