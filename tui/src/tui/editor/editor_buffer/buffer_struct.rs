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
use std::{collections::HashMap,
          fmt::{Debug, Formatter, Result}};

use history::EditorHistory;
use r3bl_core::{call_if_true,
                ch,
                format_as_kilobytes_with_commas,
                glyphs,
                height,
                i16,
                row,
                string_storage,
                width,
                with_mut,
                CaretRaw,
                CaretScrAdj,
                CharStorage,
                ColWidth,
                Dim,
                GCString,
                GCStringExt,
                RingBufferHeap,
                RowHeight,
                RowIndex,
                ScrOfs,
                SegString,
                Size,
                StringStorage};
use size_of::SizeOf as _;
use sizing::VecEditorContentLines;
use smallvec::{smallvec, SmallVec};

use super::SelectionList;
use crate::{caret_locate,
            editor_engine::engine_public_api,
            validate_buffer_mut::{EditorBufferMutNoDrop, EditorBufferMutWithDrop},
            EditorEngine,
            HasFocus,
            RenderArgs,
            RenderOps,
            DEBUG_TUI_COPY_PASTE,
            DEBUG_TUI_MOD,
            DEFAULT_SYN_HI_FILE_EXT};

/// Stores the data for a single editor buffer. Please do not construct this struct
/// directly and use [new_empty](EditorBuffer::new_empty) instead.
///
/// 1. This struct is stored in the app's state.
/// 2. And it is paired w/ [EditorEngine] at runtime; which is responsible for rendering
///    it to TUI, and handling user input.
///
/// # Modifying the buffer
///
/// [crate::InputEvent] is converted into an [crate::EditorEvent] (by
/// [engine_public_api::apply_event], which is then used to modify the [EditorBuffer] via:
/// 1. [crate::EditorEvent::apply_editor_event]
/// 2. [crate::EditorEvent::apply_editor_events]
///
/// In order for the commands to be executed, the functions in
/// [mod@crate::editor_engine::engine_internal_api] are used.
///
/// These functions take any one of the following args:
/// 1. [crate::EditorArgsMut]
/// 3. [EditorBuffer] and [EditorEngine]
///
/// # Accessing and mutating the fields (w/ validation)
///
/// All the fields in this struct are private. In order to access them you have to use the
/// accessor associated functions. To mutate them, you have to use the
/// [get_mut](EditorBuffer::get_mut) method, which returns a struct of mutable references
/// to the fields. This struct [crate::EditorBufferMut] implements the [Drop] trait, which
/// allows for validation
/// [crate::validate_buffer_mut::perform_validation_checks_after_mutation] operations to
/// be applied post mutation.
///
/// # Different kinds of caret positions
///
/// There are two variants for the caret position value:
/// 1. [CaretRaw] - this is the position of the caret (unadjusted for `scr_ofs`) and this
///    represents the position of the caret in the viewport.
/// 2. [CaretScrAdj] - this is the position of the caret (adjusted for `scr_ofs`) and
///    represents the position of the caret in the buffer (not the viewport).
///
/// # Fields
///
/// Please don't mutate these fields directly, they are not marked `pub` to guard from
/// unintentional mutation. To mutate or access access it, use
/// [get_mut](EditorBuffer::get_mut).
///
/// ## `lines`
///
/// A list of lines representing the document being edited.
///
/// ## `caret_raw`
///
/// This is the "display" col index (grapheme cluster based) and not "logical" col index
/// (byte based) position (both are defined in [r3bl_core::tui_core::graphemes]).
///
/// > Please take a look at [r3bl_core::tui_core::graphemes::GCString], specifically
/// > the methods in [mod@r3bl_core::tui_core::graphemes::gc_string] for more details on how
/// > the conversion between "display" and "logical" indices is done.
/// >
/// > This results from the fact that `UTF-8` is a variable width text encoding scheme,
/// > that can use between 1 and 4 bytes to represent a single character. So the width a
/// > human perceives and it's byte size in RAM can be different.
/// >
/// >  Videos:
/// >
/// >  - [Live coding video on Rust
/// >    String](https://youtu.be/7I11degAElQ?si=xPDIhITDro7Pa_gq)
/// >  - [UTF-8 encoding video](https://youtu.be/wIVmDPc16wA?si=D9sTt_G7_mBJFLmc)
///
/// 1. It represents the current caret position (relative to the
///    [style_adjusted_origin_pos](crate::FlexBox::style_adjusted_origin_pos) of the
///    enclosing [crate::FlexBox]).
/// 2. It works w/ [crate::RenderOp::MoveCursorPositionRelTo] as well.
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
/// The col and row offset for scrolling if active. This is not marked pub in order to
/// guard mutation. In order to access it, use [get_mut](EditorBuffer::get_mut).
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
/// This is used for syntax highlighting. It is a 2 character string, eg: `rs` or `md`
/// that is used to lookup the syntax highlighting rules for the language in
/// [find_syntax_by_extension[syntect::parsing::SyntaxSet::find_syntax_by_extension].
///
/// ## `selection_map`
///
/// The [SelectionList] is used to keep track of the selections in the buffer. Each entry
/// in the list represents a row of text in the buffer.
/// - The row index is the key [r3bl_core::RowIndex].
/// - The value is the [crate::SelectionRange].
#[derive(Clone, PartialEq, Default, size_of::SizeOf)]
pub struct EditorBuffer {
    pub content: EditorContent,
    pub history: EditorHistory,
    pub render_cache: HashMap<StringStorage, RenderOps>,
}

