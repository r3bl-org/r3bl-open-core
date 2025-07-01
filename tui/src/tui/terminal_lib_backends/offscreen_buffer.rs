/*
 *   Copyright (c) 2022-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */
use std::{fmt::{self, Debug, Write},
          ops::{Deref, DerefMut}};

use diff_chunks::PixelCharDiffChunks;
use smallvec::smallvec;

use super::{FlushKind, RenderOps};
use crate::{col,
            dim_underline,
            fg_green,
            fg_magenta,
            get_mem_size,
            inline_string,
            ok,
            row,
            tiny_inline_string,
            ColWidth,
            GetMemSize,
            InlineString,
            InlineVec,
            List,
            LockedOutputDevice,
            Pos,
            Size,
            TinyInlineString,
            TuiColor,
            TuiStyle};

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
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct OffscreenBuffer {
    pub buffer: PixelCharLines,
    pub window_size: Size,
    pub my_pos: Pos,
    pub my_fg_color: Option<TuiColor>,
    pub my_bg_color: Option<TuiColor>,
}

impl GetMemSize for OffscreenBuffer {
    fn get_mem_size(&self) -> usize {
        self.buffer.get_mem_size()
            + std::mem::size_of::<Size>()
            + std::mem::size_of::<Pos>()
            + std::mem::size_of::<Option<TuiColor>>()
            + std::mem::size_of::<Option<TuiColor>>()
    }
}

pub mod diff_chunks {
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
                        acc.push((pos, other_pixel_char.clone()));
                    }
                }
            }
            Some(PixelCharDiffChunks::from(acc))
        }

        /// Create a new buffer and fill it with empty chars.
        #[must_use]
        pub fn new_with_capacity_initialized(window_size: Size) -> Self {
            Self {
                buffer: PixelCharLines::new_with_capacity_initialized(window_size),
                window_size,
                my_pos: Pos::default(),
                my_fg_color: None,
                my_bg_color: None,
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
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PixelCharLines {
    pub lines: InlineVec<PixelCharLine>,
}

mod pixel_char_lines_impl {
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
        pub fn new_with_capacity_initialized(window_size: Size) -> Self {
            let window_height = window_size.row_height;
            let window_width = window_size.col_width;
            Self {
                lines: smallvec![
                    PixelCharLine::new_with_capacity_initialized(window_width);
                    window_height.as_usize()
                ],
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PixelCharLine {
    pub pixel_chars: InlineVec<PixelChar>,
}

impl GetMemSize for PixelCharLine {
    fn get_mem_size(&self) -> usize {
        get_mem_size::slice_size(self.pixel_chars.as_ref())
    }
}

mod pixel_char_line_impl {
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
                match current_range.is_empty() {
                    true => CurrentRange::DoesNotExist,
                    false => CurrentRange::Exists,
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
                // Start new current range.
                (
                    helpers::Peek::NextItemContinuesRange,
                    helpers::CurrentRange::DoesNotExist,
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
                // The next value continues the current range.
                (
                    helpers::Peek::NextItemContinuesRange,
                    helpers::CurrentRange::Exists,
                ) => {
                    acc_current_range.push(*value);
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
        pub fn new_with_capacity_initialized(window_width: ColWidth) -> Self {
            Self {
                pixel_chars: smallvec![PixelChar::Spacer; window_width.as_usize()],
            }
        }
    }

    impl Deref for PixelCharLine {
        type Target = InlineVec<PixelChar>;
        fn deref(&self) -> &Self::Target { &self.pixel_chars }
    }

    impl DerefMut for PixelCharLine {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.pixel_chars }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PixelChar {
    Void,
    Spacer,
    PlainText {
        text: TinyInlineString,
        maybe_style: Option<TuiStyle>,
    },
}

impl GetMemSize for PixelChar {
    fn get_mem_size(&self) -> usize {
        match self {
            PixelChar::Void => std::mem::size_of::<PixelChar>(),
            PixelChar::Spacer => std::mem::size_of::<PixelChar>(),
            PixelChar::PlainText {
                text,
                maybe_style: _,
            } => {
                std::mem::size_of::<PixelChar>()
                    + text.len()
                    + std::mem::size_of::<Option<TuiStyle>>()
            }
        }
    }
}

const EMPTY_CHAR: char = '╳';
const VOID_CHAR: char = '❯';

mod pixel_char_impl {
    use super::*;

    impl Default for PixelChar {
        fn default() -> Self { Self::Spacer }
    }

    impl Debug for PixelChar {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            const WIDTH: usize = 16;

            fn truncate(s: &str, max_chars: usize) -> &str {
                match s.char_indices().nth(max_chars) {
                    None => s,
                    Some((idx, _)) => &s[..idx],
                }
            }

            match self {
                PixelChar::Void => {
                    write!(f, " V {VOID_CHAR:░^WIDTH$}")?;
                }
                PixelChar::Spacer => {
                    write!(f, " S {EMPTY_CHAR:░^WIDTH$}")?;
                }
                PixelChar::PlainText { text, maybe_style } => {
                    // Need `acc_tmp` to be able to truncate the text if it's too long.
                    let mut acc_tmp = InlineString::with_capacity(WIDTH);
                    match maybe_style {
                        // Content + style.
                        Some(style) => {
                            write!(acc_tmp, "'{text}'→{style}")?;
                        }
                        // Content, no style.
                        _ => {
                            write!(acc_tmp, "'{text}'")?;
                        }
                    }
                    let trunc_output = truncate(&acc_tmp, WIDTH);
                    write!(f, " {} {trunc_output: ^WIDTH$}", fg_magenta("P"))?;
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
    use super::*;
    use crate::{assert_eq2, height, new_style, tui_color, width};

    #[test]
    fn test_offscreen_buffer_construction() {
        let window_size = width(10) + height(2);
        let my_offscreen_buffer =
            OffscreenBuffer::new_with_capacity_initialized(window_size);
        assert_eq2!(my_offscreen_buffer.buffer.len(), 2);
        assert_eq2!(my_offscreen_buffer.buffer[0].len(), 10);
        assert_eq2!(my_offscreen_buffer.buffer[1].len(), 10);
        for line in my_offscreen_buffer.buffer.iter() {
            for pixel_char in line.iter() {
                assert_eq2!(pixel_char, &PixelChar::Spacer);
            }
        }
        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
    }

    #[test]
    fn test_offscreen_buffer_re_init() {
        let window_size = width(10) + height(2);
        let mut my_offscreen_buffer =
            OffscreenBuffer::new_with_capacity_initialized(window_size);

        let text_1 = "a".into();
        my_offscreen_buffer.buffer[0][0] = PixelChar::PlainText {
            text: text_1,
            maybe_style: Some(new_style!(color_bg: {tui_color!(green)})),
        };

        let text_2 = "z".into();
        my_offscreen_buffer.buffer[1][9] = PixelChar::PlainText {
            text: text_2,
            maybe_style: Some(new_style!(color_bg: {tui_color!(red)})),
        };

        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
        my_offscreen_buffer.clear();
        for line in my_offscreen_buffer.buffer.iter() {
            for pixel_char in line.iter() {
                assert_eq2!(pixel_char, &PixelChar::Spacer);
            }
        }
        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
    }
}
