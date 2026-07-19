// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core types and implementation for [`OfsBuf`].
//!
//! This module defines the main `OfsBuf` struct, its core properties, lifecycle methods,
//! and primary delegation down to its `Flat2DArray` backing store.

use super::pixel_char::PixelChar;
use crate::{CanvasStorage, Flat2DArray, GetMemSize, RangeExt, VPHeight, VPPos, VPRow,
            VPSize, ch, fg_green, inline_string, vp_size};
use std::{fmt::{self, Debug},
          mem::size_of,
          ops::{Deref, DerefMut}};

/// Core terminal screen buffer structure with [`VT-100`]/[`ANSI`] support.
///
/// This struct represents the main terminal screen buffer as a continuous 2D grid (the
/// "Canvas" in the [Canvas and Viewport Concept]) where each cell maps directly to a
/// terminal screen position. It handles variable-width characters (like emoji) using
/// [`PixelChar::Void`] placeholders.
///
/// For comprehensive architectural overview and integration details, see the [module
/// documentation].
///
/// # Key Features
///
/// - **Dual Integration**: Works with both render pipeline and [`ANSI`] terminal
///   emulation.
/// - **Variable-ColWidth Support**: Proper handling of emoji and Unicode characters.
/// - **[`VT-100`] Compliance**: Full terminal specification compliance.
/// - **Performance Optimized**: Pre-calculated memory sizes and efficient operations.
///
/// # Architecture
///
/// [`OfsBuf`] serves as the core foundation for the [Canvas and Viewport Concept]
/// orchestrated by [`OfsBufVT100`]. While [`OfsBufVT100`] owns the actual [`VT-100`]
/// state machine and emulation logic, [`OfsBuf`] provides the physical grid mechanisms.
///
/// ## Dependency Injection ([`DI`])
///
/// This struct is implemented as a generic shell over a [`CanvasStorage`] backend
/// (`storage: S`):
/// - It natively owns all the complex 2D grid mathematics (e.g., [`diff()`]), cursor
///   bounds checking, and character placement logic (e.g., [`set_char()`],
///   [`copy_chars_within_line()`]).
/// - It delegates raw memory operations (allocating, fetching rows, shifting lines) to
///   the injected `S` ([`CanvasStorage`]).
///
/// This achieves zero duplication for grid operations across different memory backends,
/// such as:
/// - A fast, fixed-size contiguous slice ([`Flat2DArray`])
/// - An infinite, variable-width scrollback canvas ([`GrowableBuffer`])
///
/// ## Underlying Protocol Parser
///
/// - [`vt_100_pty_output_parser`]: The [`ANSI`] parser that processes [`PTY`] output and
///   updates the higher-level [`OfsBufVT100`] via [`apply_ansi_bytes`].
/// - [`AnsiToOfsBufPerformer`]: The [`vte`] [`Perform`] implementation that translates
///   [`ANSI`] sequences into terminal state operations.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]: crate::core::ansi::AnsiToOfsBufPerformer
/// [`apply_ansi_bytes`]: crate::core::ansi::OfsBufVT100::apply_ansi_bytes
/// [`copy_chars_within_line()`]: OfsBuf::copy_chars_within_line
/// [`DI`]: https://en.wikipedia.org/wiki/Dependency_injection
/// [`diff()`]: OfsBuf::diff
/// [`Flat2DArray`]: crate::core::common::flat_2d_array::Flat2DArray
/// [`GrowableBuffer`]: crate::tui::GrowableBuffer
/// [`OfsBufVT100::apply_ansi_bytes`]: crate::core::ansi::OfsBufVT100::apply_ansi_bytes
/// [`OfsBufVT100`]: crate::core::ansi::OfsBufVT100
/// [`Perform`]: vte::Perform
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`RenderOpCommon`]: enum@crate::tui::RenderOpCommon
/// [`set_char()`]: OfsBuf::set_char
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
/// [`vte`]: https://docs.rs/vte
/// [Canvas and Viewport Concept]: crate::tui::GrowableBuffer#canvas-and-viewport-concept
/// [module documentation]: super
#[derive(Clone, PartialEq)]
pub struct OfsBuf<S: CanvasStorage = Flat2DArray<PixelChar>> {
    storage: S,
    cursor_pos: VPPos,
}

impl<S: CanvasStorage> OfsBuf<S> {
    /// Accessor for the underlying storage backend (since the field is private).
    pub fn get_storage(&self) -> &S { &self.storage }

