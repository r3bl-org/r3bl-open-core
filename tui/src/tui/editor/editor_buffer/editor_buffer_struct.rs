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
/// 1. This struct is stored in the [r3bl_redux::Store]'s
///    [state](r3bl_redux::Store::state) field.
/// 2. And it is paired w/ [EditorEngine] at runtime; which is responsible for rendering
///    it to TUI, and handling user input.
///
/// # Modifying the buffer
///
/// [InputEvent] is coverted into an [EditorEvent] (by
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
/// ## `maybe_selection`
/// TODO: add docs here
#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct EditorBuffer {
    lines: Vec<UnicodeString>,
    caret_display_position: Position,
    scroll_offset: ScrollOffset,
    maybe_file_extension: Option<String>,
    selection_map: SelectionMap,
}

mod selection {
    use super::*;

    pub type RowIndex = ChUnit;
    /// Key is the row index, value is the selected range in that line (display col index
    /// range).
    ///
    /// Note that both column indices are [Scroll adjusted](CaretKind::ScrollAdjusted) and
    /// not [raw](CaretKind::Raw)).
    pub type SelectionMap = HashMap<RowIndex, SelectedRangeInLine>;

    /// Note that both column indices are [Scroll adjusted](CaretKind::ScrollAdjusted) and
    /// not [raw](CaretKind::Raw)).
    #[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, GetSize, Copy)]
    pub struct SelectedRangeInLine {
        /// [Scroll adjusted](CaretKind::ScrollAdjusted) col index (not
        /// [raw](CaretKind::Raw)).
        pub start_display_col_index: ChUnit,
        /// [Scroll adjusted](CaretKind::ScrollAdjusted) col index (not
        /// [raw](CaretKind::Raw)).
        pub end_display_col_index: ChUnit,
    }

    pub struct EditorBufferApi;
    impl EditorBufferApi {
        pub fn update_selection_based_on_caret_movement(
            editor_buffer: &mut EditorBuffer,
            caret_previous: Position,
            caret_current: Position,
        ) {
            let movement_in_single_line =
                caret_previous.row_index == caret_current.row_index;

            let maybe_diffs = match movement_in_single_line {
                true => generate_diffs_from_single_line_caret_movement(
                    &editor_buffer.selection_map,
                    caret_previous.row_index, // Same as caret_current.row_index.
                    caret_previous.col_index,
                    caret_current.col_index,
                ),
                false => generate_diffs_from_multiline_caret_movement(),
            };

            let Some(diffs) = maybe_diffs else { return };

            // DBG: remove
            log_debug(format!("\n📦📦📦 diffs: \n{}", format!("{:#?}", diffs)));

            // Apply diffs to create new selection, or modify or remove existing selection.
            Self::apply_diffs_to_change_selection(diffs, editor_buffer);
        }

        fn apply_diffs_to_change_selection(
            diffs: SelectionChanges,
            editor_buffer: &mut EditorBuffer,
        ) -> Option<()> {
            match diffs {
                // Handle left, right, home, end.
                SelectionChanges::SingleLine(diff) => {
                    match diff {
                        // DONE: NewSelection
                        SingleLineDiff::NewSelection { row_index, range } => {
                            editor_buffer
                                .get_selection_map_mut()
                                .insert(row_index, range);
                        }

                        // DONE: RemoveSelection
                        SingleLineDiff::RemoveSelection { row_index } => {
                            editor_buffer.get_selection_map_mut().remove(&row_index);
                        }

                        // DONE: ExtendToRight
                        SingleLineDiff::ExtendToRight {
                            row_index,
                            display_column_count,
                        } => {
                            let selection_map = editor_buffer.get_selection_map_mut();
                            if let Some(range) = selection_map.get(&row_index) {
                                selection_map.insert(row_index, {
                                    let mut new_range = *range;
                                    new_range.end_display_col_index +=
                                        display_column_count;
                                    new_range
                                });
                            }
                        }

                        // TODO: gen diff ShrinkFromRight
                        SingleLineDiff::ShrinkFromRight {
                            row_index,
                            display_column_count,
                        } => {
                            todo!();
                        }

                        // TODO: gen diff ExtendToLeft
                        SingleLineDiff::ExtendToLeft {
                            row_index,
                            display_column_count: count,
                        } => {
                            todo!();
                        }

                        // TODO: gen diff ShrinkFromLeft
                        SingleLineDiff::ShrinkFromLeft {
                            row_index,
                            display_column_count: count,
                        } => {
                            todo!();
                        }
                    }
                }
                // Handle up, down, page up, page down.
                // TODO: figure out diff MultiLine support
                SelectionChanges::MultiLine(_) => {
                    todo!();
                }
            }

            None
        }
    }
}
pub use selection::*;