pub(in crate::tui::editor) mod sizing {
    use super::*;

    pub type VecEditorContentLines = SmallVec<[GCString; DEFAULT_EDITOR_LINES_SIZE]>;
    const DEFAULT_EDITOR_LINES_SIZE: usize = 32;

    /// The version history is stored on the heap, as a ring buffer.
    pub type VersionHistory = RingBufferHeap<EditorContent, MAX_UNDO_REDO_SIZE>;
    /// This is the absolute maximum number of undo/redo steps that will ever be stored.
    pub const MAX_UNDO_REDO_SIZE: usize = 16;

    impl size_of::SizeOf for EditorContent {
        fn size_of_children(&self, context: &mut size_of::Context) {
            context.add(size_of_val(&self.lines)); /* use for fields that can expand or contract */
            context.add(size_of_val(&self.maybe_file_extension)); /* use for fields that can expand or contract */
            context.add(size_of_val(&self.maybe_file_path)); /* use for fields that can expand or contract */
            context.add(self.caret_raw.size_of().total_bytes());
            context.add(self.scr_ofs.size_of().total_bytes());
            context.add(self.sel_list.size_of().total_bytes());
        }
    }

    impl size_of::SizeOf for EditorHistory {
        fn size_of_children(&self, context: &mut size_of::Context) {
            context.add(size_of_val(&self.versions)); /* use for fields that can expand or contract */
            context.add(self.current_index.size_of().total_bytes());
        }
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct EditorContent {
    pub lines: sizing::VecEditorContentLines,
    /// The caret is stored as a "raw" [EditorContent::caret_raw].
    /// - This is the col and row index that is relative to the viewport.
    /// - In order to get the "scroll adjusted" caret position, use
    ///   [EditorBuffer::get_caret_scr_adj], which incorporates the
    ///   [EditorContent::scr_ofs].
    pub caret_raw: CaretRaw,
    pub scr_ofs: ScrOfs,
    pub maybe_file_extension: Option<CharStorage>,
    pub maybe_file_path: Option<StringStorage>,
    pub sel_list: SelectionList,
}

mod constructor {
    use super::*;

    impl EditorBuffer {
        /// Marker method to make it easy to search for where an empty instance is created.
        pub fn new_empty(
            maybe_file_extension: &Option<&str>,
            maybe_file_path: &Option<&str>,
        ) -> Self {
            let it = Self {
                content: EditorContent {
                    lines: { smallvec!["".grapheme_string()] },
                    maybe_file_extension: maybe_file_extension.map(|it| it.into()),
                    maybe_file_path: maybe_file_path.map(|it| it.into()),
                    ..Default::default()
                },
                ..Default::default()
            };

            call_if_true!(DEBUG_TUI_MOD, {
                let message =
                    format!("Construct EditorBuffer {ch}", ch = glyphs::CONSTRUCT_GLYPH);
                // % is Display, ? is Debug.
                tracing::info!(
                    message = message,
                    file_extension = ?maybe_file_extension,
                    file_path = ?maybe_file_path
                );
            });

            it
        }
    }
}

pub mod cache {
    use super::*;

    pub fn clear(buffer: &mut EditorBuffer) { buffer.render_cache.clear(); }

    /// Cache key is combination of scroll_offset and window_size.
    fn generate_key(buffer: &EditorBuffer, window_size: Size) -> StringStorage {
        string_storage!(
            "{offset:?}{size:?}",
            offset = buffer.get_scr_ofs(),
            size = window_size,
        )
    }

    pub enum UseCache {
        Yes,
        No,
    }

