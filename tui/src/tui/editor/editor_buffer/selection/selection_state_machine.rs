// Copyright (c) 2023-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CCaret, CCol, CRow, CursorBoundsCheck, EditorBuffer, InlineVec,
            SelectionLine, c_caret, c_col, c_row, c_sel_line};

/// Represents the anchor state of the selection list when resolving `anchor_caret`.
///
/// # Deterministic Selection Engine
///
/// Selection calculation in the editor buffer is driven deterministically by an **anchor
/// caret** (`anchor_caret`) and an **active caret** (`active_caret`).
///
/// ## Overview of the Anchor Model
///
/// Modern text editors (such as VS Code and `JetBrains` IDEs) compute selections
/// deterministically using an anchor model. In contrast to stateful selection engines
/// that track movement direction history, the anchor model maintains a single reference
/// point (`anchor_caret`) representing where the selection was initiated. The active
/// caret (`active_caret`) represents the current cursor position.
///
/// ```text
/// Selection Range Calculation:
///
/// Forward selection (anchor <= active):
///   anchor_caret = (row: 0, col: 4)  ───►  start_caret = (row: 0, col: 4)
///   active_caret = (row: 2, col: 5)  ───►  end_caret   = (row: 2, col: 5)
///
/// Backward selection (active < anchor):
///   active_caret = (row: 0, col: 4)  ───►  start_caret = (row: 0, col: 4)
///   anchor_caret = (row: 2, col: 5)  ───►  end_caret   = (row: 2, col: 5)
///
/// Single-Line Selection (start_caret.row == end_caret.row):
///
///                start_caret (col: 4)                    end_caret (col: 14)
///                         ↓                                       ↓
/// Col:    0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17
///       ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Char: │ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │   │ f │ o │
///       └───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┴───┘
///                       ╰─────────── SelectionLine ─────────────╯
///                       [start_col = 4 ................. end_col = 14)
///
/// Multi-Line Selection (start_caret.row < end_caret.row):
///
///          Col: 0   1   2   3   4   5   6   7   8   9  10  11  12  13  14
///               ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///  Row 0 (start)│ T │ h │ e │   │ q │ u │ i │ c │ k │   │ b │ r │ o │ w │ n │
///               └───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┤
///                               ╰──────────── SelectionLine 0 ──────────────╯
///                                           (start_col=4 to EOL)
///
///               ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///  Row 1 (mid)  │ f │ o │ x │   │ j │ u │ m │ p │ s │   │ o │ v │ e │ r │
///               ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┤
///               ╰──────────────────── SelectionLine 1 ──────────────────╯
///                                      (entire line)
///               ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///  Row 2 (end)  │ t │ h │ e │   │ l │ a │ z │ y │   │ d │ o │ g │
///               ├───┴───┴───┴───┴───┼───┴───┴───┴───┴───┴───┴───┘
///               ╰─ SelectionLine 2 ─╯
///            (start_col 0 to end_col=5)
/// ```
///
/// This deterministic approach guarantees consistent, predictable behavior across all
/// selection operations without requiring direction-change tracking or complex state
/// transitions.
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorState {
    /// [`anchor_caret`] is already explicitly stored in [`SelectionContainer`].
    ///
    /// [`anchor_caret`]: crate::SelectionContainer::anchor_caret
    /// [`SelectionContainer`]: crate::SelectionContainer
    AlreadySet(CCaret),

    /// [`anchor_caret`] is [`None`] and selection list is empty (starting a brand new
    /// selection).
    ///
    /// [`anchor_caret`]: crate::SelectionContainer::anchor_caret
    FromNewSelection,

    /// [`anchor_caret`] is [`None`] but selections exist (inferring anchor from selection
    /// boundaries).
    ///
    /// [`anchor_caret`]: crate::SelectionContainer::anchor_caret
    FromExistingSelection {
        first: SelectionLine,
        last: SelectionLine,
    },
}

impl AnchorState {
    /// Inspects the buffer's selection list to determine the current [`AnchorState`].
    #[must_use]
    pub fn from_buffer(buffer: &EditorBuffer) -> Self {
        let sel_list = buffer.get_selection_container();
        if let Some(anchor) = sel_list.anchor_caret {
            return Self::AlreadySet(anchor);
        }

        let maybe_first_and_last_selection = sel_list.boundaries();
        match maybe_first_and_last_selection {
            Some((first, last)) => Self::FromExistingSelection {
                first: first.clone(),
                last: last.clone(),
            },
            None => Self::FromNewSelection,
        }
    }

