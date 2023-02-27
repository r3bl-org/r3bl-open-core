/*
 *   Copyright (c) 2022 R3BL LLC
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

use std::{fmt::{self, Debug},
          ops::{Deref, DerefMut}};

use async_trait::async_trait;
use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::*;

/// Represents a grid of cells where the row/column index maps to the terminal screen. This works
/// regardless of the size of each cell. Cells can contain emoji who's display width is greater than
/// one. This complicates things since a "😃" takes up 2 display widths.
///
/// Let's say one cell has a "😃" in it. The cell's display width is 2. The cell's byte size is 4.
/// The next cell after it will have to contain nothing or void.
///
/// Why? This is because the col & row indices of the grid map to display col & row indices of the
/// terminal screen. By inserting a [PixelChar::Void] pixel char in the next cell, we signal the
/// rendering logic to skip it since it has already been painted. And this is different than a
/// [PixelChar::Spacer] which has to be painted!
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash, GetSize)]
pub struct OffscreenBuffer {
    pub buffer: PixelCharLines,
    pub window_size: Size,
    pub my_pos: Position,
    pub my_fg_color: Option<TuiColor>,
    pub my_bg_color: Option<TuiColor>,
}

pub enum OffscreenBufferDiffResult {
    NotComparable,
    Comparable(PixelCharDiffChunks),
}

pub type PixelCharDiffChunks = List<DiffChunk>;
pub type DiffChunk = (Position, PixelChar);

mod offscreen_buffer_impl {
    use super::*;

    impl PixelCharDiffChunks {
        pub fn pretty_print(&self) -> String {
            let mut it = String::new();
            for (pos, pixel_char) in self.iter() {
                it.push_str(&format!("\t{:?}: {}\n", pos, pixel_char.pretty_print()));
            }
            it
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
            write!(
                f,
                "window_size: {:?}, \n{}\n",
                self.window_size,
                self.pretty_print()
            )
        }
    }

    impl OffscreenBuffer {
        /// Checks for differences between self and other. Returns a list of positions and pixel
        /// chars if there are differences (from other).
        pub fn diff(&self, other: &Self) -> OffscreenBufferDiffResult {
            if self.window_size != other.window_size {
                return OffscreenBufferDiffResult::NotComparable;
            }

            let mut it = List::default();
            for (row, (self_row, other_row)) in
                self.buffer.iter().zip(other.buffer.iter()).enumerate()
            {
                for (col, (self_pixel_char, other_pixel_char)) in
                    self_row.iter().zip(other_row.iter()).enumerate()
                {
                    if self_pixel_char != other_pixel_char {
                        it.push((
                            position!(col_index: col, row_index: row),
                            other_pixel_char.clone(),
                        ));
                    }
                }
            }
            OffscreenBufferDiffResult::Comparable(it)
        }

        /// Create a new buffer and fill it with empty chars.
        pub fn new_with_capacity_initialized(window_size: Size) -> Self {
            Self {
                buffer: PixelCharLines::new_with_capacity_initialized(window_size),
                window_size,
                my_pos: Default::default(),
                my_fg_color: None,
                my_bg_color: None,
            }
        }

        // Make sure each line is full of empty chars.
        pub fn clear(&mut self) {
            self.buffer = PixelCharLines::new_with_capacity_initialized(self.window_size);
        }

        pub fn pretty_print(&self) -> String {
            let mut lines = vec![];
            for row_index in 0..ch!(@to_usize self.window_size.row_count) {
                if let Some(row) = self.buffer.get(row_index) {
                    lines.push({
                        let row_index_text = format!("row_index: {row_index}");
                        let row_index_text = style_error(&row_index_text).to_string();
                        let row_text = format!("{}\n{}", row_index_text, row.pretty_print());
                        row_text
                    });
                }
            }
            lines.join("\n")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, GetSize)]
pub struct PixelCharLines {
    pub lines: Vec<PixelCharLine>,
}

mod pixel_char_lines_impl {
    use super::*;

    impl Deref for PixelCharLines {
        type Target = Vec<PixelCharLine>;
        fn deref(&self) -> &Self::Target { &self.lines }
    }

    impl DerefMut for PixelCharLines {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.lines }
    }

    impl PixelCharLines {
        pub fn new_with_capacity_initialized(window_size: Size) -> Self {
            let window_height = ch!(@to_usize window_size.row_count);
            let window_width = ch!(@to_usize window_size.col_count);
            Self {
                lines: vec![
                    PixelCharLine::new_with_capacity_initialized(window_width);
                    window_height
                ],
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, GetSize)]
pub struct PixelCharLine {
    pub pixel_chars: Vec<PixelChar>,
}

mod pixel_char_line_impl {
    use super::*;

    // This represents a single row on the screen (i.e. a line of text).
    impl PixelCharLine {
        pub fn pretty_print(&self) -> String {
            let mut it = vec![];
            let mut void_indices: Vec<usize> = vec![];
            let mut spacer_indices: Vec<usize> = vec![];
            let mut void_count: Vec<String> = vec![];
            let mut spacer_count: Vec<String> = vec![];

            // Pretty print only so many chars per line (depending on the terminal width in which
            // log.fish is run).
            const MAX_PIXEL_CHARS_PER_LINE: usize = 6;
            let mut char_count = 0;

            // Loop: for each PixelChar in a line (pixel_chars_lines[row_index]).
            for (col_index, pixel_char) in self.iter().enumerate() {
                match pixel_char {
                    PixelChar::Void => {
                        void_count.push(col_index.to_string());
                        void_indices.push(col_index);
                    }
                    PixelChar::Spacer => {
                        spacer_count.push(col_index.to_string());
                        spacer_indices.push(col_index);
                    }
                    _ => {}
                }

                let index_txt = format!("{col_index:03}");
                let pixel_char_txt = pixel_char.pretty_print();
                let index_msg = format!("{}{}", style_dim_underline(&index_txt), pixel_char_txt);
                it.push(index_msg);

                // Add \n every MAX_CHARS_PER_LINE characters.
                char_count += 1;
                if char_count >= MAX_PIXEL_CHARS_PER_LINE {
                    char_count = 0;
                    it.push("\n".to_string());
                }
            }

            // Pretty print the spacers & voids (of any of either or both).
            {
                let mut void_spacer_output = vec![];

                if !void_count.is_empty() {
                    void_spacer_output.push(format!(
                        "void [ {} ]",
                        PixelCharLine::pretty_print_index_values(&void_indices)
                    ));
                }

                if !spacer_count.is_empty() {
                    match void_spacer_output.is_empty() {
                        true => {
                            void_spacer_output.push(format!(
                                "spacer [ {} ]",
                                PixelCharLine::pretty_print_index_values(&spacer_indices)
                            ));
                        }
                        false => {
                            void_spacer_output.push(format!(
                                ", spacer [ {} ]",
                                PixelCharLine::pretty_print_index_values(&spacer_indices)
                            ));
                        }
                    }
                }

                it.push(void_spacer_output.join(" | "));
            }

            it.join("")
        }

        pub fn pretty_print_index_values(values: &[usize]) -> String {
            // Track state thru loop iteration.
            let mut current_range: Vec<usize> = vec![];
            let mut it: Vec<String> = vec![];

            mod helpers {
                pub enum Peek {
                    NextItemContinuesRange,
                    NextItemDoesNotContinueRange,
                }

                pub fn peek_does_next_item_continues_range(values: &[usize], index: usize) -> Peek {
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

                pub fn does_current_range_exist(current_range: &Vec<usize>) -> CurrentRange {
                    match current_range.is_empty() {
                        true => CurrentRange::DoesNotExist,
                        false => CurrentRange::Exists,
                    }
                }
            }

            // Main loop.
            pub use helpers::*;
            for (i, value) in values.iter().enumerate() {
                match (
                    peek_does_next_item_continues_range(values, i),
                    does_current_range_exist(&current_range),
                ) {
                    (Peek::NextItemContinuesRange, CurrentRange::DoesNotExist) => {
                        current_range.push(*value); // Start new current range.
                    }
                    (Peek::NextItemDoesNotContinueRange, CurrentRange::DoesNotExist) => {
                        it.push(format!("{value}"));
                    }
                    // The next value continues the current range.
                    (Peek::NextItemContinuesRange, CurrentRange::Exists) => {
                        current_range.push(*value);
                    }
                    // The next value does not continue the current range.
                    (Peek::NextItemDoesNotContinueRange, CurrentRange::Exists) => {
                        current_range.push(*value);
                        it.push(format!(
                            "{}-{}",
                            current_range[0],
                            current_range[current_range.len() - 1]
                        ));
                        current_range.clear();
                    }
                }
            }

            it.join(", ")
        }

        /// Create a new row with the given width and fill it with the empty chars.
        pub fn new_with_capacity_initialized(window_width: usize) -> Self {
            Self {
                pixel_chars: vec![PixelChar::Spacer; window_width],
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, GetSize)]
pub enum PixelChar {
    Void,
    Spacer,
    PlainText {
        content: GraphemeClusterSegment,
        maybe_style: Option<Style>,
    },
}

const EMPTY_CHAR: char = '╳';
const VOID_CHAR: char = '❯';

mod pixel_char_impl {
    use super::*;

    impl Default for PixelChar {
        fn default() -> Self { Self::Spacer }
    }

    impl PixelChar {
        pub fn pretty_print(&self) -> String {
            fn truncate(s: &str, max_chars: usize) -> &str {
                match s.char_indices().nth(max_chars) {
                    None => s,
                    Some((idx, _)) => &s[..idx],
                }
            }

            let width = 16;

            let it = match self {
                PixelChar::Void => {
                    format!(" V {VOID_CHAR:░^width$}")
                }
                PixelChar::Spacer => {
                    format!(" S {EMPTY_CHAR:░^width$}")
                }
                PixelChar::PlainText {
                    content: character,
                    maybe_style,
                } => {
                    let output = match maybe_style {
                        // Content + style.
                        Some(style) => format!("'{}'→{}", character.string, style.pretty_print()),
                        // Content, no style.
                        _ => format!("'{}'", character.string),
                    };
                    let trunc_output = truncate(&output, width);
                    format!(" {} {trunc_output: ^width$}", style_primary("P"))
                }
            };
            
            it
        }
    }
}

#[async_trait]
pub trait OffscreenBufferPaint {
    async fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOps;

    async fn render_diff(&mut self, diff_chunks: &PixelCharDiffChunks) -> RenderOps;

    async fn paint(
        &mut self,
        render_ops: RenderOps,
        flush_kind: FlushKind,
        shared_global_data: &SharedGlobalData,
    );

    async fn paint_diff(&mut self, render_ops: RenderOps, shared_global_data: &SharedGlobalData);
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_macro::style;

    use super::*;

    #[test]
    fn test_offscreen_buffer_construction() {
        let window_size = size! { col_count: 10, row_count: 2};
        let my_offscreen_buffer = OffscreenBuffer::new_with_capacity_initialized(window_size);
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
        let window_size = size! { col_count: 10, row_count: 2};
        let mut my_offscreen_buffer = OffscreenBuffer::new_with_capacity_initialized(window_size);
        my_offscreen_buffer.buffer[0][0] = PixelChar::PlainText {
            content: GraphemeClusterSegment::from("a"),
            maybe_style: Some(style! {color_bg: color!(@green) }),
        };
        my_offscreen_buffer.buffer[1][9] = PixelChar::PlainText {
            content: GraphemeClusterSegment::from("z"),
            maybe_style: Some(style! {color_bg: color!(@red) }),
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
