/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use crate::{get_scroll_adjusted_row_index,
            locate_cursor_in_viewport,
            AnsiStyledText,
            CalculateResizeHint,
            CaretVerticalViewportLocation,
            ChUnit,
            HowToChoose,
            InlineString,
            InlineVec,
            ItemsOwned,
            Size};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct State {
    /// Does not include the header row.
    pub max_display_height: ChUnit,
    pub max_display_width: ChUnit,
    /// This is not adjusted for [scroll_offset_row_index](State::scroll_offset_row_index).
    pub raw_caret_row_index: ChUnit,
    pub scroll_offset_row_index: ChUnit,
    pub items: ItemsOwned,
    pub selected_items: ItemsOwned,
    pub header: Header,
    pub selection_mode: HowToChoose,
    /// This is used to determine if the terminal has been resized.
    pub resize_hint: Option<ResizeHint>,
    /// This is used to determine if the terminal has been resized.
    pub window_size: Option<Size>,
}

#[derive(Debug, PartialEq, Clone, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Header {
    /// Single line header.
    SingleLine(InlineString),
    /// Multi line header.
    MultiLine(InlineVec<InlineVec<AnsiStyledText>>),
}

/// Convert various types to a header:
/// - `Vec<Vec<AnsiStyledText>>`,
/// - `InlineString`,
/// - `String`, etc.
mod convert_to_header {
    use super::*;

    impl From<Vec<Vec<AnsiStyledText>>> for Header {
        fn from(header: Vec<Vec<AnsiStyledText>>) -> Self {
            Header::MultiLine(header.into_iter().map(InlineVec::from).collect())
        }
    }

    impl From<InlineVec<InlineVec<AnsiStyledText>>> for Header {
        fn from(header: InlineVec<InlineVec<AnsiStyledText>>) -> Self {
            Header::MultiLine(header)
        }
    }

    impl From<InlineString> for Header {
        fn from(header: InlineString) -> Self { Header::SingleLine(header) }
    }

    impl From<String> for Header {
        fn from(header: String) -> Self { Header::SingleLine(InlineString::from(header)) }
    }

    impl From<&str> for Header {
        fn from(header: &str) -> Self { Header::SingleLine(InlineString::from(header)) }
    }

    impl Default for Header {
        fn default() -> Self { Header::SingleLine(InlineString::new()) }
    }
}

#[cfg(test)]
mod tests {
    use smallvec::smallvec;

    use super::*;
    use crate::{assert_eq2, ast};

    #[test]
    fn test_header_enum() {
        let state = State {
            header: Header::MultiLine(smallvec![smallvec![ast(
                "line1",
                smallvec::smallvec![],
            )]]),
            ..Default::default()
        };
        let lhs = state.header;
        let rhs =
            Header::MultiLine(smallvec![smallvec![ast("line1", smallvec::smallvec![])]]);
        assert_eq2!(lhs, rhs);
    }
}

impl CalculateResizeHint for State {
    fn set_size(&mut self, new_size: Size) {
        self.window_size = Some(new_size);
        self.clear_resize_hint();
    }

    fn get_resize_hint(&self) -> Option<ResizeHint> { self.resize_hint.clone() }

    fn set_resize_hint(&mut self, new_size: Size) {
        self.resize_hint = if let Some(old_size) = self.window_size {
            if new_size != old_size {
                if (new_size.col_width > old_size.col_width)
                    || (new_size.row_height > old_size.row_height)
                {
                    Some(ResizeHint::GotBigger)
                } else if (new_size.col_width < old_size.col_width)
                    || (new_size.row_height < old_size.row_height)
                {
                    Some(ResizeHint::GotSmaller)
                } else {
                    Some(ResizeHint::NoChange)
                }
            } else {
                None
            }
        } else {
            None
        };

        if self.window_size.is_some() {
            self.set_size(new_size)
        }
    }

    fn clear_resize_hint(&mut self) { self.resize_hint = None; }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone)]
pub enum ResizeHint {
    GotBigger,
    GotSmaller,
    #[default]
    NoChange,
}

impl State {
    /// This the row index that currently has keyboard focus.
    pub fn get_focused_index(&self) -> ChUnit {
        get_scroll_adjusted_row_index(
            self.raw_caret_row_index,
            self.scroll_offset_row_index,
        )
    }

    pub fn locate_cursor_in_viewport(&self) -> CaretVerticalViewportLocation {
        locate_cursor_in_viewport(
            self.raw_caret_row_index,
            self.scroll_offset_row_index,
            self.max_display_height,
            self.items.len().into(),
        )
    }
}
