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
use std::fmt::{Debug, Display, Formatter, Result};

use smallvec::smallvec;

use super::{history::EditorHistory, render_cache::RenderCache, sizing, SelectionList};
use crate::{caret_locate, format_as_kilobytes_with_commas, glyphs, height,
            inline_string, ok, row,
            validate_buffer_mut::{EditorBufferMutNoDrop, EditorBufferMutWithDrop},
            width, with_mut, CaretRaw, CaretScrAdj, ColWidth, GCString, GCStringExt,
            InlineString, MemoizedMemorySize, RowHeight, RowIndex, ScrOfs, SegString,
            Size, TinyInlineString, DEBUG_TUI_COPY_PASTE, DEBUG_TUI_MOD,
            DEFAULT_SYN_HI_FILE_EXT};

/// Stores the data for a single editor buffer. Please do not construct this struct
/// directly and use [`new_empty`](EditorBuffer::new_empty) instead.
///
/// 1. This struct is stored in the app's state.
/// 2. And it is paired w/ [`crate::EditorEngine`] at runtime; which is responsible for
///    rendering it to TUI, and handling user input.
///
/// # Change state during render
///
/// This struct is not mutable during render phase. If you need to make changes during
/// the render phase, then you should use the [`crate::EditorEngine`] struct, which is
/// mutable during render phase.
///
/// # Modifying the buffer
///
/// [`crate::InputEvent`] is converted into an [`crate::EditorEvent`] (by
/// [`crate::engine_public_api::apply_event`], which is then used to modify the
/// [`EditorBuffer`] via:
/// 1. [`crate::EditorEvent::apply_editor_event`]
/// 2. [`crate::EditorEvent::apply_editor_events`]
///
/// In order for the commands to be executed, the functions in
/// [`mod@crate::editor_engine::engine_internal_api`] are used.
///
/// These functions take any one of the following args:
/// 1. [`crate::EditorArgsMut`]
/// 3. [`EditorBuffer`] and [`crate::EditorEngine`]
///
/// # Accessing and mutating the fields (w/ validation)
///
/// All the fields in this struct are private. In order to access them you have to use the
/// accessor associated functions. To mutate them, you have to use the
/// [`get_mut`](EditorBuffer::get_mut) method, which returns a struct of mutable
/// references to the fields. This struct [`crate::EditorBufferMut`] implements the [Drop]
/// trait, which allows for validation
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`] operations to
/// be applied post mutation.
///
/// # Kinds of caret positions
///
/// There are two variants for the caret position value:
/// 1. [`CaretRaw`] - this is the position of the caret (unadjusted for `scr_ofs`) and
///    this represents the position of the caret in the viewport.
/// 2. [`CaretScrAdj`] - this is the position of the caret (adjusted for `scr_ofs`) and
///    represents the position of the caret in the buffer (not the viewport).
///
/// # Fields
///
/// Please don't mutate these fields directly, they are not marked `pub` to guard from
/// unintentional mutation. To mutate or access it, use
/// [`get_mut`](EditorBuffer::get_mut).
///
/// ## `lines`
///
/// A list of lines representing the document being edited.
///
/// ## `caret_raw`
///
/// This is the "display" col index (grapheme-cluster-based) and not "logical" col index
/// (byte-based) position (both are defined in [`crate::graphemes`]).
///
/// > Please review [crate::graphemes::GCString], specifically the
/// > methods in [mod@crate::graphemes::gc_string] for more details on how
/// > the conversion between "display" and "logical" indices is done.
/// >
/// > This results from the fact that `UTF-8` is a variable width text encoding scheme,
/// > that can use between 1 and 4 bytes to represent a single character. So the width a
/// > human perceives, and it's byte size in RAM can be different.
/// >
/// > Videos:
/// >
/// > - [Live coding video on Rust String](https://youtu.be/7I11degAElQ?)
/// > - [UTF-8 encoding video](https://youtu.be/wIVmDPc16wA)
///
/// 1. It represents the current caret position (relative to the
///    [`style_adjusted_origin_pos`](crate::FlexBox::style_adjusted_origin_pos) of the
///    enclosing [`crate::FlexBox`]).
/// 2. It works w/ [`crate::RenderOp::MoveCursorPositionRelTo`] as well.
///
/// > 💡 For the diagrams below, the caret is where `⮬` and `❱` intersects.
///
/// Start of line:
/// ```text
/// R ┌──────────┐
/// 0 ❱abcab     │
///   └⮬─────────┘
///   C0123456789
/// ```
///
/// Middle of line:
/// ```text
/// R ┌──────────┐
/// 0 ❱abcab     │
///   └───⮬──────┘
///   C0123456789
/// ```
///
/// End of line:
/// ```text
/// R ┌──────────┐
/// 0 ❱abcab     │
///   └─────⮬────┘
///   C0123456789
/// ```
///
/// ## `scr_ofs`
///
/// The col and row offset for scrolling if active. This is not marked pub to guard
/// against unintentional mutation. To access it, use [`get_mut`](EditorBuffer::get_mut).
///
/// # Vertical scrolling and viewport
///
/// ```text
///                    ╭0────────────────────╮
///                    0                     │
///                    │        above        │ <- caret_row_adj
///                    │                     │
///                    ├─── scroll_offset ───┤
///              ->    │         ↑           │      ↑
///              │     │                     │      │
///   caret.row_index  │      within vp      │  vp height
///              │     │                     │      │
///              ->    │         ↓           │      ↓
///                    ├─── scroll_offset ───┤
///                    │    + vp height      │
///                    │                     │
///                    │        below        │ <- caret_row_adj
///                    │                     │
///                    ╰─────────────────────╯
/// ```
///
/// # Horizontal scrolling and viewport
///
/// ```text
///           <-   vp width   ->
/// ╭0────────┼────────────────┼─────────>
/// 0         │                │
/// │ left of │<-  within vp ->│ right of
/// │         │                │
/// ╰─────────┼────────────────┼─────────>
///       scroll_offset    scroll_offset
///                        + vp width
/// ```
///
/// ## `file_extension`
///
/// This is used for syntax highlighting. It is a 2-character string, eg: `rs` or `md`
/// that is used to look up the syntax highlighting rules for the language in
/// [`find_syntax_by_extension`[`syntect::parsing::SyntaxSet::find_syntax_by_extension`].
///
/// ## `selection_map`
///
/// The [`SelectionList`] is used to keep track of the selections in the buffer. Each
/// entry in the list represents a row of text in the buffer.
/// - The row index is the key [`crate::RowIndex`].
/// - The value is the [`crate::SelectionRange`].
#[derive(Clone, PartialEq, Default)]
pub struct EditorBuffer {
    pub content: EditorContent,
    pub history: EditorHistory,
    pub render_cache: RenderCache,
    /// Memoized memory size calculation for [`std::fmt::Display`] trait performance.
    memory_size_calc_cache: MemoizedMemorySize,
}

