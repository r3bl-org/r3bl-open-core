// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.
//! ANSI terminal-specific bounds checking operations for `OffscreenBuffer`.
//!
//! This module provides helper methods for common bounds checking patterns
//! used by ANSI escape sequence operations, including scroll region boundaries,
//! cursor position clamping, and terminal dimension conversions.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{BoundsOverflowStatus::Overflowed,
            ColIndex, ColWidth, RowHeight, RowIndex,
            core::{pty_mux::ansi_parser::term_units::TermRow,
                   units::bounds_check::BoundsCheck},
            row};

impl OffscreenBuffer {
    /// Get the top boundary of the scroll region (0 if no region set).
    ///
    /// This resolves the ANSI parser's scroll region top boundary, converting
    /// from 1-based ANSI coordinates to 0-based buffer indices.
    pub fn get_scroll_top_boundary(&self) -> RowIndex {
        self.ansi_parser_support
            .scroll_region_top
            .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
            .map_or(/* None */ row(0), /* Some */ Into::into)
    }

    /// Get the bottom boundary of the scroll region (screen bottom if no region set).
    ///
    /// This resolves the ANSI parser's scroll region bottom boundary, converting
    /// from 1-based ANSI coordinates to 0-based buffer indices.
    pub fn get_scroll_bottom_boundary(&self) -> RowIndex {
        self.ansi_parser_support
            .scroll_region_bottom
            .and_then(TermRow::to_zero_based) // Convert 1 to 0 based
            .map_or(
                /* None */ self.window_size.row_height.convert_to_row_index(),
                /* Some */ Into::into,
            )
    }

    /// Clamp a column index to the valid range [0, `max_col_index`].
    ///
    /// Clamp to `max_col-1` if it would overflow. This ensures column positions stay
    /// within the terminal width, using type-safe overflow checking.
    #[must_use]
    pub fn clamp_column(&self, max_col: ColIndex) -> ColIndex {
        if max_col.check_overflows(self.window_size.col_width) == Overflowed {
            self.window_size.col_width.convert_to_col_index()
        } else {
            max_col
        }
    }

    /// Get the maximum valid column index (0-based).
    ///
    /// This converts the 1-based column width to the maximum valid 0-based index.
    #[must_use]
    pub fn max_col_index(&self) -> ColIndex {
        self.window_size.col_width.convert_to_col_index()
    }

    /// Clamp a row to stay within the scroll region boundaries.
    ///
    /// This ensures row positions respect ANSI scroll region settings,
    /// keeping the cursor within the defined scrollable area.
    #[must_use]
    pub fn clamp_row_to_scroll_region(&self, row: RowIndex) -> RowIndex {
        let top = self.get_scroll_top_boundary();
        let bottom = self.get_scroll_bottom_boundary();

        if row < top {
            top
        } else if row > bottom {
            bottom
        } else {
            row
        }
    }

    /// Get the maximum valid row index (0-based).
    ///
    /// This converts the 1-based row height to the maximum valid 0-based index.
    #[must_use]
    pub fn max_row_index(&self) -> RowIndex {
        self.window_size.row_height.convert_to_row_index()
    }

    /// Move cursor forward, clamping to screen width.
    ///
    /// This updates the cursor position while ensuring it doesn't exceed
    /// the terminal width using type-safe bounds checking.
    pub fn move_cursor_forward(&mut self, amount: ColWidth) {
        let new_col = self.cursor_pos.col_index + amount;
        self.cursor_pos.col_index = self.clamp_column(new_col);
    }

    /// Move cursor backward, stopping at column 0.
    ///
    /// This updates the cursor position while ensuring it doesn't go
    /// below column 0 using type-safe underflow protection.
    pub fn move_cursor_backward(&mut self, amount: ColWidth) {
        self.cursor_pos.col_index -= amount;
    }

    /// Move cursor up, respecting scroll region boundaries.
    ///
    /// This updates the cursor position while ensuring it stays within
    /// the current scroll region using ANSI-compliant boundary checking.
    pub fn move_cursor_up(&mut self, amount: RowHeight) {
        let new_row = self.cursor_pos.row_index - amount;
        self.cursor_pos.row_index = self.clamp_row_to_scroll_region(new_row);
    }

    /// Move cursor down, respecting scroll region boundaries.
    ///
    /// This updates the cursor position while ensuring it stays within
    /// the current scroll region using ANSI-compliant boundary checking.
    pub fn move_cursor_down(&mut self, amount: RowHeight) {
        let new_row = self.cursor_pos.row_index + amount;
        self.cursor_pos.row_index = self.clamp_row_to_scroll_region(new_row);
    }
}
