// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use std::fmt::{Debug, Display, Formatter, Result};

use super::{SelectionList, history::EditorHistory, render_cache::RenderCache};
use crate::{CaretRaw, CaretScrAdj, ColWidth, CursorPositionBoundsStatus,
            DEBUG_TUI_COPY_PASTE, DEBUG_TUI_MOD, DEFAULT_SYN_HI_FILE_EXT,
            EditorBufferMutWithDrop, GapBufferLine, GetMemSize, InlineString,
            MemoizedMemorySize, MemorySize, RowHeight, RowIndex, ScrOfs, SegStringOwned,
            Size, TinyInlineString, UnitCompare, ZeroCopyGapBuffer,
            caret_locate::locate_col, format_as_kilobytes_with_commas, glyphs, height,
            inline_string, ok, row, validate_buffer_mut::EditorBufferMutNoDrop, width,
            with_mut};

/// Stores the data for a single editor buffer using [`ZeroCopyGapBuffer`] for efficient
/// text storage.
///
/// Please do not construct this struct directly and use
/// [`new_empty`](EditorBuffer::new_empty) instead.
///
/// As of 2025, `EditorBuffer` uses [`ZeroCopyGapBuffer`] directly as a concrete type
/// for efficient content storage with zero-copy access.
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
/// > Please review [`crate::graphemes::GCStringOwned`], specifically the
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
    pub memory_size_calc_cache: MemoizedMemorySize,
}

