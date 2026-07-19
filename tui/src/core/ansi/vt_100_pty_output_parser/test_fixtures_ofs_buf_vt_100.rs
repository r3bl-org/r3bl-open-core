// Copyright (c) 2022-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ActiveScreenBuffer, List, OfsBufVT100, PixelChar, PixelCharDiffChunks,
            RangeExclusive, RangeExt, VPCol, VPHeight, VPPos, VPRow, ok, vp_height,
            vp_width};
use std::cmp::min;

impl OfsBufVT100 {
    #[must_use]
    pub fn get_cursor_vp_pos(&self) -> VPPos {
        self.get_active_screen_buffer().get_cursor_vp_pos()
    }

    #[must_use]
    pub fn get_cursor_pos(&self) -> VPPos { self.get_cursor_vp_pos() }

    pub fn set_cursor_vp_pos(&mut self, vp_pos: VPPos) {
        self.get_active_screen_buffer_mut()
            .set_cursor_vp_pos(vp_pos);
    }

    pub fn set_cursor_pos(&mut self, vp_pos: VPPos) { self.set_cursor_vp_pos(vp_pos); }

    pub fn update_cursor_vp_pos(&mut self, mut fn_mut: impl FnMut(&mut VPPos)) {
        let mut pos = self.get_active_screen_buffer().get_cursor_vp_pos();
        fn_mut(&mut pos);
        self.get_active_screen_buffer_mut().set_cursor_vp_pos(pos);
    }

    pub fn update_cursor_pos(&mut self, fn_mut: impl FnMut(&mut VPPos)) {
        self.update_cursor_vp_pos(fn_mut);
    }

    #[must_use]
    pub fn get_row(&self, vp_row: VPRow) -> Option<&[PixelChar]> {
        self.get_active_screen_buffer().get_row(vp_row)
    }

    pub fn get_row_mut(&mut self, vp_row: VPRow) -> Option<&mut [PixelChar]> {
        self.get_active_screen_buffer_mut().get_row_mut(vp_row)
    }

    #[must_use]
    pub fn get_char(&self, vp_pos: VPPos) -> Option<PixelChar> {
        self.get_active_screen_buffer().get_char(vp_pos)
    }

    #[must_use]
    pub fn get_height(&self) -> VPHeight {
        self.get_active_screen_buffer().get_viewport().get_height()
    }

    #[must_use]
    pub fn get_line(&self, row: VPRow) -> Option<&[PixelChar]> {
        self.get_active_screen_buffer().get_row(row)
    }

    /// # Errors
    ///
    /// Returns an error if the row is out of bounds.
    pub fn set_line(&mut self, row: VPRow, line: &[PixelChar]) -> miette::Result<()> {
        let Some(slice) = self.get_active_screen_buffer_mut().get_row_mut(row) else {
            return Err(miette::miette!("out of bounds"));
        };

        let copy_len = min(line.len(), slice.len());
        slice[..copy_len].copy_from_slice(&line[..copy_len]);

        ok!()
    }

    pub fn apply_changes(&mut self, changes: Vec<(VPPos, PixelChar)>) -> usize {
        match self.get_terminal_mode().active_screen_buffer {
            ActiveScreenBuffer::Primary => {
                self.primary_buffer_mut().apply_changes(changes)
            }
            ActiveScreenBuffer::Alternate => {
                self.get_alternate_buffer_mut().apply_changes(changes)
            }
        }
    }

    /// # Errors
    ///
    /// Returns an error if the position is out of bounds.
    pub fn set_char(&mut self, pos: VPPos, char: PixelChar) -> miette::Result<()> {
        self.get_active_screen_buffer_mut().set_char(pos, char)
    }

    /// # Errors
    ///
    /// Returns an error if either row is out of bounds.
    pub fn swap_lines(&mut self, row1: VPRow, row2: VPRow) -> miette::Result<()> {
        match self.get_terminal_mode().active_screen_buffer {
            ActiveScreenBuffer::Primary => {
                self.primary_buffer_mut().swap_lines(row1, row2)
            }
            ActiveScreenBuffer::Alternate => {
                self.get_alternate_buffer_mut().swap_lines(row1, row2)
            }
        }
    }

    /// # Errors
    ///
    /// Returns an error if the range is out of bounds.
    pub fn fill_char_range(
        &mut self,
        row: VPRow,
        col_range: RangeExclusive<VPCol>,
        char: PixelChar,
    ) -> miette::Result<()> {
        self.get_active_screen_buffer_mut()
            .fill_char_range(row, col_range, char)
    }

    /// # Errors
    ///
    /// Returns an error if the range is out of bounds.
    pub fn copy_chars_within_line(
        &mut self,
        row: VPRow,
        source_range: RangeExclusive<VPCol>,
        dest_start: VPCol,
    ) -> miette::Result<()> {
        self.get_active_screen_buffer_mut().copy_chars_within_line(
            row,
            source_range,
            dest_start,
        )
    }

    #[must_use]
    pub fn diff(&self, other: &Self) -> Option<PixelCharDiffChunks> {
        use List;

        let self_vp = self.get_active_screen_buffer().get_viewport();
        let other_vp = other.get_active_screen_buffer().get_viewport();
        if self_vp.get_width() != other_vp.get_width()
            || self_vp.get_height() != other_vp.get_height()
        {
            return None;
        }
        let row_range = ..vp_height(self_vp.get_height());
        let col_range = ..vp_width(self_vp.get_width());
        let mut diffs = List::default();
        for r in row_range.as_index_iter() {
            for c in col_range.as_index_iter() {
                let pos = c + r;
                let char1 = self.get_active_screen_buffer().get_char(pos);
                let char2 = other.get_active_screen_buffer().get_char(pos);
                if char1 != char2 {
                    diffs.push((pos, char2.unwrap_or(PixelChar::Spacer)));
                }
            }
        }
        Some(PixelCharDiffChunks { inner: diffs })
    }
}
