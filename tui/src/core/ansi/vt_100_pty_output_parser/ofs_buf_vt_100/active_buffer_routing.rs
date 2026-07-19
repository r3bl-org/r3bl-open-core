// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! To avoid writing `match` statements everywhere in the parser code, this module groups
//! the [`OfsBufOpsVT100`] trait and its implementations.

use super::{AlternateBuffer, OfsBufVT100, PrimaryBuffer};
use crate::{ActiveScreenBuffer, CanvasStorage, DEBUG_TUI_VT100_PARSER,
            NarrowingCastToU16, PixelChar, RangeExclusive, ScrollbackAmount,
            ShiftLinesDirection, VPCol, VPLength, VPPos, VPRow, VPSize, VPWidth,
            Viewport, ViewportPanValidity, vp_col};

impl OfsBufVT100 {
    /// Returns a reference to the currently active screen buffer (either the
    /// [`Self::get_primary_buffer()`] or [`Self::get_alternate_buffer()`]), based on the
    /// current [`ActiveScreenBuffer`] state. This abstracts away the multiplexing between
    /// the two buffers, allowing terminal operations to be applied to the correct buffer
    /// transparently.
    #[must_use]
    pub fn get_active_screen_buffer(&self) -> &dyn OfsBufOpsVT100 {
        match self.terminal_mode.active_screen_buffer {
            ActiveScreenBuffer::Primary => &self.primary_buffer,
            ActiveScreenBuffer::Alternate => &self.alternate_buffer,
        }
    }

    /// See [`Self::get_active_screen_buffer()`].
    pub fn get_active_screen_buffer_mut(&mut self) -> &mut dyn OfsBufOpsVT100 {
        match self.terminal_mode.active_screen_buffer {
            ActiveScreenBuffer::Primary => &mut self.primary_buffer,
            ActiveScreenBuffer::Alternate => &mut self.alternate_buffer,
        }
    }

    /// Returns the number of lines in the primary buffer's scrollback history.
    ///
    /// If the terminal is currently in the alternate screen, this returns `0` since the
    /// alternate screen does not have a scrollback history.
    #[must_use]
    pub fn get_history_len(&self) -> usize {
        match self.terminal_mode.active_screen_buffer {
            ActiveScreenBuffer::Primary => {
                self.primary_buffer.get_viewport().get_history_len()
            }
            ActiveScreenBuffer::Alternate => 0,
        }
    }

    /// Pans the primary buffer's viewport to the left by the given `amount`.
    ///
    /// This has no effect if the terminal is currently in the alternate screen, as it
    /// does not support horizontal panning.
    pub fn pan_viewport_left(&mut self, amount: VPWidth) {
        if self.terminal_mode.active_screen_buffer == ActiveScreenBuffer::Alternate {
            return;
        }
        let mut pos_copy = self.primary_buffer.get_viewport().get_origin_pos();
        pos_copy.col_index -= amount.as_usize();
        if let Err(ViewportPanValidity::InvalidVerticalPan {
            expected_row,
            requested_row,
        }) = self.primary_buffer.try_pan_viewport_to(pos_copy)
        {
            DEBUG_TUI_VT100_PARSER.then(|| {
                // % is Display, ? is Debug.
                tracing::error! {
                    message = "active_buffer_routing::pan_viewport_left failed",
                    expected_row = ?expected_row,
                    requested_row = ?requested_row,
                };
            });
        }
    }

    /// Pans the primary buffer's viewport to the right by the given `amount`.
    ///
    /// This has no effect if the terminal is currently in the alternate screen, as it
    /// does not support horizontal panning.
    pub fn pan_viewport_right(&mut self, amount: VPWidth) {
        if self.terminal_mode.active_screen_buffer == ActiveScreenBuffer::Alternate {
            return;
        }
        let mut pos_copy = self.primary_buffer.get_viewport().get_origin_pos();
        pos_copy.col_index += amount.as_usize();
        if let Err(ViewportPanValidity::InvalidVerticalPan {
            expected_row,
            requested_row,
        }) = self.primary_buffer.try_pan_viewport_to(pos_copy)
        {
            DEBUG_TUI_VT100_PARSER.then(|| {
                // % is Display, ? is Debug.
                tracing::error! {
                    message = "active_buffer_routing::pan_viewport_right failed",
                    expected_row = ?expected_row,
                    requested_row = ?requested_row,
                };
            });
        }
    }

    /// Returns the current horizontal pan offset (column index) of the viewport.
    ///
    /// Returns `0` if the terminal is currently in the alternate screen, as it does not
    /// support horizontal panning.
    #[must_use]
    pub fn get_viewport_col_offset(&self) -> VPCol {
        match self.terminal_mode.active_screen_buffer {
            ActiveScreenBuffer::Primary => vp_col(
                self.primary_buffer
                    .get_viewport()
                    .get_origin_pos()
                    .col_index
                    .as_usize()
                    .as_u16_narrowing(),
            ),
            ActiveScreenBuffer::Alternate => vp_col(0),
        }
    }