    /// Mutable accessor for the underlying storage backend (since the field is private).
    pub fn get_storage_mut(&mut self) -> &mut S { &mut self.storage }
}

impl<S: CanvasStorage> GetMemSize for OfsBuf<S> {
    fn get_mem_size(&self) -> usize { self.storage.get_mem_size() + size_of::<VPPos>() }
}

impl<S: CanvasStorage> Deref for OfsBuf<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target { &self.storage }
}

impl<S: CanvasStorage> DerefMut for OfsBuf<S> {
    /// Returns a mutable reference to the buffer.
    ///
    /// Code like the following will call this method:
    /// - `self.buffer[row][col] = something`
    /// - `self.buffer.get_mut(row)`
    /// - Any operation that goes through the `&mut self.buffer` dereference
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.storage }
}

impl<S: CanvasStorage> Debug for OfsBuf<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "window_size: {:?}, ", self.get_window_size())?;

        let height = *self.get_viewport().get_height();
        let row_range = ..height;
        for row_index in row_range.as_index_iter() {
            if let Some(row) = self.get_row(row_index.into()) {
                // Print row separator if needed (not the first item).
                if row_index > ch(0) {
                    writeln!(f)?;
                }

                // Print the row index (styled) in "this" line.
                writeln!(
                    f,
                    "{}",
                    fg_green(&inline_string!("row_index: {:?}", row_index))
                )?;

                // Print the row itself in the "next" line.
                write!(f, "{row:?}")?;
            }
        }

        writeln!(f)
    }
}

