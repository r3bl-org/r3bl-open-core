/*
 *   Copyright (c) 2025 R3BL LLC
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

//! For more information on scrolling, take a look at the
//! [`super::scroll_editor_content::inc_caret_col_by`] docs. The functions in this module
//! need information from both [`EditorBuffer`] and [`super::EditorEngine`] in order to
//! work.
//! - [`EditorBuffer`] provides [`crate::EditorContent`].
//! - [`super::EditorEngine`] provides [`super::EditorEngine::viewport()`].

use std::cmp::Ordering;

use super::{caret_mut, SelectMode};
use crate::{caret_scroll_index,
            ch,
            col,
            height,
            row,
            width,
            BoundsCheck,
            BoundsStatus,
            CaretDirection,
            CaretRaw,
            ColIndex,
            ColWidth,
            EditorArgsMut,
            EditorBuffer,
            RowHeight,
            RowIndex,
            ScrOfs};

/// # Scrolling not active
///
/// Note that a caret is allowed to "go past" the end of its max index, so max index +
/// 1 is a valid position. This is without taking scrolling into account. The max
/// index must still be within the viewport (max index) bounds.
///
/// - Let's assume the caret is represented by "░".
/// - Think about typing "hello", and you expected the caret "░" to go past the end of the
///   string "hello░".
/// - So the caret's col index is 5 in this case. Still within viewport bounds (max
///   index). But greater than the line content max index (4).
///
/// ```text
/// R ┌──────────┐
/// 0 ▸hello░    │
///   └─────▴────┘
///   C0123456789
/// ```
///
/// # Scrolling active
///
/// When scrolling is introduced (or activated), this behavior changes a bit. The
/// caret can't be allowed to go past the viewport bounds. So the caret must be
/// adjusted to the end of the line. In this case if the text is "helloHELLOhello"
/// then the following will be displayed (the caret is at the end of the line on top
/// of the "o"). You can see this in action in the test
/// `test_editor_ops::editor_move_caret_home_end_overflow_viewport()`.
// <!-- cspell:disable -->
/// ```text
/// R ┌──────────┐
/// 0 ▸ELLOhello░│
///   └─────────▴┘
///   C0123456789
/// ```
// <!-- cspell:enable -->
///
/// And scroll offset will be adjusted to show the end of the line. So the numbers will be
/// as follows:
/// - `caret_raw`: col(9) + row(0)
/// - `scr_ofs`:   col(6) + row(0)
///
/// Once this function runs, it is necessary to run the [Drop] impl for
/// [`crate::validate_buffer_mut::EditorBufferMut`], which runs this function:
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`]. Due to the
/// nature of `UTF-8` and its variable width characters, where the memory size is not the
/// same as display size. Eg: `a` is 1 byte and 1 display width (unicode segment width
/// display). `😄` is 3 bytes but it's display width is 2! To ensure that caret position
/// and scroll offset positions are not in the middle of a unicode segment character, we
/// need to run the validation checks.
pub fn inc_caret_col_by(
    caret_raw: &mut CaretRaw,
    scr_ofs: &mut ScrOfs,
    col_amt: ColWidth,
    line_display_width: ColWidth,
    vp_width: ColWidth,
) {
    // Just move the caret right.
    caret_raw.add_col_with_bounds(col_amt, line_display_width);

    if caret_raw.col_index.check_overflows(vp_width) == BoundsStatus::Overflowed {
        // The following is equivalent to:
        // `let diff_overflow = (caret_raw.col_index + ch!(1)) - vp_width;`
        let diff_overflow = caret_raw.col_index.convert_to_width() /*+1*/ - vp_width;
        scr_ofs.col_index += diff_overflow; // Activate horiz scroll.
        caret_raw.col_index -= diff_overflow; // Shift caret.
    }
}