#[derive(Clone, PartialEq, Default)]
pub struct EditorContent {
    pub lines: sizing::VecEditorContentLines,
    /// The caret is stored as a "raw" [`EditorContent::caret_raw`].
    /// - This is the col and row index that is relative to the viewport.
    /// - In order to get the "scroll adjusted" caret position, use
    ///   [`EditorBuffer::get_caret_scr_adj`], which incorporates the
    ///   [`EditorContent::scr_ofs`].
    pub caret_raw: CaretRaw,
    pub scr_ofs: ScrOfs,
    pub maybe_file_extension: Option<TinyInlineString>,
    pub maybe_file_path: Option<InlineString>,
    pub sel_list: SelectionList,
}

mod construct {
    use super::{glyphs, inline_string, smallvec, EditorBuffer, EditorContent,
                GCStringExt, DEBUG_TUI_MOD};

    impl EditorBuffer {
        /// Marker method to make it easy to search for where an empty instance is
        /// created.
        #[must_use]
        pub fn new_empty(
            maybe_file_extension: Option<&str>,
            maybe_file_path: Option<&str>,
        ) -> Self {
            let it = Self {
                content: EditorContent {
                    lines: { smallvec!["".grapheme_string()] },
                    maybe_file_extension: maybe_file_extension.map(Into::into),
                    maybe_file_path: maybe_file_path.map(Into::into),
                    ..Default::default()
                },
                ..Default::default()
            };

            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::info!(
                    message = %inline_string!("Construct EditorBuffer {ch}", ch = glyphs::CONSTRUCT_GLYPH),
                    file_extension = ?maybe_file_extension,
                    file_path = ?maybe_file_path
                );
            });