    /// Render the content of the editor buffer to the screen from the cache if the content
    /// has not been modified.
    ///
    /// The cache miss occurs if
    /// - Scroll Offset changes
    /// - Window size changes
    /// - Content of the editor changes
    pub fn render_content(
        buffer: &mut EditorBuffer,
        engine: &mut EditorEngine,
        window_size: Size,
        has_focus: &mut HasFocus,
        render_ops: &mut RenderOps,
        use_cache: UseCache,
    ) {
        match use_cache {
            UseCache::Yes => {
                let key = generate_key(buffer, window_size);
                if let Some(cached_output) = buffer.render_cache.get(&key) {
                    // Cache hit
                    *render_ops = cached_output.clone();
                    return;
                }

                // Cache miss, due to either:
                // - Content has been modified.
                // - Scroll Offset or Window size has been modified.
                buffer.render_cache.clear();
                let render_args = RenderArgs {
                    engine,
                    buffer,
                    has_focus,
                };

                // Re-render content, generate & write to render_ops.
                engine_public_api::render_content(&render_args, render_ops);

                // Snapshot the render_ops in the cache.
                buffer.render_cache.insert(key, render_ops.clone());
            }
            UseCache::No => {
                buffer.render_cache.clear();
                let render_args = RenderArgs {
                    engine,
                    buffer,
                    has_focus,
                };
                // Re-render content, generate & write to render_ops.
                engine_public_api::render_content(&render_args, render_ops);
            }
        }
    }
}

pub mod content {
    use super::*;

    // Relating to line display width at caret row or given row index (scroll adjusted).
    impl EditorBuffer {
        pub fn get_max_row_index(&self) -> RowIndex {
            // Subtract 1 from the height to get the last row index.
            height(self.get_lines().len()).convert_to_row_index()
        }

        /// Get line display with at caret's scroll adjusted row index.
        pub fn get_line_display_width_at_caret_scr_adj(&self) -> ColWidth {
            Self::impl_get_line_display_width_at_caret_scr_adj(
                self.get_caret_raw(),
                self.get_scr_ofs(),
                self.get_lines(),
            )
        }

        /// Get line display with at caret's scroll adjusted row index. Use this when you
        /// don't have access to this struct. Eg: in [crate::EditorBufferMut].
        pub fn impl_get_line_display_width_at_caret_scr_adj(
            caret_raw: CaretRaw,
            scr_ofs: ScrOfs,
            lines: &VecEditorContentLines,
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
        pub fn get_line_display_width_at_row_index(
            &self,
            row_index: RowIndex,
        ) -> ColWidth {
            Self::impl_get_line_display_width_at_row_index(row_index, self.get_lines())
        }

        /// Get line display with at given scroll adjusted row index. Use this when you
        /// don't have access to this struct.
        pub fn impl_get_line_display_width_at_row_index(
            row_index: RowIndex,
            lines: &VecEditorContentLines,
        ) -> ColWidth {
            let maybe_line_gcs = lines.get(row_index.as_usize());
            if let Some(line_gcs) = maybe_line_gcs {
                line_gcs.display_width
            } else {
                width(0)
            }
        }
    }

    // Relating to content around the caret.
    impl EditorBuffer {
        pub fn line_at_caret_is_empty(&self) -> bool {
            self.get_line_display_width_at_caret_scr_adj() == width(0)
        }

        pub fn line_at_caret_scr_adj(&self) -> Option<&GCString> {
            if self.is_empty() {
                return None;
            }
            let row_index_scr_adj = self.get_caret_scr_adj().row_index;
            let line = self.get_lines().get(row_index_scr_adj.as_usize())?;
            Some(line)
        }

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

        pub fn string_at_caret(&self) -> Option<SegString> {
            if self.is_empty() {
                return None;
            }
            let line = self.line_at_caret_scr_adj()?;
            let caret_str_adj_col_index = self.get_caret_scr_adj().col_index;
            let seg_string = line.get_string_at(caret_str_adj_col_index)?;
            Some(seg_string)
        }

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
    use super::*;

    impl EditorBuffer {
        pub fn is_file_extension_default(&self) -> bool {
            match self.content.maybe_file_extension {
                Some(ref ext) => ext == DEFAULT_SYN_HI_FILE_EXT,
                None => false,
            }
        }

        pub fn has_file_extension(&self) -> bool {
            self.content.maybe_file_extension.is_some()
        }

        pub fn get_maybe_file_extension(&self) -> Option<&str> {
            match self.content.maybe_file_extension {
                Some(ref s) => Some(s.as_str()),
                None => None,
            }
        }

