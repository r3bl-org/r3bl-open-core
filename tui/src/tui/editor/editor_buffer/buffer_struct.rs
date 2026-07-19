// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{EditorBufferConfig, SelectionContainer, history::EditorHistory,
            render_cache::RenderCache};
use crate::{CCaret, CCol, CHeight, CPos, CRow, CWidth, CursorPositionBoundsStatus,
            DEBUG_TUI_COPY_PASTE, DEBUG_TUI_MOD, DEFAULT_SYN_HI_FILE_EXT, DocSeg,
            EditorBufferMutWithDrop, GapBufferLine, GetMemSize, InlineString,
            MemoizedMemorySize, MemorySize, NumericValue, TinyInlineString, VPCaret,
            VPSize, Viewport, ZeroCopyGapBuffer, format_as_kilobytes_with_commas,
            glyphs, inline_string, locate_col, ok,
            validate_buffer_mut::EditorBufferMutNoDrop, vp_caret, vp_col, vp_pos,
            vp_row, with_mut};
use std::fmt::{Debug, Display, Formatter};

/// Stores the data for a single editor buffer using [`ZeroCopyGapBuffer`] for efficient
/// text storage.
///
/// Please do not construct this struct directly and use [`new_empty`] instead.
///
/// As of 2025, [`EditorBuffer`] uses [`ZeroCopyGapBuffer`] directly as a concrete type
/// for efficient content storage with zero-copy access. Prior to that it was using an
/// inefficient [`Vec<String>`].
///
/// 1. This struct is stored in the app's state.
/// 2. And it is paired w/ [`EditorEngine`] at runtime; which is responsible for rendering
///    it to TUI, and handling user input.
///
/// # Architecture Overview
///
/// The [`EditorBuffer`] manages text editing using a high-performance architecture:
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │ EditorBuffer                                                │
/// │ ├─ content: EditorContent                                   │
/// │ │   ├─ lines: ZeroCopyGapBuffer (high-perf gap buffer)      │
/// │ │   ├─ c_caret: CCaret (canvas cursor position)             │
/// │ │   ├─ viewport: Viewport (origin position & dimensions)    │
/// │ │   └─ selection: SelectionContainer (selection state)      │
/// │ ├─ history: EditorHistory (undo/redo buffer)                │
/// │ ├─ render_cache: RenderCache (syntax highlight cache)       │
/// │ └─ memory_size_calc_cache: MemoizedMemorySize               │
/// └─────────────────────────────────────────────────────────────┘
/// ```
///
/// ## Key Components
///
/// 1. **Storage Layer**: [`ZeroCopyGapBuffer`] stores text lines efficiently without
///    string allocations per character typed by the user.
/// 2. **Cursor Management**: Positions are maintained in [`Canvas`] coordinates
///    ([`CCaret`]) and mapped to Viewport coordinates ([`VPCaret`]) using [`Viewport`]
///    origin position.
/// 3. **Selection Tracking**: [`SelectionContainer`] tracks text selections using
///    per-line [`SelectionLine`] instances.
/// 4. **Render Optimizations**: [`RenderCache`] caches rendered [`ANSI`] lines and syntax
///    highlighting. The cache automatically invalidates whenever buffer contents change.
/// 5. **Safe Mutation Pattern**: Core text and caret mutations should go through
///    [`get_mut`] (or [`get_mut_no_drop`]), which produces an [`EditorBufferMutWithDrop`]
///    handle that validates invariants upon drop.
///
/// # Why [`EditorBuffer`] and [`EditorContent`] are split
///
/// The buffer is intentionally split into two structs to separate **core document state**
/// from **temporary caches and history management**:
///
/// 1. **Lightweight Undo/Redo Snapshots**: [`EditorHistory`] stores past versions of
///    [`EditorContent`] in a ring buffer ([`RingBufferHeap`]`<EditorContent,
///    MAX_UNDO_REDO_SIZE>`). Keeping [`EditorContent`] as a standalone struct ensures
///    undo snapshots only save text, caret, and selection state (without duplicating
///    history or heavy caches).
/// 2. **Separating Temporary Caches**: Temporary caches like [`RenderCache`]
///    (pre-formatted syntax-highlighted lines) and [`MemoizedMemorySize`] (memory
///    telemetry) are derived from [`EditorContent`]. They live exclusively on the outer
///    [`EditorBuffer`] so they can be cleared on edit without bloating undo/redo history.
///
/// # Change state during render
///
/// This struct is a document model and is immutable during the render phase; all document
/// and caret mutations take place during input event processing. Any transient UI layout
/// updates (such as updating component box bounds) belong on [`EditorEngine`], which is
/// mutable during the render phase.
///
/// # Modifying the buffer
///
/// [`InputEvent`] is converted into an [`EditorEvent`] (by [`apply_event`], which is then
/// used to modify the [`EditorBuffer`] via:
/// 1. [`apply_editor_event`]
/// 2. [`apply_editor_events`]
///
/// In order for the commands to be executed, the functions in [`engine_internal_api`] are
/// used.
///
/// These functions take any one of the following args:
/// 1. [`EditorArgsMut`]
/// 2. [`EditorBuffer`] and [`EditorEngine`]
///
/// # Accessing and mutating the fields (w/ validation)
///
/// To perform validated mutations on the buffer, use the [`get_mut`] method. It returns
/// an [`EditorBufferMutWithDrop`] handle which implements the [`Drop`] trait to
/// automatically execute post-mutation invariant checks
/// ([`perform_validation_checks_after_mutation`]).
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`apply_editor_event`]: crate::EditorEvent::apply_editor_event
/// [`apply_editor_events`]: crate::EditorEvent::apply_editor_events
/// [`apply_event`]: crate::engine_public_api::apply_event
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [`CCaret`]: crate::CCaret
/// [`Drop`]: crate::EditorBufferMutWithDrop#method.drop
/// [`EditorArgsMut`]: crate::EditorArgsMut
/// [`EditorBufferMut`]: crate::EditorBufferMut
/// [`EditorBufferMutWithDrop`]: crate::EditorBufferMutWithDrop
/// [`EditorContent`]: crate::EditorContent
/// [`EditorEngine`]: crate::EditorEngine
/// [`EditorEvent`]: crate::EditorEvent
/// [`EditorHistory`]: crate::EditorHistory
/// [`engine_internal_api`]: mod@crate::editor_engine::engine_internal_api
/// [`get_mut_no_drop`]: crate::EditorBuffer::get_mut_no_drop
/// [`get_mut`]: crate::EditorBuffer::get_mut
/// [`InputEvent`]: crate::InputEvent
/// [`MemoizedMemorySize`]: crate::MemoizedMemorySize
/// [`new_empty`]: EditorBuffer::new_empty
/// [`perform_validation_checks_after_mutation`]:
///     crate::validate_buffer_mut::perform_validation_checks_after_mutation
/// [`RenderCache`]: super::render_cache::RenderCache
/// [`RingBufferHeap`]: crate::RingBufferHeap
/// [`SelectionContainer`]: crate::SelectionContainer
/// [`SelectionLine`]: crate::SelectionLine
/// [`Viewport`]: crate::Viewport
/// [`VPCaret`]: crate::VPCaret
/// [`ZeroCopyGapBuffer`]: crate::ZeroCopyGapBuffer
#[derive(Clone, PartialEq, Default)]
pub struct EditorBuffer {
    /// Core text content, cursor position, viewport scrolling state, and selections
    /// ([`EditorContent`]).
    content: EditorContent,

