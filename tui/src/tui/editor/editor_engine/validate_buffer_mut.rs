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

//! [`EditorBufferMut`] holds a few important mutable references to the editor buffer. It
//! also contains some data copied from the editor engine. This is necessary when you need
//! to mutate the buffer and then run validation checks on the buffer.
//!
//! The ["newtype"
//! pattern](https://doc.rust-lang.org/rust-by-example/generics/new_types.html) is used
//! here to wrap the underlying [`EditorBufferMut`] struct, so that it be used in one of
//! two distinct use cases:
//! 1. Once [`EditorBuffer::get_mut()`] is called, the buffer is mutated and then the
//!    validation checks are run. This is done by using [`EditorBufferMutWithDrop`].
//! 2. If you don't want the buffer to be mutated, then you can use
//!    [`EditorBufferMutNoDrop`] by calling [`EditorBuffer::get_mut_no_drop()`].

use super::scroll_editor_content;
use crate::{col,
            editor::sizing::VecEditorContentLines,
            usize,
            width,
            CaretRaw,
            ColWidth,
            EditorBuffer,
            ScrOfs,
            SelectionList,
            Size};

pub struct EditorBufferMut<'a> {
    pub lines: &'a mut VecEditorContentLines,
    pub caret_raw: &'a mut CaretRaw,
    pub scr_ofs: &'a mut ScrOfs,
    pub sel_list: &'a mut SelectionList,
    /// - Viewport width is optional because it's only needed for caret validation. And
    ///   you can get it from [`crate::EditorEngine`]. You can pass `0` if you don't have
    ///   it.
    /// - Viewport height is optional because it's only needed for caret validation. And
    ///   you can get it from [`crate::EditorEngine`]. You can pass `0` if you don't have
    ///   it.
    pub vp: Size,
}

mod editor_buffer_mut_impl_block {
    use super::*;

    impl EditorBufferMut<'_> {
        /// Returns the display width of the line at the caret (at it's scroll adjusted
        /// row index).
        #[must_use]
        pub fn get_line_display_width_at_caret_scr_adj_row_index(&self) -> ColWidth {
            EditorBuffer::impl_get_line_display_width_at_caret_scr_adj(
                *self.caret_raw,
                *self.scr_ofs,
                self.lines,
            )
        }

        pub fn new<'a>(
            lines: &'a mut VecEditorContentLines,
            caret_raw: &'a mut CaretRaw,
            scr_ofs: &'a mut ScrOfs,
            sel_list: &'a mut SelectionList,
            vp: Size,
        ) -> EditorBufferMut<'a> {
            EditorBufferMut {
                lines,
                caret_raw,
                scr_ofs,
                sel_list,
                vp,
            }
        }
    }
}

pub struct EditorBufferMutNoDrop<'a> {
    pub inner: EditorBufferMut<'a>,
}

mod editor_buffer_mut_no_drop_impl_block {
    use super::*;

    impl EditorBufferMutNoDrop<'_> {
        pub fn new<'a>(
            lines: &'a mut VecEditorContentLines,
            caret_raw: &'a mut CaretRaw,
            scr_ofs: &'a mut ScrOfs,
            sel_list: &'a mut SelectionList,
            vp: Size,
        ) -> EditorBufferMutNoDrop<'a> {
            EditorBufferMutNoDrop {
                inner: EditorBufferMut::new(lines, caret_raw, scr_ofs, sel_list, vp),
            }
        }
    }
}

// XMARK: Clever Rust, use of Drop to perform transaction close / end. And also of
// "newtype" idiom / pattern.

/// See the [Drop] implementation of `EditorBufferMut` which runs
/// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`].
///
/// Due to the nature of `UTF-8` and its variable width characters, where the memory size
/// is not the same as display size. Eg: `a` is 1 byte and 1 display width (unicode
/// segment width display). `😄` is 3 bytes but it's display width is 2! To ensure that
/// caret position and scroll offset positions are not in the middle of a unicode segment
/// character, we need to run the validation checks.
pub struct EditorBufferMutWithDrop<'a> {
    pub inner: EditorBufferMut<'a>,
}

mod editor_buffer_mut_with_drop_impl_block {
    use super::*;

    impl EditorBufferMutWithDrop<'_> {
        pub fn new<'a>(
            lines: &'a mut VecEditorContentLines,
            caret_raw: &'a mut CaretRaw,
            scr_ofs: &'a mut ScrOfs,
            sel_list: &'a mut SelectionList,
            vp: Size,
        ) -> EditorBufferMutWithDrop<'a> {
            EditorBufferMutWithDrop {
                inner: EditorBufferMut::new(lines, caret_raw, scr_ofs, sel_list, vp),
            }
        }
    }

    impl Drop for EditorBufferMutWithDrop<'_> {
        /// Once [`crate::validate_buffer_mut::EditorBufferMut`] is used to modify the
        /// buffer, it needs to run the validation checks to ensure that the
        /// buffer is in a valid state. This is done using
        /// [`crate::validate_buffer_mut::perform_validation_checks_after_mutation`].
        ///
        /// Due to the nature of `UTF-8` and its variable width characters, where the
        /// memory size is not the same as display size. Eg: `a` is 1 byte and 1
        /// display width (unicode segment width display). `😄` is 3 bytes but
        /// it's display width is 2! To ensure that caret position and scroll
        /// offset positions are not in the middle of a unicode segment character,
        /// we need to run the validation checks.
        fn drop(&mut self) { perform_validation_checks_after_mutation(self); }
    }
}

