// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core types and implementation for [`OfsBuf`].
//!
//! This module defines the main `OfsBuf` struct, its core properties, lifecycle
//! methods, and primary delegation down to its `Flat2DArray` backing store.

use super::{super::{FlushKind, RenderOpOutputVec},
            PixelCharDiffChunks,
            pixel_char::PixelChar};
use crate::{ColIndex, Flat2DArray, GetMemSize, List, LockedOutputDevice, Pos, RowIndex,
            Size, fg_green, inline_string, ok};
use std::{fmt::{self, Debug},
          mem::size_of,
          ops::{Deref, DerefMut}};

/// Core terminal screen buffer structure with [`VT-100`]/[`ANSI`] support.
///
/// For comprehensive architectural overview and integration details, see the [module
/// documentation].
///
/// This struct represents the main terminal screen buffer as a 2D grid where each cell
/// maps directly to a terminal screen position. It handles variable-width characters
/// (like emoji) using [`PixelChar::Void`] placeholders.
///
/// ## Key Features
/// - **Dual Integration**: Works with both render pipeline and [`ANSI`] terminal
///   emulation
/// - **Variable-ColWidth Support**: Proper handling of emoji and Unicode characters
/// - **[`VT-100`] Compliance**: Full terminal specification compliance
/// - **Performance Optimized**: Pre-calculated memory sizes and efficient operations
///
/// ## Field Organization
///
/// The struct is organized into logical groups:
/// - **Core Buffer**: The 2D grid and window dimensions
/// - **Cursor Management**: Primary cursor position for all subsystems
/// - **Performance**: Pre-calculated memory usage tracking
///
/// ## Underlying protocol parser
///
/// - [`vt_100_pty_output_parser`]: The [`ANSI`] parser that processes [`PTY`] output and
///   updates the higher-level [`OfsBufVT100`] via [`apply_ansi_bytes`]
/// - [`AnsiToOfsBufPerformer`]: The VTE `Perform` implementation that translates [`ANSI`]
///   sequences into terminal state operations
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
/// [`apply_ansi_bytes`]: crate::OfsBufVT100::apply_ansi_bytes
/// [`OfsBufVT100`]: crate::OfsBufVT100
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`RenderOpCommon`]: enum@crate::RenderOpCommon
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
/// [module documentation]: super
#[derive(Clone, PartialEq)]
pub struct OfsBuf {
    pub(super) buffer: Flat2DArray<PixelChar>,
    pub(super) cursor_pos: Pos,
}

impl GetMemSize for OfsBuf {
    fn get_mem_size(&self) -> usize { self.buffer.get_mem_size() + size_of::<Pos>() }
}

/// Trait for painting offscreen buffer content to terminal output.
///
/// This trait converts an [`OfsBuf`] (post-Compositor) into
/// [`RenderOpOutputVec`] (terminal-executable operations), then executes them on the
/// terminal.
///
/// # Type Safety Note
///
/// This trait works with [`RenderOpOutputVec`] (post-Compositor operations), not
/// [`RenderOpIRVec`]. The Compositor has already applied all necessary transformations
/// (clipping, Unicode handling, etc.) when these methods are called.
///
/// [`RenderOpIRVec`]: crate::RenderOpIRVec
pub trait OfsBufPaint {
    /// Converts offscreen buffer to terminal operations.
    fn render(&mut self, ofs_buf: &OfsBuf) -> RenderOpOutputVec;

    /// Converts diff chunks to terminal operations (for selective redraw).
    fn render_diff(
        &mut self,
        diff_chunks: &super::diff_chunks::PixelCharDiffChunks,
    ) -> RenderOpOutputVec;

    /// Execute terminal operations on the display.
    fn paint(
        &mut self,
        render_ops: RenderOpOutputVec,
        flush_kind: FlushKind,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
    );

    /// Execute diff operations on the display (selective redraw).
    fn paint_diff(
        &mut self,
        render_ops: RenderOpOutputVec,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
    );
}