    /// Clears the viewport (visible area) of both the primary and alternate screen
    /// buffers by replacing all characters with [`PixelChar::Spacer`].
    ///
    /// This does not clear the scrollback history of the primary buffer.
    pub fn clear(&mut self) {
        self.primary_buffer.clear_viewport_with(PixelChar::Spacer);
        self.alternate_buffer.clear_viewport_with(PixelChar::Spacer);
    }

    /// Returns the window size ([`VPSize`]) of the currently active screen buffer.
    #[must_use]
    pub fn get_window_size(&self) -> VPSize {
        let vp = self.get_active_screen_buffer().get_viewport();
        vp.get_size()
    }
}

/// Defines all operations that can be performed on an active screen buffer. The structs
/// that impl this trait are [`PrimaryBuffer`] and [`AlternateBuffer`].
///
/// The [`VT-100`] parser constantly switches between the primary screen buffer (supported
/// by [`GrowableBuffer`]) and the alternate screen buffer (supported by [`Flat2DArray`]).
///
/// Rather than writing `match` blocks for every screen operation, the coordinator struct
/// ([`OfsBufVT100`]) defines exactly two helper methods: [`get_active_screen_buffer()`]
/// and [`get_active_screen_buffer_mut()`]. These methods contain the only `match` blocks
/// in the system, and they return one of the following trait objects:
/// - `&dyn OfsBufOpsVT100`
/// - `&mut dyn OfsBufOpsVT100`
///
/// The rest of the parser code simply queries these helpers to obtain the trait object,
/// then calls the operations directly on it. The Rust runtime then dynamically dispatches
/// the calls to the correct concrete buffer implementation, avoiding duplicate matching
/// logic.
///
/// [`Flat2DArray`]: crate::core::Flat2DArray
/// [`get_active_screen_buffer()`]: OfsBufVT100::get_active_screen_buffer
/// [`get_active_screen_buffer_mut()`]: OfsBufVT100::get_active_screen_buffer_mut
/// [`GrowableBuffer`]: crate::tui::GrowableBuffer
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
pub trait OfsBufOpsVT100 {
    #[must_use]
    fn get_cursor_vp_pos(&self) -> VPPos;

    fn set_cursor_vp_pos(&mut self, vp_pos: VPPos);

    #[must_use]
    fn get_cursor_pos(&self) -> VPPos { self.get_cursor_vp_pos() }

    fn set_cursor_pos(&mut self, vp_pos: VPPos) { self.set_cursor_vp_pos(vp_pos); }

    #[must_use]
    fn get_viewport(&self) -> Viewport;

    /// Sets the character at the specified position.
    ///
    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
    fn set_char(&mut self, vp_pos: VPPos, char: PixelChar) -> miette::Result<()>;

    #[must_use]
    fn get_row(&self, vp_row: VPRow) -> Option<&[PixelChar]>;

    fn get_row_mut(&mut self, vp_row: VPRow) -> Option<&mut [PixelChar]>;

    #[must_use]
    fn get_row_with_scrollback(
        &self,
        vp_row: VPRow,
        scrollback_amt: ScrollbackAmount,
    ) -> Option<&[PixelChar]>;

    /// Copies a range of characters within the same line.
    ///
    /// # Errors
    ///
    /// Returns an error if the range or destination is out of bounds.
    fn copy_chars_within_line(
        &mut self,
        vp_row: VPRow,
        source_range: RangeExclusive<VPCol>,
        dest_start: VPCol,
    ) -> miette::Result<()>;

    fn shift_lines_in_range(
        &mut self,
        direction: ShiftLinesDirection,
        row_range: RangeExclusive<VPRow>,
        amount: VPLength,
        empty_char: PixelChar,
    );

    fn allocate_new_lines_at_bottom(&mut self, amount: VPLength, empty_char: PixelChar);

    fn clear_viewport_with(&mut self, empty_char: PixelChar);

    fn fill_row_range(&mut self, row_range: RangeExclusive<VPRow>, empty_char: PixelChar);

    /// Fills a range of characters in the specified row.
    ///
    /// # Errors
    ///
    /// Returns an error if the range is out of bounds.
    fn fill_char_range(
        &mut self,
        vp_row: VPRow,
        col_range: RangeExclusive<VPCol>,
        fill_char: PixelChar,
    ) -> miette::Result<()>;

    #[must_use]
    fn get_char(&self, vp_pos: VPPos) -> Option<PixelChar> {
        let row_slice = self.get_row(vp_pos.row_index)?;
        let item = row_slice.get(vp_pos.col_index.as_usize())?;
        Some(*item)
    }
}

impl OfsBufOpsVT100 for PrimaryBuffer {
    fn get_cursor_vp_pos(&self) -> VPPos { self.get_cursor_vp_pos() }