mod diff {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub enum SelectionChanges {
        // DONE: SingleLine
        SingleLine(SingleLineDiff),
        // TODO: figure out MultiLine
        MultiLine(MultilineDiff),
    }

    #[derive(Debug, Clone, Copy)]
    pub enum SingleLineDiff {
        NewSelection {
            row_index: RowIndex,
            range: SelectedRangeInLine,
        },
        RemoveSelection {
            row_index: RowIndex,
        },
        ExtendToRight {
            row_index: RowIndex,
            display_column_count: ChUnit,
        },
        ShrinkFromRight {
            row_index: RowIndex,
            display_column_count: ChUnit,
        },
        ExtendToLeft {
            row_index: RowIndex,
            display_column_count: ChUnit,
        },
        ShrinkFromLeft {
            row_index: RowIndex,
            display_column_count: ChUnit,
        },
    }

    #[derive(Debug, Clone, Copy)]
    pub enum MultilineDiff {}

    pub fn generate_diffs_from_single_line_caret_movement(
        selection_map: &SelectionMap,
        row_index: ChUnit,
        caret_previous_display_col_index: ChUnit,
        caret_current_display_col_index: ChUnit,
    ) -> Option<SelectionChanges> {
        match selection_map.get(&row_index) {
            // Could not find a range for row index, so create and add a new one.
            None => create_new_selection_for_single_line(
                caret_previous_display_col_index,
                caret_current_display_col_index,
                row_index,
            ),
            // Found a range for row index, so modify it.
            Some(range) => extend_or_shrink_existing_selection_for_single_line(
                range,
                caret_previous_display_col_index,
                caret_current_display_col_index,
                row_index,
            ),
        }
    }

    fn create_new_selection_for_single_line(
        caret_previous_display_col_index: ChUnit,
        caret_current_display_col_index: ChUnit,
        row_index: ChUnit,
    ) -> Option<SelectionChanges> {
        match (
            caret_previous_display_col_index,
            caret_current_display_col_index,
        ) {
            // Caret moved right.
            (previous, current) if current > previous => {
                Some(SelectionChanges::SingleLine(SingleLineDiff::NewSelection {
                    row_index,
                    range: SelectedRangeInLine {
                        start_display_col_index: previous,
                        end_display_col_index: current,
                    },
                }))
            }
            // Caret moved left.
            (previous, current) if current < previous => {
                Some(SelectionChanges::SingleLine(SingleLineDiff::NewSelection {
                    row_index,
                    range: SelectedRangeInLine {
                        start_display_col_index: current,
                        end_display_col_index: previous,
                    },
                }))
            }
            (_, _) => None,
        }
    }

    fn extend_or_shrink_existing_selection_for_single_line(
        existing_selection: &SelectedRangeInLine,
        caret_previous_display_col_index: ChUnit,
        caret_current_display_col_index: ChUnit,
        row_index: ChUnit,
    ) -> Option<SelectionChanges> {
        let SelectedRangeInLine {
            start_display_col_index: range_start,
            end_display_col_index: range_end,
        } = existing_selection;

        match (
            caret_previous_display_col_index,
            caret_current_display_col_index,
        ) {
            // Carets overlap, so remove selection.
            (previous, current) if current == previous => {
                Some(SelectionChanges::SingleLine(
                    SingleLineDiff::RemoveSelection { row_index },
                ))
            }

            // Add to right by count (ie, going right).
            (previous, current) if current > previous => {
                let count = current - *range_end;
                Some(SelectionChanges::SingleLine(
                    SingleLineDiff::ExtendToRight {
                        row_index,
                        display_column_count: count,
                    },
                ))
            }

            // TODO: apply: Remove from right by count (ie, going left).
            (previous, current) if current < previous => {
                let count = *range_end - current;
                Some(SelectionChanges::SingleLine(
                    SingleLineDiff::ShrinkFromRight {
                        row_index,
                        display_column_count: count,
                    },
                ))
            }

            // TODO: apply: Add to left by count (ie, going left).
            (previous, current) if current < previous => {
                let count = *range_start - current;
                Some(SelectionChanges::SingleLine(SingleLineDiff::ExtendToLeft {
                    row_index,
                    display_column_count: count,
                }))
            }

            // TODO: apply: Remove from left by count (ie, going right).
            (_, _) => None,
        }
    }