        pub fn is_empty(&self) -> bool { self.content.lines.is_empty() }

        pub fn line_at_row_index(&self, row_index: RowIndex) -> Option<&GCString> {
            self.content.lines.get(row_index.as_usize())
        }

        pub fn len(&self) -> RowHeight { height(self.content.lines.len()) }

        pub fn get_lines(&self) -> &VecEditorContentLines { &self.content.lines }

        pub fn get_as_string_with_comma_instead_of_newlines(&self) -> StringStorage {
            self.get_as_string_with_separator(", ")
        }

        pub fn get_as_string_with_newlines(&self) -> StringStorage {
            self.get_as_string_with_separator("\n")
        }

        /// Helper function to format the [EditorBuffer] as a delimited string.
        pub fn get_as_string_with_separator(&self, separator: &str) -> StringStorage {
            with_mut!(
                StringStorage::new(),
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
        // XMARK: Clever Rust, use of `IntoIterator` to efficiently and flexibly load data.
        pub fn set_lines<'a>(&mut self, arg_lines: impl IntoIterator<Item = &'a str>) {
            // Clear existing lines.
            self.content.lines.clear();

            // Populate lines with the new data.
            for line in arg_lines {
                self.content.lines.push(line.grapheme_string());
            }

            // Reset caret.
            self.content.caret_raw = CaretRaw::default();

            // Reset scroll_offset.
            self.content.scr_ofs = ScrOfs::default();

            // Empty the content render cache.
            cache::clear(self);

            // Reset undo/redo history.
            history::clear(self);
        }

        pub fn get_caret_raw(&self) -> CaretRaw { self.content.caret_raw }

        pub fn get_caret_scr_adj(&self) -> CaretScrAdj {
            self.content.caret_raw + self.content.scr_ofs
        }

        pub fn get_scr_ofs(&self) -> ScrOfs { self.content.scr_ofs }

        /// Even though this struct is mutable by editor_ops.rs, this method is provided
        /// to mark when mutable access is made to this struct.
        ///
        /// This makes it easy to determine what code mutates this struct, since it is
        /// necessary to validate things after mutation quite a bit in editor_ops.rs.
        ///
        /// [crate::EditorBufferMut] implements the [Drop] trait, which ensures that any
        /// validation changes are applied after making changes to the [EditorBuffer].
        ///
        /// Note that if `vp` is [r3bl_core::ChUnitPrimitiveType::MAX] x
        /// [r3bl_core::ChUnitPrimitiveType::MAX] that means that the viewport argument
        /// was not passed in from an [EditorEngine], since this method can be called
        /// without having an instance of that type.
        pub fn get_mut(&mut self, vp: Dim) -> EditorBufferMutWithDrop<'_> {
            EditorBufferMutWithDrop::new(
                &mut self.content.lines,
                &mut self.content.caret_raw,
                &mut self.content.scr_ofs,
                &mut self.content.sel_list,
                vp,
            )
        }

        /// This is a special case of [EditorBuffer::get_mut] where the [Drop] trait is
        /// not used to perform validation checks after mutation. This is useful when you
        /// don't want to run validation checks after mutation, which happens when the
        /// window is resized using [mod@crate::validate_scroll_on_resize].
        pub fn get_mut_no_drop(&mut self, vp: Dim) -> EditorBufferMutNoDrop<'_> {
            EditorBufferMutNoDrop::new(
                &mut self.content.lines,
                &mut self.content.caret_raw,
                &mut self.content.scr_ofs,
                &mut self.content.sel_list,
                vp,
            )
        }

        pub fn has_selection(&self) -> bool { !self.content.sel_list.is_empty() }

        pub fn clear_selection(&mut self) { self.content.sel_list.clear(); }

        pub fn get_selection_list(&self) -> &SelectionList { &self.content.sel_list }
    }
}

pub mod history {
    use super::*;

    pub const MIN_INDEX: CurIndexNumber = -1;