    /// Resolves the anchor caret, updates `buffer.sel_list.anchor_caret` if needed,
    /// and returns the resolved anchor [`CCaret`].
    pub fn resolve_and_update(&self, buffer: &mut EditorBuffer, prev: CCaret) -> CCaret {
        match self {
            Self::AlreadySet(anchor) => *anchor,
            Self::FromNewSelection => {
                buffer.mutate_selection(|sel_list| sel_list.anchor_caret = Some(prev));
                prev
            }
            Self::FromExistingSelection { first, last } => {
                let inferred_anchor =
                    if prev.row_index == last.row && prev.col_index == last.get_end() {
                        c_caret(first.get_start() + first.row)
                    } else if prev.row_index == first.row
                        && prev.col_index == first.get_start()
                    {
                        c_caret(last.get_end() + last.row)
                    } else {
                        prev
                    };

                buffer.mutate_selection(|sel_list| {
                    sel_list.anchor_caret = Some(inferred_anchor);
                });
                inferred_anchor
            }
        }
    }
}

/// Primary enum representing the categorized selection span between anchor and active
/// carets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionRange {
    /// Anchor and active carets are identical (no text selected).
    Empty,

    /// Selection stays on a single line.
    SingleLine {
        row: CRow,
        start_col: CCol,
        end_col: CCol,
    },

    /// Selection spans across multiple lines.
    MultiLine {
        start_caret: CCaret,
        end_caret: CCaret,
    },
}

impl SelectionRange {
    /// Classifies `(anchor, active)` carets into a [`SelectionRange`].
    #[must_use]
    pub fn from_carets(anchor: CCaret, active: CCaret) -> Self {
        use std::cmp::Ordering::{Equal, Greater, Less};

        let anchor_vs_active_row = anchor.row_index.cmp(&active.row_index);
        match anchor_vs_active_row {
            // Anchor and active carets are on the same line.
            Equal => {
                let start_col = anchor.col_index.min(active.col_index);
                let end_col = anchor.col_index.max(active.col_index);
                if start_col == end_col {
                    Self::Empty
                } else {
                    Self::SingleLine {
                        row: anchor.row_index,
                        start_col,
                        end_col,
                    }
                }
            }
            // Anchor caret is above active caret.
            Less => Self::MultiLine {
                start_caret: anchor,
                end_caret: active,
            },
            // Anchor caret is below active caret.
            Greater => Self::MultiLine {
                start_caret: active,
                end_caret: anchor,
            },
        }
    }

    /// Generates line selections ([`InlineVec<SelectionLine>`]) for this selection range
    /// based on line display widths in the buffer.
    ///
    /// - **[`SelectionRange::Empty`]**: Generates 0 selection lines.
    /// - **[`SelectionRange::SingleLine`]**: Generates 1 [`SelectionLine`] for the active
    ///   row.
    /// - **[`SelectionRange::MultiLine`]**: Generates [`SelectionLine`]s spanning across
    ///   all rows between `start_caret` and `end_caret`:
    ///   1. `start_row`: from `start_col` to the line's end of line (EOL).
    ///   2. Intermediate rows: from column 0 to the row's EOL.
    ///   3. `end_row`: from column 0 to `end_col`.
    ///
    /// # Arguments
    /// * `buffer` - Reference to the [`EditorBuffer`] providing line display widths.
    ///
    /// # Returns
    /// An [`InlineVec<SelectionLine>`] containing the computed selection lines.
    ///
    /// [`EditorBuffer`]: crate::EditorBuffer
    /// [`InlineVec<SelectionLine>`]: crate::InlineVec
    /// [`SelectionLine`]: crate::SelectionLine
    #[must_use]
    pub fn compute_line_selections(
        &self,
        buffer: &EditorBuffer,
    ) -> InlineVec<SelectionLine> {
        let mut new_selections = InlineVec::new();

        match *self {
            Self::Empty => {}
            Self::SingleLine {
                row,
                start_col,
                end_col,
            } => {
                if start_col < end_col {
                    new_selections.push(c_sel_line(row, start_col..end_col));
                }
            }
            Self::MultiLine {
                start_caret,
                end_caret,
            } => {
                let start_row_idx = start_caret.row_index.as_usize();
                let end_row_idx = end_caret.row_index.as_usize();

                for row_index in start_row_idx..=end_row_idx {
                    let row = c_row(row_index);
                    if row == start_caret.row_index {
                        let start_col = start_caret.col_index;
                        let line_len = buffer
                            .get_lines()
                            .get_line_display_width(row)
                            .unwrap_or_default()
                            .eol_cursor_position();
                        new_selections.push(c_sel_line(row, start_col..line_len));
                    } else if row == end_caret.row_index {
                        let start_col = c_col(0);
                        let end_col = end_caret.col_index;
                        new_selections.push(c_sel_line(row, start_col..end_col));
                    } else {
                        let start_col = c_col(0);
                        let line_len = buffer
                            .get_lines()
                            .get_line_display_width(row)
                            .unwrap_or_default()
                            .eol_cursor_position();
                        new_selections.push(c_sel_line(row, start_col..line_len));
                    }
                }
            }
        }

        new_selections
    }
}