    /// Undo and redo history buffer stack ([`EditorHistory`]).
    history: EditorHistory,

    /// Cached syntax highlighting and formatted [`ANSI`] line cache ([`RenderCache`]).
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    render_cache: RenderCache,

    /// Memoized memory size calculation for [`std::fmt::Display`] trait performance
    /// ([`MemoizedMemorySize`]).
    memory_size_calc_cache: MemoizedMemorySize,
}

/// Contains the core text content and editing state using [`ZeroCopyGapBuffer`] for
/// storage.
///
/// # Kinds of Caret Positions
///
/// There are two variants for the caret position value:
/// 1. [`VPCaret`] - this is the position of the caret (unadjusted for `vp_origin`) and
///    represents the position of the caret in the viewport.
/// 2. [`CCaret`] - this is the position of the caret (adjusted for `vp_origin`) and
///    represents the position of the caret in the buffer (not the viewport).
///
/// # Caret Position Diagrams
///
/// This is the "display" col index (grapheme-cluster-based) and not "logical" col index
/// (byte-based) position (both are defined in [`graphemes_module`]).
///
/// > Please review [`GCStringOwned`], specifically the methods in [`gc_string`] for more
/// > details on how the conversion between "display" and "logical" indices is done.
/// >
/// > This results from the fact that [`UTF-8`] is a variable width text encoding scheme,
/// > that can use between 1 and 4 bytes to represent a single character. So the width a
/// > human perceives, and it's byte size in RAM can be different.
/// >
/// > Videos:
/// >
/// > - [Live coding video on Rust String](https://youtu.be/7I11degAElQ?)
/// > - [UTF-8 encoding video](https://youtu.be/wIVmDPc16wA)
///
/// 1. It represents the current caret position (relative to the [`style_adjusted_bounds`]
///    of the enclosing [`FlexBox`]).
///
/// 2. It works w/ [`MoveCursorPositionRelTo`] as well.
///
/// > 💡 For the diagrams below, the caret is where `▲` and `►` intersects.
///
/// Start of line:
/// ```text
/// Caret : ▲, ►
/// R ┌──────────┐
/// 0 ►abcab     │
///   └▲─────────┘
///   C0123456789
/// ```
///
/// Middle of line:
/// ```text
/// Caret : ▲, ►
/// R ┌──────────┐
/// 0 ►abcab     │
///   └───▲──────┘
///   C0123456789
/// ```
///
/// End of line:
/// ```text
/// Caret : ▲, ►
/// R ┌──────────┐
/// 0 ►abcab     │
///   └─────▲────┘
///   C0123456789
/// ```
///
/// # Viewport Scrolling Diagrams
///
/// ## Vertical Scrolling and Viewport
///
/// ```text
/// ╭0────────────────────╮
/// 0                     │
/// │   above viewport    │ <- c_caret.row_index < vp_origin
/// │                     │
/// ├───── vp_origin ─────┤
/// │          ▲          │      ▲
/// │                     │      │
/// │   within viewport   │  vp height (visible region)
/// │                     │      │
/// │          ▼          │      ▼
/// ├───── vp_origin ─────┤
/// │      + vp height    │
/// │                     │
/// │   below viewport    │ <- c_caret.row_index >= vp_origin + vp_height
/// │                     │
/// ╰─────────────────────╯
/// ```
///
/// ## Horizontal Scrolling and Viewport
///
/// ```text
///           ◄─   vp width   ─►
/// ╭0────────┼────────────────┼─────────>
/// 0         │                │
/// │ left of │◄─  within vp ─►│ right of│
/// │         │                │         │
/// ╰─────────┼────────────────┼─────────>
///        vp_origin           vp_origin
///                            + vp width
/// ```
///
/// # Selection Architecture
///
/// The [`SelectionContainer`] is used to keep track of the selections in the buffer.
/// Each entry in the list represents a row of text in the buffer.
/// - The row index is [`CRow`].
/// - The value is [`SelectionLine`].
///
/// [`CCaret`]: crate::CCaret
/// [`CRow`]: crate::CRow
/// [`find_syntax_by_extension`]: syntect::parsing::SyntaxSet::find_syntax_by_extension
/// [`FlexBox`]: crate::tui::FlexBox
/// [`gc_string`]: mod@crate::graphemes::gc_string
/// [`GCStringOwned`]: crate::graphemes::GCStringOwned
/// [`graphemes_module`]: crate::graphemes
/// [`MoveCursorPositionRelTo`]: crate::tui::RenderOpCommon::MoveCursorPositionRelTo
/// [`SelectionContainer`]: crate::SelectionContainer
/// [`SelectionLine`]: crate::SelectionLine
/// [`style_adjusted_bounds`]: crate::tui::FlexBox::style_adjusted_bounds
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`Viewport`]: crate::Viewport
/// [`VPCaret`]: crate::VPCaret
/// [`ZeroCopyGapBuffer`]: crate::ZeroCopyGapBuffer
#[derive(Clone, PartialEq, Default)]
pub struct EditorContent {
    /// A list of lines representing the document being edited ([`ZeroCopyGapBuffer`]).
    pub(super) lines: ZeroCopyGapBuffer,