    /// # Undo/Redo Algorithm
    ///
    /// The `EditorBufferHistory` struct manages the undo/redo functionality for the
    /// `EditorBuffer`. It uses a ring buffer (`versions`) to store the different states of
    /// the `EditorContent`. The `current_index` field points to the current state in the
    /// `versions` buffer.
    ///
    /// ## Pushing a new state (history::push)
    ///
    /// 1. The current render cache is cleared to invalidate any cached rendering.
    /// 2. A copy of the current `EditorContent` is created.
    /// 3. If the `current_index` is not the last index in the `versions` buffer, the history
    ///    from `current_index + 1` to the end of the buffer is truncated (removed). This
    ///    discards any "future" states that were previously redone.
    /// 4. The copy of the `EditorContent` is added to the `versions` buffer.
    /// 5. The `current_index` is incremented to point to the newly added state.
    ///
    /// ## Undoing (history::undo)
    ///
    /// 1. The current render cache is cleared to invalidate any cached rendering.
    /// 2. The current caret position is retained.
    /// 3. If there is a previous state in the `versions` buffer (i.e., `current_index > 0`),
    ///    the `current_index` is decremented.
    /// 4. The `EditorContent` at the new `current_index` is retrieved from the `versions`
    ///    buffer and set as the current content of the `EditorBuffer`.
    /// 5. The caret position is restored.
    ///
    /// ## Redoing (history::redo)
    ///
    /// 1. The current render cache is cleared to invalidate any cached rendering.
    /// 2. If there is a next state in the `versions` buffer (i.e., `current_index <
    ///    versions.len() - 1`), the `current_index` is incremented.
    /// 3. The `EditorContent` at the new `current_index` is retrieved from the `versions`
    ///    buffer and set as the current content of the `EditorBuffer`.
    ///
    /// ## Notes
    ///
    /// - The `versions` buffer has a maximum size (`MAX_UNDO_REDO_SIZE`). When the buffer is
    ///   full, adding a new state will overwrite the oldest state in the buffer.
    /// - The `current_index` can be -1 if the buffer is empty.
    /// - The caret position is retained during undo operations.
    #[derive(Clone, PartialEq)]
    pub struct EditorHistory {
        pub versions: sizing::VersionHistory,
        pub current_index: CurIndex,
    }

    type CurIndexNumber = i16;

    #[derive(Clone, Copy, PartialEq, size_of::SizeOf, Debug)]
    pub struct CurIndex(pub CurIndexNumber);

    impl Default for CurIndex {
        fn default() -> Self { Self(MIN_INDEX) }
    }

    impl From<usize> for CurIndex {
        fn from(val: usize) -> Self { Self(val as CurIndexNumber) }
    }

    impl From<isize> for CurIndex {
        fn from(val: isize) -> Self { Self(val as CurIndexNumber) }
    }

    impl From<i32> for CurIndex {
        fn from(val: i32) -> Self { Self(val as CurIndexNumber) }
    }

    impl From<i16> for CurIndex {
        fn from(val: i16) -> Self { Self(val) }
    }

    impl CurIndex {
        /// This won't be negative. Even if a negative number is passed in, it will be
        /// converted to 0.
        pub fn as_usize(self) -> usize {
            if self.0 < 0 {
                0
            } else {
                self.0.try_into().unwrap_or(self.0 as usize)
            }
        }

        /// If the history buffer is empty, this will return `None`. Otherwise, it will
        /// return `Some(true)` if the current index is at the start of the history
        /// buffer, and `Some(false)` otherwise.
        pub fn is_at_start(&self, versions: &sizing::VersionHistory) -> Option<bool> {
            if versions.is_empty() {
                None
            } else {
                Some(self.0 == MIN_INDEX)
            }
        }

        /// If the history buffer is empty, this will return `None`. Otherwise, it will
        /// return `Some(true)` if the current index is at the end of the history buffer,
        /// and `Some(false)` otherwise.
        pub fn is_at_end(&self, versions: &sizing::VersionHistory) -> Option<bool> {
            if versions.is_empty() {
                None
            } else {
                let max_index = ch(versions.len()) - ch(1);
                Some(self.0 == i16(max_index))
            }
        }

        /// Reset the current index to the start of the history buffer.
        pub fn clear(&mut self) { self.0 = MIN_INDEX; }

        /// Increment the current index. If the current index is at the end of the history
        /// buffer, or the buffer is empty, this does nothing.
        pub fn inc(&mut self, versions: &sizing::VersionHistory) {
            match self.is_at_end(versions) {
                None => {
                    // Is empty. Nothing to increment.
                }
                Some(true) => {
                    // Already at end of history buffer. Nothing to increment.
                }
                Some(false) => {
                    // Increment index.
                    self.0 += 1;
                }
            }
        }

        /// Decrement the current index. If the current index is at the start of the
        /// history buffer, or the buffer is empty, this does nothing.
        pub fn dec(&mut self, versions: &sizing::VersionHistory) {
            match self.is_at_start(versions) {
                None => {
                    // Is empty. Nothing to decrement.
                }
                Some(true) => {
                    // Already at start of history buffer. Nothing to decrement.
                }
                Some(false) => {
                    // Decrement index.
                    self.0 -= 1;
                }
            }
        }
    }