/// Contains the core text content and editing state using `ZeroCopyGapBuffer` for
/// storage.
#[derive(Clone, PartialEq, Default)]
pub struct EditorContent {
    pub lines: ZeroCopyGapBuffer,
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl EditorBuffer {
        /// Marker method to make it easy to search for where an empty instance is
        /// created.
        #[must_use]
        pub fn new_empty(
            maybe_file_extension: Option<&str>,
            maybe_file_path: Option<&str>,
        ) -> Self {
            let mut lines = ZeroCopyGapBuffer::default();
            lines.push_line("");

            let it = Self {
                content: EditorContent {
                    lines,
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use crate::core::units::bounds_check::LengthMarker;

    impl EditorBuffer {
        #[must_use]
        pub fn get_max_row_index(&self) -> RowIndex {
            // Subtract 1 from the height to get the last row index.
            height(self.get_lines().len().as_usize()).convert_to_index()
        }

        /// Get line display with at caret's scroll adjusted row index.
        #[must_use]
        pub fn get_line_display_width_at_caret_scr_adj(&self) -> ColWidth {
            let caret_scr_adj = self.get_caret_raw() + self.get_scr_ofs();
            let row_index = caret_scr_adj.row_index;

            // Use the concrete method directly for display width.
            if let Some(display_width) =
                self.get_lines().get_line_display_width(row_index)
            {
                display_width
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
            // Use the concrete method directly for display width.
            if let Some(display_width) =
                self.get_lines().get_line_display_width(row_index)
            {
                display_width
            } else {
                width(0)
            }
        }
    }
}

/// Relating to content around the caret.
pub mod content_near_caret {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl EditorBuffer {
        #[must_use]
        pub fn line_at_caret_is_empty(&self) -> bool {
            self.get_line_display_width_at_caret_scr_adj().is_zero()
        }

        #[must_use]
        pub fn line_at_caret_scr_adj(&self) -> Option<GapBufferLine<'_>> {
            if self.is_empty() {
                return None;
            }
            let row_index_scr_adj = self.get_caret_scr_adj().row_index;

            // Return the native GapBufferLine - let callers adapt if needed.
            self.content.lines.get_line(row_index_scr_adj)
        }

        #[must_use]
        pub fn string_at_end_of_line_at_caret_scr_adj(&self) -> Option<SegStringOwned> {
            if self.is_empty() {
                return None;
            }

            let row_index_scr_adj = self.get_caret_scr_adj().row_index;

            if let CursorPositionBoundsStatus::AtEnd = locate_col(self) {
                // Use the efficient GapBufferLine approach directly.
                if let Some(line_with_info) =
                    self.content.lines.get_line(row_index_scr_adj)
                {
                    return line_with_info
                        .info()
                        .get_string_at_end(line_with_info.content());
                }
            }
            None
        }

        #[must_use]
        pub fn string_to_right_of_caret(&self) -> Option<SegStringOwned> {
            if self.is_empty() {
                return None;
            }

            let row_index_scr_adj = self.get_caret_scr_adj().row_index;
            let col_index_scr_adj = self.get_caret_scr_adj().col_index;

            if let Some(line_with_info) = self.content.lines.get_line(row_index_scr_adj) {
                match locate_col(self) {
                    // Caret is at end of line, past the last character.
                    CursorPositionBoundsStatus::AtEnd => line_with_info
                        .info()
                        .get_string_at_end(line_with_info.content()),
                    // Caret is not at end of line.
                    _ => line_with_info.info().get_string_at_right_of(
                        line_with_info.content(),
                        col_index_scr_adj,
                    ),
                }
            } else {
                None
            }
        }

        #[must_use]
        pub fn string_to_left_of_caret(&self) -> Option<SegStringOwned> {
            if self.is_empty() {
                return None;
            }

            let row_index_scr_adj = self.get_caret_scr_adj().row_index;
            let col_index_scr_adj = self.get_caret_scr_adj().col_index;

            if let Some(line_with_info) = self.content.lines.get_line(row_index_scr_adj) {
                match locate_col(self) {
                    // Caret is at end of line, past the last character.
                    CursorPositionBoundsStatus::AtEnd => line_with_info
                        .info()
                        .get_string_at_end(line_with_info.content()),
                    // Caret is not at end of line.
                    _ => line_with_info.info().get_string_at_left_of(
                        line_with_info.content(),
                        col_index_scr_adj,
                    ),
                }
            } else {
                None
            }
        }

        #[must_use]
        pub fn prev_line_above_caret(&self) -> Option<&str> {
            if self.is_empty() {
                return None;
            }
            let row_index_scr_adj = self.get_caret_scr_adj().row_index;
            if row_index_scr_adj == row(0) {
                return None;
            }
            let prev_row_index = row_index_scr_adj - row(1);
            // Use the concrete method that delegates to get_line.
            self.get_lines().get_line_content(prev_row_index)
        }

        #[must_use]
        pub fn string_at_caret(&self) -> Option<SegStringOwned> {
            if self.is_empty() {
                return None;
            }

            let row_index_scr_adj = self.get_caret_scr_adj().row_index;
            let col_index_scr_adj = self.get_caret_scr_adj().col_index;

            if let Some(line_with_info) = self.content.lines.get_line(row_index_scr_adj) {
                line_with_info
                    .info()
                    .get_string_at(line_with_info.content(), col_index_scr_adj)
            } else {
                None
            }
        }

        #[must_use]
        pub fn next_line_below_caret_to_string(&self) -> Option<&str> {
            if self.is_empty() {
                return None;
            }
            let caret_scr_adj_row_index = self.get_caret_scr_adj().row_index;
            let next_line_row_index = caret_scr_adj_row_index + 1;
            // Use the concrete method that delegates to get_line.
            self.get_lines().get_line_content(next_line_row_index)
        }
    }
}

pub mod access_and_mutate {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
        pub fn line_at_row_index(&self, row_index: RowIndex) -> Option<&str> {
            // Use the concrete method that delegates to get_line.
            self.content.lines.get_line_content(row_index)
        }

        #[must_use]
        pub fn len(&self) -> RowHeight { height(self.content.lines.len().as_usize()) }

        #[must_use]
        pub fn get_lines(&self) -> &ZeroCopyGapBuffer { &self.content.lines }

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
                    for (index, line_with_info) in self.content.lines.iter_lines().enumerate() {
                        // Add separator if it's not the first line.
                        if index > 0 {
                            acc.push_str(separator);
                        }
                        // Append the current line to the accumulator.
                        acc.push_str(line_with_info.content());
                    }
                }
            )
        }

        // XMARK: Clever Rust, use `IntoIterator` to efficiently & flexibly load data.

        /// You can load a file into the editor buffer using this method. Since this is a
        /// text editor and not binary editor, it operates on UTF-8 encoded text files and
        /// not binary files (which just contain `u8`s).
        ///
        /// You can convert a `&[u8]` to a `&str` using [`std::str::from_utf8`].
        /// Initializes the buffer with the given lines, clearing all state including
        /// history. This is meant to be used when loading a new file or
        /// completely replacing buffer content.
        ///
        /// For normal editing operations that preserve history, use [`Self::get_mut()`]
        /// and the mutation API [`mod@crate::content_mut`].
        ///
        /// - A [`Vec<u8>`] can be converted into a `&[u8]` using `&vec[..]` or
        ///   `vec.as_slice()` or `vec.as_bytes()`.
        /// - Then you can convert the `&[u8]` to a `&str` using [`std::str::from_utf8`].
        /// - And then call [`str::lines()`] on the `&str` to get an iterator over the
        ///   lines which can be passed to this method.
        pub fn init_with<I>(&mut self, arg_lines: I)
        where
            I: IntoIterator,
            I::Item: AsRef<str>,
        {
            // Clear existing lines.
            self.content.lines.clear();

            // Populate lines with the new data.
            for line in arg_lines {
                self.content.lines.push_line(line.as_ref());
            }

            // Reset caret.
            self.content.caret_raw = CaretRaw::default();

            // Reset scroll_offset.
            self.content.scr_ofs = ScrOfs::default();

            // Empty the content render cache.
            self.render_cache.clear();

            // Invalidate and recalculate memory size cache.
            self.invalidate_memory_size_calc_cache();

            // Reset undo/redo history since this is a complete re-initialization
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

        /// Even though this struct is mutable by `editor_ops_insert`, this method is
        /// provided to mark when mutable access is made to this struct.
        ///
        /// This makes it easy to determine what code mutates this struct, since it is
        /// necessary to validate things after mutation quite a bit in
        /// `editor_ops_insert`.
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

/// Efficient Display implementation for telemetry logging.
mod display_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Display for EditorBuffer {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffer. This is used for telemetry reporting, and it is expected
        /// to be fast, since it is called in a hot loop, on every render.
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Note: Display requires &self not &mut self, so we access the cache
            // directly. The cache is populated elsewhere in the buffer's lifecycle
            // via invalidate_memory_size_calc_cache(). Use MemorySize's Display impl
            // which handles the "?" case automatically.
            let memory_size = self
                .memory_size_calc_cache
                .get_cached()
                .cloned()
                .unwrap_or_else(MemorySize::unknown);

            // Format basic info.
            let line_count = self.content.lines.len().as_usize();
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
            write!(f, "[lines={line_count}, size={memory_size}]")?;

            ok!()
        }
    }
}

mod debug_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
            let mem_size = self.get_mem_size();
            let mem_size_fmt = format_as_kilobytes_with_commas(mem_size);