    /// Absolute canvas position of the caret ([`CCaret`]).
    pub(super) c_caret: CCaret,

    /// Viewport offset and dimensions ([`Viewport`]).
    pub(super) viewport: Viewport,

    /// Optional file extension (e.g. `rs` or `md`) used for syntax highlighting rules.
    pub(super) maybe_file_extension: Option<TinyInlineString>,

    /// Optional file path used for display purposes only.
    pub(super) maybe_file_path: Option<InlineString>,

    /// Selection container ([`SelectionContainer`]).
    pub(super) selection: SelectionContainer,
}

impl EditorContent {
    /// Returns a reference to the internal [`ZeroCopyGapBuffer`].
    #[must_use]
    pub fn get_lines(&self) -> &ZeroCopyGapBuffer { &self.lines }

    /// Returns a reference to the text selection container ([`SelectionContainer`]).
    #[must_use]
    pub fn get_selection_container(&self) -> &SelectionContainer { &self.selection }
}

mod construct {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl EditorBuffer {
        /// Creates a new, empty [`EditorBuffer`].
        ///
        /// Pass unit `()`, [`FileExtensionToken`], [`FilePathToken`], a combination
        /// using `+`, or an [`EditorBufferConfig`] struct directly. See
        /// [`EditorBufferConfig`] for details and usage examples.
        ///
        /// [`EditorBufferConfig`]: crate::EditorBufferConfig
        /// [`FileExtensionToken`]: crate::FileExtensionToken
        /// [`FilePathToken`]: crate::FilePathToken
        #[must_use]
        pub fn new_empty<'a>(arg_config: impl Into<EditorBufferConfig<'a>>) -> Self {
            let config: EditorBufferConfig<'a> = arg_config.into();
            let mut lines = ZeroCopyGapBuffer::default();
            lines.push_line("");

            let it = Self {
                content: EditorContent {
                    lines,
                    maybe_file_extension: config.maybe_file_extension.map(Into::into),
                    maybe_file_path: config.maybe_file_path.map(Into::into),
                    ..Default::default()
                },
                ..Default::default()
            };

            DEBUG_TUI_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::info!(
                    message = %inline_string!("Construct EditorBuffer {ch}", ch = glyphs::CONSTRUCT_GLYPH),
                    file_extension = ?config.maybe_file_extension,
                    file_path = ?config.maybe_file_path
                );
            });

            it
        }
    }
}

mod versions {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl EditorBuffer {
        /// Saves a snapshot of the current [`EditorContent`] to the undo stack.
        ///
        /// Clears internal caches to prepare for new mutations.
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

        /// Reverts the [`EditorContent`] to the previous state on the undo stack.
        ///
        /// Clears internal caches to trigger a re-render.
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

        /// Re-applies the next [`EditorContent`] state on the redo stack.
        ///
        /// Clears internal caches to trigger a re-render.
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

/// Relating to line display width at caret row or given row index.
mod content_display_width {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl EditorBuffer {
        /// Returns the maximum row index ([`CRow`]) present in the buffer.
        #[must_use]
        pub fn get_max_row_index(&self) -> CRow { self.get_lines().get_max_row_index() }

        /// Gets the line display width ([`CWidth`]) at the current caret row index.
        #[must_use]
        pub fn get_line_display_width_at_c_caret(&self) -> CWidth {
            self.get_lines()
                .get_line_display_width_at_row_index(self.get_c_caret().row_index)
        }

        /// Gets the line display width ([`CWidth`]) at the specified row index.
        #[must_use]
        pub fn get_line_display_width_at_row_index(&self, row_index: CRow) -> CWidth {
            self.get_lines()
                .get_line_display_width_at_row_index(row_index)
        }
    }
}

/// Relating to content around the caret.
mod content_near_caret {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl EditorBuffer {
        fn get_line_and_col_at_caret(&self) -> Option<(GapBufferLine<'_>, CCol)> {
            if self.is_empty() {
                return None;
            }
            let caret = self.get_c_caret();
            let line = self.content.lines.get_line(caret.row_index)?;
            Some((line, caret.col_index))
        }

        /// Returns `true` if the line containing the caret is empty or out of bounds.
        #[must_use]
        pub fn is_line_at_caret_empty(&self) -> bool {
            self.get_lines().is_line_empty(self.get_c_caret().row_index)
        }

