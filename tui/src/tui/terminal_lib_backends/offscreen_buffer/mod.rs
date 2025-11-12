// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Offscreen buffer module for terminal rendering and VT100/ANSI terminal emulation.
//!
//! # You Are Here
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor]
//!   ↓
//!   ┌──────────────────────────────────────┐
//!   │  OffscreenBuffer (YOU ARE HERE)      │
//!   │  (Virtual terminal grid)             │
//!   └──────────────────────────────────────┘
//!   ↓
//! [Stage 4: Backend Converter]
//!   ↓
//! [Stage 5: Backend Executor]
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! **Input**: Written to by the Compositor (Stage 3)
//! **Output**: Read by the Backend Converter (Stage 4) for diff and optimization
//! **Role**: The 2D virtual terminal that bridges rendering and execution
//!
//! > **For the complete pipeline architecture**, see [`super`] (parent module).
//!
//! ## What This Structure Is
//!
//! The `OffscreenBuffer` is a **2D grid representing the entire terminal screen**. Each
//! cell contains a styled character. This is not a stream of escape sequences (like
//! traditional terminal output), but a 2D array that can be easily compared
//! frame-to-frame to determine what changed.
//!
//! This module serves **two integration points**:
//! 1. **As the Compositor's Output**: Receives rendering operations from the pipeline
//! 2. **As a VT100/ANSI Terminal Emulator**: Processes escape sequences from child
//!    processes
//!
//! # Architecture Overview
//!
//! The offscreen buffer serves as the central data structure for terminal emulation,
//! bridging the gap between raw terminal output and visual rendering:
//!
//! ```text
//! ╭─────────────────╮    ╭───────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │───▶│ PTY Master    │───▶│ VTE Parser      │───▶│ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream) │    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰───────────────╯    ╰─────────────────╯    │  buffer)     │
//!        │                                                           ╰──────────────╯
//!        │                                    ╭─────────────────╮           │
//!        │                                    │ RenderPipeline  │◀──────────╯
//!        │                                    │ paint()         │
//!        ╰────────────────────────────────────▶ Terminal Output │
//!                                             ╰─────────────────╯
//! ```
//!
//! # Dual Integration Points
//!
//! The offscreen buffer is designed to work seamlessly with two major subsystems:
//!
//! ## 1. ANSI/VT100 Terminal Emulation
//!
//! - **Parser Integration**: Processes escape sequences via [`vt_100_ansi_impl`]
//!   implementations
//! - **State Management**: Maintains cursor position, character sets, scrolling regions
//! - **Protocol Compliance**: Full VT100 specification compliance with conformance tests
//! - **Character Handling**: Supports both ASCII and DEC graphics character sets
//!
//! ## 2. Render Pipeline Integration
//!
//! - **Visual Rendering**: Direct integration with [`RenderPipeline::paint()`]
//! - **Styling Support**: Rich text styling with [`TuiStyle`] for colors and attributes
//! - **Performance**: Efficient diff-based rendering to minimize screen updates
//! - **Multi-width Characters**: Proper handling of emoji and Unicode display widths
//!
//! # Grid Representation & Variable-ColWidth Characters
//!
//! The buffer implements a sophisticated grid system that handles the complexity of
//! modern terminal content:
//!
//! ```text
//! Terminal Grid (cell-by-cell mapping):
//! ┌───┬───┬───┬───┬───┐
//! │ H │ e │ 😃│ ∅ │ ! │  ← Row 0
//! └───┴───┴───┴───┴───┘
//!       │   │   │
//!       │   │   └─ PixelChar::Void (placeholder for wide char)
//!       │   └─ PixelChar::PlainText { display_char: '😃', width: 2 }
//!       └─ PixelChar::PlainText { display_char: 'e', width: 1 }
//! ```
//!
//! **Key Design Principles:**
//! - **Cell Mapping**: Each grid position maps directly to a terminal screen position
//! - **Wide Character Handling**: Multi-width characters use [`PixelChar::Void`]
//!   placeholders
//! - **Rendering Integrity**: Void cells ensure proper visual alignment and cursor
//!   positioning
//! - **Unicode Support**: Full support for emoji, combining characters, and international
//!   text
//!
//! # VT100 Implementation Architecture - 1:1 Mapping
//!
//! The buffer's VT100 support follows a clean architectural pattern with perfect file
//! mapping:
//!
//! ```text
//! vt_100_pty_output_parser/operations/             offscreen_buffer/vt_100_ansi_impl/
//! ├── vt_100_shim_char_ops         →         ├── vt_100_impl_char_ops    (print_char, ICH, DCH, ECH)
//! ├── vt_100_shim_control_ops      →         ├── vt_100_impl_control_ops (BS, TAB, LF, CR)
//! ├── vt_100_shim_cursor_ops       →         ├── vt_100_impl_cursor_ops  (movement, positioning)
//! ├── vt_100_shim_line_ops         →         ├── vt_100_impl_line_ops    (insert/delete lines)
//! ├── vt_100_shim_scroll_ops       →         ├── vt_100_impl_scroll_ops  (scrolling, regions)
//! ├── vt_100_shim_terminal_ops     →         ├── vt_100_impl_terminal_ops(reset, clear, charset)
//! └── bounds_check.rs              →         └── vt_100_impl_ansi_scroll_helper (scroll region utilities)
//! ```
//!
//! This 1:1 mapping provides:
//! - **Predictable Navigation**: Easy to find implementation for any ANSI operation
//! - **Clear Separation**: Parser logic separate from buffer implementation
//! - **Comprehensive Testing**: Each implementation file has full unit test coverage
//!
//! # API Design Philosophy
//!
//! The [`OffscreenBuffer`] API follows a consistent design philosophy for method return
//! types and error handling that balances terminal emulation resilience with development
//! safety.
//!
//! ## Return Type Patterns
//!
//! ### Mutation Methods → `bool`
//!
//! Methods that modify buffer state and can validate input return `bool`:
//! - `true`: Operation succeeded
//! - `false`: Operation failed due to invalid input/bounds
//!
//! **Examples:**
//! - [`set_char()`], [`fill_char_range()`], [`copy_chars_within_line()`]
//! - [`clear_line()`], [`shift_lines_up()`], [`shift_lines_down()`]
//! - [`insert_chars_at_cursor()`], [`delete_chars_at_cursor()`]
//!
//! **Usage Pattern:**
//! ```rust
//! # use r3bl_tui::*;
//! # let mut buffer = OffscreenBuffer::new_empty(Size { col_width: width(10), row_height: height(5) });
//! # let pos = Pos { row_index: row(1), col_index: col(1) };
//! # let pixel_char = PixelChar::default();
//! // In production code, failures are often ignored for terminal resilience
//! buffer.set_char(pos, pixel_char);
//!
//! // In development, use debug_assert! to catch issues early
//! let success = buffer.set_char(pos, pixel_char);
//! debug_assert!(success.is_ok(), "Failed to set char at {:?}", pos);
//! ```
//!
//! ### Query Methods → `Option<T>`
//!
//! Methods that retrieve data return [`Option<T>`]:
//! - `Some(value)`: Data exists at the requested location
//! - `None`: No data or out of bounds
//!
//! **Examples:**
//! - [`get_char()`] → [`Option<PixelChar>`]
//! - [`get_line()`] → [`Option<&PixelCharLine>`]
//! - [`diff()`] → [`Option<PixelCharDiffChunks>`]
//!
//! **Usage Pattern:**
//! ```rust
//! # use r3bl_tui::*;
//! # let buffer = OffscreenBuffer::new_empty(Size { col_width: width(10), row_height: height(5) });
//! # let pos = Pos { row_index: row(1), col_index: col(1) };
//! if let Some(char) = buffer.get_char(pos) {
//!     // Process the character
//! }
//! ```
//!
//! ### Infallible Operations → `void`
//!
//! Methods that are designed to always succeed return nothing:
//!
//! **Categories:**
//! - **Cursor operations**: Always clamp to valid bounds (VT100 behavior)
//!   - [`cursor_up()`], [`cursor_down()`], [`cursor_forward()`], [`cursor_backward()`]
//! - **Style operations**: No failure mode for attribute changes
//!   - [`set_foreground_color()`], [`reset_all_style_attributes()`]
//! - **Control operations**: Terminal emulation resilience
//!   - [`handle_backspace()`], [`handle_tab()`], [`handle_line_feed()`]
//!
//! These operations follow VT100 terminal behavior where operations are resilient
//! and clamp values rather than failing.
//!
//! ## Error Handling Strategy
//!
//! ### Production Behavior (Release Builds)
//!
//! Terminal emulators must be resilient and continue functioning even with invalid input:
//! - Failed mutations are silently ignored
//! - Invalid positions are clamped to valid ranges
//! - The terminal remains usable regardless of input
//!
//! ### Development Safety (Debug Builds)
//!
//! Debug assertions catch issues during development:
//! ```rust
//! # use r3bl_tui::*;
//! # let mut buffer = OffscreenBuffer::new_empty(Size { col_width: width(10), row_height: height(5) });
//! # buffer.cursor_pos = Pos { row_index: row(1), col_index: col(1) };
//! # let count = Length::from(1);
//! // In parser operations
//! let success = buffer.delete_chars_at_cursor(count);
//! debug_assert!(success.is_ok(), "Failed to delete {:?} chars at cursor", count);
//!
//! # let row = RowIndex::from(1);
//! # let source = ColIndex::from(0);
//! # let end = ColIndex::from(1);
//! # let dest = ColIndex::from(2);
//! // In internal operations with edge case awareness
//! let success = buffer.copy_chars_within_line(row, source..end, dest);
//! debug_assert!(success.is_ok() || source >= end,
//!     "Failed to copy chars, range: {:?}..{:?}", source, end);
//! ```
//!
//! ## Design Rationale
//!
//! This design balances multiple requirements:
//!
//! 1. **Terminal Resilience**: Production terminals never crash on bad input
//! 2. **Development Safety**: Issues are caught early during testing
//! 3. **Zero Cost**: `debug_assert!` compiles to nothing in release builds
//! 4. **VT100 Compliance**: Follows terminal emulation standards for clamping behavior
//! 5. **API Clarity**: Consistent patterns make the API predictable
//!
//! The philosophy aligns with terminal emulation best practices where the terminal
//! must remain functional regardless of the input it receives, while still providing
//! developers with tools to catch integration issues early.
//!
//! ## Implementation Details
//!
//! ### Type-Safe Bounds Checking
//!
//! All bounds checking uses type-safe utilities from [`bounds_check`]:
//! - [`IndexOps`] for 0-based indices
//! - [`LengthOps`] for 1-based lengths
//! - [`Pos`] for 2D positions combining row and column indices
//!
//! ### Validation Helpers - Preferred Pattern
//!
//! All buffer operations **should use** the standardized validation helper methods from
//! [`ofs_buf_range_validation`]:
//!
//! #### For Column Range Operations
//! ```text
//! // ✅ Preferred: Use validation helpers
//! pub fn my_column_operation(&mut self, row: RowIndex, col_range: Range<ColIndex>) -> bool {
//!     let Some((start_col, end_col, line)) =
//!         self.validate_col_range_mut(row, col_range) else {
//!         return false;
//!     };
//!
//!     // Safe to use start_col..end_col on line
//!     line[start_col..end_col].fill(PixelChar::Spacer);
//!     true
//! }
//!
//! // ❌ Avoid: Manual bounds checking
//! pub fn avoid_this_pattern(&mut self, row: RowIndex, col: ColIndex) -> bool {
//!     if row.as_usize() >= self.buffer.len() { return false; }
//!     if col.as_usize() >= self.buffer[row.as_usize()].len() { return false; }
//!     // Manual validation is error-prone and inconsistent
//!     true
//! }
//! ```
//!
//! #### For Row Range Operations
//! ```text
//! // ✅ Preferred: Use validation helpers
//! pub fn my_row_operation(&mut self, row_range: Range<RowIndex>) -> bool {
//!     let Some((start_row, end_row, lines)) =
//!         self.validate_row_range_mut(row_range) else {
//!         return false;
//!     };
//!
//!     // Safe to use start_row..end_row indices with lines slice
//!     for line in lines.iter_mut() {
//!         line.fill(PixelChar::Spacer);
//!     }
//!     true
//! }
//! ```
//!
//! #### For Single Position Operations
//! ```text
//! // ✅ Preferred: Use single-row validation for consistency
//! pub fn my_position_operation(&mut self, pos: Pos) -> bool {
//!     let row_range = pos.row_index..row(pos.row_index.as_usize() + 1);
//!     let Some((_, _, lines)) = self.validate_row_range_mut(row_range) else {
//!         return false;
//!     };
//!
//!     if pos.col_index.as_usize() >= lines[0].len() {
//!         return false;
//!     }
//!
//!     lines[0][pos.col_index.as_usize()] = PixelChar::Spacer;
//!     true
//! }
//! ```
//!
//! #### Core Validation Methods
//! - [`validate_col_range_mut()`] for column range validation
//! - [`validate_row_range_mut()`] for row range validation
//!
//! #### Validation Benefits
//!
//! These ensure consistent validation across all buffer operations. The standardized
//! validation helpers provide:
//! - **Consistency**: Single source of truth for bounds checking logic
//! - **Type Safety**: Leverages [`RangeBoundsExt`] trait for correct exclusive range
//!   semantics
//! - **No `unwrap()` calls**: All validation returns `Option` for safe access
//! - **Zero allocation**: Methods return references to existing buffer data
//! - **Error Prevention**: Eliminates common off-by-one errors in manual bounds checking
//!
//! [`IndexOps`]: crate::core::coordinates::bounds_check::IndexOps
//! [`LengthOps`]: crate::core::coordinates::bounds_check::LengthOps
//! [`Option<&PixelCharLine>`]: std::option::Option
//! [`Option<PixelChar>`]: std::option::Option
//! [`Option<PixelCharDiffChunks>`]: std::option::Option
//! [`Option<T>`]: std::option::Option
//! [`PixelChar::Void`]: PixelChar::Void
//! [`Pos`]: crate::Pos
//! [`RangeBoundsExt`]: crate::core::coordinates::bounds_check::RangeBoundsExt
//! [`RenderPipeline::paint()`]: crate::RenderPipeline::paint
//! [`TuiStyle`]: crate::TuiStyle
//! [`bounds_check`]: crate::core::coordinates::bounds_check
//! [`clear_line()`]: crate::OffscreenBuffer::clear_line
//! [`copy_chars_within_line()`]: crate::OffscreenBuffer::copy_chars_within_line
//! [`cursor_backward()`]: crate::OffscreenBuffer::cursor_backward
//! [`cursor_down()`]: crate::OffscreenBuffer::cursor_down
//! [`cursor_forward()`]: crate::OffscreenBuffer::cursor_forward
//! [`cursor_up()`]: crate::OffscreenBuffer::cursor_up
//! [`delete_chars_at_cursor()`]: crate::OffscreenBuffer::delete_chars_at_cursor
//! [`diff()`]: crate::OffscreenBuffer::diff
//! [`fill_char_range()`]: crate::OffscreenBuffer::fill_char_range
//! [`get_char()`]: crate::OffscreenBuffer::get_char
//! [`get_line()`]: crate::OffscreenBuffer::get_line
//! [`handle_backspace()`]: crate::OffscreenBuffer::handle_backspace
//! [`handle_line_feed()`]: crate::OffscreenBuffer::handle_line_feed
//! [`handle_tab()`]: crate::OffscreenBuffer::handle_tab
//! [`insert_chars_at_cursor()`]: crate::OffscreenBuffer::insert_chars_at_cursor
//! [`ofs_buf_range_validation`]:
//! [`reset_all_style_attributes()`]: crate::OffscreenBuffer::reset_all_style_attributes
//! [`set_char()`]: crate::OffscreenBuffer::set_char
//! [`set_foreground_color()`]: crate::OffscreenBuffer::set_foreground_color
//! [`shift_lines_down()`]: crate::OffscreenBuffer::shift_lines_down
//! [`shift_lines_up()`]: crate::OffscreenBuffer::shift_lines_up
//! [`validate_col_range_mut()`]: crate::OffscreenBuffer::validate_col_range_mut
//! [`validate_row_range_mut()`]: crate::OffscreenBuffer::validate_row_range_mut

// Attach.
pub mod diff_chunks;
pub mod ofs_buf_bulk_ops;
pub mod ofs_buf_char_ops;
pub mod ofs_buf_core;
pub mod ofs_buf_line_level_ops;
pub mod ofs_buf_range_validation;
pub mod paint_impl;
pub mod pixel_char;
pub mod pixel_char_line;
pub mod pixel_char_lines;
pub mod vt_100_ansi_impl;

// Re-export all implementations.
pub use diff_chunks::*;
pub use ofs_buf_core::*;
pub use paint_impl::*;
pub use pixel_char::*;
pub use pixel_char_line::*;
pub use pixel_char_lines::*;

// Test fixtures (only available during testing).
#[cfg(any(test, doc))]
pub mod test_fixtures_ofs_buf;