/// Try and leave the caret where it is, however, if the caret is out of the viewport,
/// then scroll.
///
/// Once this function runs, it is necessary to run the [Drop] impl for
/// [`crate::validate_buffer_mut::EditorBufferMut`], which runs this function:
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`]. Due to the
/// nature of `UTF-8` and its variable width characters, where the memory size is not the
/// same as display size. Eg: `a` is 1 byte and 1 display width (unicode segment width
/// display). `😄` is 3 bytes but it's display width is 2! To ensure that caret position
/// and scroll offset positions are not in the middle of a unicode segment character, we
/// need to run the validation checks.
pub fn clip_caret_to_content_width(args: EditorArgsMut<'_>) {
    let EditorArgsMut { buffer, engine } = args;

    let caret_scr_adj = buffer.get_caret_scr_adj();
    let line_display_width = buffer.get_line_display_width_at_caret_scr_adj();

    if caret_scr_adj.col_index.check_overflows(line_display_width)
        == BoundsStatus::Overflowed
    {
        caret_mut::to_end_of_line(buffer, engine, SelectMode::Disabled);
    }
}

/// Once this function runs, it is necessary to run the [Drop] impl for
/// [`crate::validate_buffer_mut::EditorBufferMut`], which runs this function:
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`]. Due to the
/// nature of `UTF-8` and its variable width characters, where the memory size is not the
/// same as display size. Eg: `a` is 1 byte and 1 display width (unicode segment width
/// display). `😄` is 3 bytes but it's display width is 2! To ensure that caret position
/// and scroll offset positions are not in the middle of a unicode segment character, we
/// need to run the validation checks.
pub fn set_caret_col_to(
    desired_col_index: ColIndex,
    caret_raw: &mut CaretRaw,
    scr_ofs: &mut ScrOfs,
    vp_width: ColWidth,
    line_content_display_width: ColWidth,
) {
    let curr_caret_scr_adj_col = (*caret_raw + *scr_ofs).col_index;

    match curr_caret_scr_adj_col.cmp(&desired_col_index) {
        Ordering::Less => {
            // Move caret right.
            let diff = desired_col_index - curr_caret_scr_adj_col;
            inc_caret_col_by(
                caret_raw,
                scr_ofs,
                width(*diff),
                line_content_display_width,
                vp_width,
            );
        }
        Ordering::Greater => {
            // Move caret left.
            let diff = curr_caret_scr_adj_col - desired_col_index;
            dec_caret_col_by(caret_raw, scr_ofs, width(*diff));
        }
        Ordering::Equal => {
            // Do nothing.
        }
    }
}

/// This does not simply decrement the `caret.col_index` but mutates `scroll_offset` if
/// scrolling is active.
///
/// Once this function runs, it is necessary to run the [Drop] impl for
/// [`crate::validate_buffer_mut::EditorBufferMut`], which runs this function:
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`]. Due to the
/// nature of `UTF-8` and its variable width characters, where the memory size is not the
/// same as display size. Eg: `a` is 1 byte and 1 display width (unicode segment width
/// display). `😄` is 3 bytes but it's display width is 2! To ensure that caret position
/// and scroll offset positions are not in the middle of a unicode segment character, we
/// need to run the validation checks.
pub fn dec_caret_col_by(
    caret_raw: &mut CaretRaw,
    scr_ofs: &mut ScrOfs,
    col_amt: ColWidth,
) {
    enum HorizScr {
        Active,
        Inactive,
    }

    enum VpHorizLoc {
        AtStart,
        NotAtStart,
    }

    let horiz_scr = if scr_ofs.col_index > col(0) {
        HorizScr::Active
    } else {
        HorizScr::Inactive
    };

    let vp_horiz_pos = if caret_raw.col_index > col(0) {
        VpHorizLoc::NotAtStart
    } else {
        VpHorizLoc::AtStart
    };

    match (horiz_scr, vp_horiz_pos) {
        // Scroll inactive. Simply move caret left by col_amt.
        (HorizScr::Inactive, _) => {
            caret_raw.col_index -= col_amt;
        }
        // Scroll active & At start of viewport.
        (HorizScr::Active, VpHorizLoc::AtStart) => {
            // Safe to sub, since scroll_offset.col_index can never be negative.
            scr_ofs.col_index -= col_amt;
        }
        // Scroll active & Not at start of viewport.
        (HorizScr::Active, VpHorizLoc::NotAtStart) => {
            // The line below used to be: `col_amt > caret_raw.col_index`
            let need_to_scroll_left =
                caret_scroll_index::col_index_for_width(col_amt) > caret_raw.col_index;

            // Move caret left by col_amt.
            caret_raw.col_index -= col_amt;

            // Adjust scroll_offset if needed.
            if need_to_scroll_left {
                // Move scroll left by diff.
                scr_ofs.col_index -= {
                    // Due to scroll reasons, the `lhs` is the same value as the
                    // `col_amt`, ie, it goes past the viewport width. See the
                    // `scroll_col_index_for_width()` for more details.
                    let lhs = caret_scroll_index::col_index_for_width(col_amt);
                    let rhs = caret_raw.col_index;
                    lhs - rhs
                };
            }
        }
    }
}

/// Once this function runs, it is necessary to run the [Drop] impl for
/// [`crate::validate_buffer_mut::EditorBufferMut`], which runs this function:
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`]. Due to the
/// nature of `UTF-8` and its variable width characters, where the memory size is not the
/// same as display size. Eg: `a` is 1 byte and 1 display width (unicode segment width
/// display). `😄` is 3 bytes but it's display width is 2! To ensure that caret position
/// and scroll offset positions are not in the middle of a unicode segment character, we
/// need to run the validation checks.
pub fn reset_caret_col(caret_raw: &mut CaretRaw, scr_ofs: &mut ScrOfs) {
    *scr_ofs.col_index = ch(0);
    *caret_raw.col_index = ch(0);
}