        /// Returns a [`GapBufferLine`] reference for the line containing the caret, or
        /// [`None`] if empty.
        #[must_use]
        pub fn get_line_at_c_caret(&self) -> Option<GapBufferLine<'_>> {
            if self.is_empty() {
                return None;
            }

            let row_index = self.get_c_caret().row_index;
            self.content.lines.get_line(row_index)
        }

        /// Returns the grapheme segment ([`DocSeg`]) at the end of the line containing
        /// the caret if the caret is at the end.
        #[must_use]
        pub fn get_seg_at_end_of_line_at_c_caret(&self) -> Option<DocSeg> {
            let (line, _) = self.get_line_and_col_at_caret()?;

            let CursorPositionBoundsStatus::AtEnd = locate_col(self) else {
                return None;
            };

            line.info().get_seg_at_end()
        }

        /// Returns the grapheme segment ([`DocSeg`]) directly to the right of the caret.
        #[must_use]
        pub fn get_seg_to_right_of_caret(&self) -> Option<DocSeg> {
            let (line, col_index) = self.get_line_and_col_at_caret()?;

            match locate_col(self) {
                CursorPositionBoundsStatus::AtEnd => line.info().get_seg_at_end(),
                _ => line.info().get_seg_at_right_of(col_index),
            }
        }

        /// Returns the grapheme segment ([`DocSeg`]) directly to the left of the caret.
        #[must_use]
        pub fn get_seg_to_left_of_caret(&self) -> Option<DocSeg> {
            let (line, col_index) = self.get_line_and_col_at_caret()?;

            match locate_col(self) {
                CursorPositionBoundsStatus::AtEnd => line.info().get_seg_at_end(),
                _ => line.info().get_seg_at_left_of(col_index),
            }
        }

        /// Returns the string slice of the previous line directly above the caret, or
        /// [`None`] if at row 0.
        #[must_use]
        pub fn get_prev_line_above_caret(&self) -> Option<&str> {
            if self.is_empty() {
                return None;
            }

            let row_index = self.get_c_caret().row_index;
            if row_index.is_zero() {
                return None;
            }

            // This is safe since we already checked that row_index is not zero.
            let prev_row_index = row_index - 1;

            // Use the concrete method that delegates to get_line.
            self.get_lines().get_line_content(prev_row_index)
        }

        #[must_use]
        fn get_seg_and_line_at_caret(&self) -> Option<(DocSeg, GapBufferLine<'_>)> {
            let (line, col_index) = self.get_line_and_col_at_caret()?;
            let doc_seg = line.info().get_seg_containing(col_index)?;
            Some((doc_seg, line))
        }

        /// Returns the string slice of the grapheme cluster under the caret position.
        #[must_use]
        pub fn get_str_at_caret(&self) -> Option<&str> {
            let (doc_seg, line) = self.get_seg_and_line_at_caret()?;
            Some(doc_seg.get_str(line.content()))
        }

        /// Returns the grapheme segment ([`DocSeg`]) under the caret position.
        #[must_use]
        pub fn get_seg_at_caret(&self) -> Option<DocSeg> {
            let (doc_seg, _) = self.get_seg_and_line_at_caret()?;
            Some(doc_seg)
        }