            it
        }
    }
}

pub mod versions {
    use super::{EditorBuffer, DEBUG_TUI_COPY_PASTE};

    impl EditorBuffer {
        pub fn add(&mut self) {
            // Invalidate the content cache, since the content just changed.
            self.render_cache.clear();

            // Invalidate memory size cache.
            self.invalidate_memory_size_calc_cache();

            // Normal history insertion.
            let content_copy = self.content.clone();
            self.history.add(content_copy);

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "🍎🍎🍎 add_content_to_undo_stack buffer",
                    buffer = ?self
                );
            });
        }

        pub fn undo(&mut self) {
            // Invalidate the content cache, since the content just changed.
            self.render_cache.clear();

            // Invalidate memory size cache.
            self.invalidate_memory_size_calc_cache();

            if let Some(content) = self.history.undo() {
                self.content = content;
            }

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "🍎🍎🍎 undo buffer",
                    buffer = ?self
                );
            });
        }

        pub fn redo(&mut self) {
            // Invalidate the content cache, since the content just changed.
            self.render_cache.clear();

            // Invalidate memory size cache.
            self.invalidate_memory_size_calc_cache();

            if let Some(content) = self.history.redo() {
                self.content = content;
            }

            DEBUG_TUI_COPY_PASTE.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(message = "🍎🍎🍎 redo buffer",
                    buffer = ?self
                );
            });
        }
    }
}

/// Relating to line display width at caret row or given row index (scroll adjusted).
pub mod content_display_width {
    use super::{height, sizing, width, CaretRaw, ColWidth, EditorBuffer, RowIndex,
                ScrOfs};

    impl EditorBuffer {
        #[must_use]
        pub fn get_max_row_index(&self) -> RowIndex {
            // Subtract 1 from the height to get the last row index.
            height(self.get_lines().len()).convert_to_row_index()
        }

        /// Get line display with at caret's scroll adjusted row index.
        #[must_use]
        pub fn get_line_display_width_at_caret_scr_adj(&self) -> ColWidth {
            Self::impl_get_line_display_width_at_caret_scr_adj(
                self.get_caret_raw(),
                self.get_scr_ofs(),
                self.get_lines(),
            )
        }

        /// Get line display with at caret's scroll adjusted row index. Use this when you
        /// don't have access to this struct. Eg: in [`crate::EditorBufferMut`].
        #[must_use]
        pub fn impl_get_line_display_width_at_caret_scr_adj(
            caret_raw: CaretRaw,
            scr_ofs: ScrOfs,
            lines: &sizing::VecEditorContentLines,
        ) -> ColWidth {
            let caret_scr_adj = caret_raw + scr_ofs;
            let row_index = caret_scr_adj.row_index;
            let maybe_line_gcs = lines.get(row_index.as_usize());
            if let Some(line_gcs) = maybe_line_gcs {
                line_gcs.display_width
            } else {
                width(0)
            }
        }

        /// Get line display with at given scroll adjusted row index.
        #[must_use]
        pub fn get_line_display_width_at_row_index(
            &self,
            row_index: RowIndex,
        ) -> ColWidth {
            Self::impl_get_line_display_width_at_row_index(row_index, self.get_lines())
        }

