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
//! This module provides **integration tests** that validate the complete ANSI sequence
//! processing pipeline using the public API. This is a key component of the three-layer
//! testing strategy:
//!
//! ## Integration Testing Role
//!
//! ```text
//! ANSI Bytes → apply_ansi_bytes() → VTE Parser → Shim → Implementation → Buffer Updates
//!      ↑                                                                        ↑
//!  Test Input                                                            Test Assertions
//! ```
//!
//! These conformance tests **intentionally** test the entire pipeline using
//! [`OffscreenBuffer::apply_ansi_bytes`], not individual shim or implementation
//! functions. This approach:
//!
//! 1. **Tests Real-World Usage**: Uses the same public API that production code uses
//! 2. **Validates Complete Pipeline**: Ensures ANSI parsing → shim → impl → buffer works
//!    together
//! 3. **Complements Unit Tests**: While [`vt_100_ansi_impl`] files have unit tests, these
//!    test the integrated system
//! 4. **Replaces Shim Tests**: Since [`operations`] shims are pure delegation, these
//!    tests provide their coverage
//!
//! ## Testing Strategy Relationship
//!
//! This integration testing layer works in concert with unit testing:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                    INTEGRATION TESTS (this module)                         │
//! │  • Uses apply_ansi_bytes() public API                                      │
//! │  • Tests complete ANSI sequence → buffer update pipeline                   │
//! │  • Provides coverage for operations/* shims                                │
//! └────────────────────────────────────────────────────────────────────────────┘
//!                                     ↕ complements
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                        UNIT TESTS (vt_100_ansi_impl)                       │
//! │  • Direct method calls to implementation functions                         │
//! │  • Tests isolated buffer manipulation logic                                │
//! │  • Fast execution, precise error diagnosis                                 │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The conformance approach ensures compatibility with real-world terminal applications
//! by testing patterns extracted from actual usage scenarios rather than isolated
//! sequence fragments.
//!
//! ## Navigation to Related Layers
//!
//! When working with any test file, you can navigate to its related implementation
//! layers:
//! - **Shim Layer**: [`operations`] - The delegation layer being tested indirectly
//! - **Implementation Layer**: [`vt_100_ansi_impl`] - The business logic being tested
//! - **Testing Philosophy**: See [parser module docs] for the complete three-layer
//!   strategy
//!
//! For example, when working on [`test_char_ops`]:
//! 1. **Integration Tests**: [`test_char_ops`] (this module) - Full ANSI sequence testing
//! 2. **Shim**: [`operations::char_ops`] - Parameter translation (tested indirectly here)
//! 3. **Implementation**: [`impl_char_ops`] - Buffer logic (has separate unit tests)
//!
//! ## Test Organization
//!
//! - **[`conformance_data`]/**: Reusable sequence builder functions organized by category
//! - **[`tests`]/**: Test modules that validate sequence processing behavior
//! - **[`test_fixtures_vt_100_ansi_conformance`]**: Shared test utilities and helper
//!   functions
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
//!     CsiSequence::CursorPosition { row: term_row(nz(2)), col: term_col(nz(5)) },
//!     SgrCode::ForegroundBasic(ANSIBasicColor::Red),
//!     "Error",
//!     SgrCode::Reset
//! );
//! ```
//!
//! ## Bidirectional Sequence Pattern
//!
//! Many sequence types in this codebase follow a **bidirectional pattern**: they can both
//! **parse** incoming ANSI sequences and **generate** outgoing test sequences. This provides
//! type-safe, infallible sequence construction for tests.
//!
//! ### Pattern Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        BIDIRECTIONAL ENUM                               │
//! │  • Parsing:    ANSI bytes → Enum variant                                │
//! │  • Generation: Enum variant → ANSI string (via Display/FastStringify)   │
//! │  • Type-safe:  Compile-time validation, no raw escape string mistakes   │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Types Using This Pattern
//!
//! | Type | Parsing Method | Generation | Purpose |
//! |------|----------------|------------|---------|
//! | [`CsiSequence`] | Manual parsing | `Display` + `FastStringify` | CSI control sequences |
//! | [`EscSequence`] | Manual parsing | `Display` + `FastStringify` | ESC control sequences |
//! | [`OscSequence`] | Manual parsing | `Display` + `FastStringify` | OSC sequences (titles, hyperlinks) |
//! | [`DsrSequence`] | Manual parsing | `Display` + `FastStringify` | Device Status Report responses |
//! | [`ExtendedColorSequence`] | `parse_from_slice()` | `Display` + `FastStringify` | 256-color & RGB colors |
//!
//! ### Benefits of Bidirectional Pattern
//!
//! 1. **Infallible Generation**: Enums can only represent valid sequences
//! 2. **Type Safety**: Compiler prevents invalid color indices, RGB values, etc.
//! 3. **Self-Documenting**: Enum variants clearly describe what they do
//! 4. **Consistent API**: Same types used for parsing and test data
//! 5. **Easy Refactoring**: Changes to sequence format happen in one place
//!
//! ### Example: Extended Color Sequences
//!
//! ```rust,ignore
//! use crate::protocols::csi_codes::{
//!     ExtendedColorSequence,
//!     extended_color_test_helpers::*
//! };
//!
//! // ❌ Raw escape strings (error-prone, unclear)
//! let bad_fg = "\x1b[38:5:196m";  // What color index? Typo-prone!
//! let bad_bg = "\x1b[48:2:255:128:0m";  // RGB components unclear
//!
//! // ✅ Type-safe generation (compiler-validated)
//! let good_fg = ExtendedColorSequence::SetForegroundAnsi256(196).to_string();
//! let good_bg = ExtendedColorSequence::SetBackgroundRgb(255, 128, 0).to_string();
//!
//! // ✅ Even better: Use test helpers
//! let helper_fg = fg_ansi256(196);
//! let helper_bg = bg_rgb(255, 128, 0);
//! ```
//!
//! ### FastStringify Trait
//!
//! The [`FastStringify`] trait provides efficient string building for complex sequences:
//!
//! ```rust,ignore
//! pub trait FastStringify: Display {
//!     fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result;
//!     fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> Result;
//! }
//! ```
//!
//! - **Performance**: Builds string in a buffer, then writes once to formatter
//! - **Required bound**: All `FastStringify` types must implement `Display`
//! - **Usage**: Automatically available via `.to_string()` through `Display`
//!
//! ### Test Helper Functions
//!
//! Each bidirectional type provides ergonomic test helpers:
//!
//! ```rust,ignore
//! // Extended colors
//! use crate::protocols::csi_codes::extended_color_test_helpers::*;
//! let fg = fg_ansi256(196);      // → "\x1b[38:5:196m"
//! let bg = bg_rgb(255, 128, 0);  // → "\x1b[48:2:255:128:0m"
//!
//! // DSR sequences
//! use crate::protocols::dsr_codes::dsr_test_helpers::*;
//! let cursor_pos = dsr_cursor_position_response(term_row(nz(10)), term_col(nz(25)));
//!
//! // CSI sequences (via Display)
//! let cursor_move = CsiSequence::CursorPosition {
//!     row: term_row(nz(5)),
//!     col: term_col(nz(10))
//! }.to_string();
//! ```
//!
//! ### When to Use This Pattern
//!
//! Use bidirectional enums when:
//! - ✅ The sequence has **multiple variants** (colors, positions, modes)
//! - ✅ Sequences are **parameterized** (indices, RGB values, coordinates)
//! - ✅ You need **type-safe test data** generation
//! - ✅ Parsing and generation **share the same structure**
//!
//! Don't use when:
//! - ❌ Sequences are **one-off** strings without structure
//! - ❌ Parsing is handled entirely by [`vte`] library
//! - ❌ No need for test sequence generation
//!
//! ### Adding a New Bidirectional Type
//!
//! 1. **Define the enum** with variants for each sequence type
//! 2. **Implement parsing** (either `parse_from_*` methods or `From` traits)
//! 3. **Implement `FastStringify`** for efficient string building
//! 4. **Implement `Display`** using `FastStringify` methods
//! 5. **Add test helpers** in a `#[cfg(any(test, doc))]` module
//! 6. **Write comprehensive tests** for both parsing and generation
//!
//! See [`ExtendedColorSequence`] for a complete example implementation.
//!
//! [`CsiSequence`]: crate::protocols::csi_codes::CsiSequence
//! [`EscSequence`]: crate::protocols::esc_codes::EscSequence
//! [`OscSequence`]: crate::core::osc::osc_codes::OscSequence
//! [`DsrSequence`]: crate::protocols::dsr_codes::DsrSequence
//! [`ExtendedColorSequence`]: crate::protocols::csi_codes::ExtendedColorSequence
//! [`FastStringify`]: crate::core::common::fast_stringify::FastStringify
//! [`vte`]: https://docs.rs/vte
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
//!
//! [`operations`]: super::operations
//! [`vt_100_ansi_impl`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl
//! [`operations::char_ops`]: super::operations::vt_100_shim_char_ops
//! [`impl_char_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_char_ops
//! [`test_char_ops`]: tests::vt_100_test_char_ops
//! [parser module docs]: super
//! [`OffscreenBuffer::apply_ansi_bytes`]: crate::tui::terminal_lib_backends::offscreen_buffer::OffscreenBuffer::apply_ansi_bytes

#[cfg(any(test, doc))]
pub mod conformance_data;

#[cfg(any(test, doc))]
pub mod test_fixtures_vt_100_ansi_conformance; // Re-export existing test fixtures

#[cfg(any(test, doc))]
pub mod tests;
