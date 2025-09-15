// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI vertical scrolling operations for `OffscreenBuffer`.
//!
//! This module provides methods for vertical line-based scrolling operations,
//! including index operations (IND/RI) and scroll operations (SU/SD).
//! These operations respect DECSTBM scroll region margins and handle
//! cursor positioning as required by ANSI terminal emulation standards.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{RowHeight, len};

impl OffscreenBuffer {
    /// Move cursor down one line, scrolling the buffer if at bottom.
    /// Implements the ESC D (IND) escape sequence.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Index down at scroll region bottom triggers scroll
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A                              │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D  ← cursor at scroll_bottom   │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After IND:
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ Line B (moved up)                   │
    ///              │  2  │ Line C (moved up)                   │
    ///              │  3  │ Line D (moved up)                   │
    ///              │  4  │ (blank line)  ← cursor stays here   │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: Buffer scrolled up, cursor stays at scroll region bottom
    /// ```
    pub fn index_down(&mut self) {
        let current_row = /* 0-based */ self.cursor_pos.row_index;

        // Get bottom boundary of scroll region using helper method.
        let scroll_bottom_boundary = self.get_scroll_bottom_boundary();

        // Check if we're at the bottom of the scroll region.
        if current_row >= scroll_bottom_boundary {
            // At scroll region bottom - scroll buffer content up by one line.
            self.scroll_buffer_up();
        } else {
            // Not at scroll region bottom - just move cursor down.
            self.cursor_down(RowHeight::from(1));
        }
    }

    /// Move cursor up one line, scrolling the buffer if at top.
    /// Implements the ESC M (RI) escape sequence.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Reverse index up at scroll region top triggers scroll
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A  ← cursor at scroll_top       │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After RI:
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ (blank line)  ← cursor stays here   │
    ///              │  2  │ Line A (moved down)                 │
    ///              │  3  │ Line B (moved down)                 │
    ///              │  4  │ Line C (moved down)                 │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: Buffer scrolled down, cursor stays at scroll region top
    /// ```
    pub fn reverse_index_up(&mut self) {
        let current_row = /* 0-based */ self.cursor_pos.row_index;

        // Get top boundary of scroll region using helper method.
        let scroll_top_boundary = self.get_scroll_top_boundary();

        // Check if we're at the top of the scroll region.
        if current_row <= scroll_top_boundary {
            // At scroll region top - scroll buffer content down by one line.
            self.scroll_buffer_down();
        } else {
            // Not at scroll region top - just move cursor up.
            self.cursor_up(RowHeight::from(1));
        }
    }

    /// Scroll buffer content up by one line (for ESC D at bottom).
    /// The top line is lost, and a new empty line appears at bottom.
    /// Respects DECSTBM scroll region margins.
    /// See [`crate::OffscreenBuffer::shift_lines_up`] for detailed behavior and examples.
    pub fn scroll_buffer_up(&mut self) {
        // Get scroll region boundaries using helper methods.
        let scroll_top = self.get_scroll_top_boundary();
        let scroll_bottom = self.get_scroll_bottom_boundary();

        // Use shift_lines_up to shift lines up within the scroll region.
        self.shift_lines_up(
            {
                let start = scroll_top;
                let end = scroll_bottom + 1;
                start..end
            },
            len(1),
        );
    }

    /// Scroll buffer content down by one line (for ESC M at top).
    /// The bottom line is lost, and a new empty line appears at top.
    /// Respects DECSTBM scroll region margins.
    /// See [`crate::OffscreenBuffer::shift_lines_down`] for detailed behavior and examples.
    pub fn scroll_buffer_down(&mut self) {
        // Get scroll region boundaries using helper methods.
        let scroll_top = self.get_scroll_top_boundary();
        let scroll_bottom = self.get_scroll_bottom_boundary();

        // Use shift_lines_down to shift lines down within the scroll region.
        self.shift_lines_down(
            {
                let start = scroll_top;
                let end = scroll_bottom + 1;
                start..end
            },
            len(1),
        );
    }

    /// Handle SU (Scroll Up) - scroll display up by n lines.
    /// Multiple lines at the top are lost, new empty lines appear at bottom.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Scrolling up by 2 lines
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A (will be lost)               │
    ///              │  2  │ Line B (will be lost)               │
    ///              │  3  │ Line C                              │
    ///              │  4  │ Line D                              │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After scroll_up(2):
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ Line C (moved up 2)                 │
    ///              │  2  │ Line D (moved up 2)                 │
    ///              │  3  │ (blank line)                        │
    ///              │  4  │ (blank line)                        │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: 2 lines scrolled up, Lines A and B lost, 2 blank lines added at bottom
    /// ```
    pub fn scroll_up(&mut self, how_many: RowHeight) {
        for _ in 0..how_many.as_u16() {
            self.scroll_buffer_up();
        }
    }

    /// Handle SD (Scroll Down) - scroll display down by n lines.
    /// Multiple lines at the bottom are lost, new empty lines appear at top.
    /// Respects DECSTBM scroll region margins.
    ///
    /// Example - Scrolling down by 2 lines
    ///
    /// ```text
    /// Before:        Row: 0-based
    /// max_height=6 ╮  ▼  ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤ ← scroll_top (row 1, 0-based)
    ///              │  1  │ Line A                              │
    ///              │  2  │ Line B                              │
    ///              │  3  │ Line C (will be lost)               │
    ///              │  4  │ Line D (will be lost)               │
    ///              │     ├─────────────────────────────────────┤ ← scroll_bottom (row 4, 0-based)
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// After scroll_down(2):
    /// max_height=6 ╮     ┌─────────────────────────────────────┐
    /// (1-based)    │  0  │ Header line (outside scroll region) │
    ///              │     ├─────────────────────────────────────┤
    ///              │  1  │ (blank line)                        │
    ///              │  2  │ (blank line)                        │
    ///              │  3  │ Line A (moved down 2)               │
    ///              │  4  │ Line B (moved down 2)               │
    ///              │     ├─────────────────────────────────────┤
    ///              ╰  5  │ Footer line (outside scroll region) │
    ///                    └─────────────────────────────────────┘
    ///
    /// Result: 2 lines scrolled down, Lines C and D lost, 2 blank lines added at top
    /// ```
    pub fn scroll_down(&mut self, how_many: RowHeight) {
        for _ in 0..how_many.as_u16() {
            self.scroll_buffer_down();
        }
    }
}