// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Conformance test data for VT100 ANSI sequence validation.
//!
//! This module provides type-safe, reusable sequence builder functions organized by
//! functionality. Each module contains functions that generate ANSI sequences using
//! the codebase's sequence builders (`CsiSequence`, `EscSequence`, `SgrCode`, etc.)
//! rather than hardcoded escape strings.
//!
//! # Design Philosophy
//!
//! Traditional ANSI testing often uses hardcoded escape sequences that are difficult
//! to read, maintain, and validate. This module takes a different approach:
//!
//! ```rust,ignore
//! // ❌ Traditional approach: hardcoded, error-prone
//! let old_way = b"\x1b[2J\x1b[H\x1b[31mError\x1b[0m";
//!
//! // ✅ Builder approach: type-safe, self-documenting
//! let new_way = format!("{}{}{}Error{}",
//!     CsiSequence::EraseDisplay(2),           // Clear entire screen
//!     CsiSequence::CursorPosition {           // Move to home
//!         row: term_row(1), col: term_col(1)
//!     },
//!     SgrCode::ForegroundBasic(ANSIBasicColor::Red),  // Red text
//!     SgrCode::Reset                                   // Reset styling
//! );
//! ```
//!
//! ## Benefits of Builder-Based Sequences
//!
//! - **Compile-time validation**: Invalid sequences cause compilation errors
//! - **Self-documenting**: Function names clearly indicate sequence purpose
//! - **Refactoring safety**: Changes to sequence builders update all tests automatically
//! - **Type safety**: Cannot accidentally create malformed sequences
//! - **Composability**: Complex sequences built from simple, reusable components
//! - **VT100 specification mapping**: Each builder corresponds to documented commands
//! - **IDE support**: Full autocomplete and error checking
//!
//! ## Module Organization
//!
//! | Module | Purpose | Key Functions | VT100 Spec Coverage |
//! |--------|---------|---------------|---------------------|
//! | [`basic_sequences`] | Simple operations | `clear_and_home()`, `move_and_print()` | Cursor positioning, display control |
//! | [`cursor_sequences`] | Cursor control | `save_do_restore()`, `move_to_position()` | ESC 7/8, CSI H commands |
//! | [`display_sequences`] | Screen manipulation | `clear_screen()`, `clear_to_end_of_screen()` | ED, EL commands |
//! | [`styling_sequences`] | Text formatting | `colored_text()`, `rainbow_text()` | SGR codes 30-37, 40-47 |
//! | [`vim_sequences`] | Vim editor patterns | `vim_status_line()`, `vim_syntax_highlighting()` | Real vim output sequences |
//! | [`emacs_sequences`] | Emacs editor patterns | `emacs_mode_line()`, `emacs_buffer_list()` | Real emacs output sequences |
//! | [`tmux_sequences`] | Terminal multiplexer | `tmux_status_bar()`, `tmux_pane_split_horizontal()` | Real tmux output sequences |
//! | [`edge_case_sequences`] | Boundary conditions | `malformed_sequences()`, `boundary_cursor_tests()` | Error handling validation |
//!
//! ## Sequence Construction Patterns
//!
//! ### Simple Sequences
//! Basic operations using single commands:
//!
//! ```rust,ignore
//! use crate::ansi_parser::protocols::csi_codes::CsiSequence;
//!
//! pub fn clear_screen() -> String {
//!     CsiSequence::EraseDisplay(2).to_string()
//! }
//! ```
//!
//! ### Composed Sequences
//! Complex operations combining multiple commands:
//!
//! ```rust,ignore
//! pub fn clear_and_home() -> String {
//!     format!("{}{}",
//!         CsiSequence::EraseDisplay(2),      // Clear screen
//!         CsiSequence::CursorPosition {      // Move to home
//!             row: term_row(1),
//!             col: term_col(1)
//!         }
//!     )
//! }
//! ```
//!
//! ### Parameterized Sequences
//! Functions that generate sequences based on parameters:
//!
//! ```rust,ignore
//! pub fn move_and_print(row: u16, col: u16, text: &str) -> String {
//!     format!("{}{}",
//!         CsiSequence::CursorPosition {
//!             row: term_row(row),
//!             col: term_col(col)
//!         },
//!         text
//!     )
//! }
//! ```
//!
//! ### Real-World Application Sequences
//! Complex patterns extracted from actual terminal applications:
//!
//! ```rust,ignore
//! pub fn vim_status_line(mode: &str, status_row: u16) -> String {
//!     format!("{}{}{}{}{}{}",
//!         EscSequence::SaveCursor,           // Save current position
//!         CsiSequence::CursorPosition {      // Move to status line
//!             row: term_row(status_row),
//!             col: term_col(1)
//!         },
//!         SgrCode::Invert,                   // Reverse video
//!         format!("-- {} --", mode),        // Status text
//!         SgrCode::Reset,                    // Reset styling
//!         EscSequence::RestoreCursor         // Restore position
//!     )
//! }
//! ```
//!
//! ## Testing Integration
//!
//! These sequence functions integrate seamlessly with the test framework:
//!
//! ```rust,ignore
//! #[test]
//! fn test_vim_status_line_display() {
//!     let mut ofs_buf = create_realistic_terminal_buffer();
//!
//!     // Apply sequence using conformance data
//!     let sequence = vim_sequences::vim_status_line("INSERT", 25);
//!     let (osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(sequence);
//!
//!     // Validate behavior
//!     assert_eq!(osc_events.len(), 0);
//!     assert_eq!(dsr_responses.len(), 0);
//!     assert_styled_char_at(&ofs_buf, 24, 0, '-',
//!         |style| matches!(style.attribs.invert, Some(_)),
//!         "status line reverse video");
//! }
//! ```
//!
//! ## Usage Examples
//!
//! ### Basic Operations
//! ```rust,ignore
//! use crate::vt_100_ansi_conformance_tests::conformance_data::basic_sequences;
//!
//! // Clear screen and move cursor to home
//! let sequence = basic_sequences::clear_and_home();
//! ofs_buf.apply_ansi_bytes(sequence);
//!
//! // Print text at specific position
//! let text_seq = basic_sequences::move_and_print(5, 10, "Hello World");
//! ofs_buf.apply_ansi_bytes(text_seq);
//! ```
//!
//! ### Styling Operations
//! ```rust,ignore
//! use crate::vt_100_ansi_conformance_tests::conformance_data::styling_sequences;
//! use crate::ANSIBasicColor;
//!
//! // Apply colored text
//! let red_text = styling_sequences::colored_text(ANSIBasicColor::Red, "Error");
//! ofs_buf.apply_ansi_bytes(red_text);
//!
//! // Create rainbow text
//! let rainbow = styling_sequences::rainbow_text("RAINBOW");
//! ofs_buf.apply_ansi_bytes(rainbow);
//! ```
//!
//! ### Real-World Scenarios
//! ```rust,ignore
//! use crate::vt_100_ansi_conformance_tests::conformance_data::{vim_sequences, tmux_sequences};
//!
//! // Simulate vim status line
//! let vim_status = vim_sequences::vim_status_line("NORMAL", 24);
//! ofs_buf.apply_ansi_bytes(vim_status);
//!
//! // Simulate tmux status bar
//! let tmux_status = tmux_sequences::tmux_status_bar();
//! ofs_buf.apply_ansi_bytes(tmux_status);
//! ```

pub mod basic_sequences;
pub mod cursor_sequences;
pub mod display_sequences;
pub mod edge_case_sequences;
pub mod emacs_sequences;
pub mod styling_sequences;
pub mod tmux_sequences;
pub mod vim_sequences;

// Re-export public functions that are actually used.