        /// Get line display with at given scroll adjusted row index. Use this when you
        /// don't have access to this struct.
        #[must_use]
        pub fn impl_get_line_display_width_at_row_index(
            row_index: RowIndex,
            lines: &sizing::VecEditorContentLines,
        ) -> ColWidth {
            let maybe_line_gcs = lines.get(row_index.as_usize());
            if let Some(line_gcs) = maybe_line_gcs {
                line_gcs.display_width
            } else {
                width(0)
            }
        }
    }
}

/// Relating to content around the caret.
pub mod content_near_caret {
    use super::{caret_locate, row, width, EditorBuffer, GCString, SegString};

    impl EditorBuffer {
        #[must_use]
        pub fn line_at_caret_is_empty(&self) -> bool {
            self.get_line_display_width_at_caret_scr_adj() == width(0)
        }

        #[must_use]
        pub fn line_at_caret_scr_adj(&self) -> Option<&GCString> {
            if self.is_empty() {
                return None;
            }
            let row_index_scr_adj = self.get_caret_scr_adj().row_index;
            let line = self.get_lines().get(row_index_scr_adj.as_usize())?;
            Some(line)
        }

        #[must_use]
        pub fn string_at_end_of_line_at_caret_scr_adj(&self) -> Option<SegString> {
            if self.is_empty() {
                return None;
            }
            let line = self.line_at_caret_scr_adj()?;
            if let caret_locate::CaretColLocationInLine::AtEnd =
                caret_locate::locate_col(self)
            {
                let maybe_last_seg_string = line.get_string_at_end();
                return maybe_last_seg_string;
            }
            None
        }

        #[must_use]
        pub fn string_to_right_of_caret(&self) -> Option<SegString> {
            if self.is_empty() {
                return None;
            }
            let line = self.line_at_caret_scr_adj()?;
            match caret_locate::locate_col(self) {
                // Caret is at end of line, past the last character.
                caret_locate::CaretColLocationInLine::AtEnd => line.get_string_at_end(),
                // Caret is not at end of line.
                _ => line.get_string_at_right_of(self.get_caret_scr_adj().col_index),
            }
        }

        #[must_use]
        pub fn string_to_left_of_caret(&self) -> Option<SegString> {
            if self.is_empty() {
                return None;
            }
            let line = self.line_at_caret_scr_adj()?;
            match caret_locate::locate_col(self) {
                // Caret is at end of line, past the last character.
                caret_locate::CaretColLocationInLine::AtEnd => line.get_string_at_end(),
                // Caret is not at end of line.
                _ => line.get_string_at_left_of(self.get_caret_scr_adj().col_index),
            }
        }

        #[must_use]
        pub fn prev_line_above_caret(&self) -> Option<&GCString> {
            if self.is_empty() {
                return None;
            }
            let row_index_scr_adj = self.get_caret_scr_adj().row_index;
            if row_index_scr_adj == row(0) {
                return None;
            }
            let line = self
                .get_lines()
                .get((row_index_scr_adj - row(1)).as_usize())?;
            Some(line)
        }

        #[must_use]
        pub fn string_at_caret(&self) -> Option<SegString> {
            if self.is_empty() {
                return None;
            }
            let line = self.line_at_caret_scr_adj()?;
            let caret_str_adj_col_index = self.get_caret_scr_adj().col_index;
            let seg_string = line.get_string_at(caret_str_adj_col_index)?;
            Some(seg_string)
        }

        #[must_use]
        pub fn next_line_below_caret_to_string(&self) -> Option<&GCString> {
            if self.is_empty() {
                return None;
            }
            let caret_scr_adj_row_index = self.get_caret_scr_adj().row_index;
            let next_line_row_index = caret_scr_adj_row_index + row(1);
            let line = self.get_lines().get(next_line_row_index.as_usize())?;
            Some(line)
        }
    }
}

pub mod access_and_mutate {
    use super::{height, sizing, with_mut, CaretRaw, CaretScrAdj, EditorBuffer,
                EditorBufferMutNoDrop, EditorBufferMutWithDrop, GCString, GCStringExt,
                InlineString, RowHeight, RowIndex, ScrOfs, SelectionList, Size,
                DEFAULT_SYN_HI_FILE_EXT};

