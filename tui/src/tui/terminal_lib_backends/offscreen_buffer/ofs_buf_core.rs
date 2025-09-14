// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::Debug;

use super::super::{FlushKind, RenderOps};
use crate::{core::pty_mux::ansi_parser::term_units::TermRow, osc::OscEvent, CachedMemorySize, GetMemSize, LockedOutputDevice, MemoizedMemorySize, Pos, Size, TuiColor, TuiStyle};

/// Character set modes for terminal emulation.
///
/// Used by [`crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer`] to handle
/// ESC ( sequences that switch between ASCII and DEC line-drawing graphics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterSet {
    /// Normal ASCII character set (ESC ( B).
    #[default]
    Ascii,
    /// DEC Special Graphics character set for line drawing (ESC ( 0).
    /// Maps ASCII characters to box-drawing Unicode characters.
    DECGraphics,
}

/// Support structure for ANSI escape sequence parsing and terminal state management.
///
/// This struct groups together all fields related to [`ANSI parser`] functionality that
/// need to be maintained by the [`OffscreenBuffer`] for proper terminal emulation.
///
/// One field missing from here is [`OffscreenBuffer::my_pos`] which tracks the current
/// cursor position. This is because `my_pos` is used by multiple subsystems and is the
/// primary cursor position tracker for the entire offscreen buffer system.
///
/// [`ANSI parser`]: crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer
/// [`OffscreenBuffer::my_pos`]: crate::offscreen_buffer::OffscreenBuffer::my_pos
#[derive(Debug, Clone, PartialEq)]
pub struct AnsiParserSupport {
    /// Temporary cursor position storage for DECSC/DECRC escape sequences only.
    ///
    /// This field is ONLY used for ESC 7 (DECSC) save and ESC 8 (DECRC) restore
    /// operations, as well as their CSI equivalents (CSI s and CSI u). It does NOT
    /// track the current cursor position - that's stored in
    /// [`OffscreenBuffer::my_pos`].
    ///
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer`] to implement
    /// the DECSC (ESC 7) and DECRC (ESC 8) escape sequences for saving and restoring
    /// cursor position.
    ///
    /// ## Data Flow:
    /// ```text
    /// 1. Child process (e.g., vim) sends ESC 7 to save cursor
    ///                             ↓
    /// 2. AnsiToOfsBufPerformer::esc_dispatch() handles ESC 7
    ///                             ↓
    /// 3. Saves current cursor_pos to buffer.ansi_parser_support.cursor_pos_for_esc_save_and_restore
    ///                             ↓
    /// 4. Later, child sends ESC 8 to restore cursor
    ///                             ↓
    /// 5. AnsiToOfsBufPerformer::esc_dispatch() handles ESC 8
    ///                             ↓
    /// 6. Restores cursor_pos from buffer.ansi_parser_support.cursor_pos_for_esc_save_and_restore
    /// ```
    pub cursor_pos_for_esc_save_and_restore: Option<Pos>,

    /// Active character set for ANSI escape sequence support.
    ///
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer`] to implement
    /// character set switching via ESC ( B (ASCII) and ESC ( 0 (DEC graphics).
    /// When in Graphics mode, characters like 'q' are translated to box-drawing
    /// characters like '─' during the `print()` operation.
    ///
    /// ## Character Set Usage:
    /// ```text
    /// ASCII Mode (ESC ( B):   'q' → 'q' (literal)
    /// Graphics Mode (ESC ( 0): 'q' → '─' (horizontal line)
    /// ```
    pub character_set: CharacterSet,

    /// Auto-wrap mode (DECAWM) for ANSI escape sequence support.
    ///
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer`] to control
    /// line wrapping behavior when printing characters. This implements the VT100
    /// DECAWM (Auto Wrap Mode) specification.
    ///
    /// ## DECAWM Control:
    /// ```text
    /// ESC[?7h: Enable auto-wrap (default)  - Characters wrap to next line
    /// ESC[?7l: Disable auto-wrap          - Characters overwrite at right margin
    /// ```
    ///
    /// When enabled (default), characters that would exceed the right margin
    /// automatically wrap to the beginning of the next line. When disabled,
    /// the cursor stays at the right margin and subsequent characters overwrite.
    pub auto_wrap_mode: bool,

