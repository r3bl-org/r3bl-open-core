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

use std::{collections::HashMap,
          fmt::{Debug, Formatter, Result}};

use get_size::GetSize;
use r3bl_rs_utils_core::*;
use serde::*;

use crate::*;

/// Stores the data for a single editor buffer. Please do not construct this struct
/// directly and use [new_empty](EditorBuffer::new_empty) instead.
///
/// 1. This struct is stored in the app's state.
/// 2. And it is paired w/ [EditorEngine] at runtime; which is responsible for rendering
///    it to TUI, and handling user input.
///
/// # Modifying the buffer
///
/// [InputEvent] is converted into an [EditorEvent] (by
/// [EditorEngineApi]::[apply_event](EditorEngineApi::apply_event)), which is then used to
/// modify the [EditorBuffer] via:
/// 1. [EditorEvent::apply_editor_event](EditorEvent::apply_editor_event)
/// 2. [EditorEvent::apply_editor_events](EditorEvent::apply_editor_events)
///
/// In order for the commands to be executed, the functions in [EditorEngineInternalApi]
/// are used.
///
/// These functions take any one of the following args:
/// 1. [EditorArgsMut]
/// 2. [EditorArgs]
/// 3. [EditorBuffer] and [EditorEngine]
///
/// # Accessing and mutating the fields (w/ validation)
///
/// All the fields in this struct are private. In order to access them you have to use the
/// accessor associated functions. To mutate them, you have to use the
/// [get_mut](EditorBuffer::get_mut) method, which returns a tuple w/ mutable references
/// to the fields. This rather strange design allows for all mutations to be tracked
/// easily and allows for validation operations to be applied post mutation (by
/// [validate_editor_buffer_change::apply_change]).
///
/// # Different kinds of caret positions
///
/// There are two variants for the caret position value:
/// 1. [CaretKind::Raw] - this is the position of the caret (unadjusted for scroll_offset)
///    and this represents the position of the caret in the viewport.
/// 2. [CaretKind::ScrollAdjusted] - this is the position of the caret (adjusted for
///    scroll_offset) and represents the position of the caret in the buffer (not the
///    viewport).
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
/// ## `caret_display_position`
///
/// This is the "display" (or `display_col_index`) and not "logical" (or `logical_index`)
/// position (both are defined in [tui_core::graphemes]). Please take a look at
/// [tui_core::graphemes::UnicodeString], specifically the methods in
/// [tui_core::graphemes::access] for more details on how the conversion between "display"
/// and "logical" indices is done.
///
/// 1. It represents the current caret position (relative to the
///    [style_adjusted_origin_pos](FlexBox::style_adjusted_origin_pos) of the enclosing
///    [FlexBox]).
/// 2. It works w/ [crate::RenderOp::MoveCursorPositionRelTo] as well.
///
/// > 💡 For the diagrams below, the caret is where `▴` and `▸` intersects.
///
/// Start of line:
/// ```text
/// R ┌──────────┐
/// 0 ▸abcab     │
///   └▴─────────┘
///   C0123456789
/// ```
///
/// Middle of line:
/// ```text
/// R ┌──────────┐
/// 0 ▸abcab     │
///   └───▴──────┘
///   C0123456789
/// ```
///
/// End of line:
/// ```text
/// R ┌──────────┐
/// 0 ▸abcab     │
///   └─────▴────┘
///   C0123456789
/// ```
///
/// ## `scroll_offset`
///
/// The col and row offset for scrolling if active. This is not marked pub in order to
/// guard mutation. In order to access it, use [get_mut](EditorBuffer::get_mut).
///
/// ### Vertical scrolling and viewport
///
/// ```text
///                    +0--------------------+
///                    0                     |
///                    |        above        | <- caret_row_adj
///                    |                     |
///                    +--- scroll_offset ---+
///              ->    |         ↑           |      ↑
///              |     |                     |      |
///   caret.row  |     |      within vp      |  vp height
///              |     |                     |      |
///              ->    |         ↓           |      ↓
///                    +--- scroll_offset ---+
///                    |    + vp height      |
///                    |                     |
///                    |        below        | <- caret_row_adj
///                    |                     |
///                    +---------------------+
/// ```
///
/// ### Horizontal scrolling and viewport
///
/// ```text
///           <-   vp width   ->
/// +0--------+----------------+---------->
/// 0         |                |
/// | left of |<-  within vp ->| right of
/// |         |                |
/// +---------+----------------+---------->
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
/// The [SelectionMap] is used to keep track of the selections in the buffer. Each entry
/// in the map represents a row of text in the buffer.
/// - The row index is the key.
/// - The value is the [SelectionRange].
#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize, Default)]
pub struct EditorBuffer {
    pub editor_content: EditorContent,
    pub history: EditorBufferHistory,
    pub render_cache: HashMap<String, RenderOps>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize, Default)]
