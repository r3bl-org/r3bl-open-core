// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CalculateResizeHint, CaretVerticalViewportLocation, ChUnit, CliTextInline,
            HowToChoose, InlineString, InlineVec, ItemsOwned, Size,
            get_scroll_adjusted_row_index, locate_cursor_in_viewport};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct State {
    /// Does not include the header row.
    pub max_display_height: ChUnit,
    pub max_display_width: ChUnit,
    /// This is not adjusted for
    /// [`scroll_offset_row_index`](State::scroll_offset_row_index).
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
    MultiLine(InlineVec<InlineVec<CliTextInline>>),
}

/// Convert various types to a header:
/// - `Vec<Vec<AnsiStyledText>>`,
/// - `InlineString`,
/// - `String`, etc.
mod convert_to_header {
    use super::{CliTextInline, Header, InlineString, InlineVec};

    impl From<Vec<Vec<CliTextInline>>> for Header {
        fn from(header: Vec<Vec<CliTextInline>>) -> Self {
            Header::MultiLine(header.into_iter().map(InlineVec::from).collect())
        }
    }

    impl From<InlineVec<InlineVec<CliTextInline>>> for Header {
        fn from(header: InlineVec<InlineVec<CliTextInline>>) -> Self {
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
    use super::*;
    use crate::{assert_eq2, cli_text_inline, TuiStyle};
    use smallvec::smallvec;

    #[test]
    fn test_header_enum() {
        let state = State {
            header: Header::MultiLine(smallvec![smallvec![cli_text_inline(
                "line1",
                TuiStyle::default(),
            )]]),
            ..Default::default()
        };
        let lhs = state.header;
        let rhs = Header::MultiLine(smallvec![smallvec![cli_text_inline(
            "line1",
            TuiStyle::default()
        )]]);
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
        self.resize_hint = self
            .window_size
            .and_then(|old_size| Self::calculate_resize_hint(old_size, new_size));

        if self.window_size.is_some() {
            self.set_size(new_size);
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
    #[must_use]
    pub fn get_focused_index(&self) -> ChUnit {
        get_scroll_adjusted_row_index(
            self.raw_caret_row_index,
            self.scroll_offset_row_index,
        )
    }

    #[must_use]
    pub fn locate_cursor_in_viewport(&self) -> CaretVerticalViewportLocation {
        locate_cursor_in_viewport(
            self.raw_caret_row_index,
            self.scroll_offset_row_index,
            self.max_display_height,
            self.items.len().into(),
        )
    }

    /// Helper method to determine resize hint based on old and new sizes.
    fn calculate_resize_hint(old_size: Size, new_size: Size) -> Option<ResizeHint> {
        if new_size == old_size {
            return None;
        }

        let width_increased = new_size.col_width > old_size.col_width;
        let height_increased = new_size.row_height > old_size.row_height;
        let width_decreased = new_size.col_width < old_size.col_width;
        let height_decreased = new_size.row_height < old_size.row_height;

        if width_increased || height_increased {
            Some(ResizeHint::GotBigger)
        } else if width_decreased || height_decreased {
            Some(ResizeHint::GotSmaller)
        } else {
            Some(ResizeHint::NoChange)
        }
    }
}
