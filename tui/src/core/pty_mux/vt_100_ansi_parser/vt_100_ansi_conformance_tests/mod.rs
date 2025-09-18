// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! VT100 ANSI conformance tests for terminal sequence processing.
//!
//! This module provides comprehensive testing of ANSI/VT escape sequence processing
//! using a structured conformance test approach similar to the markdown parser tests.
//! Tests are organized by functionality and use type-safe sequence builders for
//! maintainability and specification compliance.
//!
//! # Testing Philosophy
//!
//! These tests validate the complete ANSI sequence processing pipeline:
//!
//! ```text
//! ANSI Sequences → VTE Parser → Perform Trait → OffscreenBuffer Updates
//! ```
//!
//! The conformance approach ensures compatibility with real-world terminal applications
//! by testing patterns extracted from actual usage scenarios rather than isolated
//! sequence fragments.
//!
//! ## Test Organization
//!
//! - **[`conformance_data`]/**: Reusable sequence builder functions organized by category
//! - **[`tests`]/**: Test modules that validate sequence processing behavior
//! - **[`test_fixtures`].rs**: Shared test utilities and helper functions
//!
//! ## Conformance Data Modules
//!
//! - **[`conformance_data::basic_sequences`]**: Simple, single-purpose ANSI sequences
//! - **[`conformance_data::cursor_sequences`]**: Cursor movement and positioning patterns
//! - **[`conformance_data::display_sequences`]**: Screen manipulation and display control
//! - **[`conformance_data::styling_sequences`]**: SGR text formatting and color sequences
//! - **[`conformance_data::vim_sequences`]**: Vim editor-specific sequence patterns
//! - **[`conformance_data::emacs_sequences`]**: Emacs editor sequence patterns
//! - **[`conformance_data::tmux_sequences`]**: Terminal multiplexer control sequences
//! - **[`conformance_data::edge_case_sequences`]**: Boundary conditions and error cases
//!
//! ## Real-World Testing Scenarios
//!
//! The tests include realistic terminal application patterns:
//!
//! ### Terminal Dimensions
//! Tests use authentic **80x25** terminal dimensions instead of constrained buffers,
//! ensuring real-world compatibility:
//!
//! ```rust,ignore
//! fn create_realistic_terminal_buffer() -> OffscreenBuffer {
//!     OffscreenBuffer::new_empty(height(25) + width(80))
//! }
//! ```
//!
//! ### Editor Application Patterns
//! - **Vim**: Status lines, syntax highlighting, visual selection, error messages
//! - **Emacs**: Mode lines, minibuffer prompts, buffer switching
//! - **Terminal multiplexers**: tmux status bars, pane management, session switching
//!
//! ### Complex Interaction Scenarios
//! - Cursor save/restore with intervening operations
//! - Nested styling with partial resets
//! - Line wrapping with scrolling margins
//! - Multi-colored syntax highlighting patterns
//!
//! ## Type-Safe Sequence Construction
//!
//! Instead of error-prone hardcoded escape strings, tests use compile-time validated
//! builders:
//!
//! ```rust,ignore
//! // ❌ Hardcoded sequences (brittle, unclear intent)
//! let bad_sequence = "\x1b[2;5H\x1b[31mError\x1b[0m";
//!
//! // ✅ Type-safe builders (validated, self-documenting)
//! let good_sequence = format!("{}{}{}",
//!     CsiSequence::CursorPosition { row: term_row(2), col: term_col(5) },
//!     SgrCode::ForegroundBasic(ANSIBasicColor::Red),
//!     "Error",
//!     SgrCode::Reset
//! );
//! ```
//!
//! ## VT100 Specification Mapping
//!
//! Each conformance data module includes specification references:
//! - **VT100 User Guide**: Section references for command behavior
//! - **ANSI X3.64**: Standard compliance notes
//! - **`XTerm` Control Sequences**: Extended sequence support
//!
//! ## Running Conformance Tests
//!
//! ```bash
//! # All 101+ conformance tests
//! cargo test vt_100_ansi_conformance_tests
//!
//! # Real-world application scenarios
//! cargo test test_real_world_scenarios
//!
//! # Specific sequence categories
//! cargo test test_cursor_operations
//! cargo test test_sgr_and_character_sets
//! cargo test test_line_wrap_and_scroll_control
//! ```
//!
//! ## Benefits of This Approach
//!
//! - **Type Safety**: Compile-time validation using sequence builders
//! - **Maintainability**: Single source of truth for test sequences
//! - **Readability**: Self-documenting test code with clear intent
//! - **Specification Compliance**: Easy mapping to VT100/ANSI standards
//! - **Real-world Validation**: Tests mirror actual terminal application usage
//! - **Extensibility**: Simple to add new conformance test patterns
//! - **Performance Testing**: Validates behavior under realistic load conditions
//!
//! ## Adding New Conformance Tests
//!
//! 1. **Create sequence patterns** in appropriate `conformance_data` module
//! 2. **Use type-safe builders** (`CsiSequence`, `EscSequence`, `SgrCode`)
//! 3. **Include VT100 spec references** in documentation
//! 4. **Test with realistic terminal dimensions** (80x25 or similar)
//! 5. **Validate complete behavior**, not just individual sequences
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use crate::vt_100_ansi_conformance_tests::conformance_data::vim_sequences;
//!
//! // Create realistic terminal buffer
//! let mut ofs_buf = OffscreenBuffer::new_empty(height(25) + width(80));
//!
//! // Apply vim status line sequence
//! let sequence = vim_sequences::vim_status_line("INSERT", 25);
//! let (osc_events, dsr_responses) = ofs_buf.apply_ansi_bytes(sequence);
//!
//! // Verify status line appears at bottom with correct styling
//! assert_styled_char_at(&ofs_buf, 24, 0, '-', |style| {
//!     matches!(style.attribs.invert, Some(_))
//! }, "status line reverse video");
//! ```

#[cfg(any(test, doc))]
pub mod conformance_data;

#[cfg(any(test, doc))]
pub mod test_fixtures_vt_100_ansi_conformance; // Re-export existing test fixtures

#[cfg(any(test, doc))]
pub mod tests;