    impl EditorBuffer {
        #[must_use]
        pub fn is_file_extension_default(&self) -> bool {
            match self.content.maybe_file_extension {
                Some(ref ext) => ext == DEFAULT_SYN_HI_FILE_EXT,
                None => false,
            }
        }

        #[must_use]
        pub fn has_file_extension(&self) -> bool {
            self.content.maybe_file_extension.is_some()
        }

        #[must_use]
        pub fn get_maybe_file_extension(&self) -> Option<&str> {
            match self.content.maybe_file_extension {
                Some(ref s) => Some(s.as_str()),
                None => None,
            }
        }

        #[must_use]
        pub fn is_empty(&self) -> bool { self.content.lines.is_empty() }

        #[must_use]
        pub fn line_at_row_index(&self, row_index: RowIndex) -> Option<&GCString> {
            self.content.lines.get(row_index.as_usize())
        }

        #[must_use]
        pub fn len(&self) -> RowHeight { height(self.content.lines.len()) }

        #[must_use]
        pub fn get_lines(&self) -> &sizing::VecEditorContentLines { &self.content.lines }

        #[must_use]
        pub fn get_as_string_with_comma_instead_of_newlines(&self) -> InlineString {
            self.get_as_string_with_separator(", ")
        }

        #[must_use]
        pub fn get_as_string_with_newlines(&self) -> InlineString {
            self.get_as_string_with_separator("\n")
        }

        /// Helper function to format the [`EditorBuffer`] as a delimited string.
        #[must_use]
        pub fn get_as_string_with_separator(&self, separator: &str) -> InlineString {
            with_mut!(
                InlineString::new(),
                as acc,
                run {
                    let lines = &self.content.lines;
                    for (index, line) in lines.iter().enumerate() {
                        // Add separator if it's not the first line.
                        if index > 0 {
                            acc.push_str(separator);
                        }
                        // Append the current line to the accumulator.
                        acc.push_str(&line.string);
                    }
                }
            )
        }

        // XMARK: Clever Rust, use `IntoIterator` to efficiently & flexibly load data.

        /// You can load a file into the editor buffer using this method. Since this is a
        /// text editor and not binary editor, it operates on UTF-8 encoded text files and
        /// not binary files (which just contain `u8`s).
        ///
        /// You can convert a `&[u8]` to a `&str` using `std::str::from_utf8`.
        /// - A `Vec<u8>` can be converted into a `&[u8]` using `&vec[..]` or
        ///   `vec.as_slice()` or `vec.as_bytes()`.
        /// - Then you can convert the `&[u8]` to a `&str` using `std::str::from_utf8`.
        /// - And then call `.lines()` on the `&str` to get an iterator over the lines
        ///   which can be passed to this method.
        pub fn set_lines<I>(&mut self, arg_lines: I)
        where
            I: IntoIterator,
            I::Item: AsRef<str>,
        {
            // Clear existing lines.
            self.content.lines.clear();

            // Populate lines with the new data.
            for line in arg_lines {
                self.content.lines.push(line.as_ref().grapheme_string());
            }

            // Reset caret.
            self.content.caret_raw = CaretRaw::default();

            // Reset scroll_offset.
            self.content.scr_ofs = ScrOfs::default();

            // Empty the content render cache.
            self.render_cache.clear();

            // Invalidate memory size cache.
            self.invalidate_memory_size_calc_cache();

            // Reset undo/redo history.
            self.history.clear();
        }

        #[must_use]
        pub fn get_caret_raw(&self) -> CaretRaw { self.content.caret_raw }

        #[must_use]
        pub fn get_caret_scr_adj(&self) -> CaretScrAdj {
            self.content.caret_raw + self.content.scr_ofs
        }

        #[must_use]
        pub fn get_scr_ofs(&self) -> ScrOfs { self.content.scr_ofs }