    fn set_cursor_vp_pos(&mut self, vp_pos: VPPos) { self.set_cursor_vp_pos(vp_pos); }

    fn get_viewport(&self) -> Viewport { self.get_storage().get_viewport() }

    fn set_char(&mut self, vp_pos: VPPos, char: PixelChar) -> miette::Result<()> {
        self.set_char(vp_pos, char)
    }

    fn get_row(&self, vp_row: VPRow) -> Option<&[PixelChar]> { self.get_row(vp_row) }

    fn get_row_mut(&mut self, vp_row: VPRow) -> Option<&mut [PixelChar]> {
        self.get_row_mut(vp_row)
    }

    fn get_row_with_scrollback(
        &self,
        vp_row: VPRow,
        scrollback_amt: ScrollbackAmount,
    ) -> Option<&[PixelChar]> {
        self.get_storage()
            .get_row_with_scrollback(vp_row, scrollback_amt)
    }

    fn copy_chars_within_line(
        &mut self,
        vp_row: VPRow,
        source_range: RangeExclusive<VPCol>,
        dest_start: VPCol,
    ) -> miette::Result<()> {
        self.copy_chars_within_line(vp_row, source_range, dest_start)
    }

    fn shift_lines_in_range(
        &mut self,
        direction: ShiftLinesDirection,
        row_range: RangeExclusive<VPRow>,
        amount: VPLength,
        empty_char: PixelChar,
    ) {
        self.get_storage_mut()
            .shift_lines_in_range(direction, row_range, amount, empty_char);
    }

    fn allocate_new_lines_at_bottom(&mut self, amount: VPLength, empty_char: PixelChar) {
        self.get_storage_mut()
            .allocate_new_lines_at_bottom(amount, empty_char);
    }

    fn clear_viewport_with(&mut self, empty_char: PixelChar) {
        self.get_storage_mut().clear_viewport_with(empty_char);
    }

    fn fill_row_range(
        &mut self,
        row_range: RangeExclusive<VPRow>,
        empty_char: PixelChar,
    ) {
        self.get_storage_mut().fill_row_range(row_range, empty_char);
    }

    fn fill_char_range(
        &mut self,
        vp_row: VPRow,
        col_range: RangeExclusive<VPCol>,
        fill_char: PixelChar,
    ) -> miette::Result<()> {
        self.fill_char_range(vp_row, col_range, fill_char)
    }
}

impl OfsBufOpsVT100 for AlternateBuffer {
    fn get_cursor_vp_pos(&self) -> VPPos { self.get_cursor_vp_pos() }

    fn set_cursor_vp_pos(&mut self, vp_pos: VPPos) { self.set_cursor_vp_pos(vp_pos); }

    fn get_viewport(&self) -> Viewport { self.get_storage().get_viewport() }

    fn set_char(&mut self, vp_pos: VPPos, char: PixelChar) -> miette::Result<()> {
        self.set_char(vp_pos, char)
    }

    fn get_row(&self, vp_row: VPRow) -> Option<&[PixelChar]> { self.get_row(vp_row) }

    fn get_row_mut(&mut self, vp_row: VPRow) -> Option<&mut [PixelChar]> {
        self.get_row_mut(vp_row)
    }

    fn get_row_with_scrollback(
        &self,
        vp_row: VPRow,
        _scrollback_amt: ScrollbackAmount,
    ) -> Option<&[PixelChar]> {
        self.get_row(vp_row)
    }

    fn copy_chars_within_line(
        &mut self,
        vp_row: VPRow,
        source_range: RangeExclusive<VPCol>,
        dest_start: VPCol,
    ) -> miette::Result<()> {
        self.copy_chars_within_line(vp_row, source_range, dest_start)
    }

    fn shift_lines_in_range(
        &mut self,
        direction: ShiftLinesDirection,
        row_range: RangeExclusive<VPRow>,
        amount: VPLength,
        empty_char: PixelChar,
    ) {
        self.get_storage_mut()
            .shift_lines_in_range(direction, row_range, amount, empty_char);
    }

    fn allocate_new_lines_at_bottom(&mut self, amount: VPLength, empty_char: PixelChar) {
        self.get_storage_mut()
            .allocate_new_lines_at_bottom(amount, empty_char);
    }

    fn clear_viewport_with(&mut self, empty_char: PixelChar) {
        self.get_storage_mut().clear_viewport_with(empty_char);
    }

    fn fill_row_range(
        &mut self,
        row_range: RangeExclusive<VPRow>,
        empty_char: PixelChar,
    ) {
        self.get_storage_mut().fill_row_range(row_range, empty_char);
    }

    fn fill_char_range(
        &mut self,
        vp_row: VPRow,
        col_range: RangeExclusive<VPCol>,
        fill_char: PixelChar,
    ) -> miette::Result<()> {
        self.fill_char_range(vp_row, col_range, fill_char)
    }
}