impl Debug for PixelCharDiffChunks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (pos, pixel_char) in self.iter() {
            writeln!(f, "\t{pos:?}: {pixel_char:?}")?;
        }
        ok!()
    }
}

impl Deref for OfsBuf {
    type Target = Flat2DArray<PixelChar>;

    fn deref(&self) -> &Self::Target { &self.buffer }
}

impl DerefMut for OfsBuf {
    /// Returns a mutable reference to the buffer.
    ///
    /// Code like the following will call this method:
    /// - `self.buffer[row][col] = something`
    /// - `self.buffer.get_mut(row)`
    /// - Any operation that goes through the `&mut self.buffer` dereference
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.buffer }
}

impl Debug for OfsBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "window_size: {:?}, ", self.get_window_size())?;

        let height = self.buffer.height.as_usize();
        for row_index in 0..height {
            if let Some(row) = self.buffer.get_row(row_index) {
                // Print row separator if needed (not the first item).
                if row_index > 0 {
                    writeln!(f)?;
                }

                // Print the row index (styled) in "this" line.
                writeln!(
                    f,
                    "{}",
                    fg_green(&inline_string!("row_index: {}", row_index))
                )?;

                // Print the row itself in the "next" line.
                write!(f, "{row:?}")?;
            }
        }

        writeln!(f)
    }
}

impl OfsBuf {
    /// Returns the current cursor position.
    #[must_use]
    pub fn get_cursor_pos(&self) -> Pos { self.cursor_pos }

    /// Sets the current cursor position.
    pub fn set_cursor_pos(&mut self, pos: Pos) { self.cursor_pos = pos; }