/// Decrement `caret.row_index` by 1, and adjust scrolling if active. This won't check
/// whether it is inside or outside the buffer content boundary. You should check that
/// before calling this function.
///
/// This does not simply decrement the `caret.row_index` but mutates `scroll_offset` if
/// scrolling is active. This can end up deactivating vertical scrolling as well.
///
/// > Since caret.row_index can never be negative, this function must handle changes to
/// > scroll_offset itself, and can't rely on the validations in
/// > [crate::validate_buffer_mut::perform_validation_checks_after_mutation].
///
/// Once this function runs, it is necessary to run the [Drop] impl for
/// [`crate::validate_buffer_mut::EditorBufferMut`], which runs this function:
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`]. Due to the
/// nature of `UTF-8` and its variable width characters, where the memory size is not the
/// same as display size. Eg: `a` is 1 byte and 1 display width (unicode segment width
/// display). `😄` is 3 bytes but it's display width is 2! To ensure that caret position
/// and scroll offset positions are not in the middle of a unicode segment character, we
/// need to run the validation checks.
pub fn dec_caret_row(caret_raw: &mut CaretRaw, scr_ofs: &mut ScrOfs) -> RowIndex {
    enum VertScr {
        Active,
        Inactive,
    }

    enum VpVertPos {
        AtTop,
        NotAtTop,
    }

    let vert_scr = if scr_ofs.row_index > row(0) {
        VertScr::Active
    } else {
        VertScr::Inactive
    };

    let vp_pos = if caret_raw.row_index > row(0) {
        VpVertPos::AtTop
    } else {
        VpVertPos::NotAtTop
    };

    match (vert_scr, vp_pos) {
        // Vertical scroll inactive.
        (VertScr::Inactive, _) => {
            // Scroll inactive.
            // Safe to minus 1, since caret.row_index can never be negative.
            caret_raw.row_index -= row(1);
        }
        // Scroll active & Not at top of viewport.
        (VertScr::Active, VpVertPos::AtTop) => {
            caret_raw.row_index -= height(1);
        }
        // Scroll active & At top of viewport.
        (VertScr::Active, VpVertPos::NotAtTop) => {
            // Safe to minus 1, since scroll_offset.row_index can never be negative.
            scr_ofs.row_index -= height(1);
        }
    }

    (*caret_raw + *scr_ofs).row_index
}

/// Try to increment `caret.row_index` by `row_amt`. This will not scroll past the bottom
/// of the buffer. It will also activate scrolling if needed.
///
/// ```text
/// +---------------------+
/// 0                     |
/// |        above        | <- caret_row_adj
/// |                     |
/// +--- scroll_offset ---+
/// |         ↑           |
/// |                     |
/// |      within vp      |
/// |                     |
/// |         ↓           |
/// +--- scroll_offset ---+
/// |    + vp height      |
/// |                     |
/// |        below        | <- caret_row_adj
/// |                     |
/// +---------------------+
/// ```
pub fn change_caret_row_by(
    args: EditorArgsMut<'_>,
    row_amt: RowHeight,
    direction: CaretDirection,
) {
    let EditorArgsMut { buffer, engine } = args;

    match direction {
        CaretDirection::Down => {
            let current_caret_adj_row = buffer.get_caret_scr_adj().row_index;
            let mut desired_caret_adj_row = current_caret_adj_row + row_amt;
            clip_caret_row_to_content_height(buffer, &mut desired_caret_adj_row);

            // Calculate how many rows we need to increment caret row by.
            let mut diff = desired_caret_adj_row - current_caret_adj_row;

            // When buffer_mut goes out of scope, it will be dropped &
            // validation performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());

                while diff > row(0) {
                    inc_caret_row(
                        buffer_mut.inner.caret_raw,
                        buffer_mut.inner.scr_ofs,
                        buffer_mut.inner.vp.row_height,
                    );
                    diff -= row(1);
                }
            }
        }
        CaretDirection::Up => {
            let mut diff = row_amt;

            // When buffer_mut goes out of scope, it will be dropped & validation
            // performed.
            {
                let buffer_mut = buffer.get_mut(engine.viewport());

                while diff > height(0) {
                    dec_caret_row(buffer_mut.inner.caret_raw, buffer_mut.inner.scr_ofs);
                    diff -= height(1);
                    let row_index = {
                        let lhs = *buffer_mut.inner.caret_raw;
                        let rhs = *buffer_mut.inner.scr_ofs;
                        let it = lhs + rhs;
                        it.row_index
                    };
                    if row_index == row(0) {
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

/// Clip `desired_caret_adj_row` (to the max buffer length) if it overflows past the
/// bottom of the buffer.
pub fn clip_caret_row_to_content_height(
    buffer: &EditorBuffer,
    desired_caret_scr_adj_row_index: &mut RowIndex,
) {
    // Clip desired_caret_adj_row if it overflows past the bottom of the buffer.
    let max_row_index = buffer.get_max_row_index();
    let is_past_end_of_buffer = *desired_caret_scr_adj_row_index > max_row_index;
    if is_past_end_of_buffer {
        *desired_caret_scr_adj_row_index = max_row_index;
    }
}

/// Increment `caret.row_index` by 1, and adjust scrolling if active. This won't check
/// whether it is inside or outside the buffer content boundary. You should check that
/// before calling this function.
///
/// Returns the new scroll adjusted caret row.
///
/// This increments the `caret.row_index` and can activate vertical scrolling if the
/// `caret.row_index` goes past the viewport height.
///
/// Once this function runs, it is necessary to run the [Drop] impl for
/// [`crate::validate_buffer_mut::EditorBufferMut`], which runs this function:
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`]. Due to the
/// nature of `UTF-8` and its variable width characters, where the memory size is not the
/// same as display size. Eg: `a` is 1 byte and 1 display width (unicode segment width
/// display). `😄` is 3 bytes but it's display width is 2! To ensure that caret position
/// and scroll offset positions are not in the middle of a unicode segment character, we
/// need to run the validation checks.
pub fn inc_caret_row(
    caret: &mut CaretRaw,
    scroll_offset: &mut ScrOfs,
    viewport_height: RowHeight,
) -> RowIndex {
    match caret.row_index.check_overflows(viewport_height) {
        BoundsStatus::Overflowed => {
            scroll_offset.row_index += row(1); // Activate vertical scroll.
        }
        BoundsStatus::Within => {
            caret.row_index += row(1); // Scroll inactive & Not at bottom of viewport.
        }
    }

    (*caret + *scroll_offset).row_index
}
