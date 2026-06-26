// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{super::{FlushKind, RenderOpOutputVec},
            PixelCharDiffChunks,
            pixel_char::PixelChar,
            pixel_char_lines::PixelCharLines};
use crate::{GetMemSize, List, LockedOutputDevice, MemorySize, Pos, Size, col,
            fg_green, inline_string, ok, row};
use std::{fmt::{Debug, {self}},
          mem::size_of,
          ops::{Deref, DerefMut}};

/// Core terminal screen buffer structure with VT100/[`ANSI`] support.
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
/// - **VT100 Compliance**: Full terminal specification compliance
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
/// [`vt_100_pty_output_parser`]: mod@crate::core::ansi::vt_100_pty_output_parser
/// [module documentation]: super
#[derive(Clone, PartialEq)]
pub struct OffscreenBuffer {
    // The actual 2D grid of pixel characters representing the terminal screen.
    pub buffer: PixelCharLines,

    // Size of the terminal window in rows and columns (1-based).
    pub window_size: Size,

    /// The current active cursor position in the buffer.
    ///
    /// This is the primary cursor position tracker for the entire offscreen buffer
    /// system, used by multiple subsystems:
    /// - **Render pipeline**: Updated when processing [`RenderOpCommon`] variants
    ///   [`MoveCursorPositionAbs`] and [`MoveCursorPositionRelTo`]
    /// - **Text rendering**: Starting position for [`print_text_with_attributes()`]
    /// - **[`ANSI`] parser**: Directly reads from and writes to this position during
    ///   sequence processing
    /// - **Terminal emulation**: Tracks where the next character should be rendered
    ///
    /// Note: This is different from [`cursor_pos_for_esc_save_and_restore`] which is
    /// only used for [`DECSC`]/[`DECRC`] (`ESC 7` / `ESC 8`) save/restore operations.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`cursor_pos_for_esc_save_and_restore`]:
    ///     crate::ParserGlobalState::cursor_pos_for_esc_save_and_restore
    /// [`DECRC`]: https://vt100.net/docs/vt510-rm/contents.html
    /// [`DECSC`]: https://vt100.net/docs/vt510-rm/contents.html
    /// [`MoveCursorPositionAbs`]: crate::RenderOpCommon::MoveCursorPositionAbs
    /// [`MoveCursorPositionRelTo`]: crate::RenderOpCommon::MoveCursorPositionRelTo
    /// [`print_text_with_attributes()`]: crate::print_text_with_attributes
    /// [`RenderOpCommon`]: crate::RenderOpCommon
    pub cursor_pos: Pos,

    /// Cached memory size of this buffer to provide O(1) retrieval.
    ///
    /// Because the buffer grid dimensions are strictly fixed upon creation (resizing the
    /// terminal creates a brand new buffer), we can safely calculate its total memory
    /// cost exactly once during [`new_empty()`] and cache it here. This avoids expensive
    /// O(N) traversal recalculations later.
    ///
    /// Used in [`log_telemetry_info`] which is called in a hot loop on every render.
    ///
    /// [`log_telemetry_info`]:
    ///     crate::main_event_loop::EventLoopState::log_telemetry_info()
    /// [`new_empty()`]: Self::new_empty()
    pub cached_memory_size: MemorySize,

    /// Scrollback history for lines that have scrolled off the top of the screen.
    pub scrollback: super::ScrollbackBuffer,
}

impl GetMemSize for OffscreenBuffer {
    /// Acts as a fast O(1) getter for the cached memory size.
    ///
    /// Instead of iterating through all internal vectors and states on the fly, this
    /// method returns the value that was pre-calculated during [`new_empty()`]. This
    /// caching pattern is an optimization for performance, as this method is called
    /// frequently during hot-loop render cycles.
    ///
    /// [`new_empty()`]: Self::new_empty()
    fn get_mem_size(&self) -> usize { self.cached_memory_size.size().unwrap_or(0) }
}

// Forward declarations for types defined in their own modules, just for this file.
use super::pixel_char_line::PixelCharLine;

/// Trait for painting offscreen buffer content to terminal output.
///
/// This trait converts an [`OffscreenBuffer`] (post-Compositor) into
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
pub trait OffscreenBufferPaint {
    /// Converts offscreen buffer to terminal operations.
    fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOpOutputVec;

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

impl Deref for OffscreenBuffer {
    type Target = PixelCharLines;

    fn deref(&self) -> &Self::Target { &self.buffer }
}

impl DerefMut for OffscreenBuffer {
    /// Returns a mutable reference to the buffer.
    ///
    /// Code like the following will call this method:
    /// - `self.buffer[row][col] = something`
    /// - `self.buffer.get_mut(row)`
    /// - Any operation that goes through the `&mut self.buffer` dereference
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.buffer }
}

