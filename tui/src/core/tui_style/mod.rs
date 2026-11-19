// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod tui_color;
#[macro_use]
mod tui_style_lite;
#[macro_use]
mod tui_stylesheet;

// Attach.
mod color_degradation;
mod crossterm_color_converter;
mod hex_color_parser;
mod tui_style_attribs;
mod tui_style_impl;

// Re-export.
pub use color_degradation::*;
pub use hex_color_parser::*;
pub use tui_color::*;
pub use tui_style_attribs::*;
pub use tui_style_impl::*;
pub use tui_stylesheet::*;