    // TODO: fill out all cases for detecting selection change across multiple line
    pub fn generate_diffs_from_multiline_caret_movement() -> Option<SelectionChanges> {
        None
    }
}
use diff::*;

mod constructor {
    use super::*;

    impl EditorBuffer {
        /// Marker method to make it easy to search for where an empty instance is created.
        pub fn new_empty(file_extension: Option<&str>) -> Self {
            // Potentially do any other initialization here.
            call_if_true!(DEBUG_TUI_MOD, {
                let msg = format!(
                    "🪙 {}",
                    "construct EditorBuffer { lines, caret, lolcat, file_extension }"
                );
                log_debug(msg);
            });

            Self {
                lines: vec![UnicodeString::default()],
                caret_display_position: Position::default(),
                scroll_offset: ScrollOffset::default(),
                maybe_file_extension: file_extension.map(|s| s.to_string()),
                selection_map: Default::default(),
            }
        }
    }
}

pub enum CaretKind {
    Raw,
    ScrollAdjusted,
}

pub mod access_and_mutate {
    use super::*;

    impl EditorBuffer {
        pub fn has_file_extension(&self) -> bool { self.maybe_file_extension.is_some() }

        pub fn get_maybe_file_extension(&self) -> Option<&str> {
            match self.maybe_file_extension {
                Some(ref s) => Some(s.as_str()),
                None => None,
            }
        }

        pub fn is_empty(&self) -> bool { self.lines.is_empty() }

        pub fn len(&self) -> ChUnit { ch!(self.lines.len()) }

        pub fn get_lines(&self) -> &Vec<UnicodeString> { &self.lines }

        pub fn get_as_string(&self) -> String {
            self.get_lines()
                .iter()
                .map(|l| l.string.clone())
                .collect::<Vec<String>>()
                .join(", ")
        }

        pub fn set_lines(&mut self, lines: Vec<String>) {
            // Set lines.
            self.lines = lines.into_iter().map(UnicodeString::from).collect();
            // Reset caret.
            self.caret_display_position = Position::default();
            // Reset scroll_offset.
            self.scroll_offset = ScrollOffset::default();
        }

        /// Returns the current caret position in two variants:
        /// 1. [CaretKind::Raw] -> The raw caret position not adjusted for scrolling.
        /// 2. [CaretKind::ScrollAdjusted] -> The caret position adjusted for scrolling using
        ///    scroll_offset.
        pub fn get_caret(&self, kind: CaretKind) -> Position {
            match kind {
                CaretKind::Raw => self.caret_display_position,
                CaretKind::ScrollAdjusted => {
                    position! {
                      col_index: Self::calc_scroll_adj_caret_col(&self.caret_display_position, &self.scroll_offset),
                      row_index: Self::calc_scroll_adj_caret_row(&self.caret_display_position, &self.scroll_offset)
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

        pub fn get_scroll_offset(&self) -> ScrollOffset { self.scroll_offset }

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
        ) {
            (
                &mut self.lines,
                &mut self.caret_display_position,
                &mut self.scroll_offset,
            )
        }

        pub fn get_selection_map(&self) -> &SelectionMap { &self.selection_map }

        pub fn get_selection_map_mut(&mut self) -> &mut SelectionMap {
            &mut self.selection_map
        }
    }
}

mod debug_format_helpers {
    use super::*;

    impl Debug for EditorBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            let selection_map_str = self
                .selection_map
                .iter()
                .map(|(row_index, selected_range)| {
                    format!(
                        "✂️ ┆row: {0} => start: {1}, end: {2}┆",
                        /* 0 */ row_index,
                        /* 1 */ selected_range.start_display_col_index,
                        /* 2 */ selected_range.end_display_col_index
                    )
                })
                .collect::<Vec<String>>()
                .join(", ");

            write! {
                f,
                "\nEditorBuffer [                                  \n \
                ├ lines: {0}, size: {1},                           \n \
                ├ selection_map: {4},                              \n \
                └ ext: {2:?}, caret: {3:?}, scroll_offset: {5:?}   \n \
                ]",
                /* 0 */ self.lines.len(),
                /* 1 */ self.lines.get_heap_size(),
                /* 2 */ self.maybe_file_extension,
                /* 3 */ self.caret_display_position,
                /* 4 */ selection_map_str,
                /* 5 */ self.scroll_offset
            }
        }
    }
}