        /// Even though this struct is mutable by `editor_ops.rs`, this method is provided
        /// to mark when mutable access is made to this struct.
        ///
        /// This makes it easy to determine what code mutates this struct, since it is
        /// necessary to validate things after mutation quite a bit in `editor_ops.rs`.
        ///
        /// [`crate::EditorBufferMut`] implements the [Drop] trait, which ensures that any
        /// validation changes are applied after making changes to the [`EditorBuffer`].
        ///
        /// Note that if `vp` is [`crate::dummy_viewport()`] that means that the viewport
        /// argument was not passed in from a [`crate::EditorEngine`], since this method
        /// can be called without having an instance of that type.
        pub fn get_mut(&mut self, vp: Size) -> EditorBufferMutWithDrop<'_> {
            EditorBufferMutWithDrop::new(
                &mut self.content.lines,
                &mut self.content.caret_raw,
                &mut self.content.scr_ofs,
                &mut self.content.sel_list,
                vp,
                &mut self.memory_size_calc_cache,
            )
        }

        /// This is a special case of [`EditorBuffer::get_mut`] where the [Drop] trait is
        /// not used to perform validation checks after mutation. This is useful when you
        /// don't want to run validation checks after mutation, which happens when the
        /// window is resized using [`mod@crate::validate_scroll_on_resize`].
        pub fn get_mut_no_drop(&mut self, vp: Size) -> EditorBufferMutNoDrop<'_> {
            EditorBufferMutNoDrop::new(
                &mut self.content.lines,
                &mut self.content.caret_raw,
                &mut self.content.scr_ofs,
                &mut self.content.sel_list,
                vp,
                &mut self.memory_size_calc_cache,
            )
        }

        #[must_use]
        pub fn has_selection(&self) -> bool { !self.content.sel_list.is_empty() }

        /// Clears the text selection that the user has made in the editor.
        ///
        /// Large selections can occupy a significant amount of memory, so this method
        /// also invalidates the memory size cache to ensure accurate telemetry reporting.
        pub fn clear_selection(&mut self) {
            self.content.sel_list.clear();
            self.invalidate_memory_size_calc_cache();
        }

        #[must_use]
        pub fn get_selection_list(&self) -> &SelectionList { &self.content.sel_list }
    }
}

/// Memory size caching for performance optimization.
mod memory_size_calc_cache {
    use super::EditorBuffer;

    impl EditorBuffer {
        /// Marks the memory size cache as invalid, requiring recalculation on next
        /// access. Call this when buffer content changes.
        pub fn invalidate_memory_size_calc_cache(&mut self) {
            self.memory_size_calc_cache.invalidate();
        }

        /// Updates cache if dirty or not present.
        /// The closure is only called if recalculation is needed.
        pub fn upsert_memory_size_calc_cache(&mut self) {
            use crate::{GetMemSize, MemorySize};
            self.memory_size_calc_cache.upsert(|| {
                let size = self.content.get_mem_size() + self.history.get_mem_size();
                MemorySize::new(size)
            });
        }

        /// Gets the cached memory size value if available and not dirty.
        #[must_use]
        pub fn get_memory_size_calc_cached(&self) -> Option<usize> {
            use crate::MemorySize;
            self.memory_size_calc_cache
                .get_cached()
                .and_then(MemorySize::size)
        }
    }
}

/// Efficient Display implementation for telemetry logging.
mod display_impl {
    use super::{format_as_kilobytes_with_commas, ok, Display, EditorBuffer, Formatter,
                Result};

    impl Display for EditorBuffer {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffer. This is used for telemetry reporting, and it is expected
        /// to be fast, since it is called in a hot loop, on every render.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Get memory size from cache if available, otherwise show "?".
            let memory_str = if let Some(size) = self.get_memory_size_calc_cached() {
                format_as_kilobytes_with_commas(size)
            } else {
                "?".into()
            };

            // Format basic info.
            let line_count = self.content.lines.len();
            let has_selection = self.has_selection();