    impl EditorHistory {
        pub fn is_empty(&self) -> bool { self.versions.is_empty() }

        pub fn clear(&mut self) {
            self.versions.clear();
            self.current_index.clear();
        }

        /// Get the current index in the history buffer. If the buffer is empty, this will
        /// return `None`.
        pub fn current_index(&self) -> Option<CurIndex> {
            if self.is_empty() {
                None
            } else {
                Some(self.current_index)
            }
        }

        /// This function adds a state to the history buffer. It is called whenever the
        /// content of the editor changes. Once this is called, the current index is
        /// incremented. And [EditorHistory::prev_content] can be called to undo.
        ///
        /// Any dangling redos are truncated when a new state is added to the buffer.
        pub fn add_content(&mut self, content: EditorContent) {
            if let Some(current_index) = self.current_index() {
                // Delete the history from the current version index + 1 to the end.
                self.versions.truncate(current_index.as_usize() + 1);
            }
            self.versions.add(content);
            self.current_index.inc(&self.versions);
        }

        /// This is the underlying function that enables undo. It changes the current
        /// index to the previous index in the versions buffer.
        ///
        /// Once called, you can use [EditorHistory::next_content] to redo, as long as the
        /// current index is not at the end of the versions buffer.
        pub fn prev_content(&mut self) -> Option<EditorContent> {
            match self.current_index.is_at_start(&self.versions) {
                None => {
                    // Is empty. Nothing to undo.
                    None
                }
                Some(true) => {
                    // At start of history. Nothing to undo.
                    None
                }
                Some(false) => {
                    // Decrement index.
                    self.current_index.dec(&self.versions);

                    // Return item at index.
                    self.versions.get(self.current_index.as_usize()).cloned()
                }
            }
        }

        /// This is the underlying function that enables redo. It changes the current
        /// index to the next index in the versions buffer.
        ///
        /// You can call [EditorHistory::prev_content] to undo, as long as the current
        /// index is not at the start of the versions buffer.
        pub fn next_content(&mut self) -> Option<EditorContent> {
            match self.current_index.is_at_end(&self.versions) {
                None => {
                    // Is empty. Nothing to redo.
                    None
                }
                Some(true) => {
                    // At end of history. Nothing to redo.
                    None
                }
                Some(false) => {
                    // Increment index.
                    self.current_index.inc(&self.versions);

                    // Return item at index.
                    self.versions.get(self.current_index.as_usize()).cloned()
                }
            }
        }
    }

    impl Default for EditorHistory {
        fn default() -> Self {
            Self {
                versions: sizing::VersionHistory::new(),
                current_index: CurIndex::default(),
            }
        }
    }

    pub fn clear(buffer: &mut EditorBuffer) { buffer.history.clear(); }

    pub fn add(buffer: &mut EditorBuffer) {
        // Invalidate the content cache, since the content just changed.
        cache::clear(buffer);

        // Normal history insertion.
        let content_copy = buffer.content.clone();
        buffer.history.add_content(content_copy);

        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            tracing::debug!("🍎🍎🍎 add_content_to_undo_stack buffer: {:?}", buffer);
        });
    }

    pub fn undo(buffer: &mut EditorBuffer) {
        // Invalidate the content cache, since the content just changed.
        cache::clear(buffer);

        // REVIEW: [ ] does the following behavior make sense?
        // Save the caret position & restore it after undo.
        // let retain_caret_pos = buffer.content.caret_raw;
        if let Some(content) = buffer.history.prev_content() {
            buffer.content = content;
            // buffer.content.caret_raw = retain_caret_pos;
            // _ = buffer.get_mut(dummy_viewport()); // Validate the caret position.
        }

        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            tracing::debug!("🍎🍎🍎 undo buffer: {:?}", buffer);
        });
    }

    pub fn redo(buffer: &mut EditorBuffer) {
        // Invalidate the content cache, since the content just changed.
        cache::clear(buffer);

        if let Some(content) = buffer.history.next_content() {
            buffer.content = content;
        }

        call_if_true!(DEBUG_TUI_COPY_PASTE, {
            tracing::debug!("🍎🍎🍎 redo buffer: {:?}", buffer);
        });
    }
}

mod impl_debug_format {
    use super::*;