/// Core deterministic algorithm that calculates and updates all [`SelectionLine`] ranges
/// in `buffer.selection` based on `(anchor_caret, active_caret)`.
///
/// If `buffer.selection.anchor_caret` is [`None`], it is initialized using `prev` (or
/// inferred from an existing selection). `curr` becomes the active caret position.
pub fn update_selection_from_anchor_and_active_carets(
    buffer: &mut EditorBuffer,
    prev: CCaret,
    curr: CCaret,
) {
    let anchor = AnchorState::from_buffer(buffer).resolve_and_update(buffer, prev);
    let active = curr;

    let range = SelectionRange::from_carets(anchor, active);
    let new_selections = range.compute_line_selections(buffer);

    buffer.mutate_selection(|sel_list| {
        sel_list.list = new_selections;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_anchor_state_classification() {
        let mut buffer = EditorBuffer::new_empty(());
        let prev = c_caret(c_col(0) + c_row(0));

        // Unset and empty list -> FromNewSelection
        let state = AnchorState::from_buffer(&buffer);
        assert_eq2!(state, AnchorState::FromNewSelection);

        // Resolve anchor -> returns prev and stores it in buffer
        let resolved = state.resolve_and_update(&mut buffer, prev);
        assert_eq2!(resolved, prev);
        assert_eq2!(buffer.get_selection_container().anchor_caret, Some(prev));

        // Anchor is now set -> AlreadySet
        let state2 = AnchorState::from_buffer(&buffer);
        assert_eq2!(state2, AnchorState::AlreadySet(prev));
    }

    #[test]
    fn test_anchor_state_from_existing_selection() {
        let mut buffer = EditorBuffer::new_empty(());
        let first_sel = c_sel_line(c_row(0), c_col(2)..c_col(8));
        let last_sel = c_sel_line(c_row(2), c_col(0)..c_col(5));

        buffer.mutate_selection(|sel_list| {
            sel_list.insert(first_sel.clone());
            sel_list.insert(last_sel.clone());
            sel_list.anchor_caret = None;
        });

        // Anchor caret is None & list is non-empty -> FromExistingSelection
        let state = AnchorState::from_buffer(&buffer);
        assert_eq2!(
            state,
            AnchorState::FromExistingSelection {
                first: first_sel,
                last: last_sel,
            }
        );

        // Prev is at end of last selection -> inferred anchor is head of first selection
        let prev_at_end_of_last = c_caret(c_col(5) + c_row(2));
        let inferred = state.resolve_and_update(&mut buffer, prev_at_end_of_last);
        assert_eq2!(inferred, c_caret(c_col(2) + c_row(0)));
    }

    #[test]
    fn test_selection_range_from_carets() {
        let c1 = c_caret(c_col(2) + c_row(0));
        let c2 = c_caret(c_col(8) + c_row(0));
        let c3 = c_caret(c_col(4) + c_row(2));

        // Equal carets -> Empty
        assert_eq2!(SelectionRange::from_carets(c1, c1), SelectionRange::Empty);

        // Single line forward
        assert_eq2!(
            SelectionRange::from_carets(c1, c2),
            SelectionRange::SingleLine {
                row: c_row(0),
                start_col: c_col(2),
                end_col: c_col(8),
            }
        );

        // Single line backward (result is normalized)
        assert_eq2!(
            SelectionRange::from_carets(c2, c1),
            SelectionRange::SingleLine {
                row: c_row(0),
                start_col: c_col(2),
                end_col: c_col(8),
            }
        );

        // Multi-line selection
        assert_eq2!(
            SelectionRange::from_carets(c1, c3),
            SelectionRange::MultiLine {
                start_caret: c1,
                end_caret: c3,
            }
        );
    }

    #[test]
    fn test_anchor_state_from_buffer() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Line 0", "Line 1", "Line 2"]);

        // 1. Empty buffer -> FromNewSelection.
        assert_eq2!(
            AnchorState::from_buffer(&buffer),
            AnchorState::FromNewSelection
        );

        // 2. AlreadySet when anchor_caret is present.
        let anchor = c_caret(c_col(2) + c_row(0));
        buffer.mutate_selection(|sel_list| {
            sel_list.anchor_caret = Some(anchor);
        });
        assert_eq2!(
            AnchorState::from_buffer(&buffer),
            AnchorState::AlreadySet(anchor)
        );

        // 3. FromExistingSelection with 1 line when anchor_caret is None.
        let sel0 = c_sel_line(c_row(0), c_col(2)..c_col(5));
        buffer.mutate_selection(|sel_list| {
            sel_list.anchor_caret = None;
            sel_list.insert(sel0.clone());
        });
        assert_eq2!(
            AnchorState::from_buffer(&buffer),
            AnchorState::FromExistingSelection {
                first: sel0.clone(),
                last: sel0.clone(),
            }
        );

        // 4. FromExistingSelection with multiple lines when anchor_caret is None.
        let sel2 = c_sel_line(c_row(2), c_col(0)..c_col(4));
        buffer.mutate_selection(|sel_list| {
            sel_list.insert(sel2.clone());
        });
        assert_eq2!(
            AnchorState::from_buffer(&buffer),
            AnchorState::FromExistingSelection {
                first: sel0,
                last: sel2,
            }
        );
    }

    #[test]
    fn test_update_selection_from_anchor_and_active_new_range() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Hello world"]);

        // Simulate selection from col 0 to col 5 on row 0.
        let prev = c_caret(c_col(0) + c_row(0));
        let curr = c_caret(c_col(5) + c_row(0));

        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        // Verify selection was created.
        let selection = buffer.get_selection_container().get(c_row(0));
        assert!(selection.is_some());
        let range = selection.expect("conversion error");
        assert_eq2!(range.get_start(), c_col(0));
        assert_eq2!(range.get_end(), c_col(5));
    }

    #[test]
    fn test_update_selection_from_anchor_and_active_extend_range() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Hello world example"]);

        // First create a selection from col 0 to col 5.
        let prev1 = c_caret(c_col(0) + c_row(0));
        let curr1 = c_caret(c_col(5) + c_row(0));
        update_selection_from_anchor_and_active_carets(&mut buffer, prev1, curr1);

        // Then extend it to col 10.
        let prev2 = c_caret(c_col(5) + c_row(0));
        let curr2 = c_caret(c_col(10) + c_row(0));
        update_selection_from_anchor_and_active_carets(&mut buffer, prev2, curr2);

        // Verify selection was extended.
        let selection = buffer.get_selection_container().get(c_row(0));
        assert!(selection.is_some());
        let range = selection.expect("conversion error");
        assert_eq2!(range.get_start(), c_col(0));
        assert_eq2!(range.get_end(), c_col(10));
    }

    #[test]
    fn test_handle_selection_single_line_shrink_range() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Hello world example"]);

        // Create a selection from col 0 to col 10.
        buffer.mutate_selection(|sel_list| {
            sel_list.insert((c_row(0), c_col(0)..c_col(10)));
        });

        // Now shrink it by moving left from col 10 to col 5.
        let prev = c_caret(c_col(10) + c_row(0));
        let curr = c_caret(c_col(5) + c_row(0));
        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        // Verify selection was shrunk.
        let selection = buffer.get_selection_container().get(c_row(0));
        assert!(selection.is_some());
        let range = selection.expect("conversion error");
        assert_eq2!(range.get_start(), c_col(0));
        assert_eq2!(range.get_end(), c_col(5));
    }

    #[test]
    fn test_update_selection_from_anchor_and_active_down() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);

        // Simulate selection from row 0 col 2 to row 2 col 3.
        let prev = c_caret(c_col(2) + c_row(0));
        let curr = c_caret(c_col(3) + c_row(2));

        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        // Verify selections were created for all rows.
        assert!(buffer.get_selection_container().get(c_row(0)).is_some());
        assert!(buffer.get_selection_container().get(c_row(1)).is_some());
        assert!(buffer.get_selection_container().get(c_row(2)).is_some());

        // Check first row selection (from col 2 to end)
        let first_range = buffer
            .get_selection_container()
            .get(c_row(0))
            .expect("conversion error");
        assert_eq2!(first_range.get_start(), c_col(2));
        assert_eq2!(first_range.get_end(), c_col(6)); // "Line 1" has 6 chars

        // Check middle row selection (full line)
        let middle_range = buffer
            .get_selection_container()
            .get(c_row(1))
            .expect("conversion error");
        assert_eq2!(middle_range.get_start(), c_col(0));
        assert_eq2!(middle_range.get_end(), c_col(6)); // "Line 2" has 6 chars

        // Check last row selection (from start to col 3)
        let last_range = buffer
            .get_selection_container()
            .get(c_row(2))
            .expect("conversion error");
        assert_eq2!(last_range.get_start(), c_col(0));
        assert_eq2!(last_range.get_end(), c_col(3));
    }

    #[test]
    fn test_update_selection_from_anchor_and_active_up() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3"]);

        // Simulate selection from row 2 col 3 to row 0 col 2 (upward)
        let prev = c_caret(c_col(3) + c_row(2));
        let curr = c_caret(c_col(2) + c_row(0));

        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        // Verify selections were created for all rows.
        assert!(buffer.get_selection_container().get(c_row(0)).is_some());
        assert!(buffer.get_selection_container().get(c_row(1)).is_some());
        assert!(buffer.get_selection_container().get(c_row(2)).is_some());
    }

    #[test]
    fn test_handle_selection_hit_top_or_bottom_same_c_row() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["First line", "Second line", "Third line"]);

        // First create a selection on row 1.
        buffer.mutate_selection(|sel_list| {
            sel_list.insert((c_row(1), c_col(2)..c_col(8)));
        });

        // Now test movement on same row
        let prev = c_caret(c_col(8) + c_row(1));
        let curr = c_caret(c_col(5) + c_row(1)); // Same c_row, moving left

        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        // Verify selection was modified.
        let selection = buffer.get_selection_container().get(c_row(1));
        assert!(selection.is_some());
        let range = selection.expect("conversion error");
        assert_eq2!(range.get_start(), c_col(2));
        assert_eq2!(range.get_end(), c_col(5));
    }

    #[test]
    fn test_selection_multiline_with_direction_detection() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Line 1", "Line 2", "Line 3", "Line 4"]);

        // Test downward selection.
        let prev_down = c_caret(c_col(2) + c_row(0));
        let curr_down = c_caret(c_col(4) + c_row(2));

        update_selection_from_anchor_and_active_carets(&mut buffer, prev_down, curr_down);

        // Verify selections were created for downward movement.
        assert!(buffer.get_selection_container().get(c_row(0)).is_some());
        assert!(buffer.get_selection_container().get(c_row(1)).is_some());
        assert!(buffer.get_selection_container().get(c_row(2)).is_some());

        // Clear and test upward selection.
        buffer.clear_selection();

        let prev_up = c_caret(c_col(4) + c_row(3));
        let curr_up = c_caret(c_col(2) + c_row(1));

        update_selection_from_anchor_and_active_carets(&mut buffer, prev_up, curr_up);

        // Verify selections were created for upward movement.
        assert!(buffer.get_selection_container().get(c_row(1)).is_some());
        assert!(buffer.get_selection_container().get(c_row(2)).is_some());
        assert!(buffer.get_selection_container().get(c_row(3)).is_some());
    }

    #[test]
    fn test_clear_selection() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec!["Line 1", "Line 2"]);

        // Create some selections.
        let prev = c_caret(c_col(0) + c_row(0));
        let curr = c_caret(c_col(5) + c_row(0));
        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        assert!(!buffer.get_selection_container().is_empty());

        // Clear selections.
        buffer.clear_selection();
        assert!(buffer.get_selection_container().is_empty());
    }

    #[test]
    fn test_empty_buffer_selection() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec![""]);

        let prev = c_caret(c_col(0) + c_row(0));
        let curr = c_caret(c_col(0) + c_row(0));

        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        // Same position -> empty selection list.
        assert!(buffer.get_selection_container().is_empty());
    }

    #[test]
    fn test_selection_boundary_cases() {
        let mut buffer = EditorBuffer::new_empty(());
        buffer.init_with(vec![
            "Short",
            "A very long line with many characters",
            "End",
        ]);

        let prev = c_caret(c_col(0) + c_row(0));
        let curr = c_caret(c_col(10) + c_row(0));

        update_selection_from_anchor_and_active_carets(&mut buffer, prev, curr);

        let selection = buffer.get_selection_container().get(c_row(0));
        assert!(selection.is_some());
        let range = selection.expect("conversion error");
        assert_eq2!(range.get_start(), c_col(0));
        assert_eq2!(range.get_end(), c_col(10));
    }
}