            write! {
                f,
                "EditorContent [
    - lines: {lines:?}, size: {size}
    - selection_map: {map}
    - ext: {ext:?}, path:{path:?}, caret: {caret:?}, scroll_offset: {scroll:?}
    ]",
                lines = self.lines.len().as_usize(),
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
    use crate::{CaretMovementDirection, EditorEngine, RingBuffer, assert_eq2,
                caret_scr_adj, col, len};

    #[test]
    fn test_cache_invalidated_on_get_mut() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(Some("md"), None);
        let engine = EditorEngine::default();

        // Set initial content and cache the memory size.
        buffer.init_with(["Hello", "World"]);
        buffer.upsert_memory_size_calc_cache(); // Populate cache
        let initial_memory = buffer
            .memory_size_calc_cache
            .get_cached()
            .cloned()
            .expect("Cache should have value");
        let initial_size = initial_memory.size().expect("Cache should have value");
        assert!(initial_size > 0);

        // Modify content through get_mut.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut
                .inner
                .lines
                .push_line("More content with lots of text");
        }
        // When buffer_mut goes out of scope, Drop should invalidate the cache.

        // Verify cache was invalidated and new size is calculated.
        buffer.upsert_memory_size_calc_cache(); // Populate cache
        let new_memory = buffer
            .memory_size_calc_cache
            .get_cached()
            .cloned()
            .expect("Cache should have value");
        let new_size = new_memory.size().expect("Cache should have value");
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
                .push_line("Even more content");
        }
        // Cache should still have old value since we used no_drop variant.
        let cached_memory = buffer
            .memory_size_calc_cache
            .get_cached()
            .cloned()
            .unwrap_or_else(MemorySize::unknown);
        assert_eq!(cached_memory.size(), Some(cached_size));

        // Force recalculation to verify content actually changed.
        buffer.invalidate_memory_size_calc_cache();
        buffer.upsert_memory_size_calc_cache(); // Populate cache with new value
        let final_memory = buffer
            .memory_size_calc_cache
            .get_cached()
            .cloned()
            .expect("Cache should have value");
        let final_size = final_memory.size().expect("Cache should have value");
        assert!(
            final_size > new_size,
            "Memory size should increase after adding more content"
        );
    }

    #[test]
    fn test_editor_empty_state() {
        let buffer: EditorBuffer =
            EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        assert_eq2!(buffer.get_lines().len(), len(1));
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_is_empty_and_len() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(None, None);

        // New buffer has one empty line, so it's not considered empty.
        assert!(!buffer.is_empty());
        assert_eq2!(buffer.len(), height(1));

        // Add some content.
        buffer.init_with(vec!["line 1", "line 2", "line 3"]);
        assert!(!buffer.is_empty());
        assert_eq2!(buffer.len(), height(3));

        // Clear all lines
        buffer.init_with::<Vec<&str>>(vec![]);
        assert!(buffer.is_empty());
        assert_eq2!(buffer.len(), height(0));
    }

    #[test]
    fn test_file_extension_functions() {
        // Test with no extension.
        let buffer: EditorBuffer = EditorBuffer::new_empty(None, None);
        assert!(!buffer.has_file_extension());
        assert!(!buffer.is_file_extension_default());
        assert_eq2!(buffer.get_maybe_file_extension(), None);

        // Test with default extension.
        let buffer: EditorBuffer =
            EditorBuffer::new_empty(Some(DEFAULT_SYN_HI_FILE_EXT), None);
        assert!(buffer.has_file_extension());
        assert!(buffer.is_file_extension_default());
        assert_eq2!(
            buffer.get_maybe_file_extension(),
            Some(DEFAULT_SYN_HI_FILE_EXT)
        );

        // Test with custom extension.
        let buffer: EditorBuffer = EditorBuffer::new_empty(Some("rs"), None);
        assert!(buffer.has_file_extension());
        assert!(!buffer.is_file_extension_default());
        assert_eq2!(buffer.get_maybe_file_extension(), Some("rs"));
    }

    #[test]
    fn test_memory_cache_functions() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(None, None);

        // Initially, cache should be empty (dirty)
        assert!(buffer.memory_size_calc_cache.get_cached().is_none());

        // Populate the cache.
        buffer.upsert_memory_size_calc_cache();
        let initial_cache = buffer
            .memory_size_calc_cache
            .get_cached()
            .cloned()
            .expect("Cache should be populated");
        assert!(initial_cache.size().is_some());

        // Note: invalidate_memory_size_calc_cache() actually invalidates AND recalculates
        // So the cache will never be None after calling it.
        let size_before_invalidate = initial_cache.size().unwrap();
        buffer.invalidate_memory_size_calc_cache();
        let cache_after_invalidate = buffer
            .memory_size_calc_cache
            .get_cached()
            .cloned()
            .expect("Cache should be recalculated after invalidate");
        assert_eq!(
            cache_after_invalidate.size().unwrap(),
            size_before_invalidate
        );

        // When accessed through get_memory_size_calc_cached(), it auto-populates
        let auto_populated = buffer.get_memory_size_calc_cached();
        assert!(auto_populated.size().is_some());

        // Verify cache is now populated.
        assert!(buffer.memory_size_calc_cache.get_cached().is_some());
    }

    #[test]
    fn test_get_mut_invalidates_cache() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(None, None);
        let engine = EditorEngine::default();

        // Populate the cache.
        buffer.upsert_memory_size_calc_cache();
        assert!(buffer.memory_size_calc_cache.get_cached().is_some());

        // get_mut should invalidate the cache when dropped.
        {
            let _buffer_mut = buffer.get_mut(engine.viewport());
        }

        // Cache should be invalidated.
        assert!(buffer.memory_size_calc_cache.get_cached().is_none());
    }

    #[test]
    fn test_get_mut_no_drop_preserves_cache() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(None, None);
        let engine = EditorEngine::default();

        // Populate the cache.
        buffer.upsert_memory_size_calc_cache();
        assert!(buffer.get_memory_size_calc_cached().size().is_some());

        // get_mut_no_drop should NOT invalidate the cache.
        {
            let _buffer_mut_no_drop = buffer.get_mut_no_drop(engine.viewport());
        }

        // Cache should still be valid.
        assert!(buffer.get_memory_size_calc_cached().size().is_some());
    }

    #[test]
    fn test_clear_selection() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(None, None);
        let engine = EditorEngine::default();

        // Add some content and create a selection.
        buffer.init_with(vec!["line 1", "line 2"]);

        // Manually add a selection.
        let buffer_mut = buffer.get_mut(engine.viewport());
        buffer_mut.inner.sel_list.insert(
            row(0),
            (
                caret_scr_adj(col(0) + row(0)),
                caret_scr_adj(col(4) + row(0)),
            )
                .into(),
            CaretMovementDirection::Right,
        );
        drop(buffer_mut);

        // Verify selection exists.
        assert!(!buffer.get_selection_list().is_empty());
        assert_eq2!(buffer.get_selection_list().len(), 1);

        // Clear selection
        buffer.clear_selection();

        // Verify selection is cleared.
        assert!(buffer.get_selection_list().is_empty());
        assert_eq2!(buffer.get_selection_list().len(), 0);
    }

    #[test]
    fn test_history_functions() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(None, None);
        let engine = EditorEngine::default();

        // Initialize with some content.
        buffer.init_with(vec!["initial"]);
        buffer.add(); // Add initial state to history

        // Make a change using the proper mutation API.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut.inner.lines.clear();
            buffer_mut.inner.lines.push_line("changed");
        }
        buffer.add(); // Add changed state to history

        // Now history should have 2 versions.
        assert_eq2!(buffer.history.versions.len(), 2.into());
        assert_eq2!(
            buffer.get_lines().get_line_content(row(0)).unwrap(),
            "changed"
        );

        // Undo should go back to "initial".
        buffer.undo();
        assert_eq2!(
            buffer.get_lines().get_line_content(row(0)).unwrap(),
            "initial"
        );

        // Redo should go forward to "changed".
        buffer.redo();
        assert_eq2!(
            buffer.get_lines().get_line_content(row(0)).unwrap(),
            "changed"
        );

        // Another undo
        buffer.undo();
        assert_eq2!(
            buffer.get_lines().get_line_content(row(0)).unwrap(),
            "initial"
        );
    }
}