    impl Debug for EditorBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
"EditorBuffer [
  - content: {content:?}
  - history: {history:?}
]",
                content = self.content,
                history = self.history,
            }
        }
    }

    impl Debug for EditorContent {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
                "EditorContent [
    - lines: {lines}, size: {size}
    - selection_map: {map}
    - ext: {ext:?}, path:{path:?}, caret: {caret:?}, scroll_offset: {scroll:?}
    ]",
                lines = self.lines.len(),
                size = format_as_kilobytes_with_commas(self.size_of().total_bytes()),
                ext = self.maybe_file_extension,
                caret = self.caret_raw,
                map = self.sel_list.to_formatted_string(),
                scroll = self.scr_ofs,
                path = self.maybe_file_path,
            }
        }
    }

    impl Debug for EditorHistory {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
            "EditorHistory [index: {index:?} | versions: {len} | size: {size}]",
                len = self.versions.len(),
                size = format_as_kilobytes_with_commas(self.size_of().total_bytes()),
                index = self.current_index.0
            }
        }
    }
}

#[cfg(test)]
mod history_tests {
    use r3bl_core::assert_eq2;
    use smallvec::smallvec;

    use super::*;
    use crate::history::{CurIndex, MIN_INDEX};

    #[test]
    fn test_editor_history_struct_one_item() {
        let mut history = EditorHistory::default();
        assert_eq2!(history.versions.len(), 0);
        assert_eq2!(history.current_index, CurIndex(MIN_INDEX));
        assert_eq2!(history.current_index.is_at_start(&history.versions), None);
        assert_eq2!(history.current_index.is_at_end(&history.versions), None);
        assert_eq!(history.is_empty(), true);

        history.add_content(EditorContent::default());
        assert_eq!(history.versions.len(), 1);
        assert_eq!(history.current_index, 0.into());
        assert_eq!(history.is_empty(), false);
        assert_eq!(history.current_index(), Some(0.into()));
        assert_eq!(
            history.current_index.is_at_start(&history.versions),
            Some(false)
        );
        assert_eq!(
            history.current_index.is_at_end(&history.versions),
            Some(true)
        );

        // Can't redo, since there is only one version, can only undo.
        assert_eq!(history.next_content().is_none(), true);
        assert_eq!(history.current_index, 0.into());
        assert_eq!(
            history.current_index.is_at_start(&history.versions),
            Some(false)
        );
        assert_eq!(
            history.current_index.is_at_end(&history.versions),
            Some(true)
        );

        // Can undo, since there is only one version. And current_index is 0.
        assert_eq!(history.prev_content().is_some(), true);
        assert_eq!(history.current_index, CurIndex(MIN_INDEX));
        assert_eq!(
            history.current_index.is_at_start(&history.versions),
            Some(true)
        );
        assert_eq!(
            history.current_index.is_at_end(&history.versions),
            Some(false)
        );

        // Can redo, since there is only one version. And current_index is -1.
        assert_eq!(history.next_content().is_some(), true);
        assert_eq!(history.current_index, 0.into());
        assert_eq!(
            history.current_index.is_at_start(&history.versions),
            Some(false)
        );
        assert_eq!(
            history.current_index.is_at_end(&history.versions),
            Some(true)
        );
    }

    #[test]
    fn test_editor_history_struct_multiple_items() {
        let mut history = EditorHistory::default();

        // Add 3 items to the history.
        history.add_content(EditorContent::default());
        history.add_content(EditorContent::default());
        history.add_content(EditorContent::default());

        assert_eq!(history.versions.len(), 3);
        assert_eq!(history.current_index, 2.into());
        assert_eq!(history.is_empty(), false);
        assert_eq!(history.current_index(), Some(2.into()));

        // Can undo, since there are 3 versions. And current_index is 2.
        assert_eq!(history.prev_content().is_some(), true);
        assert_eq!(history.current_index, 1.into());
        assert_eq!(history.prev_content().is_some(), true);
        assert_eq!(history.current_index, 0.into());
        assert_eq!(history.prev_content().is_some(), true);
        assert_eq!(history.current_index, CurIndex(-1));
        assert_eq!(history.prev_content().is_none(), true);

        // Can redo, 3 times.
        assert_eq!(history.next_content().is_some(), true);
        assert_eq!(history.current_index, 0.into());
        assert_eq!(history.next_content().is_some(), true);
        assert_eq!(history.current_index, 1.into());
        assert_eq!(history.next_content().is_some(), true);
        assert_eq!(history.current_index, 2.into());
        assert_eq!(history.next_content().is_none(), true);
    }

