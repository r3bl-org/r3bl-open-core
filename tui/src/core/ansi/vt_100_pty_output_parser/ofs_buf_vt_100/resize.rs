// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AlternateBuffer, OfsBufVT100};
use crate::{Flat2DArray, OfsBuf, PixelChar, VPPos, VPSize, vp_col, vp_row};
use std::cmp::min;

impl OfsBufVT100 {
    /// Resizes both screen buffers to `new_size`, keeping what fits.
    ///
    /// The primary buffer keeps its scrollback history and the cursor's line (see
    /// [`GrowableBuffer::resize()`]). The alternate buffer keeps its top left corner.
    /// Cursors are clamped to the new size, and the scroll region and any pending wrap
    /// are reset, as a terminal does when its window changes size.
    ///
    /// [`GrowableBuffer::resize()`]: crate::GrowableBuffer::resize
    pub fn resize(&mut self, new_size: VPSize) {
        if self.primary_buffer.get_window_size() == new_size {
            return;
        }

        let cursor = self.primary_buffer.get_cursor_pos();
        let cursor_row = self.primary_buffer.get_storage_mut().resize(
            new_size,
            cursor.row_index,
            PixelChar::Spacer,
        );
        let cursor = clamp_pos(cursor, new_size);
        self.primary_buffer.set_cursor_pos(VPPos {
            row_index: cursor_row,
            col_index: cursor.col_index,
        });

        self.alternate_buffer = resized_alternate(&self.alternate_buffer, new_size);

        let state = &mut self.parser_global_state;
        state.scroll_region_top = None;
        state.scroll_region_bottom = None;
        state.clear_pending_wrap();
        if let Some(saved) = state.cursor_pos_for_esc_save_and_restore {
            state.cursor_pos_for_esc_save_and_restore = Some(clamp_pos(saved, new_size));
        }
    }
}

fn resized_alternate(old: &AlternateBuffer, new_size: VPSize) -> AlternateBuffer {
    let mut new = OfsBuf::new(Flat2DArray::new_empty(new_size, PixelChar::Spacer));
    let rows = min(
        old.get_window_size().row_height.as_usize(),
        new_size.row_height.as_usize(),
    );
    for row in 0..rows {
        let (Some(src), Some(dst)) =
            (old.get_row(vp_row(row)), new.get_row_mut(vp_row(row)))
        else {
            continue;
        };
        let cols = min(src.len(), dst.len());
        dst[..cols].copy_from_slice(&src[..cols]);
    }
    new.set_cursor_pos(clamp_pos(old.get_cursor_pos(), new_size));
    new
}

fn clamp_pos(pos: VPPos, size: VPSize) -> VPPos {
    let max_row = size.row_height.as_usize().saturating_sub(1);
    let max_col = size.col_width.as_usize().saturating_sub(1);
    VPPos {
        row_index: vp_row(min(pos.row_index.as_usize(), max_row)),
        col_index: vp_col(min(pos.col_index.as_usize(), max_col)),
    }
}
