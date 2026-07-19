// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of [`CanvasStorage`] for [`Flat2DArray<PixelChar>`].
//!
//! This module provides a fixed-size 2D buffer backend for [`OfsBuf`], backed by a
//! contiguous [`Flat2DArray`]. Unlike [`GrowableBuffer`], [`Flat2DArray`] has a fixed
//! viewport with no panning or scrollback history support.
//!
//! [`CanvasStorage`]: CanvasStorage
//! [`Flat2DArray<PixelChar>`]: Flat2DArray
//! [`Flat2DArray`]: crate::core::Flat2DArray
//! [`GrowableBuffer`]: super::GrowableBuffer
//! [`OfsBuf`]: crate::tui::OfsBuf

use super::super::CanvasStorage;
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CPos, NarrowingCastToU16, PixelChar,
            RangeConstructExt, RangeExclusive, ShiftLinesDirection, VPHeight, VPLength,
            VPRow, VPSize, VPWidth, Viewport, ViewportPanValidity, ViewportToCanvasExt,
            c_pos, core::common::flat_2d_array::Flat2DArray, vp_height, vp_row};

impl CanvasStorage for Flat2DArray<PixelChar> {
    fn get_viewport(&self) -> Viewport {
        Viewport::from((
            c_pos(0usize, 0usize),
            VPSize::new((
                VPWidth::from(self.width.as_usize().as_u16_narrowing()),
                VPHeight::from(self.height.as_usize().as_u16_narrowing()),
            )),
        ))
    }

    /// [`Flat2DArray`] has a fixed viewport with no panning support.
    fn try_pan_viewport_to(
        &mut self,
        _origin_pos: CPos,
    ) -> Result<(), ViewportPanValidity> {
        unimplemented!("Flat2DArray does not support viewport panning");
    }

    fn get_row(&self, row: VPRow) -> Option<&[PixelChar]> {
        let r = self.get_viewport().to_canvas(row);
        match r.overflows(self.height) {
            ArrayOverflowResult::Within => Some(&self[r]),
            ArrayOverflowResult::Overflowed => None,
        }
    }

    fn get_row_mut(&mut self, row: VPRow) -> Option<&mut [PixelChar]> {
        let r = self.get_viewport().to_canvas(row);
        match r.overflows(self.height) {
            ArrayOverflowResult::Within => Some(&mut self[r]),
            ArrayOverflowResult::Overflowed => None,
        }
    }

    fn shift_lines_in_range(
        &mut self,
        direction: ShiftLinesDirection,
        row_index_range: RangeExclusive<VPRow>,
        amount: VPLength,
        fill_char: PixelChar,
    ) {
        let mapped_range = self.get_viewport().to_canvas(row_index_range);
        let mapped_range = mapped_range.start..mapped_range.end;
        match direction {
            ShiftLinesDirection::Up => {
                self.as_simd_mut()
                    .shift_rows_up(mapped_range, amount, fill_char);
            }
            ShiftLinesDirection::Down => {
                self.as_simd_mut()
                    .shift_rows_down(mapped_range, amount, fill_char);
            }
        }
    }

    fn allocate_new_lines_at_bottom(
        &mut self,
        arg_amount: impl Into<VPLength>,
        fill_char: PixelChar,
    ) {
        let amount: VPLength = arg_amount.into();
        self.shift_lines_in_range(
            ShiftLinesDirection::Up,
            (
                vp_row(0),
                vp_height(self.height.as_usize().as_u16_narrowing()),
            )
                .to_exclusive_range(),
            amount,
            fill_char,
        );
    }

    fn clear_viewport_with(&mut self, fill_char: PixelChar) {
        self.as_simd_mut().fill_all(fill_char);
    }

    fn fill_row_range(
        &mut self,
        row_index_range: RangeExclusive<VPRow>,
        fill_char: PixelChar,
    ) {
        let mapped_range = self.get_viewport().to_canvas(row_index_range);
        self.as_simd_mut()
            .fill_rows(mapped_range.start..mapped_range.end, fill_char);
    }

    fn swap_lines(
        &mut self,
        row_index_1: VPRow,
        row_index_2: VPRow,
    ) -> miette::Result<()> {
        let vp = self.get_viewport();
        let r1 = vp.to_canvas(row_index_1);
        let r2 = vp.to_canvas(row_index_2);

        if r1.overflows(self.height) == ArrayOverflowResult::Overflowed
            || r2.overflows(self.height) == ArrayOverflowResult::Overflowed
        {
            return Err(miette::miette!("Row index out of bounds"));
        }
        self.as_simd_mut().swap_rows(r1, r2);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CHeight, CSize, CWidth, TuiStyle, c_row, vp_col, vp_row};

    fn create_test_grid() -> Flat2DArray<PixelChar> {
        Flat2DArray::new_empty(
            CSize::from((CWidth::from(4usize), CHeight::from(4usize))),
            PixelChar::PlainText {
                display_char: ' ',
                style: TuiStyle::default(),
            },
        )
    }

    fn create_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_get_row_and_get_row_mut_bounds() {
        let mut grid = create_test_grid();

        // Valid rows within viewport bounds
        assert!(grid.get_row(vp_row(0)).is_some());
        assert!(grid.get_row(vp_row(3)).is_some());
        assert!(grid.get_row(vp_row(4)).is_none());

        // Mutate valid row
        if let Some(row) = grid.get_row_mut(vp_row(1)) {
            row[0] = create_char('X');
        }

        // Verify mutation
        let row = grid.get_row(vp_row(1)).unwrap();
        assert_eq!(row[0], create_char('X'));
    }

    #[test]
    fn test_fill_row_range_and_swap_lines() -> miette::Result<()> {
        let mut grid = create_test_grid();
        let fill_x = create_char('X');

        // Fill row 1 with 'X'
        grid.fill_row_range(vp_row(1)..vp_row(2), fill_x);
        assert_eq!(grid[c_row(1)][0], fill_x);

        // Swap row 1 and row 2
        grid.swap_lines(vp_row(1), vp_row(2))?;
        assert_eq!(grid[c_row(2)][0], fill_x);

        // Swapping out-of-bounds row returns an error
        assert!(grid.swap_lines(vp_row(1), vp_row(10)).is_err());

        Ok(())
    }

    #[test]
    fn test_viewport_to_canvas_translation_and_2d_cell_ops() {
        let mut grid = create_test_grid();
        let char_val = create_char('Z');

        // Verify get_row_mut via CanvasStorage adapter
        if let Some(row) = grid.get_row_mut(vp_row(2)) {
            row[vp_col(3).as_usize()] = char_val;
        }

        let row = grid.get_row(vp_row(2)).unwrap();
        assert_eq!(row[3], char_val);

        // Direct underlying Flat2DArray verification in Canvas domain
        assert_eq!(grid[c_row(2)][3], char_val);
    }
}