    #[test]
    fn test_editor_history_struct_truncate_dangling_redos() {
        let mut history = EditorHistory::default();

        // Add 3 items to the history.
        history.add_content(EditorContent::default());
        history.add_content(EditorContent::default());
        history.add_content(EditorContent::default());
        history.add_content(EditorContent::default());

        assert_eq!(history.versions.len(), 4);
        assert_eq!(history.current_index, 3.into());
        assert_eq!(history.is_empty(), false);
        assert_eq!(history.current_index(), Some(3.into()));

        // Undo twice. Can undo 4 times, since there are 4 versions. And current_index is
        // 3.
        assert_eq!(history.prev_content().is_some(), true);
        assert_eq!(history.prev_content().is_some(), true);
        assert_eq!(history.current_index, 1.into());
        assert_eq!(history.versions.len(), 4);

        // Add new content (+1) which should truncate the 2 dangling redos (-2).
        // So net change in versions.len() 4 - 2 + 1 = 3.
        history.add_content(EditorContent::default());
        assert_eq!(history.versions.len(), 3);
        assert_eq!(history.current_index, 2.into());
        assert_eq!(history.is_empty(), false);
        assert_eq!(history.current_index(), Some(2.into()));
    }

    #[test]
    fn test_push_default() {
        let mut buffer = EditorBuffer::default();
        let content = buffer.content.clone();

        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 1);
        assert_eq2!(history_stack.get(0).unwrap(), &content);
    }

    #[test]
    fn test_push_with_contents() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 1);
        assert_eq2!(history_stack.get(0).unwrap().lines.len(), 1);
        assert_eq2!(
            history_stack.get(0).unwrap().lines[0],
            "abc".grapheme_string()
        );
    }

    #[test]
    fn test_push_and_drop_future_redos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 1.into());

        buffer.content.lines = smallvec!["ghi".grapheme_string()];
        history::add(&mut buffer);

        // 3 pushes, so the current index should be 2.
        assert_eq2!(buffer.history.current_index, 2.into());

        // Do two undos.
        history::undo(&mut buffer);
        history::undo(&mut buffer);
        // The current index should be 0.
        assert_eq!(buffer.history.current_index, 0.into());
        // There are two versions ahead of the current index.
        assert_eq!(buffer.history.versions.len(), 3);

        // Push new content. Should drop future redos (2 versions should be removed).
        buffer.content.lines = smallvec!["xyz".grapheme_string()];
        history::add(&mut buffer);
        assert_eq!(buffer.history.current_index, 1.into());
        assert_eq!(buffer.history.versions.len(), 2);

        let history = buffer.history;
        assert_eq2!(history.current_index, 1.into());

        let history_stack = history.versions;
        assert_eq2!(history_stack.len(), 2);
        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".grapheme_string());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "xyz".grapheme_string());
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_single_undo() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        // Undo.
        history::undo(&mut buffer);
        assert_eq2!(buffer.history.current_index, CurIndex(-1));
    }

    #[test]
    fn test_many_undo() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 1.into());
        let copy_of_editor_content = buffer.content.clone();

        buffer.content.lines = smallvec!["ghi".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 2.into());

        // Undo.
        history::undo(&mut buffer);
        assert_eq2!(buffer.history.current_index, 1.into());
        assert_eq2!(buffer.content, copy_of_editor_content);

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 3);

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".grapheme_string());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "def".grapheme_string());
                }
                2 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "ghi".grapheme_string());
                }
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_multiple_undos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 1.into());

        // Undo multiple times.
        history::undo(&mut buffer);
        history::undo(&mut buffer);
        history::undo(&mut buffer);

        assert_eq2!(buffer.history.current_index, CurIndex(-1));
    }

    #[test]
    fn test_undo_and_multiple_redos() {
        let mut buffer = EditorBuffer::default();
        buffer.content.lines = smallvec!["abc".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        buffer.content.lines = smallvec!["def".grapheme_string()];
        history::add(&mut buffer);
        assert_eq2!(buffer.history.current_index, 1.into());
        let snapshot_content = buffer.content.clone();

        // Undo.
        history::undo(&mut buffer);
        assert_eq2!(buffer.history.current_index, 0.into());

        // Redo.
        history::redo(&mut buffer);
        assert_eq2!(buffer.history.current_index, 1.into());

        // Current state.
        assert_eq2!(buffer.content, snapshot_content);

        // Redo.
        history::redo(&mut buffer);

        let history_stack = buffer.history.versions;
        assert_eq2!(history_stack.len(), 2);

        for (index, content) in history_stack.iter().enumerate() {
            match index {
                0 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "abc".grapheme_string());
                }
                1 => {
                    assert_eq2!(content.lines.len(), 1);
                    assert_eq2!(content.lines[0], "def".grapheme_string());
                }
                _ => unreachable!(),
            }
        }
    }
}
