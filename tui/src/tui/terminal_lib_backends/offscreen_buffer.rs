// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use std::{fmt::{self, Debug},
          ops::{Deref, DerefMut}};

use diff_chunks::PixelCharDiffChunks;
use smallvec::smallvec;

use super::{FlushKind, RenderOps};
use crate::{CachedMemorySize, ColWidth, GetMemSize, InlineVec, List, LockedOutputDevice,
            MemoizedMemorySize, MemorySize, Pos, Size, TinyInlineString, TuiColor,
            TuiStyle, col, dim_underline, fg_green, fg_magenta, get_mem_size,
            inline_string, ok, row, tiny_inline_string};

/// Character set modes for terminal emulation.
///
/// Used by [`crate::core::pty_mux::ansi_parser::AnsiToBufferProcessor`] to handle
/// ESC ( sequences that switch between ASCII and DEC line-drawing graphics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharacterSet {
    /// Normal ASCII character set (ESC ( B)
    #[default]
    Ascii,
    /// DEC Special Graphics character set for line drawing (ESC ( 0)
    /// Maps ASCII characters to box-drawing Unicode characters
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
/// [`ANSI parser`]: crate::core::pty_mux::ansi_parser::AnsiToBufferProcessor
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
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToBufferProcessor`] to implement
    /// the DECSC (ESC 7) and DECRC (ESC 8) escape sequences for saving and restoring
    /// cursor position.
    ///
    /// ## Data Flow:
    /// ```text
    /// 1. Child process (e.g., vim) sends ESC 7 to save cursor
    ///                             ↓
    /// 2. AnsiToBufferProcessor::esc_dispatch() handles ESC 7
    ///                             ↓
    /// 3. Saves current cursor_pos to buffer.ansi_parser_support.cursor_pos_for_esc_save_and_restore
    ///                             ↓
    /// 4. Later, child sends ESC 8 to restore cursor
    ///                             ↓
    /// 5. AnsiToBufferProcessor::esc_dispatch() handles ESC 8
    ///                             ↓
    /// 6. Restores cursor_pos from buffer.ansi_parser_support.cursor_pos_for_esc_save_and_restore
    /// ```
    pub cursor_pos_for_esc_save_and_restore: Option<Pos>,

    /// Active character set for ANSI escape sequence support.
    ///
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToBufferProcessor`] to implement
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
    /// Used by [`crate::core::pty_mux::ansi_parser::AnsiToBufferProcessor`] to control
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

    /// Complete computed style combining attributes and colors for efficient rendering
    pub current_style: Option<crate::TuiStyle>,

    /// Text attributes (bold, italic, underline, etc.) from SGR sequences
    pub attribs: crate::TuiStyleAttribs,

    /// Current foreground color from SGR color sequences
    pub fg_color: Option<crate::TuiColor>,

    /// Current background color from SGR color sequences
    pub bg_color: Option<crate::TuiColor>,

    /// OSC events (hyperlinks, titles, etc.) accumulated during processing
    pub pending_osc_events: Vec<crate::core::osc::OscEvent>,
}

impl Default for AnsiParserSupport {
    /// Creates a new `AnsiParserSupport` with VT100-compliant defaults.
    fn default() -> Self {
        Self {
            cursor_pos_for_esc_save_and_restore: None,
            character_set: CharacterSet::default(),
            auto_wrap_mode: true, // DECAWM default: enabled (VT100 compliant)
            current_style: None,
            attribs: crate::TuiStyleAttribs::default(),
            fg_color: None,
            bg_color: None,
            pending_osc_events: Vec::new(),
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
///    [`crate::core::pty_mux::ansi_parser::AnsiToBufferProcessor`] and the
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
    memory_size_calc_cache: MemoizedMemorySize,
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

pub mod diff_chunks {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// This is a wrapper type so the [`std::fmt::Debug`] can be implemented for it, that
    /// won't conflict with [List]'s implementation of the trait.
    #[derive(Clone, Default, PartialEq)]
    pub struct PixelCharDiffChunks {
        pub inner: List<DiffChunk>,
    }

    pub type DiffChunk = (Pos, PixelChar);

    impl Deref for PixelCharDiffChunks {
        type Target = List<DiffChunk>;

        fn deref(&self) -> &Self::Target { &self.inner }
    }

    impl From<List<DiffChunk>> for PixelCharDiffChunks {
        fn from(list: List<DiffChunk>) -> Self { Self { inner: list } }
    }
}

mod offscreen_buffer_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
            // Invalidate and recalculate cache when buffer is accessed mutably
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
        pub fn get_mem_size_cached(&mut self) -> MemorySize {
            self.get_cached_memory_size()
        }

        /// Invalidates and immediately recalculates the memory size cache.
        /// Call this when buffer content changes to ensure the cache is always valid.
        fn invalidate_memory_size_calc_cache(&mut self) {
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
        pub fn new_empty(window_size_arg: impl Into<Size>) -> Self {
            let window_size = window_size_arg.into();
            let mut buffer = Self {
                buffer: PixelCharLines::new_empty(window_size),
                window_size,
                my_pos: Pos::default(),
                my_fg_color: None,
                my_bg_color: None,
                memory_size_calc_cache: MemoizedMemorySize::default(),
                ansi_parser_support: AnsiParserSupport::default(),
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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PixelCharLines {
    pub lines: InlineVec<PixelCharLine>,
}

mod pixel_char_lines_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl GetMemSize for PixelCharLines {
        fn get_mem_size(&self) -> usize { get_mem_size::slice_size(self.lines.as_ref()) }
    }

    impl Deref for PixelCharLines {
        type Target = InlineVec<PixelCharLine>;
        fn deref(&self) -> &Self::Target { &self.lines }
    }

    impl DerefMut for PixelCharLines {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.lines }
    }

    impl PixelCharLines {
        #[must_use]
        pub fn new_empty(window_size_arg: impl Into<Size>) -> Self {
            let window_size = window_size_arg.into();
            let window_height = window_size.row_height;
            let window_width = window_size.col_width;
            Self {
                lines: smallvec![
                    PixelCharLine::new_empty(window_width);
                    window_height.as_usize()
                ],
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PixelCharLine {
    pub pixel_chars: Vec<PixelChar>,
}

impl GetMemSize for PixelCharLine {
    fn get_mem_size(&self) -> usize {
        get_mem_size::slice_size(self.pixel_chars.as_ref())
    }
}

mod pixel_char_line_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for PixelCharLine {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // Pretty print only so many chars per line (depending on the terminal width
            // in which log.fish is run).
            const MAX_PIXEL_CHARS_PER_LINE: usize = 6;

            let mut void_indices: InlineVec<usize> = smallvec![];
            let mut spacer_indices: InlineVec<usize> = smallvec![];
            let mut void_count: InlineVec<TinyInlineString> = smallvec![];
            let mut spacer_count: InlineVec<TinyInlineString> = smallvec![];

            let mut char_count = 0;

            // Loop: for each PixelChar in a line (pixel_chars_lines[row_index]).
            for (col_index, pixel_char) in self.iter().enumerate() {
                match pixel_char {
                    PixelChar::Void => {
                        void_count.push(TinyInlineString::from(col_index.to_string()));
                        void_indices.push(col_index);
                    }
                    PixelChar::Spacer => {
                        spacer_count.push(TinyInlineString::from(col_index.to_string()));
                        spacer_indices.push(col_index);
                    }
                    PixelChar::PlainText { .. } => {}
                }

                // Index message.
                write!(
                    f,
                    "{}{:?}",
                    dim_underline(&tiny_inline_string!("{col_index:03}")),
                    pixel_char
                )?;

                // Add \n every MAX_CHARS_PER_LINE characters.
                char_count += 1;
                if char_count >= MAX_PIXEL_CHARS_PER_LINE {
                    char_count = 0;
                    writeln!(f)?;
                }
            }

            // Pretty print the spacers & voids (of any of either or both) at the end of
            // the output.
            {
                if !void_count.is_empty() {
                    write!(f, "void [ ")?;
                    fmt_impl_index_values(&void_indices, f)?;
                    write!(f, " ]")?;

                    // Add spacer divider if spacer count exists (next).
                    if !spacer_count.is_empty() {
                        write!(f, " | ")?;
                    }
                }

                if !spacer_count.is_empty() {
                    // Add comma divider if void count exists (previous).
                    if !void_count.is_empty() {
                        write!(f, ", ")?;
                    }
                    write!(f, "spacer [ ")?;
                    fmt_impl_index_values(&spacer_indices, f)?;
                    write!(f, " ]")?;
                }
            }

            ok!()
        }
    }

    fn fmt_impl_index_values(
        values: &[usize],
        f: &mut fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        mod helpers {
            pub enum Peek {
                NextItemContinuesRange,
                NextItemDoesNotContinueRange,
            }

            pub fn peek_does_next_item_continues_range(
                values: &[usize],
                index: usize,
            ) -> Peek {
                if values.get(index + 1).is_none() {
                    return Peek::NextItemDoesNotContinueRange;
                }
                if values[index + 1] == values[index] + 1 {
                    Peek::NextItemContinuesRange
                } else {
                    Peek::NextItemDoesNotContinueRange
                }
            }

            pub enum CurrentRange {
                DoesNotExist,
                Exists,
            }

            pub fn does_current_range_exist(current_range: &[usize]) -> CurrentRange {
                if current_range.is_empty() {
                    CurrentRange::DoesNotExist
                } else {
                    CurrentRange::Exists
                }
            }
        }

        // Track state thru loop iteration.
        let mut acc_current_range: InlineVec<usize> = smallvec![];

        // Main loop.
        for (index, value) in values.iter().enumerate() {
            match (
                helpers::peek_does_next_item_continues_range(values, index),
                helpers::does_current_range_exist(&acc_current_range),
            ) {
                // Start new current range OR the next value continues the current range.
                (
                    helpers::Peek::NextItemContinuesRange,
                    helpers::CurrentRange::DoesNotExist | helpers::CurrentRange::Exists,
                ) => {
                    acc_current_range.push(*value);
                }
                // The next value does not continue the current range & the current range
                // does not exist.
                (
                    helpers::Peek::NextItemDoesNotContinueRange,
                    helpers::CurrentRange::DoesNotExist,
                ) => {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{value}")?;
                }
                // The next value does not continue the current range & the current range
                // exists.
                (
                    helpers::Peek::NextItemDoesNotContinueRange,
                    helpers::CurrentRange::Exists,
                ) => {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    acc_current_range.push(*value);
                    write!(
                        f,
                        "{}-{}",
                        acc_current_range[0],
                        acc_current_range[acc_current_range.len() - 1]
                    )?;
                    acc_current_range.clear();
                }
            }
        }

        ok!()
    }

    // This represents a single row on the screen (i.e. a line of text).
    impl PixelCharLine {
        /// Create a new row with the given width and fill it with the empty chars.
        #[must_use]
        pub fn new_empty(window_width_arg: impl Into<ColWidth>) -> Self {
            let window_width = window_width_arg.into();
            Self {
                pixel_chars: vec![PixelChar::Spacer; window_width.as_usize()],
            }
        }
    }

    impl Deref for PixelCharLine {
        type Target = Vec<PixelChar>;
        fn deref(&self) -> &Self::Target { &self.pixel_chars }
    }

    impl DerefMut for PixelCharLine {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.pixel_chars }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelChar {
    Void,
    Spacer,
    PlainText {
        display_char: char,
        maybe_style: Option<TuiStyle>,
    },
}

impl GetMemSize for PixelChar {
    fn get_mem_size(&self) -> usize {
        // Since PixelChar is now Copy, its size is fixed
        std::mem::size_of::<PixelChar>()
    }
}

const EMPTY_CHAR: char = '╳';
const VOID_CHAR: char = '❯';

mod pixel_char_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Default for PixelChar {
        fn default() -> Self { Self::Spacer }
    }

    impl Debug for PixelChar {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            const WIDTH: usize = 16;

            match self {
                PixelChar::Void => {
                    write!(f, " V {VOID_CHAR:░^WIDTH$}")?;
                }
                PixelChar::Spacer => {
                    write!(f, " S {EMPTY_CHAR:░^WIDTH$}")?;
                }
                PixelChar::PlainText {
                    display_char,
                    maybe_style,
                } => {
                    match maybe_style {
                        // Content + style.
                        Some(style) => {
                            write!(
                                f,
                                " {} '{display_char}'→{style: ^WIDTH$}",
                                fg_magenta("P")
                            )?;
                        }
                        // Content, no style.
                        _ => {
                            write!(f, " {} '{display_char}': ^WIDTH$", fg_magenta("P"))?;
                        }
                    }
                }
            }

            ok!()
        }
    }
}

pub trait OffscreenBufferPaint {
    fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOps;

    fn render_diff(&mut self, diff_chunks: &PixelCharDiffChunks) -> RenderOps;

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

#[cfg(test)]
mod tests {
    use super::{test_fixtures_offscreen_buffer::*, *};
    use crate::{ANSIBasicColor, assert_eq2, height, new_style, tui_color,
                tui_style_attrib::{Bold, Dim, Italic, Reverse, Underline},
                tui_style_attribs, width};

    #[test]
    fn test_offscreen_buffer_construction() {
        let window_size = width(10) + height(2);
        let ofs_buf = OffscreenBuffer::new_empty(window_size);
        assert_eq2!(ofs_buf.buffer.len(), 2);
        assert_eq2!(ofs_buf.buffer[0].len(), 10);
        assert_eq2!(ofs_buf.buffer[1].len(), 10);

        // Check all cells are empty using assert_empty_at
        for row in 0..2 {
            for col in 0..10 {
                assert_empty_at(&ofs_buf, row, col);
            }
        }
        // println!("my_offscreen_buffer: {:#?}", my_offscreen_buffer);
    }

    #[test]
    fn test_offscreen_buffer_re_init() {
        let window_size = width(10) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        ofs_buf.buffer[0][0] = PixelChar::PlainText {
            display_char: 'a',
            maybe_style: Some(new_style!(color_bg: {tui_color!(green)})),
        };

        ofs_buf.buffer[1][9] = PixelChar::PlainText {
            display_char: 'z',
            maybe_style: Some(new_style!(color_bg: {tui_color!(red)})),
        };

        // Verify the characters were set correctly
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'a',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::Green))
                )
            },
            "green background",
        );

        assert_styled_char_at(
            &ofs_buf,
            1,
            9,
            'z',
            |style_from_buffer| {
                matches!(
                    style_from_buffer.color_bg,
                    Some(TuiColor::Basic(ANSIBasicColor::Red))
                )
            },
            "red background",
        );

        // println!("my_offscreen_buffer: {:#?}", my_offscreen_buffer);
        ofs_buf.clear();

        // Check all cells are empty using assert_empty_at
        for row in 0..2 {
            for col in 0..10 {
                assert_empty_at(&ofs_buf, row, col);
            }
        }
        // println!("my_offscreen_buffer: {:#?}", my_offscreen_buffer);
    }

    #[test]
    fn test_memory_size_caching() {
        let window_size = width(10) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // First call should calculate and cache
        let size1 = ofs_buf.get_mem_size_cached();
        assert_ne!(format!("{size1}"), "?");

        // Second call should use cached value (no recalculation)
        let size2 = ofs_buf.get_mem_size_cached();
        assert_eq!(format!("{size1}"), format!("{}", size2));

        // Modify buffer through DerefMut (invalidates cache)
        ofs_buf.buffer[0][0] = PixelChar::PlainText {
            display_char: 'x',
            maybe_style: None,
        };

        // Verify the character was set
        assert_plain_char_at(&ofs_buf, 0, 0, 'x');

        // Next call should recalculate
        let size3 = ofs_buf.get_mem_size_cached();
        assert_ne!(format!("{size3}"), "?");

        // Clear should also invalidate cache
        ofs_buf.clear();
        let size4 = ofs_buf.get_mem_size_cached();
        assert_ne!(format!("{size4}"), "?");
    }

    #[test]
    fn test_buffer_text_operations() {
        let window_size = width(10) + height(3);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Write "Hello" at row 0
        for (i, ch) in "Hello".chars().enumerate() {
            ofs_buf.buffer[0][i] = PixelChar::PlainText {
                display_char: ch,
                maybe_style: None,
            };
        }

        // Verify using assert_plain_text_at
        assert_plain_text_at(&ofs_buf, 0, 0, "Hello");

        // Verify remaining cells in row 0 are empty
        for col in 5..10 {
            assert_empty_at(&ofs_buf, 0, col);
        }

        // Verify other rows are completely empty
        for row in 1..3 {
            for col in 0..10 {
                assert_empty_at(&ofs_buf, row, col);
            }
        }
    }

    #[test]
    fn test_buffer_styled_content() {
        let window_size = width(10) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Add bold red text
        ofs_buf.buffer[0][0] = PixelChar::PlainText {
            display_char: 'R',
            maybe_style: Some(new_style!(
                bold color_fg: {tui_color!(red)}
            )),
        };

        // Add italic blue text
        ofs_buf.buffer[0][1] = PixelChar::PlainText {
            display_char: 'B',
            maybe_style: Some(new_style!(
                italic color_fg: {tui_color!(blue)}
            )),
        };

        // Verify with comprehensive style checks
        assert_styled_char_at(
            &ofs_buf,
            0,
            0,
            'R',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Bold)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Red))
            },
            "bold red text",
        );

        assert_styled_char_at(
            &ofs_buf,
            0,
            1,
            'B',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Italic)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Blue))
            },
            "italic blue text",
        );
    }

    #[test]
    fn test_buffer_mixed_content() {
        let window_size = width(8) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Row 0: "Hi " followed by styled "World"
        ofs_buf.buffer[0][0] = PixelChar::PlainText {
            display_char: 'H',
            maybe_style: None,
        };
        ofs_buf.buffer[0][1] = PixelChar::PlainText {
            display_char: 'i',
            maybe_style: None,
        };
        ofs_buf.buffer[0][2] = PixelChar::PlainText {
            display_char: ' ',
            maybe_style: None,
        };

        // Add styled "World"
        for (i, ch) in "World".chars().enumerate() {
            ofs_buf.buffer[0][3 + i] = PixelChar::PlainText {
                display_char: ch,
                maybe_style: Some(new_style!(
                    underline color_fg: {tui_color!(green)}
                )),
            };
        }

        // Verify plain text
        assert_plain_text_at(&ofs_buf, 0, 0, "Hi ");

        // Verify each styled character
        for (i, ch) in "World".chars().enumerate() {
            assert_styled_char_at(
                &ofs_buf,
                0,
                3 + i,
                ch,
                |style_from_buffer| {
                    style_from_buffer.attribs == tui_style_attribs(Underline)
                        && style_from_buffer.color_fg
                            == Some(TuiColor::Basic(ANSIBasicColor::Green))
                },
                "underlined green text",
            );
        }

        // Verify row 1 is empty
        for col in 0..8 {
            assert_empty_at(&ofs_buf, 1, col);
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_comprehensive_buffer_validation() {
        // This test demonstrates comprehensive use of all test utilities
        let window_size = width(15) + height(4);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Initially, verify entire buffer is empty
        for row in 0..4 {
            for col in 0..15 {
                assert_empty_at(&ofs_buf, row, col);
            }
        }

        // Add plain text "Hello" at row 0
        for (i, ch) in "Hello".chars().enumerate() {
            ofs_buf.buffer[0][i] = PixelChar::PlainText {
                display_char: ch,
                maybe_style: None,
            };
        }

        // Verify plain text was added correctly
        assert_plain_text_at(&ofs_buf, 0, 0, "Hello");

        // Verify individual plain characters
        assert_plain_char_at(&ofs_buf, 0, 0, 'H');
        assert_plain_char_at(&ofs_buf, 0, 1, 'e');
        assert_plain_char_at(&ofs_buf, 0, 4, 'o');

        // Add styled "World!" at row 1 with different styles
        ofs_buf.buffer[1][0] = PixelChar::PlainText {
            display_char: 'W',
            maybe_style: Some(new_style!(bold color_fg: {tui_color!(red)})),
        };

        ofs_buf.buffer[1][1] = PixelChar::PlainText {
            display_char: 'o',
            maybe_style: Some(new_style!(italic color_fg: {tui_color!(blue)})),
        };

        ofs_buf.buffer[1][2] = PixelChar::PlainText {
            display_char: 'r',
            maybe_style: Some(new_style!(underline color_fg: {tui_color!(green)})),
        };

        ofs_buf.buffer[1][3] = PixelChar::PlainText {
            display_char: 'l',
            maybe_style: Some(new_style!(dim color_bg: {tui_color!(yellow)})),
        };

        ofs_buf.buffer[1][4] = PixelChar::PlainText {
            display_char: 'd',
            maybe_style: Some(new_style!(reverse)),
        };

        ofs_buf.buffer[1][5] = PixelChar::PlainText {
            display_char: '!',
            maybe_style: Some(new_style!(
                bold italic underline
                color_fg: {tui_color!(magenta)}
            )),
        };

        // Verify each styled character with specific style checks
        assert_styled_char_at(
            &ofs_buf,
            1,
            0,
            'W',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Bold)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Red))
            },
            "bold red W",
        );

        assert_styled_char_at(
            &ofs_buf,
            1,
            1,
            'o',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Italic)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Blue))
            },
            "italic blue o",
        );

        assert_styled_char_at(
            &ofs_buf,
            1,
            2,
            'r',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Underline)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Green))
            },
            "underlined green r",
        );

        assert_styled_char_at(
            &ofs_buf,
            1,
            3,
            'l',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Dim)
                    && style_from_buffer.color_bg
                        == Some(TuiColor::Basic(ANSIBasicColor::Yellow))
            },
            "dim with yellow background l",
        );

        assert_styled_char_at(
            &ofs_buf,
            1,
            4,
            'd',
            |style_from_buffer| style_from_buffer.attribs == tui_style_attribs(Reverse),
            "reversed d",
        );

        assert_styled_char_at(
            &ofs_buf,
            1,
            5,
            '!',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Bold + Italic + Underline)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Magenta))
            },
            "multi-styled exclamation",
        );

        // Add mixed content on row 2: plain and styled alternating
        for i in 0..6 {
            if i % 2 == 0 {
                // Even positions: plain text
                ofs_buf.buffer[2][i] = PixelChar::PlainText {
                    #[allow(clippy::cast_possible_truncation)]
                    display_char: char::from_digit(i as u32, 10).unwrap(),
                    maybe_style: None,
                };
            } else {
                // Odd positions: styled text
                ofs_buf.buffer[2][i] = PixelChar::PlainText {
                    #[allow(clippy::cast_possible_truncation)]
                    display_char: char::from_digit(i as u32, 10).unwrap(),
                    maybe_style: Some(new_style!(bold color_fg: {tui_color!(cyan)})),
                };
            }
        }

        // Verify alternating plain and styled characters
        for i in 0..6 {
            #[allow(clippy::cast_possible_truncation)]
            let expected_char = char::from_digit(i as u32, 10).unwrap();
            if i % 2 == 0 {
                assert_plain_char_at(&ofs_buf, 2, i, expected_char);
            } else {
                assert_styled_char_at(
                    &ofs_buf,
                    2,
                    i,
                    expected_char,
                    |style_from_buffer| {
                        style_from_buffer.attribs == tui_style_attribs(Bold)
                            && style_from_buffer.color_fg
                                == Some(TuiColor::Basic(ANSIBasicColor::Cyan))
                    },
                    "bold cyan digit",
                );
            }
        }

        // Verify remaining cells in each row are empty
        for col in 5..15 {
            assert_empty_at(&ofs_buf, 0, col); // After "Hello"
        }
        for col in 6..15 {
            assert_empty_at(&ofs_buf, 1, col); // After "World!"
            assert_empty_at(&ofs_buf, 2, col); // After "012345"
        }

        // Verify row 3 is completely empty
        for col in 0..15 {
            assert_empty_at(&ofs_buf, 3, col);
        }

        // Test partial clear: clear row 1
        for col in 0..15 {
            ofs_buf.buffer[1][col] = PixelChar::Spacer;
        }

        // Verify row 1 is now empty
        for col in 0..15 {
            assert_empty_at(&ofs_buf, 1, col);
        }

        // Verify other content is still intact
        assert_plain_text_at(&ofs_buf, 0, 0, "Hello");
        assert_plain_char_at(&ofs_buf, 2, 0, '0');
        assert_styled_char_at(
            &ofs_buf,
            2,
            1,
            '1',
            |style_from_buffer| style_from_buffer.attribs == tui_style_attribs(Bold),
            "bold cyan 1",
        );
    }

    #[test]
    fn test_diff_method() {
        // Test the diff method that compares two buffers
        let window_size = width(5) + height(3);
        let mut ofs_buf_1 = OffscreenBuffer::new_empty(window_size);
        let mut ofs_buf_2 = OffscreenBuffer::new_empty(window_size);

        // Initially both buffers are empty, diff should be None or empty
        let diff = ofs_buf_1.diff(&ofs_buf_2);
        assert!(
            diff.is_none() || diff.unwrap().is_empty(),
            "Diff of two empty buffers should be None or empty"
        );

        // Add content to buffer1
        ofs_buf_1.buffer[0][0] = PixelChar::PlainText {
            display_char: 'A',
            maybe_style: None,
        };
        ofs_buf_1.buffer[1][2] = PixelChar::PlainText {
            display_char: 'B',
            maybe_style: Some(new_style!(bold)),
        };

        // Diff should now show differences
        let diff = ofs_buf_1.diff(&ofs_buf_2);
        assert!(
            diff.is_some(),
            "Diff should return Some when there are differences"
        );
        let diff_chunks = diff.unwrap();
        assert!(!diff_chunks.is_empty(), "Diff should detect differences");

        // Verify the diff chunks contain the expected positions
        let has_00_diff = diff_chunks.iter().any(|(pos, _)| *pos == row(0) + col(0));
        let has_12_diff = diff_chunks.iter().any(|(pos, _)| *pos == row(1) + col(2));
        assert!(has_00_diff, "Diff should detect change at [0][0]");
        assert!(has_12_diff, "Diff should detect change at [1][2]");

        // Make buffer2 match buffer1
        ofs_buf_2.buffer[0][0] = PixelChar::PlainText {
            display_char: 'A',
            maybe_style: None,
        };
        ofs_buf_2.buffer[1][2] = PixelChar::PlainText {
            display_char: 'B',
            maybe_style: Some(new_style!(bold)),
        };

        // Now diff should be None or empty again
        let diff = ofs_buf_1.diff(&ofs_buf_2);
        assert!(
            diff.is_none() || diff.unwrap().is_empty(),
            "Diff of identical buffers should be None or empty"
        );

        // Test style-only changes
        ofs_buf_1.buffer[0][0] = PixelChar::PlainText {
            display_char: 'A',
            maybe_style: Some(new_style!(italic)),
        };

        let diff = ofs_buf_1.diff(&ofs_buf_2);
        assert!(
            diff.is_some() && !diff.unwrap().is_empty(),
            "Diff should detect style-only changes"
        );
    }

    #[test]
    fn test_deref_and_deref_mut() {
        // Test Deref and DerefMut implementations
        let window_size = width(3) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Test Deref (read-only access)
        let rows: &PixelCharLines = &ofs_buf;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].len(), 3);

        // Verify all cells are initially Spacer
        for row in 0..2 {
            for col in 0..3 {
                assert_empty_at(&ofs_buf, row, col);
            }
        }

        // Test DerefMut (mutable access)
        let rows_mut: &mut PixelCharLines = &mut ofs_buf;
        rows_mut[0][1] = PixelChar::PlainText {
            display_char: 'X',
            maybe_style: None,
        };

        // Verify the change
        assert_plain_char_at(&ofs_buf, 0, 1, 'X');

        // Verify cache invalidation happens with DerefMut
        let _size1 = ofs_buf.get_mem_size_cached();
        // Mutate through DerefMut
        ofs_buf.buffer[1][1] = PixelChar::PlainText {
            display_char: 'Y',
            maybe_style: None,
        };
        // Cache should be invalidated (tested indirectly through memory_size_caching
        // test)
        assert_plain_char_at(&ofs_buf, 1, 1, 'Y');
    }

    #[test]
    fn test_pixel_char_variants() {
        // Test different PixelChar variants and their behavior
        let window_size = width(4) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Test Spacer variant (default)
        assert!(matches!(ofs_buf.buffer[0][0], PixelChar::Spacer));
        assert_empty_at(&ofs_buf, 0, 0);

        // Test PlainText with no style
        ofs_buf.buffer[0][1] = PixelChar::PlainText {
            display_char: 'a',
            maybe_style: None,
        };
        assert_plain_char_at(&ofs_buf, 0, 1, 'a');

        // Test PlainText with style
        ofs_buf.buffer[0][2] = PixelChar::PlainText {
            display_char: 'b',
            maybe_style: Some(new_style!(underline color_fg: {tui_color!(red)})),
        };
        assert_styled_char_at(
            &ofs_buf,
            0,
            2,
            'b',
            |style_from_buffer| {
                style_from_buffer.attribs == tui_style_attribs(Underline)
                    && style_from_buffer.color_fg
                        == Some(TuiColor::Basic(ANSIBasicColor::Red))
            },
            "underlined red 'b'",
        );

        // Test space character with no style (should be considered empty)
        ofs_buf.buffer[1][0] = PixelChar::PlainText {
            display_char: ' ',
            maybe_style: None,
        };
        assert_empty_at(&ofs_buf, 1, 0);

        // Test space character with style (should NOT be considered empty)
        ofs_buf.buffer[1][1] = PixelChar::PlainText {
            display_char: ' ',
            maybe_style: Some(new_style!(color_bg: {tui_color!(blue)})),
        };
        // This should NOT be empty because it has a style
        match &ofs_buf.buffer[1][1] {
            PixelChar::PlainText {
                display_char,
                maybe_style,
            } => {
                assert_eq!(*display_char, ' ');
                assert!(maybe_style.is_some());
            }
            _ => panic!("Expected styled space"),
        }
    }

    #[test]
    fn test_invalidate_memory_size_calc_cache() {
        // Test cache invalidation through mutations
        let window_size = width(3) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Get initial size (calculates and caches)
        let size1 = ofs_buf.get_mem_size_cached();
        assert_ne!(format!("{size1}"), "?");

        // Modify buffer content (this should invalidate cache via DerefMut)
        ofs_buf.buffer[0][0] = PixelChar::PlainText {
            display_char: 'X',
            maybe_style: None,
        };

        // Get size again - should be recalculated
        let size2 = ofs_buf.get_mem_size_cached();
        assert_ne!(format!("{size2}"), "?");

        // The test_memory_size_caching test already covers this more thoroughly
        // This test just verifies the cache invalidation mechanism works
    }

    #[test]
    fn test_buffer_boundaries() {
        // Test edge cases and boundary conditions
        let window_size = width(2) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Test all corners
        ofs_buf.buffer[0][0] = PixelChar::PlainText {
            display_char: '1',
            maybe_style: None,
        };
        ofs_buf.buffer[0][1] = PixelChar::PlainText {
            display_char: '2',
            maybe_style: None,
        };
        ofs_buf.buffer[1][0] = PixelChar::PlainText {
            display_char: '3',
            maybe_style: None,
        };
        ofs_buf.buffer[1][1] = PixelChar::PlainText {
            display_char: '4',
            maybe_style: None,
        };

        assert_plain_char_at(&ofs_buf, 0, 0, '1');
        assert_plain_char_at(&ofs_buf, 0, 1, '2');
        assert_plain_char_at(&ofs_buf, 1, 0, '3');
        assert_plain_char_at(&ofs_buf, 1, 1, '4');

        // Test clearing and reinitializing
        ofs_buf.clear();
        for row in 0..2 {
            for col in 0..2 {
                assert_empty_at(&ofs_buf, row, col);
            }
        }
    }

    #[test]
    fn test_diff_with_styles() {
        // Test diff method with complex style changes
        let window_size = width(3) + height(2);
        let mut ofs_buf_1 = OffscreenBuffer::new_empty(window_size);
        let mut ofs_buf_2 = OffscreenBuffer::new_empty(window_size);

        // Set up buffer1 with styled content
        ofs_buf_1.buffer[0][0] = PixelChar::PlainText {
            display_char: 'A',
            maybe_style: Some(new_style!(bold color_fg: {tui_color!(red)})),
        };

        // Set up buffer2 with same char but different style
        ofs_buf_2.buffer[0][0] = PixelChar::PlainText {
            display_char: 'A',
            maybe_style: Some(new_style!(italic color_fg: {tui_color!(blue)})),
        };

        // Diff should detect the style difference
        let diff = ofs_buf_1.diff(&ofs_buf_2);
        assert!(
            diff.is_some() && !diff.unwrap().is_empty(),
            "Diff should detect style differences"
        );

        // Change buffer2 to match buffer1 exactly
        ofs_buf_2.buffer[0][0] = PixelChar::PlainText {
            display_char: 'A',
            maybe_style: Some(new_style!(bold color_fg: {tui_color!(red)})),
        };

        let diff = ofs_buf_1.diff(&ofs_buf_2);
        assert!(
            diff.is_none() || diff.unwrap().is_empty(),
            "Diff should be None or empty when styles match"
        );
    }

    #[test]
    fn test_default_pixel_char() {
        // Test PixelChar::default()
        let default_char = PixelChar::default();
        assert!(matches!(default_char, PixelChar::Spacer));

        // Test that new buffer uses default
        let window_size = width(1) + height(1);
        let ofs_buf = OffscreenBuffer::new_empty(window_size);
        assert!(matches!(ofs_buf.buffer[0][0], PixelChar::Spacer));
    }
}

#[cfg(test)]
pub mod test_fixtures_offscreen_buffer {
    use super::*;

    /// Assert a plain character at a specific position
    ///
    /// # Panics
    ///
    /// Panics if the row or column is out of bounds, or if the character at the position
    /// doesn't match the expected character.
    pub fn assert_plain_char_at(
        ofs_buf: &OffscreenBuffer,
        row: usize,
        col: usize,
        expected_char: char,
    ) {
        // Add bounds checking with custom error messages
        assert!(
            row < ofs_buf.buffer.len(),
            "Row {} is out of bounds (buffer has {} rows)",
            row,
            ofs_buf.buffer.len()
        );
        assert!(
            col < ofs_buf.buffer[row].len(),
            "Column {} is out of bounds at row {} (row has {} columns)",
            col,
            row,
            ofs_buf.buffer[row].len()
        );

        match &ofs_buf.buffer[row][col] {
            PixelChar::PlainText { display_char, .. } => {
                assert_eq!(
                    *display_char, expected_char,
                    "Expected {expected_char} at [{row}][{col}], but found {display_char}"
                );
            }
            other => panic!(
                "Expected PlainText with '{expected_char}' at [{row}][{col}], but found {other:?}"
            ),
        }
    }

    /// Assert a styled character with style validation
    ///
    /// # Panics
    ///
    /// Panics if the row or column is out of bounds, if the character doesn't match,
    /// or if the style validation fails.
    pub fn assert_styled_char_at(
        ofs_buf: &OffscreenBuffer,
        row: usize,
        col: usize,
        expected_char: char,
        check_style: impl Fn(&TuiStyle) -> bool,
        style_desc: &str,
    ) {
        // Add bounds checking with custom error messages
        assert!(
            row < ofs_buf.buffer.len(),
            "Row {} is out of bounds (buffer has {} rows)",
            row,
            ofs_buf.buffer.len()
        );
        assert!(
            col < ofs_buf.buffer[row].len(),
            "Column {} is out of bounds at row {} (row has {} columns)",
            col,
            row,
            ofs_buf.buffer[row].len()
        );

        match &ofs_buf.buffer[row][col] {
            PixelChar::PlainText {
                display_char,
                maybe_style,
            } => {
                assert_eq!(
                    *display_char, expected_char,
                    "Expected '{expected_char}' at [{row}][{col}], but found '{display_char}'"
                );

                if let Some(style_from_buffer) = maybe_style {
                    assert!(
                        check_style(style_from_buffer),
                        "Style check failed at [{row}][{col}]: {style_desc}"
                    );
                } else {
                    panic!(
                        "Expected styled character '{expected_char}' at [{row}][{col}], but no style found"
                    );
                }
            }
            other => panic!(
                "Expected styled PlainText with '{expected_char}' at [{row}][{col}], but found {other:?}"
            ),
        }
    }

    /// Assert a cell is empty (Spacer or unstyled space)
    ///
    /// # Panics
    ///
    /// Panics if the row or column is out of bounds, or if the cell is not empty.
    pub fn assert_empty_at(ofs_buf: &OffscreenBuffer, row: usize, col: usize) {
        // Add bounds checking with custom error messages
        assert!(
            row < ofs_buf.buffer.len(),
            "Row {} is out of bounds (buffer has {} rows)",
            row,
            ofs_buf.buffer.len()
        );
        assert!(
            col < ofs_buf.buffer[row].len(),
            "Column {} is out of bounds at row {} (row has {} columns)",
            col,
            row,
            ofs_buf.buffer[row].len()
        );

        match &ofs_buf.buffer[row][col] {
            PixelChar::Spacer
            | PixelChar::PlainText {
                display_char: ' ',
                maybe_style: None,
            } => {} // OK - empty or space

            other => panic!("Expected empty cell at [{row}][{col}], but found {other:?}"),
        }
    }

    /// Assert plain text string starting at position
    ///
    /// # Panics
    ///
    /// Panics if the row is out of bounds, or if the text at the position doesn't
    /// match the expected string.
    pub fn assert_plain_text_at(
        ofs_buf: &OffscreenBuffer,
        row: usize,
        start_col: usize,
        expected_text: &str,
    ) {
        // Add bounds checking for the entire text string
        assert!(
            row < ofs_buf.buffer.len(),
            "Row {} is out of bounds (buffer has {} rows)",
            row,
            ofs_buf.buffer.len()
        );

        let end_col = start_col + expected_text.chars().count();
        assert!(
            end_col <= ofs_buf.buffer[row].len(),
            "Text '{}' starting at column {} would extend to column {} which is beyond row {} width of {} columns",
            expected_text,
            start_col,
            end_col,
            row,
            ofs_buf.buffer[row].len()
        );

        for (i, expected_char) in expected_text.chars().enumerate() {
            assert_plain_char_at(ofs_buf, row, start_col + i, expected_char);
        }
    }
}