        /// Returns the string slice of the next line directly below the caret, or
        /// [`None`] if at bottom.
        #[must_use]
        pub fn get_next_line_below_caret(&self) -> Option<&str> {
            if self.is_empty() {
                return None;
            }
            let c_caret_row_index = self.get_c_caret().row_index;
            let next_line_row_index = c_caret_row_index + 1;
            // Use the concrete method that delegates to get_line.
            self.get_lines().get_line_content(next_line_row_index)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxHighlightPipeline<'a> {
    /// Our [custom Markdown parser] & AST renderer (for `.md` files or default Markdown
    /// buffers).
    ///
    /// [custom Markdown parser]: fn@crate::parse_markdown
    R3BLMarkdown,

    /// [Syntect] syntax highlighter with a specific language extension (e.g. `"rs"`,
    /// `"json"`).
    ///
    /// [Syntect]: crate::convert_syntect_to_styled_text
    Syntect(&'a str),

    /// Plain text rendering without syntax highlighting (for [`None`] or unsupported
    /// files).
    PlainText,
}

mod access_and_mutate {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl EditorBuffer {
        /// Determines the appropriate [`SyntaxHighlightPipeline`] based on the file
        /// extension.
        #[must_use]
        pub fn get_syntax_highlight_pipeline(&self) -> SyntaxHighlightPipeline<'_> {
            match self.get_maybe_file_extension() {
                Some(DEFAULT_SYN_HI_FILE_EXT) => SyntaxHighlightPipeline::R3BLMarkdown,
                Some(ext) => SyntaxHighlightPipeline::Syntect(ext),
                None => SyntaxHighlightPipeline::PlainText,
            }
        }

        /// Returns the optional file extension string slice if set.
        #[must_use]
        pub fn get_maybe_file_extension(&self) -> Option<&str> {
            match self.content.maybe_file_extension {
                Some(ref s) => Some(s.as_str()),
                None => None,
            }
        }

        /// Returns `true` if the buffer contains no text lines.
        #[must_use]
        pub fn is_empty(&self) -> bool { self.content.lines.is_empty() }

        /// Returns the string slice content at the specified [`CRow`] index.
        #[must_use]
        pub fn get_line_at_row_index(&self, row_index: CRow) -> Option<&str> {
            // Use the concrete method that delegates to get_line.
            self.content.lines.get_line_content(row_index)
        }

        /// Returns the line count ([`CHeight`]) of the buffer.
        #[must_use]
        pub fn get_c_height(&self) -> CHeight { self.content.lines.get_line_count() }

        /// Returns a reference to the internal [`ZeroCopyGapBuffer`].
        #[must_use]
        pub fn get_lines(&self) -> &ZeroCopyGapBuffer { &self.content.lines }

        /// Formats the buffer content into an [`InlineString`] using `, ` as a separator
        /// instead of newlines.
        #[must_use]
        pub fn get_as_string_with_comma_instead_of_newlines(&self) -> InlineString {
            self.get_as_string_with_separator(", ")
        }

        /// Formats the buffer content into an [`InlineString`] using `\n` as a separator.
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
        /// text editor and not binary editor, it operates on [`UTF-8`] encoded text files
        /// and not binary files (which just contain `u8`s).
        ///
        /// You can convert a `&[u8]` to a `&str` using [`std::str::from_utf8`].
        /// Initializes the buffer with the given lines, clearing all state including
        /// history. This is meant to be used when loading a new file or completely
        /// replacing buffer content.
        ///
        /// For normal editing operations that preserve history, use [`Self::get_mut()`]
        /// and the mutation API [`mod@crate::content_mut`].
        ///
        /// - A [`Vec<u8>`] can be converted into a `&[u8]` using `&vec[..]` or
        ///   `vec.as_slice()` or `vec.as_bytes()`.
        /// - Then you can convert the `&[u8]` to a `&str` using [`std::str::from_utf8`].
        /// - And then call [`str::lines()`] on the `&str` to get an iterator over the
        ///   lines which can be passed to this method.
        ///
        /// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
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
            self.content.c_caret = CCaret::default();

            // Reset viewport origin.
            self.content
                .viewport
                .set_origin_pos(|pos| *pos = CPos::default());

            // Empty the content render cache.
            self.render_cache.clear();

            // Invalidate and recalculate memory size cache.
            self.invalidate_memory_size_calc_cache();

            // Reset undo/redo history since this is a complete re-initialization.
            self.history.clear();
        }

        /// Returns the viewport caret position ([`VPCaret`]).
        ///
        /// # Panics
        ///
        /// This method panics if the `row_index` or `col_index` arithmetic overflows
        /// `u16` limits.
        #[must_use]
        pub fn get_vp_caret(&self) -> VPCaret {
            let canvas = self.content.c_caret;
            let origin = self.content.viewport.get_origin_pos();
            let row = vp_row(
                u16::try_from(
                    canvas
                        .0
                        .row_index
                        .as_usize()
                        .saturating_sub(origin.row_index.as_usize()),
                )
                .expect("Failed to convert row index to u16"),
            );
            let col = vp_col(
                u16::try_from(
                    canvas
                        .0
                        .col_index
                        .as_usize()
                        .saturating_sub(origin.col_index.as_usize()),
                )
                .expect("Failed to convert col index to u16"),
            );
            vp_caret(vp_pos(col.as_u16(), row.as_u16()))
        }

        /// Returns the canvas caret position ([`CCaret`]).
        #[must_use]
        pub fn get_c_caret(&self) -> CCaret { self.content.c_caret }

        /// Returns the top-left origin position ([`CPos`]) of the viewport.
        #[must_use]
        pub fn get_vp_origin(&self) -> CPos { self.content.viewport.get_origin_pos() }

        /// Even though this struct is mutable by `editor_ops_insert`, this method is
        /// provided to mark when mutable access is made to this struct.
        ///
        /// This makes it easy to determine what code mutates this struct, since it is
        /// necessary to validate things after mutation quite a bit in
        /// `editor_ops_insert`.
        ///
        /// [`crate::EditorBufferMut`] implements the [Drop] trait, which ensures that any
        /// validation changes are applied after making changes to the [`EditorBuffer`].
        pub fn get_mut(&mut self, vp: VPSize) -> EditorBufferMutWithDrop<'_> {
            self.content.viewport.set_size(|size| *size = vp);
            EditorBufferMutWithDrop::new(
                &mut self.content.lines,
                &mut self.content.c_caret,
                &mut self.content.viewport,
                &mut self.content.selection,
                &mut self.memory_size_calc_cache,
            )
        }

        /// This is a special case of [`EditorBuffer::get_mut`] where the [Drop] trait is
        /// not used to perform validation checks after mutation. This is useful when you
        /// don't want to run validation checks after mutation, which happens when the
        /// window is resized using [`mod@crate::validate_scroll_on_resize`].
        pub fn get_mut_no_drop(&mut self, vp: VPSize) -> EditorBufferMutNoDrop<'_> {
            self.content.viewport.set_size(|size| *size = vp);
            EditorBufferMutNoDrop::new(
                &mut self.content.lines,
                &mut self.content.c_caret,
                &mut self.content.viewport,
                &mut self.content.selection,
                &mut self.memory_size_calc_cache,
            )
        }

        /// Returns `true` if there is an active text selection.
        #[must_use]
        pub fn has_selection(&self) -> bool { !self.content.selection.is_empty() }

        /// Clears the text selection that the user has made in the editor.
        ///
        /// Large selections can occupy a significant amount of memory, so this method
        /// also invalidates the memory size cache to ensure accurate telemetry reporting.
        pub fn clear_selection(&mut self) {
            self.content.selection.clear();
            self.invalidate_memory_size_calc_cache();
        }

        /// Mutate the [`SelectionContainer`] state of the buffer and invalidate the
        /// [`RenderCache`].
        pub fn mutate_selection<F>(&mut self, mutator: F)
        where
            F: FnOnce(&mut SelectionContainer),
        {
            mutator(&mut self.content.selection);
            self.render_cache.clear();
        }

        /// Returns the optional file path inline string reference if set.
        #[must_use]
        pub fn get_file_path(&self) -> Option<&InlineString> {
            self.content.maybe_file_path.as_ref()
        }