impl<S: CanvasStorage> OfsBuf<S> {
    /// Creates a new offscreen buffer with the specified storage backend.
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            cursor_pos: VPPos::default(),
        }
    }

    /// Returns the current cursor position in the viewport.
    #[must_use]
    pub fn get_cursor_vp_pos(&self) -> VPPos { self.cursor_pos }

    /// Returns the current cursor position in the viewport.
    #[must_use]
    pub fn get_cursor_pos(&self) -> VPPos { self.get_cursor_vp_pos() }

    /// Sets the current cursor position in the viewport.
    pub fn set_cursor_vp_pos(&mut self, vp_pos: VPPos) { self.cursor_pos = vp_pos; }

    /// Sets the current cursor position in the viewport.
    pub fn set_cursor_pos(&mut self, vp_pos: VPPos) { self.set_cursor_vp_pos(vp_pos); }

    /// Updates the current cursor position using a closure. This is useful if you just
    /// want to update the row or column index without needing to create a new
    /// [`VPPos`].
    pub fn update_cursor_vp_pos<F>(&mut self, f: F)
    where
        F: FnOnce(&mut VPPos),
    {
        f(&mut self.cursor_pos);
    }

    /// Updates the current cursor position using a closure.
    pub fn update_cursor_pos<F>(&mut self, f: F)
    where
        F: FnOnce(&mut VPPos),
    {
        self.update_cursor_vp_pos(f);
    }

    /// Returns the current window size of the offscreen buffer. Fast `O(1)` access to
    /// window size.
    pub fn get_window_size(&self) -> VPSize {
        let vp = self.get_viewport();
        vp_size(*vp.get_width(), *vp.get_height())
    }

    pub fn get_height(&self) -> VPHeight { self.get_viewport().get_height() }

    pub fn get_row(&self, row: VPRow) -> Option<&[PixelChar]> {
        self.storage.get_row(row)
    }

    pub fn get_row_mut(&mut self, row: VPRow) -> Option<&mut [PixelChar]> {
        self.storage.get_row_mut(row)
    }

    /// Make sure each line is full of empty chars.
    pub fn clear(&mut self) { self.clear_with(PixelChar::Spacer); }

    /// Make sure each line is full of the given char.
    pub fn clear_with(&mut self, char: PixelChar) { self.clear_viewport_with(char); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NarrowingCastToU16, TuiStyle, vp_col, vp_height, vp_row, vp_width};
    use std::mem::size_of;

    fn create_test_buffer() -> OfsBuf {
        let size = vp_height(3) + vp_width(4);
        OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer))
    }

    fn create_test_pixel_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_ofs_buf_instantiation() {
        let size = VPSize {
            col_width: vp_width(3),
            row_height: vp_height(2),
        };
        let buffer = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));
        assert_eq!(buffer.height.as_usize(), 2);
        assert_eq!(buffer.width.as_usize(), 3);

        // Check that all positions are initialized
        for pixel_char in buffer.as_simd().as_raw_slice() {
            assert_eq!(*pixel_char, PixelChar::Spacer);
        }
    }

    #[test]
    fn test_ofs_buf_new_empty_zero_size() {
        let size = vp_height(0) + vp_width(0);
        let buffer = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));

        assert_eq!(buffer.get_window_size(), size);
        assert!(
            buffer.get_window_size().row_height.as_usize() == 0
                || buffer.get_window_size().col_width.as_usize() == 0
        );
    }

    #[test]
    fn test_ofs_buf_clear() {
        let mut buffer = create_test_buffer();

        // Modify some characters.
        buffer.get_row_mut(0u16.into()).expect("conversion error")[0] =
            create_test_pixel_char('A');
        buffer.get_row_mut(1u16.into()).expect("conversion error")[2] =
            create_test_pixel_char('B');
        buffer.get_row_mut(2u16.into()).expect("conversion error")[1] = PixelChar::Void;

        // Verify characters were set.
        assert!(matches!(
            buffer.get_row_mut(0u16.into()).expect("conversion error")[0],
            PixelChar::PlainText {
                display_char: 'A',
                ..
            }
        ));
        assert!(matches!(
            buffer.get_row_mut(1u16.into()).expect("conversion error")[2],
            PixelChar::PlainText {
                display_char: 'B',
                ..
            }
        ));
        assert!(matches!(
            buffer.get_row_mut(2u16.into()).expect("conversion error")[1],
            PixelChar::Void
        ));

        // Clear the buffer.
        buffer.clear();

        // Verify all characters are now spacers.
        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| {
            buffer
                .get_row((i.as_u16_narrowing()).into())
                .expect("conversion error")
        }) {
            for pixel_char in line {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }
    }

    #[test]
    fn test_ofs_buf_clear_already_empty() {
        let mut buffer = create_test_buffer();

        // Buffer should already be empty (all spacers).
        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| {
            buffer
                .get_row((i.as_u16_narrowing()).into())
                .expect("conversion error")
        }) {
            for pixel_char in line {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }

        // Clear should not change anything.
        buffer.clear();

        // Verify still all spacers.
        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| {
            buffer
                .get_row((i.as_u16_narrowing()).into())
                .expect("conversion error")
        }) {
            for pixel_char in line {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }
    }

    #[test]
    fn test_ofs_buf_diff_identical() {
        let buffer1 = create_test_buffer();
        let buffer2 = create_test_buffer();

        let diff = buffer1.diff(&buffer2);
        // The buffers should be identical, so diff should return None. However, if Some
        // is returned with an empty list, that's also acceptable.
        match diff {
            None => {} // Expected case
            Some(chunks) => assert!(
                chunks.is_empty(),
                "Diff chunks should be empty for identical buffers"
            ),
        }
    }

    #[test]
    fn test_ofs_buf_diff_different_sizes() {
        let buffer1 = OfsBuf::new(Flat2DArray::new_empty(
            vp_height(2) + vp_width(3),
            PixelChar::Spacer,
        ));
        let buffer2 = OfsBuf::new(Flat2DArray::new_empty(
            vp_height(3) + vp_width(2),
            PixelChar::Spacer,
        ));

        let diff = buffer1.diff(&buffer2);
        assert_eq!(diff, None);
    }

    #[test]
    fn test_ofs_buf_diff_with_changes() {
        let buffer1 = create_test_buffer();
        let mut buffer2 = create_test_buffer();

        // Make some changes to buffer2.
        buffer2.get_row_mut(0u16.into()).expect("conversion error")[0] =
            create_test_pixel_char('A');
        buffer2.get_row_mut(1u16.into()).expect("conversion error")[2] =
            create_test_pixel_char('B');
        buffer2.get_row_mut(2u16.into()).expect("conversion error")[1] = PixelChar::Void;

        let diff = buffer1.diff(&buffer2);
        assert!(diff.is_some());

        let diff_chunks = diff.expect("conversion error");
        assert_eq!(diff_chunks.len(), 3);

        // Check the diff contains the expected changes.
        let positions: Vec<crate::VPPos> =
            diff_chunks.iter().map(|(pos, _)| *pos).collect();
        assert!(positions.contains(&(vp_row(0) + vp_col(0))));
        assert!(positions.contains(&(vp_row(1) + vp_col(2))));
        assert!(positions.contains(&(vp_row(2) + vp_col(1))));
    }

    #[test]
    fn test_ofs_buf_diff_single_change() {
        let buffer1 = create_test_buffer();
        let mut buffer2 = create_test_buffer();

        // Make a single change.
        buffer2.get_row_mut(1u16.into()).expect("conversion error")[1] =
            create_test_pixel_char('X');

        let diff = buffer1.diff(&buffer2);
        assert!(diff.is_some());

        let diff_chunks = diff.expect("conversion error");
        assert_eq!(diff_chunks.len(), 1);

        let (pos, pixel_char) = &diff_chunks[0];
        assert_eq!(*pos, vp_row(1) + vp_col(1));
        assert!(matches!(
            pixel_char,
            PixelChar::PlainText {
                display_char: 'X',
                ..
            }
        ));
    }

    #[test]
    fn test_ofs_buf_cached_memory_size() {
        // TRIPWIRE: This test verifies that `GetMemSize` returns a consistent value. If
        // you added a field, ensure that `OfsBuf::new_empty` correctly includes its
        // memory size in the `cached_memory_size` calculation block!
        let buffer = create_test_buffer();

        let mem_size = buffer.get_mem_size();
        assert!(mem_size > 0);

        // Test that get_mem_size returns the same value consistently.
        let size2 = buffer.get_mem_size();
        assert_eq!(mem_size, size2);
    }

    #[test]
    fn test_ofs_buf_struct_size() {
        // TRIPWIRE: If you add or remove a field from `OfsBuf`, this test will fail. This
        // is intentional! It reminds you to:
        // 1. Update `OfsBuf::new_empty` to include your new field's size in
        //    `cached_memory_size`.
        // 2. Update this exact byte-size assertion.
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(size_of::<OfsBuf>(), 40);
        }
    }

    #[test]
    fn test_ofs_buf_deref() {
        let buffer = create_test_buffer();

        // Test deref functionality.
        assert_eq!(buffer.get_height().as_usize(), 3);
        assert_eq!(buffer[0].len(), 4);
        assert_eq!(buffer[1].len(), 4);
        assert_eq!(buffer[2].len(), 4);
    }

    #[test]
    fn test_ofs_buf_deref_mut() {
        let mut buffer = create_test_buffer();

        // Test deref_mut functionality.
        buffer[0][0] = create_test_pixel_char('M');
        buffer[2][3] = PixelChar::Void;

        assert!(matches!(
            buffer[0][0],
            PixelChar::PlainText {
                display_char: 'M',
                ..
            }
        ));
        assert!(matches!(buffer[2][3], PixelChar::Void));
    }

    #[test]
    fn test_ofs_buf_large_size() {
        let large_size = vp_height(100) + vp_width(200);
        let buffer = OfsBuf::new(Flat2DArray::new_empty(large_size, PixelChar::Spacer));

        assert_eq!(buffer.get_window_size(), large_size);
        assert_eq!(buffer.get_height().as_usize(), 100);

        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| {
            buffer
                .get_row((i.as_u16_narrowing()).into())
                .expect("conversion error")
        }) {
            assert_eq!(line.len(), 200);
        }

        // Memory size should be significant.
        let mem_size = buffer.get_mem_size();
        assert!(mem_size > 1000); // Should be substantial for this size
    }

    #[test]
    fn test_ofs_buf_diff_performance() {
        // Test diff with larger buffers to ensure it performs reasonably.
        let size = vp_height(50) + vp_width(100);
        let buffer1 = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));
        let mut buffer2 = OfsBuf::new(Flat2DArray::new_empty(size, PixelChar::Spacer));

        // Make a few scattered changes.
        buffer2.get_row_mut(0u16.into()).expect("conversion error")[0] =
            create_test_pixel_char('1');
        buffer2.get_row_mut(25u16.into()).expect("conversion error")[50] =
            create_test_pixel_char('2');
        buffer2.get_row_mut(49u16.into()).expect("conversion error")[99] =
            create_test_pixel_char('3');

        let diff = buffer1.diff(&buffer2);
        assert!(diff.is_some());

        let diff_chunks = diff.expect("conversion error");
        assert_eq!(diff_chunks.len(), 3);
    }
}