/// In addition to mutating the buffer, this function runs the following validations on
/// the [`EditorBuffer`]'s:
/// 1. `caret`:
///    - the caret is in not in the middle of a unicode segment character.
///    - if it is then it moves the caret.
/// 2. `scroll_offset`:
///    - make sure that it's not in the middle of a wide unicode segment character.
///    - if it is then it moves the `scroll_offset` and caret.
///
/// The drop implementation is split out into this separate function since that is how it
/// used to be written in earlier versions of the codebase, it used to be called
/// `apply_change()`. Also this function can be directly linked to in documentation.
pub fn perform_validation_checks_after_mutation(arg: &mut EditorBufferMutWithDrop<'_>) {
    // Check caret validity.
    adjust_caret_col_if_not_in_middle_of_grapheme_cluster(arg);
    adjust_caret_col_if_not_in_bounds_of_line(arg);
    // Check scroll_offset validity.
    if let Some(diff) = is_scroll_offset_in_middle_of_grapheme_cluster(arg) {
        adjust_scroll_offset_because_in_middle_of_grapheme_cluster(arg, diff);
    }
}

/// ```text
///     0   4    9    1    2    2
///                   4    0    5
///    ┌────┴────┴────┴────┴────┴⮮─┤ col
///  0 ┤     ├─      line     ─┤
///  1 ❱     TEXT-TEXT-TEXT-TEXT ░❬───┐Caret is out of
///  2 ┤         ▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲  ⎝bounds of line.
///    │         ├─    viewport   ─┤
///    ┴
///   row
/// ```
fn adjust_caret_col_if_not_in_bounds_of_line(
    editor_buffer_mut: &mut EditorBufferMutWithDrop<'_>,
) {
    // Check right side of line. Clip scroll adjusted caret to max line width.
    let row_width = {
        let line_display_width_at_caret_row = editor_buffer_mut
            .inner
            .get_line_display_width_at_caret_scr_adj_row_index();
        let scr_ofs_col_index = editor_buffer_mut.inner.scr_ofs.col_index;
        width(*line_display_width_at_caret_row - *scr_ofs_col_index)
    };

    // Make sure that the col_index is within the bounds of the given line width.
    let new_caret_col_index = col(std::cmp::min(
        *editor_buffer_mut.inner.caret_raw.col_index,
        *row_width,
    ));

    editor_buffer_mut.inner.caret_raw.col_index = new_caret_col_index;
}

pub fn is_scroll_offset_in_middle_of_grapheme_cluster(
    editor_buffer_mut: &mut EditorBufferMutWithDrop<'_>,
) -> Option<ColWidth> {
    let scroll_adjusted_caret =
        *editor_buffer_mut.inner.caret_raw + *editor_buffer_mut.inner.scr_ofs;

    let line_at_caret = editor_buffer_mut
        .inner
        .lines
        .get(usize(*scroll_adjusted_caret.row_index))?;

    let display_width_of_str_at_caret = {
        let str_at_caret = line_at_caret.get_string_at(scroll_adjusted_caret.col_index);
        match str_at_caret {
            None => width(0),
            Some(string_at_caret) => string_at_caret.width,
        }
    };

    if let Some(segment) = line_at_caret
        .check_is_in_middle_of_grapheme(editor_buffer_mut.inner.scr_ofs.col_index)
    {
        let diff = segment.display_width - display_width_of_str_at_caret;
        return Some(diff);
    }

    None
}

pub fn adjust_scroll_offset_because_in_middle_of_grapheme_cluster(
    editor_buffer_mut: &mut EditorBufferMutWithDrop<'_>,
    diff: ColWidth,
) -> Option<()> {
    editor_buffer_mut.inner.scr_ofs.col_index += diff;
    editor_buffer_mut.inner.caret_raw.col_index -= diff;
    None
}

/// This function is visible inside the `editor_ops.rs` module only. It is not meant to
/// be called directly, but instead is called by the [Drop] impl of [`EditorBufferMut`].
pub fn adjust_caret_col_if_not_in_middle_of_grapheme_cluster(
    editor_buffer_mut: &mut EditorBufferMutWithDrop<'_>,
) -> Option<()> {
    let caret_scr_adj =
        *editor_buffer_mut.inner.caret_raw + *editor_buffer_mut.inner.scr_ofs;
    let row_index = caret_scr_adj.row_index;
    let col_index = caret_scr_adj.col_index;
    let line = editor_buffer_mut.inner.lines.get(row_index.as_usize())?;

    // Caret is in the middle of a grapheme cluster, so jump it.
    let seg = line.check_is_in_middle_of_grapheme(col_index)?;

    scroll_editor_content::set_caret_col_to(
        seg.start_display_col_index + seg.display_width,
        editor_buffer_mut.inner.caret_raw,
        editor_buffer_mut.inner.scr_ofs,
        editor_buffer_mut.inner.vp.col_width,
        line.display_width,
    );

    None
}