    /// Complete computed style combining attributes and colors for efficient rendering.
    pub current_style: TuiStyle,

    /// OSC events (hyperlinks, titles, etc.) accumulated during processing.
    pub pending_osc_events: Vec<OscEvent>,

    /// DSR response events accumulated during processing - need to be sent back to PTY.
    pub pending_dsr_responses: Vec<crate::DsrRequestFromPtyEvent>,

    /// Top margin for the **scrollable region** (DECSTBM) - 1-based row number.
    ///
    /// This variable defines the **upper boundary** of the area where scrolling occurs.
    /// Rows above this boundary are part of the **static top region** and do not scroll.
    ///
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer`] to implement
    /// DECSTBM (Set Top and Bottom Margins) functionality via ESC [ top ; bottom r.
    ///
    /// When `None`, the default top margin is row 1 (first row), making the
    /// entire terminal screen the scrollable region.
    /// When `Some(n)`, scrolling operations only affect rows from n to
    /// `scroll_region_bottom`.
    ///
    /// ## DECSTBM Usage:
    /// ```text
    /// ESC [ 5 ; 20 r   - Set scrolling region from row 5 to row 20
    /// ESC [ r          - Reset to full screen (clears both margins)
    /// ```
    pub scroll_region_top: Option<TermRow>,

