// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

pub mod decl_macros;
pub mod stack_alloc_types;
pub mod tui_style;
pub mod tui_styled_text;
pub mod coordinates;
pub mod script;
pub mod terminal_io;
pub mod test_fixtures;
pub mod common;
pub mod log;
pub mod misc;
pub mod heap_alloc_types;
pub mod ansi;

// Consumer-only modules.
pub mod color_wheel;
pub mod glyphs;
pub mod graphemes;
pub mod osc;
pub mod pty;
pub mod resilient_reactor_thread;
pub mod storage;
pub mod term;

// Re-export.
pub use color_wheel::*;
pub use coordinates::*;
pub use decl_macros::*;
pub use glyphs::*;
pub use graphemes::*;
pub use heap_alloc_types::*;
pub use log::*;
pub use misc::*;
pub use osc::*;
pub use pty::*;
pub use script::*;
pub use stack_alloc_types::*;
pub use storage::*;
pub use terminal_io::*;
pub use test_fixtures::*;
pub use tui_style::*;
pub use tui_styled_text::*;
pub use resilient_reactor_thread::*;
pub use term::*;
pub use ansi::*;
pub use common::*;

// Rustdoc search link fixes.

#[cfg(any(test, doc))] // Guard needed: ansi::constants sub-modules are only pub in doc/test builds.
#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use ansi::{csi, dsr, esc, generic, input_sequences, raw_mode_constants, sgr, utf8};

#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use coordinates::{bounds_check, buffer_coords, byte, percent_spec, primitives,
                      vt_100_ansi_coords};
#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use graphemes::{gc_string, traits, unicode_segment, word_boundaries};
#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use pty::{pty_engine, pty_mux, pty_session};