        /// Sets the file path and clears render/telemetry caches.
        pub fn set_file_path(&mut self, path: impl Into<InlineString>) {
            self.content.maybe_file_path = Some(path.into());
            self.render_cache.clear();
            self.invalidate_memory_size_calc_cache();
        }

        /// Sets the file extension and clears render/telemetry caches.
        pub fn set_file_extension(&mut self, ext: impl Into<TinyInlineString>) {
            self.content.maybe_file_extension = Some(ext.into());
            self.render_cache.clear();
            self.invalidate_memory_size_calc_cache();
        }

        /// Returns a reference to the internal [`EditorContent`].
        #[must_use]
        pub fn get_content(&self) -> &EditorContent { &self.content }

        /// Returns a mutable reference to the internal [`EditorContent`].
        pub fn get_content_mut(&mut self) -> &mut EditorContent { &mut self.content }

        /// Returns a reference to the internal [`RenderCache`].
        #[must_use]
        pub fn get_render_cache(&self) -> &RenderCache { &self.render_cache }

        /// Returns a mutable reference to the internal [`RenderCache`].
        pub fn get_render_cache_mut(&mut self) -> &mut RenderCache {
            &mut self.render_cache
        }

        /// Returns a reference to the internal [`MemoizedMemorySize`].
        #[must_use]
        pub fn get_memory_size_calc_cache(&self) -> &MemoizedMemorySize {
            &self.memory_size_calc_cache
        }

        /// Returns a mutable reference to the internal [`MemoizedMemorySize`].
        pub fn get_memory_size_calc_cache_mut(&mut self) -> &mut MemoizedMemorySize {
            &mut self.memory_size_calc_cache
        }

        /// Returns a reference to the internal [`EditorHistory`].
        #[must_use]
        pub fn get_history(&self) -> &EditorHistory { &self.history }

        /// Returns a mutable reference to the internal [`EditorHistory`].
        pub fn get_history_mut(&mut self) -> &mut EditorHistory { &mut self.history }

        #[must_use]
        pub fn get_selection_container(&self) -> &SelectionContainer {
            &self.content.selection
        }
    }
}

/// Efficient Display implementation for telemetry logging.
mod impl_display {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Display for EditorBuffer {
        /// This must be a fast implementation, so we avoid deep traversal of the
        /// editor buffer. This is used for telemetry reporting, and it is expected
        /// to be fast, since it is called in a hot loop, on every render.
        ///
        /// # Implementation Note: Intentional Use of Raw `usize`
        ///
        /// This method uses `.as_usize()` for display formatting purposes:
        /// - Line/column numbers are converted to 1-indexed display format
        /// - Type-safe bounds checking not needed for display-only operations
        /// - Required for user-friendly editor coordinate display
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
            let line_count = self.content.lines.get_line_count().as_usize();
            let has_selection = self.has_selection();

            // Get active line/column info.
            let caret = self.get_c_caret();
            let line = caret.row_index.as_usize() + 1; // 1-indexed for display
            let col = caret.col_index.as_usize() + 1; // 1-indexed for display

            // Get file info and format output.
            let ext = match self.content.maybe_file_extension.as_ref() {
                Some(e) => e.as_str(),
                None => "txt",
            };

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
                let sel_count = self.content.selection.len();
                write!(f, ":sel({sel_count}L)")?;
            }

            // Add summary info.
            write!(f, "[lines={line_count}, size={memory_size}]")?;

            ok!()
        }
    }
}

mod impl_debug {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for EditorBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
        /// # Implementation Note: Intentional Use of Raw `usize`
        ///
        /// Uses `.as_usize()` for Debug formatting output only.
        /// Type-safe bounds checking not needed for debug display.
        #[allow(clippy::nonstandard_macro_braces)]
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let mem_size = self.get_mem_size();
            let mem_size_fmt = format_as_kilobytes_with_commas(mem_size);