impl Debug for OffscreenBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "window_size: {:?}, ", self.window_size)?;

        let height = self.window_size.row_height.as_usize();
        for row_index in 0..height {
            if let Some(row) = self.buffer.get(row_index) {
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

impl OffscreenBuffer {
    /// Checks for differences between self and other. Returns a list of positions and
    /// pixel chars if there are differences (from other).
    #[must_use]
    pub fn diff(&self, other: &Self) -> Option<PixelCharDiffChunks> {
        if self.window_size != other.window_size {
            return None;
        }

        let mut acc = List::default();

        for (row_idx, (self_row, other_row)) in
            self.buffer.iter().zip(other.buffer.iter()).enumerate()
        {
            for (col_idx, (self_pixel_char, other_pixel_char)) in
                self_row.iter().zip(other_row.iter()).enumerate()
            {
                if self_pixel_char != other_pixel_char {
                    let pos = col(col_idx) + row(row_idx);
                    acc.push((pos, *other_pixel_char));
                }
            }
        }
        Some(PixelCharDiffChunks::from(acc))
    }

    /// Creates a new buffer and fills it with empty chars.
    ///
    /// This constructor also pre-calculates the exact memory footprint of the buffer
    /// and caches it in [`cached_memory_size`]. Because the buffer's grid dimensions
    /// are fixed at creation time, calculating this upfront provides an O(1) getter
    /// for memory metrics without requiring expensive recursive traversals later.
    ///
    /// [`cached_memory_size`]: Self::cached_memory_size
    #[must_use]
    pub fn new_empty(arg_window_size: impl Into<Size>) -> Self {
        let window_size = arg_window_size.into();
        let buffer = PixelCharLines::new_empty(window_size);

        let cached_memory_size = {
            // Calculate memory size once - it will never change since buffer dimensions
            // are fixed.
            let primary_buffer_mem = buffer.get_mem_size();
            MemorySize::new(
                primary_buffer_mem
                + size_of::<Size>() // window_size
                + size_of::<Pos>(), // cursor_pos
            )
        };

        Self {
            buffer,
            window_size,
            cursor_pos: Pos::default(),
            cached_memory_size,
            scrollback: super::ScrollbackBuffer::default(),
        }
    }

    // Make sure each line is full of empty chars.
    pub fn clear(&mut self) {
        for line in self.buffer.iter_mut() {
            for pixel_char in line.iter_mut() {
                if pixel_char != &PixelChar::Spacer {
                    *pixel_char = PixelChar::Spacer;
                }
            }
        }
    }

    /// Number of lines in scrollback history.
    #[must_use]
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Get a scrollback line by index (0 = oldest).
    #[must_use]
    pub fn scrollback_get(&self, idx: usize) -> Option<&PixelCharLine> {
        self.scrollback.get(idx)
    }

    /// Number of scrollback lines evicted (overwritten) since last reset.
    #[must_use]
    pub fn scrollback_eviction_count(&self) -> usize {
        self.scrollback.eviction_count()
    }

    /// Reset the scrollback eviction counter.
    pub fn reset_scrollback_eviction_count(&mut self) {
        self.scrollback.reset_eviction_count();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TuiStyle, height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = height(3) + width(4);
        OffscreenBuffer::new_empty(size)
    }

    fn create_test_pixel_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_offscreen_buffer_new_empty() {
        let size = height(2) + width(3);
        let buffer = OffscreenBuffer::new_empty(size);

        assert_eq!(buffer.window_size, size);
        assert_eq!(buffer.buffer.len(), 2);

        // Check that all positions are initialized with spacers.
        for line in &buffer.buffer.lines {
            assert_eq!(line.len(), 3);
            for pixel_char in &line.pixel_chars {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }
    }

    #[test]
    fn test_offscreen_buffer_new_empty_zero_size() {
        let size = height(0) + width(0);
        let buffer = OffscreenBuffer::new_empty(size);

        assert_eq!(buffer.window_size, size);
        assert_eq!(buffer.buffer.len(), 0);
        assert!(buffer.buffer.is_empty());
    }

    #[test]
    fn test_offscreen_buffer_clear() {
        let mut buffer = create_test_buffer();

        // Modify some characters.
        buffer.buffer[0][0] = create_test_pixel_char('A');
        buffer.buffer[1][2] = create_test_pixel_char('B');
        buffer.buffer[2][1] = PixelChar::Void;

        // Verify characters were set.
        assert!(matches!(
            buffer.buffer[0][0],
            PixelChar::PlainText {
                display_char: 'A',
                ..
            }
        ));
        assert!(matches!(
            buffer.buffer[1][2],
            PixelChar::PlainText {
                display_char: 'B',
                ..
            }
        ));
        assert!(matches!(buffer.buffer[2][1], PixelChar::Void));

        // Clear the buffer.
        buffer.clear();

        // Verify all characters are now spacers.
        for line in &buffer.buffer.lines {
            for pixel_char in &line.pixel_chars {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }
    }

    #[test]
    fn test_offscreen_buffer_clear_already_empty() {
        let mut buffer = create_test_buffer();

        // Buffer should already be empty (all spacers).
        for line in &buffer.buffer.lines {
            for pixel_char in &line.pixel_chars {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }

        // Clear should not change anything.
        buffer.clear();

        // Verify still all spacers.
        for line in &buffer.buffer.lines {
            for pixel_char in &line.pixel_chars {
                assert!(matches!(pixel_char, PixelChar::Spacer));
            }
        }
    }

    #[test]
    fn test_offscreen_buffer_diff_identical() {
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
    fn test_offscreen_buffer_diff_different_sizes() {
        let buffer1 = OffscreenBuffer::new_empty(height(2) + width(3));
        let buffer2 = OffscreenBuffer::new_empty(height(3) + width(2));

        let diff = buffer1.diff(&buffer2);
        assert_eq!(diff, None);
    }

    #[test]
    fn test_offscreen_buffer_diff_with_changes() {
        let buffer1 = create_test_buffer();
        let mut buffer2 = create_test_buffer();

        // Make some changes to buffer2.
        buffer2.buffer[0][0] = create_test_pixel_char('A');
        buffer2.buffer[1][2] = create_test_pixel_char('B');
        buffer2.buffer[2][1] = PixelChar::Void;

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
    fn test_offscreen_buffer_diff_single_change() {
        let buffer1 = create_test_buffer();
        let mut buffer2 = create_test_buffer();

        // Make a single change.
        buffer2.buffer[1][1] = create_test_pixel_char('X');

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
    fn test_offscreen_buffer_cached_memory_size() {
        // TRIPWIRE: This test verifies that `GetMemSize` returns a consistent value.
        // If you added a field, ensure that `OffscreenBuffer::new_empty` correctly
        // includes its memory size in the `cached_memory_size` calculation block!
        let buffer = create_test_buffer();

        let mem_size = buffer.get_mem_size();
        assert!(mem_size > 0);

        // Test that get_mem_size returns the same value consistently.
        let size2 = buffer.get_mem_size();
        assert_eq!(mem_size, size2);
    }

    #[test]
    fn test_offscreen_buffer_struct_size() {
        // TRIPWIRE: If you add or remove a field from `OffscreenBuffer`, this test will
        // fail. This is intentional! It reminds you to:
        // 1. Update `OffscreenBuffer::new_empty` to include your new field's size in
        //    `cached_memory_size`.
        // 2. Update this exact byte-size assertion.
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<OffscreenBuffer>(), 416);
        }
    }

    #[test]
    fn test_offscreen_buffer_deref() {
        let buffer = create_test_buffer();

        // Test deref functionality.
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer[0].len(), 4);
        assert_eq!(buffer[1].len(), 4);
        assert_eq!(buffer[2].len(), 4);
    }

    #[test]
    fn test_offscreen_buffer_deref_mut() {
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
    fn test_offscreen_buffer_large_size() {
        let large_size = height(100) + width(200);
        let buffer = OffscreenBuffer::new_empty(large_size);

        assert_eq!(buffer.window_size, large_size);
        assert_eq!(buffer.buffer.len(), 100);

        for line in &buffer.buffer.lines {
            assert_eq!(line.len(), 200);
        }

        // Memory size should be significant.
        let mem_size = buffer.get_mem_size();
        assert!(mem_size > 1000); // Should be substantial for this size
    }

    #[test]
    fn test_offscreen_buffer_diff_performance() {
        // Test diff with larger buffers to ensure it performs reasonably.
        let size = height(50) + width(100);
        let buffer1 = OffscreenBuffer::new_empty(size);
        let mut buffer2 = OffscreenBuffer::new_empty(size);

        // Make a few scattered changes.
        buffer2.buffer[0][0] = create_test_pixel_char('1');
        buffer2.buffer[25][50] = create_test_pixel_char('2');
        buffer2.buffer[49][99] = create_test_pixel_char('3');

        let diff = buffer1.diff(&buffer2);
        assert!(diff.is_some());

        let diff_chunks = diff.unwrap();
        assert_eq!(diff_chunks.len(), 3);
    }
}