    /// Updates the current cursor position using a closure. This is useful if you just
    /// want to update the row or column index without needing to create a new [`Pos`].
    pub fn update_cursor_pos<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Pos),
    {
        f(&mut self.cursor_pos);
    }

    /// Returns the current window size of the offscreen buffer.
    #[must_use]
    pub fn get_window_size(&self) -> Size {
        Size {
            col_width: self.buffer.width,
            row_height: self.buffer.height,
        }
    }

    /// Identifies all pixel differences between two offscreen buffers.
    ///
    /// This method is highly optimized to use [SIMD] array operations to compare the
    /// internal [`Flat2DArray`] memory linearly, bypassing coordinate calculations during
    /// the scanning phase.
    ///
    /// # Behavior
    ///
    /// 1. Verifies both buffers have identical dimensions.
    /// 2. Chunks the 1D arrays row by row using [`.chunks_exact()`].
    /// 3. Returns [`None`] if dimensions mismatch, or a [`PixelCharDiffChunks`]
    ///    containing only the modified characters (from `other`).
    ///
    /// See the [Rule of Thumb for 1D vs 2D Memory Iteration] and the [Deep Dive: The
    /// Magic of SIMD Diffing] for a detailed breakdown of how this linear traversal
    /// eliminates CPU pipeline stalls and leverages multi-stream hardware prefetching.
    ///
    /// [`.chunks_exact()`]: slice::chunks_exact
    /// [`Flat2DArray`]: crate::Flat2DArray
    /// [Deep Dive: The Magic of SIMD Diffing]:
    ///     crate::Flat1DSimd#deep-dive-the-magic-of-simd-diffing
    /// [Rule of Thumb for 1D vs 2D Memory Iteration]:
    ///     crate::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    #[must_use]
    pub fn diff(&self, other: &Self) -> Option<PixelCharDiffChunks> {
        if self.buffer.width != other.buffer.width
            || self.buffer.height != other.buffer.height
        {
            return None;
        }

        let mut acc = List::default();
        let self_simd = self.buffer.as_simd();
        let other_simd = other.buffer.as_simd();
        let width = self.buffer.width.as_usize();

        let self_rows_iter = self_simd.as_raw_slice().chunks_exact(width);
        debug_assert!(
            self_rows_iter.remainder().is_empty(),
            "The data length should be a multiple of the number of columns."
        );

        let other_rows_iter = other_simd.as_raw_slice().chunks_exact(width);
        debug_assert!(
            other_rows_iter.remainder().is_empty(),
            "The data length should be a multiple of the number of columns."
        );

        let zipped_rows_iter = self_rows_iter.zip(other_rows_iter).enumerate();
        for (row_idx, (self_row_chunk, other_row_chunk)) in zipped_rows_iter {
            if self_row_chunk != other_row_chunk {
                let cols_iter = self_row_chunk
                    .iter()
                    .zip(other_row_chunk.iter())
                    .enumerate();
                for (col_idx, (self_pixel_char, other_pixel_char)) in cols_iter {
                    if self_pixel_char != other_pixel_char {
                        let pos = Pos {
                            row_index: RowIndex::from(row_idx),
                            col_index: ColIndex::from(col_idx),
                        };
                        acc.push((pos, *other_pixel_char));
                    }
                }
            }
        }

        Some(PixelCharDiffChunks::from(acc))
    }

    /// Creates a new empty offscreen buffer with the specified window size.
    pub fn new_empty(arg_window_size: impl Into<Size>) -> Self {
        let window_size = arg_window_size.into();
        Self {
            buffer: Flat2DArray::new_empty(window_size, PixelChar::Spacer),
            cursor_pos: Pos::default(),
        }
    }

    /// Make sure each line is full of empty chars.
    pub fn clear(&mut self) { self.clear_with(PixelChar::Spacer); }

    /// Make sure each line is full of the given char.
    pub fn clear_with(&mut self, char: PixelChar) {
        self.buffer.as_simd_mut().fill_all(char);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TuiStyle, col, height, row, width};

    fn create_test_buffer() -> OfsBuf {
        let size = height(3) + width(4);
        OfsBuf::new_empty(size)
    }

    fn create_test_pixel_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_ofs_buf_new_empty() {
        let size = height(2) + width(3);
        let buffer = OfsBuf::new_empty(size);

        assert_eq!(buffer.get_window_size(), size);
        assert_eq!(buffer.buffer.height.as_usize(), 2);
        assert_eq!(buffer.buffer.width.as_usize(), 3);

        // Check that all positions are initialized with spacers.
        for pixel_char in buffer.buffer.as_simd().as_raw_slice() {
            assert!(matches!(pixel_char, PixelChar::Spacer));
        }
    }

    #[test]
    fn test_ofs_buf_new_empty_zero_size() {
        let size = height(0) + width(0);
        let buffer = OfsBuf::new_empty(size);

        assert_eq!(buffer.get_window_size(), size);
        assert_eq!(buffer.get_height().as_usize(), 0);
        assert!(
            (buffer.get_height().as_usize() == 0 || buffer.get_width().as_usize() == 0)
        );
    }

    #[test]
    fn test_ofs_buf_clear() {
        let mut buffer = create_test_buffer();

        // Modify some characters.
        buffer.get_row_mut(0).unwrap()[0] = create_test_pixel_char('A');
        buffer.get_row_mut(1).unwrap()[2] = create_test_pixel_char('B');
        buffer.get_row_mut(2).unwrap()[1] = PixelChar::Void;

        // Verify characters were set.
        assert!(matches!(
            buffer.get_row_mut(0).unwrap()[0],
            PixelChar::PlainText {
                display_char: 'A',
                ..
            }
        ));
        assert!(matches!(
            buffer.get_row_mut(1).unwrap()[2],
            PixelChar::PlainText {
                display_char: 'B',
                ..
            }
        ));
        assert!(matches!(buffer.get_row_mut(2).unwrap()[1], PixelChar::Void));

        // Clear the buffer.
        buffer.clear();

        // Verify all characters are now spacers.
        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| buffer.get_row(i).unwrap()) {
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
        for line in (0..height).map(|i| buffer.get_row(i).unwrap()) {
            for pixel_char in line {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }

        // Clear should not change anything.
        buffer.clear();

        // Verify still all spacers.
        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| buffer.get_row(i).unwrap()) {
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
        let buffer1 = OfsBuf::new_empty(height(2) + width(3));
        let buffer2 = OfsBuf::new_empty(height(3) + width(2));

        let diff = buffer1.diff(&buffer2);
        assert_eq!(diff, None);
    }

    #[test]
    fn test_ofs_buf_diff_with_changes() {
        let buffer1 = create_test_buffer();
        let mut buffer2 = create_test_buffer();

        // Make some changes to buffer2.
        buffer2.get_row_mut(0).unwrap()[0] = create_test_pixel_char('A');
        buffer2.get_row_mut(1).unwrap()[2] = create_test_pixel_char('B');
        buffer2.get_row_mut(2).unwrap()[1] = PixelChar::Void;

        let diff = buffer1.diff(&buffer2);
        assert!(diff.is_some());

        let diff_chunks = diff.unwrap();
        assert_eq!(diff_chunks.len(), 3);

        // Check the diff contains the expected changes.
        let positions: Vec<Pos> = diff_chunks.iter().map(|(pos, _)| *pos).collect();
        assert!(positions.contains(&(row(0) + col(0))));
        assert!(positions.contains(&(row(1) + col(2))));
        assert!(positions.contains(&(row(2) + col(1))));
    }

    #[test]
    fn test_ofs_buf_diff_single_change() {
        let buffer1 = create_test_buffer();
        let mut buffer2 = create_test_buffer();

        // Make a single change.
        buffer2.get_row_mut(1).unwrap()[1] = create_test_pixel_char('X');

        let diff = buffer1.diff(&buffer2);
        assert!(diff.is_some());

        let diff_chunks = diff.unwrap();
        assert_eq!(diff_chunks.len(), 1);

        let (pos, pixel_char) = &diff_chunks[0];
        assert_eq!(*pos, row(1) + col(1));
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
        // TRIPWIRE: This test verifies that `GetMemSize` returns a consistent value.
        // If you added a field, ensure that `OfsBuf::new_empty` correctly
        // includes its memory size in the `cached_memory_size` calculation block!
        let buffer = create_test_buffer();

        let mem_size = buffer.get_mem_size();
        assert!(mem_size > 0);

        // Test that get_mem_size returns the same value consistently.
        let size2 = buffer.get_mem_size();
        assert_eq!(mem_size, size2);
    }

    #[test]
    fn test_ofs_buf_struct_size() {
        // TRIPWIRE: If you add or remove a field from `OfsBuf`, this test will
        // fail. This is intentional! It reminds you to:
        // 1. Update `OfsBuf::new_empty` to include your new field's size in
        //    `cached_memory_size`.
        // 2. Update this exact byte-size assertion.
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<OfsBuf>(), 32);
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
        let large_size = height(100) + width(200);
        let buffer = OfsBuf::new_empty(large_size);

        assert_eq!(buffer.get_window_size(), large_size);
        assert_eq!(buffer.get_height().as_usize(), 100);

        let height = buffer.get_height().as_usize();
        for line in (0..height).map(|i| buffer.get_row(i).unwrap()) {
            assert_eq!(line.len(), 200);
        }

        // Memory size should be significant.
        let mem_size = buffer.get_mem_size();
        assert!(mem_size > 1000); // Should be substantial for this size
    }

    #[test]
    fn test_ofs_buf_diff_performance() {
        // Test diff with larger buffers to ensure it performs reasonably.
        let size = height(50) + width(100);
        let buffer1 = OfsBuf::new_empty(size);
        let mut buffer2 = OfsBuf::new_empty(size);

        // Make a few scattered changes.
        buffer2.get_row_mut(0).unwrap()[0] = create_test_pixel_char('1');
        buffer2.get_row_mut(25).unwrap()[50] = create_test_pixel_char('2');
        buffer2.get_row_mut(49).unwrap()[99] = create_test_pixel_char('3');

        let diff = buffer1.diff(&buffer2);
        assert!(diff.is_some());

        let diff_chunks = diff.unwrap();
        assert_eq!(diff_chunks.len(), 3);
    }
}