    /// Bottom margin for the **scrollable region** (DECSTBM) - 1-based row number.
    ///
    /// This variable defines the **lower boundary** of the area where scrolling occurs.
    /// Rows below this boundary are part of the **static bottom region** and do not
    /// scroll.
    ///
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer`] to implement
    /// DECSTBM (Set Top and Bottom Margins) functionality via ESC [ top ; bottom r.
    ///
    /// When `None`, the default bottom margin is the last row of the terminal,
    /// making the entire terminal screen the scrollable region.
    /// When `Some(n)`, scrolling operations only affect rows from `scroll_region_top` to
    /// n.
    ///
    /// ## DECSTBM Behavior:
    /// - Scrolling commands (ESC D, ESC M, CSI S, CSI T) only affect the region
    /// - Cursor movement is constrained to the region boundaries
    /// - Content outside the region remains unchanged during scrolling
    pub scroll_region_bottom: Option<TermRow>,
}

impl Default for AnsiParserSupport {
    /// Creates a new `AnsiParserSupport` with VT100-compliant defaults.
    fn default() -> Self {
        Self {
            cursor_pos_for_esc_save_and_restore: None,
            character_set: CharacterSet::default(),
            auto_wrap_mode: true, // DECAWM default: enabled (VT100 compliant)
            current_style: TuiStyle::default(),
            pending_osc_events: Vec::new(),
            pending_dsr_responses: Vec::new(),
            scroll_region_top: None, // Default: no top margin (uses row 1)
            scroll_region_bottom: None, // Default: no bottom margin (uses last row)
        }
    }
}

/// Represents a grid of cells where the row/column index maps to the terminal screen.
///
/// This works regardless of the size of each cell. Cells can contain emoji who's display
/// width is greater than one. This complicates things since a "😃" takes up 2 display
/// widths.
///
/// Let's say one cell has a "😃" in it. The cell's display width is 2. The cell's byte
/// size is 4. The next cell after it will have to contain nothing or void.
///
/// Why? This is because the col & row indices of the grid map to display col & row
/// indices of the terminal screen. By inserting a [`PixelChar::Void`] pixel char in the
/// next cell, we signal the rendering logic to skip it since it has already been painted.
/// And this is different than a [`PixelChar::Spacer`] which has to be painted!
///
/// This is a very flexible representation of a terminal screen buffer, which can work
/// with both:
/// 1. [`crate::RenderPipeline::paint()`]
/// 2. ANSI escape sequences; for more details see
///    [`crate::core::pty_mux::ansi_parser::AnsiToOfsBufPerformer`] and the
///    [`OffscreenBuffer::apply_ansi_bytes()`].
#[derive(Clone, PartialEq)]
pub struct OffscreenBuffer {
    pub buffer: PixelCharLines,
    pub window_size: Size,
    /// The current active cursor position in the buffer.
    ///
    /// This is the primary cursor position tracker for the entire offscreen buffer
    /// system, used by multiple subsystems:
    /// - **Render pipeline**: Updated when processing
    ///   `RenderOp::MoveCursorPositionAbs/RelTo`
    /// - **Text rendering**: Starting position for `print_text_with_attributes()`
    /// - **ANSI parser**: Directly reads from and writes to this position during
    ///   sequence processing
    /// - **Terminal emulation**: Tracks where the next character should be rendered
    ///
    /// Note: This is different from
    /// `ansi_parser_support.cursor_pos_for_esc_save_and_restore` which is only used
    /// for DECSC/DECRC (ESC 7/8) save/restore operations.
    pub my_pos: Pos,
    pub my_fg_color: Option<TuiColor>,
    pub my_bg_color: Option<TuiColor>,
    /// Memoized memory size calculation for performance.
    /// This avoids expensive recalculation in
    /// [`crate::main_event_loop::EventLoopState::log_telemetry_info()`]
    /// which is called in a hot loop on every render.
    pub(super) memory_size_calc_cache: MemoizedMemorySize,
    /// ANSI parser support fields grouped together for better organization.
    pub ansi_parser_support: AnsiParserSupport,
}

impl GetMemSize for OffscreenBuffer {
    /// This is the actual calculation, but should rarely be called directly.
    /// Use [`Self::get_mem_size_cached()`] for performance-critical code.
    fn get_mem_size(&self) -> usize {
        self.buffer.get_mem_size()
            + std::mem::size_of::<Size>()
            + std::mem::size_of::<Pos>()
            + std::mem::size_of::<Option<TuiColor>>()
            + std::mem::size_of::<Option<TuiColor>>()
    }
}

impl CachedMemorySize for OffscreenBuffer {
    fn memory_size_cache(&self) -> &MemoizedMemorySize { &self.memory_size_calc_cache }

    fn memory_size_cache_mut(&mut self) -> &mut MemoizedMemorySize {
        &mut self.memory_size_calc_cache
    }
}

// Forward declarations for types defined in their own modules.
pub use super::{pixel_char::PixelChar, pixel_char_line::PixelCharLine,
                pixel_char_lines::PixelCharLines};

/// Trait for painting offscreen buffer content to terminal output.
pub trait OffscreenBufferPaint {
    fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOps;

    fn render_diff(
        &mut self,
        diff_chunks: &super::diff_chunks::PixelCharDiffChunks,
    ) -> RenderOps;

    fn paint(
        &mut self,
        render_ops: RenderOps,
        flush_kind: FlushKind,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );

    fn paint_diff(
        &mut self,
        render_ops: RenderOps,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}

// Core implementations moved from ofs_buf_core_impl.rs.

use std::{fmt::{self},
          ops::{Deref, DerefMut}};

use super::PixelCharDiffChunks;
use crate::{List, MemorySize, col, fg_green, inline_string, ok, row};

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
    /// **Important**: This invalidates and recalculates the `memory_size_calc_cache`
    /// field to ensure telemetry always shows accurate memory size instead of
    /// "?".
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Invalidate and recalculate cache when buffer is accessed mutably.
        self.invalidate_memory_size_calc_cache();
        &mut self.buffer
    }
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
    /// Gets the cached memory size value, recalculating if necessary.
    /// This is used in
    /// [`crate::main_event_loop::EventLoopState::log_telemetry_info()`] for
    /// performance-critical telemetry logging. The expensive memory calculation is
    /// only performed if the cache is invalid or empty.
    #[must_use]
    pub fn get_mem_size_cached(&mut self) -> MemorySize { self.get_cached_memory_size() }

    /// Invalidates and immediately recalculates the memory size cache.
    /// Call this when buffer content changes to ensure the cache is always valid.
    pub(super) fn invalidate_memory_size_calc_cache(&mut self) {
        self.invalidate_memory_size_cache();
        self.update_memory_size_cache(); // Force immediate recalculation to avoid "?" in telemetry
    }

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

    /// Create a new buffer and fill it with empty chars.
    #[must_use]
    pub fn new_empty(arg_window_size: impl Into<Size>) -> Self {
        let window_size = arg_window_size.into();
        let mut buffer = Self {
            buffer: PixelCharLines::new_empty(window_size),
            window_size,
            my_pos: Pos::default(),
            my_fg_color: None,
            my_bg_color: None,
            memory_size_calc_cache: crate::MemoizedMemorySize::default(),
            ansi_parser_support: super::AnsiParserSupport::default(),
        };
        // Explicitly calculate and cache the initial memory size.
        // We know the cache is empty (invariant), so directly populate it.
        let size = buffer.get_mem_size();
        buffer
            .memory_size_calc_cache
            .upsert(|| MemorySize::new(size));
        buffer
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
        // Invalidate and recalculate cache when buffer is cleared.
        self.invalidate_memory_size_calc_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{height, width};

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
    fn test_character_set_default() {
        let charset = CharacterSet::default();
        assert!(matches!(charset, CharacterSet::Ascii));
    }

    #[test]
    fn test_ansi_parser_support_default() {
        let support = AnsiParserSupport::default();
        assert!(matches!(support.character_set, CharacterSet::Ascii));
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

        // Check ANSI parser support is initialized.
        assert!(matches!(
            buffer.ansi_parser_support.character_set,
            CharacterSet::Ascii
        ));
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
        // The buffers should be identical, so diff should return None.
        // However, if Some is returned with an empty list, that's also acceptable.
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
    fn test_offscreen_buffer_memory_size() {
        let mut buffer = create_test_buffer();

        let mem_size = buffer.get_mem_size();
        assert!(mem_size > 0);

        // Test cached memory size.
        let cached_size = buffer.get_mem_size_cached();
        assert_eq!(mem_size, cached_size.size().unwrap());
    }

    #[test]
    fn test_offscreen_buffer_memory_size_cache_invalidation() {
        let mut buffer = create_test_buffer();

        // Get initial memory size (should cache it).
        let initial_size = buffer.get_mem_size_cached();
        assert!(initial_size.size().unwrap() > 0);

        // Modify buffer (this should not affect the test directly, but shows usage).
        buffer.buffer[0][0] = create_test_pixel_char('T');

        // Invalidate cache.
        buffer.invalidate_memory_size_calc_cache();

        // Get memory size again (should recalculate).
        let new_size = buffer.get_mem_size_cached();

        // Size might be different due to the modification, but should still be positive.
        assert!(new_size.size().unwrap() > 0);
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
    fn test_offscreen_buffer_debug() {
        let buffer = create_test_buffer();

        let debug_output = format!("{buffer:?}");

        // Should contain some basic information.
        assert!(!debug_output.is_empty());
        // Debug output should contain window_size information.
        assert!(debug_output.contains("window_size"));
    }

    #[test]
    fn test_pixel_char_diff_chunks_debug() {
        let mut list = List::new();
        list.push((row(0) + col(0), create_test_pixel_char('A')));
        list.push((row(1) + col(1), create_test_pixel_char('B')));

        let chunks = PixelCharDiffChunks::from(list);
        let debug_output = format!("{chunks:?}");

        // Should contain debug information.
        assert!(!debug_output.is_empty());
        assert!(debug_output.contains('A') || debug_output.contains('B'));
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

    #[test]
    fn test_character_set_enum() {
        // Test that CharacterSet enum variants exist.
        let ascii = CharacterSet::Ascii;
        let dec_graphics = CharacterSet::DECGraphics;

        // They should be different.
        assert_ne!(ascii, dec_graphics);

        // Test debug formatting.
        let ascii_debug = format!("{ascii:?}");
        let dec_graphics_debug = format!("{dec_graphics:?}");

        assert!(ascii_debug.contains("Ascii"));
        assert!(dec_graphics_debug.contains("DECGraphics"));
    }
}
