// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach sources.
#[macro_use]
pub mod tui_color;
#[macro_use]
pub mod tui_style_lite;
#[macro_use]
pub mod tui_stylesheet;
pub mod color_degradation;
pub mod crossterm_color_converter;
pub mod hex_color_parser;
pub mod tui_style_attribs;
pub mod tui_style_impl;

// Re-export.
pub use color_degradation::*;
pub use hex_color_parser::*;
pub use tui_color::*;
pub use tui_style_attribs::*;
pub use tui_style_impl::*;
pub use tui_stylesheet::*;