pub struct EditorContent {
    pub lines: Vec<UnicodeString>,
    pub caret_display_position: Position,
    pub scroll_offset: ScrollOffset,
    pub maybe_file_extension: Option<String>,
    pub maybe_file_path: Option<String>,
    pub selection_map: SelectionMap,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct EditorBufferHistory {
    versions: Vec<EditorContent>,
    current_index: isize,
}

impl Default for EditorBufferHistory {
    fn default() -> Self {
        Self {
            versions: vec![],
            current_index: -1,
        }
    }
}

pub mod history {
    use super::*;

    pub fn convert_isize_to_usize(index: isize) -> usize {
        index.try_into().unwrap_or(index as usize)
    }

    pub fn clear(editor_buffer: &mut EditorBuffer) {
        editor_buffer.history = EditorBufferHistory::default();
    }

    pub fn push(editor_buffer: &mut EditorBuffer) {
        // Invalidate the content cache, since the content just changed.
        cache::clear(editor_buffer);

        let content_copy = editor_buffer.editor_content.clone();

        // Delete the history from the current version index to the end.
        if let Some(current_index) = editor_buffer.history.get_current_index() {
            editor_buffer
                .history
                .versions
                .truncate(convert_isize_to_usize(current_index + 1));
        }

        // Normal history insertion.
        editor_buffer.history.push_content(content_copy);

        if DEBUG_TUI_COPY_PASTE {
            log_debug(format!(
                "🍎🍎🍎 add_content_to_undo_stack editor_buffer: {:?}",
                editor_buffer
            ));
        }
    }

    pub fn undo(editor_buffer: &mut EditorBuffer) {
        // Invalidate the content cache, since the content just changed.
        cache::clear(editor_buffer);

        let retain_caret_position = editor_buffer.editor_content.caret_display_position;
        if let Some(content) = editor_buffer.history.previous_content() {
            editor_buffer.editor_content = content;
            editor_buffer.editor_content.caret_display_position = retain_caret_position;
        }

        if DEBUG_TUI_COPY_PASTE {
            log_debug(format!("🍎🍎🍎 undo editor_buffer: {:?}", editor_buffer));
        }
    }

    pub fn redo(editor_buffer: &mut EditorBuffer) {
        // Invalidate the content cache, since the content just changed.
        cache::clear(editor_buffer);

        if let Some(content) = editor_buffer.history.next_content() {
            editor_buffer.editor_content = content;
        }

        if DEBUG_TUI_COPY_PASTE {
            log_debug(format!("🍎🍎🍎 redo editor_buffer: {:?}", editor_buffer));
        }
    }

    impl EditorBufferHistory {
        pub(crate) fn is_empty(&self) -> bool { self.versions.is_empty() }

        fn get_last_index(&self) -> Option<ChUnit> {
            if self.is_empty() {
                None
            } else {
                Some(ch!(self.versions.len()) - 1)
            }
        }

        fn get_current_index(&self) -> Option<isize> {
            if self.is_empty() {
                None
            } else {
                Some(self.current_index)
            }
        }

        fn increment_index(&mut self) {
            // Don't do anything if is empty.
            if let Some(max_index) = self.get_last_index() {
                // Make sure it doesn't go past the end.
                if self.current_index == ch!(@to_isize max_index) {
                    return;
                }
                // Increment index.
                self.current_index += 1;
            }
        }

        fn decrement_index(&mut self) {
            // Don't do anything if is empty.
            if let Some(current_index) = self.get_current_index() {
                // Make sure it doesn't go past the start.
                if current_index == 0 {
                    return;
                }
                // Decrement index.
                self.current_index -= 1;
            }
        }

        fn push_content(&mut self, content: EditorContent) {
            self.versions.push(content);
            self.increment_index();
        }

        fn previous_content(&mut self) -> Option<EditorContent> {
            if self.is_empty() {
                None
            } else {
                // At start of history.
                if self.current_index == -1 {
                    return None;
                }

                // Decrement index.
                self.decrement_index();

                // Return item at index.
                let it = self
                    .versions
                    .get(convert_isize_to_usize(self.current_index))
                    .cloned();
                it
            }
        }