            write! {
                f,
"EditorContent [
    - lines: {lines:?}, size: {size}
    - selection_map: {map}
    - ext: {ext:?}, path:{path:?}, caret: {caret:?}, vp_origin: {vp_origin:?}
]",
                lines = self.lines.get_line_count().as_usize(),
                size = mem_size_fmt,
                ext = self.maybe_file_extension,
                caret = self.c_caret,
                map = self.selection.to_formatted_string(),
                vp_origin = self.viewport.get_origin_pos(),
                path = self.maybe_file_path,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CPos, EditorEngine, FileExtensionToken, FilePathToken, RingBuffer,
                assert_eq2, c_caret, c_col, c_height, c_len, c_row, c_width, vp_caret,
                vp_pos};

    #[test]
    fn test_cache_invalidated_on_get_mut() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(FileExtensionToken("md"));
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
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        assert_eq2!(buffer.get_lines().get_line_count(), c_height(1));
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_is_empty_and_height() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(());

        // New buffer has one empty line, so it's not considered empty.
        assert!(!buffer.is_empty());
        assert_eq2!(buffer.get_c_height(), c_height(1));

        // Add some content.
        buffer.init_with(vec!["line 1", "line 2", "line 3"]);
        assert!(!buffer.is_empty());
        assert_eq2!(buffer.get_c_height(), c_height(3));

        // Clear all lines.
        buffer.init_with::<Vec<&str>>(vec![]);
        assert!(buffer.is_empty());
        assert_eq2!(buffer.get_c_height(), c_height(0));
    }

    #[test]
    fn test_file_extension_functions() {
        // Test with no extension.
        let buffer: EditorBuffer = EditorBuffer::new_empty(());
        assert_eq2!(
            buffer.get_syntax_highlight_pipeline(),
            SyntaxHighlightPipeline::PlainText
        );
        assert_eq2!(buffer.get_maybe_file_extension(), None);

        // Test with default extension.
        let buffer: EditorBuffer =
            EditorBuffer::new_empty(FileExtensionToken(DEFAULT_SYN_HI_FILE_EXT));
        assert_eq2!(
            buffer.get_syntax_highlight_pipeline(),
            SyntaxHighlightPipeline::R3BLMarkdown
        );
        assert_eq2!(
            buffer.get_maybe_file_extension(),
            Some(DEFAULT_SYN_HI_FILE_EXT)
        );

        // Test with custom extension.
        let buffer: EditorBuffer = EditorBuffer::new_empty(FileExtensionToken("rs"));
        assert_eq2!(
            buffer.get_syntax_highlight_pipeline(),
            SyntaxHighlightPipeline::Syntect("rs")
        );
        assert_eq2!(buffer.get_maybe_file_extension(), Some("rs"));
    }

    #[test]
    fn test_str_and_seg_at_caret() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Hello", "World 😃"]);

        // Line 0, Caret at col 0 -> "H"
        assert_eq2!(buffer.get_str_at_caret(), Some("H"));
        assert!(buffer.get_seg_at_caret().is_some());

        // Move caret to col 6 on line 1 ("😃")
        {
            let buffer_mut = buffer.get_mut(EditorEngine::default().viewport());
            *buffer_mut.inner.c_caret = c_caret(c_col(6) + c_row(1));
        }
        assert_eq2!(buffer.get_str_at_caret(), Some("😃"));
        assert!(buffer.get_seg_at_caret().is_some());

        // Empty buffer -> None
        buffer.init_with::<Vec<&str>>(vec![]);
        assert_eq2!(buffer.get_str_at_caret(), None);
        assert_eq2!(buffer.get_seg_at_caret(), None);
    }

    #[test]
    fn test_memory_cache_functions() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(());

        // Initially, cache should be empty (dirty).
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
        let size_before_invalidate = initial_cache.size().expect("conversion error");
        buffer.invalidate_memory_size_calc_cache();
        let cache_after_invalidate = buffer
            .memory_size_calc_cache
            .get_cached()
            .cloned()
            .expect("Cache should be recalculated after invalidate");
        assert_eq!(
            cache_after_invalidate.size().expect("conversion error"),
            size_before_invalidate
        );

        // When accessed through get_memory_size_calc_cached(), it auto-populates.
        let auto_populated = buffer.get_memory_size_calc_cached();
        assert!(auto_populated.size().is_some());

        // Verify cache is now populated.
        assert!(buffer.memory_size_calc_cache.get_cached().is_some());
    }

    #[test]
    fn test_get_mut_invalidates_cache() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(());
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
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(());
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
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(());
        let engine = EditorEngine::default();

        // Add some content and create a selection.
        buffer.init_with(vec!["line 1", "line 2"]);

        // Manually add a selection.
        let buffer_mut = buffer.get_mut(engine.viewport());
        buffer_mut
            .inner
            .selection
            .insert((c_row(0), c_col(0)..c_col(4)));
        drop(buffer_mut);

        // Verify selection exists.
        assert!(!buffer.get_selection_container().is_empty());
        assert_eq2!(buffer.get_selection_container().len(), 1);

        // Clear selection.
        buffer.clear_selection();

        // Verify selection is cleared.
        assert!(buffer.get_selection_container().is_empty());
        assert_eq2!(buffer.get_selection_container().len(), 0);
    }

    #[test]
    fn test_history_functions() {
        let mut buffer: EditorBuffer = EditorBuffer::new_empty(());
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
        assert_eq2!(buffer.get_history().versions.len(), c_len(2));
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "changed"
        );

        // Undo should go back to "initial".
        buffer.undo();
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "initial"
        );

        // Redo should go forward to "changed".
        buffer.redo();
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "changed"
        );

        // Another undo.
        buffer.undo();
        assert_eq2!(
            buffer
                .get_lines()
                .get_line_content(c_row(0))
                .expect("conversion error"),
            "initial"
        );
    }

    #[test]
    fn test_editor_buffer_config_dsl() {
        // Test FilePath conversion.
        let config: EditorBufferConfig = FilePathToken("test.rs").into();
        assert_eq2!(config.maybe_file_extension, None);
        assert_eq2!(config.maybe_file_path, Some("test.rs"));

        // Test FileExtension conversion.
        let config: EditorBufferConfig = FileExtensionToken("rs").into();
        assert_eq2!(config.maybe_file_extension, Some("rs"));
        assert_eq2!(config.maybe_file_path, None);

        // Test FileExtension + FilePath DSL.
        let config = FileExtensionToken("rs") + FilePathToken("src/main.rs");
        assert_eq2!(config.maybe_file_extension, Some("rs"));
        assert_eq2!(config.maybe_file_path, Some("src/main.rs"));

        // Test FilePath + FileExtension DSL.
        let config = FilePathToken("src/main.rs") + FileExtensionToken("rs");
        assert_eq2!(config.maybe_file_extension, Some("rs"));
        assert_eq2!(config.maybe_file_path, Some("src/main.rs"));

        // Test EditorBuffer::new_empty with combined config DSL.
        let buffer = EditorBuffer::new_empty(
            FileExtensionToken("rs") + FilePathToken("src/main.rs"),
        );
        assert_eq2!(buffer.get_maybe_file_extension(), Some("rs"));
        assert_eq2!(
            buffer.get_file_path().map(smallstr::SmallString::as_str),
            Some("src/main.rs")
        );
    }

    #[test]
    fn test_display_and_debug_impls() {
        // Unnamed new buffer.
        let buffer = EditorBuffer::new_empty(FileExtensionToken("txt"));
        let display_str = format!("{buffer}");
        assert!(display_str.starts_with("editor:<new-buffer>.txt:L1:C1[lines=1, size="));

        // Named buffer with path.
        let mut buffer = EditorBuffer::new_empty(
            FileExtensionToken("rs") + FilePathToken("src/lib.rs"),
        );
        let display_str = format!("{buffer}");
        assert!(display_str.starts_with("editor:lib.rs.rs:L1:C1[lines=1, size="));

        // Display output with selection.
        let engine = EditorEngine::default();
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            buffer_mut
                .inner
                .selection
                .insert((c_row(0), c_col(0)..c_col(5)));
        }
        let display_str_sel = format!("{buffer}");
        assert!(display_str_sel.contains(":sel(1L)"));

        // Debug formatting.
        let debug_buffer_str = format!("{buffer:?}");
        assert!(debug_buffer_str.contains("EditorBuffer ["));

        let debug_content_str = format!("{:?}", buffer.get_content());
        assert!(debug_content_str.contains("EditorContent ["));
    }

    #[test]
    fn test_content_near_caret_helpers() {
        let mut buffer = EditorBuffer::new_empty(());
        let engine = EditorEngine::default();

        // Empty line at caret initially.
        assert!(buffer.is_line_at_caret_empty());
        assert!(buffer.get_line_at_c_caret().is_some());
        assert_eq2!(buffer.get_line_at_c_caret().unwrap().content(), "");
        assert_eq2!(buffer.get_prev_line_above_caret(), None);
        assert_eq2!(buffer.get_next_line_below_caret(), None);

        // Add 3 lines of content.
        buffer.init_with(["first line", "second line", "third line"]);
        assert!(!buffer.is_line_at_caret_empty());

        // At line 0 caret (col 0): "first line".
        assert_eq2!(buffer.get_prev_line_above_caret(), None);
        assert_eq2!(buffer.get_next_line_below_caret(), Some("second line"));

        // Move caret to line 1.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            *buffer_mut.inner.c_caret = c_caret(c_col(0) + c_row(1));
        }
        assert_eq2!(buffer.get_prev_line_above_caret(), Some("first line"));
        assert_eq2!(buffer.get_next_line_below_caret(), Some("third line"));

        // Test get_seg_to_left_of_caret at col 2.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            *buffer_mut.inner.c_caret = c_caret(c_col(2) + c_row(1));
        }
        assert!(buffer.get_seg_to_left_of_caret().is_some());

        // Move caret to end of line 1 (col 11) to test get_seg_at_end_of_line_at_c_caret
        // and get_seg_to_right_of_caret.
        {
            let buffer_mut = buffer.get_mut(engine.viewport());
            *buffer_mut.inner.c_caret = c_caret(c_col(11) + c_row(1));
        }
        assert!(buffer.get_seg_at_end_of_line_at_c_caret().is_some());
        assert!(buffer.get_seg_to_right_of_caret().is_some());
    }

    #[test]
    fn test_content_display_width_helpers() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(["hello world", "abc"]);

        assert_eq2!(buffer.get_max_row_index(), c_row(1));
        assert_eq2!(buffer.get_line_display_width_at_c_caret(), c_width(11));
        assert_eq2!(
            buffer.get_line_display_width_at_row_index(c_row(1)),
            c_width(3)
        );
    }

    #[test]
    fn test_access_and_mutate_helpers() {
        let mut buffer = EditorBuffer::new_empty(());

        // Set and get file path.
        assert_eq2!(buffer.get_file_path(), None);
        buffer.set_file_path("path/to/my_file.rs");
        assert_eq2!(
            buffer.get_file_path().map(smallstr::SmallString::as_str),
            Some("path/to/my_file.rs")
        );

        // Set and get file extension.
        buffer.set_file_extension("rs");
        assert_eq2!(buffer.get_maybe_file_extension(), Some("rs"));

        // String formatting helpers.
        buffer.init_with(["line 1", "line 2"]);
        assert_eq2!(buffer.get_line_at_row_index(c_row(0)), Some("line 1"));
        assert_eq2!(buffer.get_line_at_row_index(c_row(2)), None);

        assert_eq2!(
            buffer
                .get_as_string_with_comma_instead_of_newlines()
                .as_str(),
            "line 1, line 2"
        );
        assert_eq2!(
            buffer.get_as_string_with_newlines().as_str(),
            "line 1\nline 2"
        );
        assert_eq2!(
            buffer.get_as_string_with_separator(" | ").as_str(),
            "line 1 | line 2"
        );

        // Viewport and caret getters.
        assert_eq2!(buffer.get_vp_origin(), CPos::default());
        assert_eq2!(buffer.get_vp_caret(), vp_caret(vp_pos(0, 0)));

        // Selection mutation.
        assert!(!buffer.has_selection());
        buffer.mutate_selection(|sel| {
            sel.insert((c_row(0), c_col(0)..c_col(4)));
        });
        assert!(buffer.has_selection());

        // Direct component accessors.
        assert_eq2!(
            buffer.get_content().get_lines().get_line_count(),
            c_height(2)
        );
        assert_eq2!(
            buffer.get_content_mut().get_lines().get_line_count(),
            c_height(2)
        );
        assert!(buffer.get_render_cache().entry.is_none());
        assert!(buffer.get_render_cache_mut().entry.is_none());
        assert!(buffer.get_memory_size_calc_cache().get_cached().is_some());
        assert!(
            buffer
                .get_memory_size_calc_cache_mut()
                .get_cached()
                .is_some()
        );
        assert_eq2!(buffer.get_history_mut().versions.len(), c_len(0));
    }
}