            // Get active line/column info.
            let caret = self.get_caret_scr_adj();
            let line = caret.row_index.as_usize() + 1; // 1-indexed for display.
            let col = caret.col_index.as_usize() + 1; // 1-indexed for display.

            // Get file info and format output.
            let ext = self
                .content
                .maybe_file_extension
                .as_ref()
                .map_or("txt", |e| e.as_str());

            // Format editor identifier: extract filename from path for named buffers,
            // or use placeholder for new/unnamed buffers.
            match self.content.maybe_file_path.as_ref() {
                Some(path) => {
                    let file_name = path.rsplit('/').next().unwrap_or("<unnamed>");
                    write!(f, "editor:{file_name}.{ext}:L{line}:C{col}")?;
                }
                None => {
                    write!(f, "editor:<new-buffer>.{ext}:L{line}:C{col}")?;
                }
            }

            // Add selection info if present.
            if has_selection {
                let sel_count = self.content.sel_list.len();
                write!(f, ":sel({sel_count}L)")?;
            }

            // Add summary info.
            write!(f, "[lines={line_count}, size={memory_str}]")?;

            ok!()
        }
    }
}

mod debug_impl {
    use super::{format_as_kilobytes_with_commas, Debug, EditorBuffer, EditorContent,
                Formatter, Result};

    impl Debug for EditorBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(
                f,
                "EditorBuffer [
  - content: {content:?}
  - history: {history:?}
]",
                content = self.content,
                history = self.history,
            )
        }
    }

    impl Debug for EditorContent {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            use crate::GetMemSize;
            let mem_size = self.get_mem_size();
            let mem_size_fmt = format_as_kilobytes_with_commas(mem_size);

            write! {
                f,
                "EditorContent [
    - lines: {lines}, size: {size}
    - selection_map: {map}
    - ext: {ext:?}, path:{path:?}, caret: {caret:?}, scroll_offset: {scroll:?}
    ]",
                lines = self.lines.len(),
                size = mem_size_fmt,
                ext = self.maybe_file_extension,
                caret = self.caret_raw,
                map = self.sel_list.to_formatted_string(),
                scroll = self.scr_ofs,
                path = self.maybe_file_path,
            }
        }
    }
}

#[cfg(test)]
mod test_memory_cache_invalidation {
    use super::*;
    use crate::EditorEngine;

    #[test]
    fn test_cache_invalidated_on_get_mut() {
        let mut buffer = EditorBuffer::new_empty(Some("md"), None);
        let engine = EditorEngine::default();

        // Set initial content and cache the memory size.
        buffer.set_lines(["Hello", "World"]);
        buffer.upsert_memory_size_calc_cache();
        let initial_size = buffer
            .get_memory_size_calc_cached()
            .expect("Cache should have value");
        assert!(initial_size > 0);

        // Modify content through get_mut.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut
                .inner
                .lines
                .push("More content with lots of text".grapheme_string());
        }
        // When buffer_mut goes out of scope, Drop should invalidate the cache.

        // Verify cache was invalidated and new size is calculated.
        buffer.upsert_memory_size_calc_cache();
        let new_size = buffer
            .get_memory_size_calc_cached()
            .expect("Cache should have value");
        assert!(
            new_size > initial_size,
            "Memory size should increase after adding content"
        );

        // Test that cache is not invalidated with get_mut_no_drop.
        let cached_size = new_size;
        {
            let buffer_mut_no_drop = buffer.get_mut_no_drop(engine.viewport());
            buffer_mut_no_drop
                .inner
                .lines
                .push("Even more content".grapheme_string());
        }
        // Cache should still have old value since we used no_drop variant.
        assert_eq!(buffer.get_memory_size_calc_cached(), Some(cached_size));

        // Force recalculation to verify content actually changed.
        buffer.invalidate_memory_size_calc_cache();
        buffer.upsert_memory_size_calc_cache();
        let final_size = buffer
            .get_memory_size_calc_cached()
            .expect("Cache should have value");
        assert!(
            final_size > new_size,
            "Memory size should increase after adding more content"
        );
    }
}