        fn next_content(&mut self) -> Option<EditorContent> {
            if self.is_empty() {
                None
            } else {
                // At end of versions.
                if let Some(max_index) = self.get_last_index() {
                    let max_index = ch!(@to_isize max_index);
                    if self.current_index == max_index {
                        return None;
                    }
                }

                // Increment index.
                self.increment_index();

                // Return item at index.
                self.versions
                    .get(convert_isize_to_usize(self.current_index))
                    .cloned()
            }
        }
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn test_push_default() {
        let mut editor_buffer = EditorBuffer::default();
        let content = editor_buffer.editor_content.clone();

        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        let history_stack = editor_buffer.history.versions;
        assert_eq2!(history_stack.len(), 1);
        assert_eq2!(history_stack[0], content);
    }

    #[test]
    fn test_push_with_contents() {
        let mut editor_buffer = EditorBuffer::default();
        editor_buffer.editor_content.lines = vec![UnicodeString::from("abc")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        let history_stack = editor_buffer.history.versions;
        assert_eq2!(history_stack.len(), 1);
        assert_eq2!(history_stack[0].lines.len(), 1);
        assert_eq2!(history_stack[0].lines[0].string, "abc");
    }

    #[test]
    fn test_push_and_drop_future_redos() {
        let mut editor_buffer = EditorBuffer::default();
        editor_buffer.editor_content.lines = vec![UnicodeString::from("abc")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        editor_buffer.editor_content.lines = vec![UnicodeString::from("def")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 1);

        editor_buffer.editor_content.lines = vec![UnicodeString::from("ghi")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 2);

        // Do two undos.
        history::undo(&mut editor_buffer);
        history::undo(&mut editor_buffer);

        // Push new content. Should drop future redos.
        editor_buffer.editor_content.lines = vec![UnicodeString::from("xyz")];
        history::push(&mut editor_buffer);

        let history = editor_buffer.history;
        assert_eq2!(history.current_index, 1);

        let history_stack = history.versions;
        assert_eq2!(history_stack.len(), 2);
        assert_eq2!(history_stack[0].lines.len(), 1);
        assert_eq2!(history_stack[0].lines[0].string, "abc");
        assert_eq2!(history_stack[1].lines.len(), 1);
        assert_eq2!(history_stack[1].lines[0].string, "xyz");
    }

    #[test]
    fn test_single_undo() {
        let mut editor_buffer = EditorBuffer::default();
        editor_buffer.editor_content.lines = vec![UnicodeString::from("abc")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        // Undo.
        history::undo(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);
    }

    #[test]
    fn test_many_undo() {
        let mut editor_buffer = EditorBuffer::default();
        editor_buffer.editor_content.lines = vec![UnicodeString::from("abc")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        editor_buffer.editor_content.lines = vec![UnicodeString::from("def")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 1);
        let copy_of_editor_content = editor_buffer.editor_content.clone();

        editor_buffer.editor_content.lines = vec![UnicodeString::from("ghi")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 2);

        // Undo.
        history::undo(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 1);
        assert_eq2!(editor_buffer.editor_content, copy_of_editor_content);

        let history_stack = editor_buffer.history.versions;
        assert_eq2!(history_stack.len(), 3);
        assert_eq2!(history_stack[0].lines.len(), 1);
        assert_eq2!(history_stack[0].lines[0].string, "abc");
        assert_eq2!(history_stack[1].lines.len(), 1);
        assert_eq2!(history_stack[1].lines[0].string, "def");
        assert_eq2!(history_stack[2].lines.len(), 1);
        assert_eq2!(history_stack[2].lines[0].string, "ghi");
    }

    #[test]
    fn test_multiple_undos() {
        let mut editor_buffer = EditorBuffer::default();
        editor_buffer.editor_content.lines = vec![UnicodeString::from("abc")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        editor_buffer.editor_content.lines = vec![UnicodeString::from("def")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 1);

        // Undo multiple times.
        history::undo(&mut editor_buffer);
        history::undo(&mut editor_buffer);
        history::undo(&mut editor_buffer);

        assert_eq2!(editor_buffer.history.current_index, 0);
    }

    #[test]
    fn test_undo_and_multiple_redos() {
        let mut editor_buffer = EditorBuffer::default();
        editor_buffer.editor_content.lines = vec![UnicodeString::from("abc")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        editor_buffer.editor_content.lines = vec![UnicodeString::from("def")];
        history::push(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 1);
        let snapshot_content = editor_buffer.editor_content.clone();

        // Undo.
        history::undo(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 0);

        // Redo.
        history::redo(&mut editor_buffer);
        assert_eq2!(editor_buffer.history.current_index, 1);

        // Current state.
        assert_eq2!(editor_buffer.editor_content, snapshot_content);

        // Redo.
        history::redo(&mut editor_buffer);

        let history_stack = editor_buffer.history.versions;
        assert_eq2!(history_stack.len(), 2);
        assert_eq2!(history_stack[0].lines.len(), 1);
        assert_eq2!(history_stack[0].lines[0].string, "abc");
        assert_eq2!(history_stack[1].lines.len(), 1);
        assert_eq2!(history_stack[1].lines[0].string, "def");
    }
}

mod constructor {
    use super::*;

    impl EditorBuffer {
        /// Marker method to make it easy to search for where an empty instance is created.
        pub fn new_empty(
            maybe_file_extension: &Option<String>,
            maybe_file_path: &Option<String>,
        ) -> Self {
            // Potentially do any other initialization here.
            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!(
                    "🪙 {}",
                    "construct EditorBuffer { lines, caret, lolcat, file_extension }"
                );
                log_debug(msg);
            });

            Self {
                editor_content: EditorContent {
                    lines: vec![UnicodeString::default()],
                    maybe_file_extension: maybe_file_extension.clone(),
                    maybe_file_path: maybe_file_path.clone(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }
    }
}

pub mod cache {
    use super::*;

    pub fn clear(editor_buffer: &mut EditorBuffer) { editor_buffer.render_cache.clear(); }

    /// Cache key is combination of scroll_offset and window_size.
    fn generate_key(editor_buffer: &EditorBuffer, window_size: Size) -> String {
        format!("{}{}", editor_buffer.get_scroll_offset(), window_size,)
    }

    /// Render the content of the editor buffer to the screen from the cache if the content
    /// has not been modified.
    ///
    /// The cache miss occurs if
    /// - Scroll Offset changes
    /// - Window size changes
    /// - Content of the editor changes
    pub fn render_content(
        editor_buffer: &mut EditorBuffer,
        editor_engine: &mut EditorEngine,
        window_size: Size,
        has_focus: &mut HasFocus,
        render_ops: &mut RenderOps,
    ) {
        let key = generate_key(editor_buffer, window_size);
        if let Some(cached_output) = editor_buffer.render_cache.get(&key) {
            // Cache hit
            *render_ops = cached_output.clone();
            return;
        }

        // Cache miss, due to either:
        // - Content has been modified.
        // - Scroll Offset or Window size has been modified.
        editor_buffer.render_cache.clear();
        let render_args = RenderArgs {
            editor_engine,
            editor_buffer,
            has_focus,
        };

        // Re-render content, generate & write to render_ops.
        EditorEngineApi::render_content(&render_args, render_ops);

        // Snapshot the render_ops in the cache.
        editor_buffer.render_cache.insert(key, render_ops.clone());
    }
}

pub enum CaretKind {
    Raw,
    ScrollAdjusted,
}

pub mod access_and_mutate {
    use super::*;

    impl EditorBuffer {
        pub fn is_file_extension_default(&self) -> bool {
            match self.editor_content.maybe_file_extension {
                Some(ref ext) => ext == DEFAULT_SYN_HI_FILE_EXT,
                None => false,
            }
        }

        pub fn has_file_extension(&self) -> bool {
            self.editor_content.maybe_file_extension.is_some()
        }

        pub fn get_maybe_file_extension(&self) -> Option<&str> {
            match self.editor_content.maybe_file_extension {
                Some(ref s) => Some(s.as_str()),
                None => None,
            }
        }

        pub fn is_empty(&self) -> bool { self.editor_content.lines.is_empty() }

        pub fn len(&self) -> ChUnit { ch!(self.editor_content.lines.len()) }

        pub fn get_line_display_width(&self, row_index: ChUnit) -> ChUnit {
            if let Some(line) = self.editor_content.lines.get(ch!(@to_usize row_index)) {
                ch!(line.display_width)
            } else {
                ch!(0)
            }
        }

        pub fn get_lines(&self) -> &Vec<UnicodeString> { &self.editor_content.lines }

        pub fn get_as_string_with_comma_instead_of_newlines(&self) -> String {
            self.get_lines()
                .iter()
                .map(|it| it.string.clone())
                .collect::<Vec<String>>()
                .join(", ")
        }

        pub fn get_as_string_with_newlines(&self) -> String {
            self.get_lines()
                .iter()
                .map(|it| it.string.clone())
                .collect::<Vec<String>>()
                .join("\n")
        }

        pub fn set_lines(&mut self, lines: Vec<String>) {
            // Set lines.
            self.editor_content.lines =
                lines.into_iter().map(UnicodeString::from).collect();

            // Reset caret.
            self.editor_content.caret_display_position = Position::default();

            // Reset scroll_offset.
            self.editor_content.scroll_offset = ScrollOffset::default();

            // Empty the content render cache.
            cache::clear(self);

            // Reset undo/redo history.
            history::clear(self);
        }

        /// Returns the current caret position in two variants:
        /// 1. [CaretKind::Raw] -> The raw caret position not adjusted for scrolling.
        /// 2. [CaretKind::ScrollAdjusted] -> The caret position adjusted for scrolling using
        ///    scroll_offset.
        pub fn get_caret(&self, kind: CaretKind) -> Position {
            match kind {
                CaretKind::Raw => self.editor_content.caret_display_position,
                CaretKind::ScrollAdjusted => {
                    position! {
                      col_index: Self::calc_scroll_adj_caret_col(&self.editor_content.caret_display_position, &self.editor_content.scroll_offset),
                      row_index: Self::calc_scroll_adj_caret_row(&self.editor_content.caret_display_position, &self.editor_content.scroll_offset)
                    }
                }
            }
        }

        /// Scroll adjusted caret row = caret.row + scroll_offset.row.
        pub fn calc_scroll_adj_caret_row(
            caret: &Position,
            scroll_offset: &ScrollOffset,
        ) -> usize {
            ch!(@to_usize caret.row_index + scroll_offset.row_index)
        }

        /// Scroll adjusted caret col = caret.col + scroll_offset.col.
        pub fn calc_scroll_adj_caret_col(
            caret: &Position,
            scroll_offset: &ScrollOffset,
        ) -> usize {
            ch!(@to_usize caret.col_index + scroll_offset.col_index)
        }

        pub fn get_scroll_offset(&self) -> ScrollOffset {
            self.editor_content.scroll_offset
        }

        /// Returns:
        /// 1. /* lines */ &mut `Vec<UnicodeString>`,
        /// 2. /* caret */ &mut Position,
        /// 3. /* scroll_offset */ &mut ScrollOffset,
        ///
        /// Even though this struct is mutable by editor_ops.rs, this method is provided
        /// to mark when mutable access is made to this struct. This makes it easy to
        /// determine what code mutates this struct, since it is necessary to validate
        /// things after mutation quite a bit in editor_ops.rs.
        pub fn get_mut(
            &mut self,
        ) -> (
            /* lines */ &mut Vec<UnicodeString>,
            /* caret */ &mut Position,
            /* scroll_offset */ &mut ScrollOffset,
            /* selection_map */ &mut SelectionMap,
        ) {
            (
                &mut self.editor_content.lines,
                &mut self.editor_content.caret_display_position,
                &mut self.editor_content.scroll_offset,
                &mut self.editor_content.selection_map,
            )
        }

        pub fn has_selection(&self) -> bool {
            !self.editor_content.selection_map.is_empty()
        }

        pub fn clear_selection(&mut self) { self.editor_content.selection_map.clear(); }

        pub fn get_selection_map(&self) -> &SelectionMap {
            &self.editor_content.selection_map
        }
    }
}

pub mod debug_format_helpers {
    use super::*;

    impl Debug for EditorBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
                "\nEditorBuffer [                                    \n \
                ├ content: {0:?}                                     \n \
                └ history: {1:?}                                     \n \
                ]",
                /* 0 */ self.editor_content,
                /* 1 */ self.history,
            }
        }
    }

    impl Debug for EditorContent {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
                "\n\tEditorContent [                                 \n \
                \t├ lines: {0}, size: {1}                            \n \
                \t├ selection_map: {4}                               \n \
                \t└ ext: {2:?}, path:{6:?}, caret: {3:?}, scroll_offset: {5:?}   \n \
                \t]",
                /* 0 */ self.lines.len(),
                /* 1 */ self.lines.get_heap_size(),
                /* 2 */ self.maybe_file_extension,
                /* 3 */ self.caret_display_position,
                /* 4 */ self.selection_map.to_formatted_string(),
                /* 5 */ self.scroll_offset,
                /* 6 */ self.maybe_file_path,
            }
        }
    }

    impl Debug for EditorBufferHistory {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! {
                f,
                "\n\tEditorBufferHistory [                           \n \
                \t├ stack: {0}, size: {1}                            \n \
                \t└ index: {2}                                       \n \
                \t]",
                /* 0 */ self.versions.len(),
                /* 1 */ self.versions.get_heap_size(),
                /* 2 */ self.current_index
            }
        }
    }
}
