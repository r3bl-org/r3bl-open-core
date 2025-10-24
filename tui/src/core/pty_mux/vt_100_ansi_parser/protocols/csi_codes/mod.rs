// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control Sequence Introducer (CSI) codes for terminal control.
//!
//! CSI sequences are the most common type of ANSI escape sequences used in modern
//! terminals. They provide parameterized control over cursor movement, text formatting,
//! colors, and display manipulation.
//!
//! ## Evolution from ESC Sequences
//!
//! CSI sequences evolved from the simpler direct ESC sequences to provide greater
//! flexibility:
//!
//! - **ESC sequences** (the predecessors): Simple, non-parameterized commands like `ESC
//!   7` (save cursor) or `ESC D` (move down one line). See [`esc_codes`] for details.
//! - **CSI sequences** (modern approach): Parameterized commands like `ESC[s` (save
//!   cursor) or `ESC[5B` (move down 5 lines). The parameters make them much more
//!   flexible.
//!
//! Many operations can be performed using either approach for backward compatibility.
//! Modern applications typically prefer CSI for their flexibility.
//!
//! ## Structure
//! CSI sequences follow the pattern: `ESC [ parameters final_character`
//! - Start with ESC (0x1B) followed by `[`
//! - Optional numeric parameters separated by `;`
//! - End with a single letter that determines the action
//!
//! ## Common Uses
//! - **Cursor Movement**: Move cursor to specific positions or by relative amounts
//! - **Text Formatting**: Apply colors, bold, italic, underline, and other text
//!   attributes
//! - **Display Control**: Clear screen/lines, scroll content, save/restore cursor
//!   position
//! - **Terminal Modes**: Configure terminal behavior and features
//!
//! ## Examples
//! - `ESC[2J` - Clear entire screen
//! - `ESC[1;5H` - Move cursor to row 1, column 5
//! - `ESC[31m` - Set text color to red
//! - `ESC[1A` - Move cursor up 1 line
//!
//! [`esc_codes`]: crate::vt_100_ansi_parser::protocols::esc_codes

// Attach private module to avoid naming conflicts (hide inner details).
mod csi_constants;
mod margin;
mod private_mode;
mod sequence;
mod sgr_color_sequences;

// Re-export public items for easier access.
pub use csi_constants::*;
pub use margin::*;
pub use private_mode::*;
pub use sequence::*;
pub use sgr_color_sequences::*;
