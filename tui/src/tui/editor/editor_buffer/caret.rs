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

use std::ops::Deref;

use r3bl_core::{Position, ScrollOffset};

/// The "raw" position is the `col_index` and `row_index` of the caret INSIDE the
/// viewport, without making any adjustments for scrolling.
/// - It does not take into account the amount of scrolling (vertical, horizontal) that is
///   currently active.
/// - When scrolling is "active", this position will be different from the "scroll
///   adjusted" position.
/// - This is the default `CaretKind`.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct RawCaret {
    pub inner: Position,
}

impl Deref for RawCaret {
    type Target = Position;

    fn deref(&self) -> &Self::Target { &self.inner }
}

/// The "scroll adjusted" position is the `col_index` and `row_index` of the caret OUTSIDE
/// the viewport, after making adjustments for scrolling.
/// - It takes into account the amount of scrolling (vertical, horizontal) that is
///   currently active.
/// - When scrolling is "active", this position will be different from the "raw" position.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct ScrollAdjustedCaret {
    pub inner: Position,
}

impl Deref for ScrollAdjustedCaret {
    type Target = Position;

    fn deref(&self) -> &Self::Target { &self.inner }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Caret {
    Raw(RawCaret),
    ScrollAdjusted(ScrollAdjustedCaret),
}

impl Deref for Caret {
    type Target = Position;

    fn deref(&self) -> &Self::Target {
        match self {
            Caret::Raw(raw_caret) => raw_caret,
            Caret::ScrollAdjusted(scroll_adjusted_caret) => scroll_adjusted_caret,
        }
    }
}

impl Default for Caret {
    fn default() -> Self { Caret::Raw(RawCaret::default()) }
}

mod convert_to_scroll_adjusted_caret {
    use super::*;

    impl From<(RawCaret, ScrollOffset)> for ScrollAdjustedCaret {
        fn from((raw_caret, scroll_offset): (RawCaret, ScrollOffset)) -> Self {
            let position = raw_caret.inner + scroll_offset;
            ScrollAdjustedCaret { inner: position }
        }
    }

    impl From<Position> for ScrollAdjustedCaret {
        fn from(position: Position) -> Self { ScrollAdjustedCaret { inner: position } }
    }
}

mod convert_to_raw_caret {
    use super::*;

    impl From<(ScrollAdjustedCaret, ScrollOffset)> for RawCaret {
        fn from(
            (scroll_adjusted_caret, scroll_offset): (ScrollAdjustedCaret, ScrollOffset),
        ) -> Self {
            let position = scroll_adjusted_caret.inner - scroll_offset;
            RawCaret { inner: position }
        }
    }

    impl From<Position> for RawCaret {
        fn from(position: Position) -> Self { RawCaret { inner: position } }
    }
}

mod convert_to_caret {
    use super::*;

    impl From<ScrollAdjustedCaret> for Caret {
        fn from(scroll_adjusted_caret: ScrollAdjustedCaret) -> Self {
            Caret::ScrollAdjusted(scroll_adjusted_caret)
        }
    }

    impl From<RawCaret> for Caret {
        fn from(raw_caret: RawCaret) -> Self { Caret::Raw(raw_caret) }
    }

    impl From<(RawCaret, ScrollOffset)> for Caret {
        fn from((raw_caret, scroll_offset): (RawCaret, ScrollOffset)) -> Self {
            let scroll_adjusted_caret: ScrollAdjustedCaret =
                (raw_caret, scroll_offset).into();
            Caret::ScrollAdjusted(scroll_adjusted_caret)
        }
    }

    impl From<(ScrollAdjustedCaret, ScrollOffset)> for Caret {
        fn from(
            (scroll_adjusted_caret, scroll_offset): (ScrollAdjustedCaret, ScrollOffset),
        ) -> Self {
            let raw_caret: RawCaret = (scroll_adjusted_caret, scroll_offset).into();
            Caret::Raw(raw_caret)
        }
    }

    impl From<Position> for Caret {
        fn from(position: Position) -> Self {
            let raw_caret: RawCaret = position.into();
            Caret::Raw(raw_caret)
        }
    }
}

#[cfg(test)]
mod tests {
    use r3bl_core::ch;

    use super::*;

    #[test]
    fn test_default_caret_kind() {
        let default_caret = Caret::default();

        assert!(matches!(default_caret, Caret::Raw(_)));
        assert_eq!(default_caret, Caret::Raw(RawCaret::default()));
        assert_eq!(*default_caret, Position::default());

        let caret: Caret = Position::default().into();

        assert!(matches!(caret, Caret::Raw(_)));
        assert_eq!(caret, Caret::Raw(RawCaret::default()));
    }

    #[test]
    fn test_raw_to_scroll_adjusted() {
        let position = Position {
            col_index: ch(5),
            row_index: ch(5),
        };

        let scroll_offset = ScrollOffset {
            col_index: ch(2),
            row_index: ch(3),
        };

        // Create RawCaret from Position.
        let raw_caret: RawCaret = position.into();

        assert_eq!(raw_caret.inner, position);
        assert_eq!(*raw_caret, position);

        // Convert RawCaret (and ScrollOffset) to ScrollAdjustedCaret.
        let scroll_adjusted_caret: ScrollAdjustedCaret =
            (raw_caret, scroll_offset).into();

        assert_eq!(
            scroll_adjusted_caret.inner,
            Position {
                col_index: ch(7),
                row_index: ch(8)
            }
        );
        assert_eq!(
            *scroll_adjusted_caret,
            Position {
                col_index: ch(7),
                row_index: ch(8)
            }
        );

        // Convert RawCaret (and ScrollOffset) to Caret.
        let caret = Caret::from((raw_caret, scroll_offset));

        assert!(matches!(caret, Caret::ScrollAdjusted(_)));
        assert!(!matches!(caret, Caret::Raw(_)));
        assert_eq!(*caret, *scroll_adjusted_caret);
    }

    #[test]
    fn test_scroll_adjusted_to_raw() {
        let scroll_adjusted_caret: ScrollAdjustedCaret = Position {
            col_index: ch(7),
            row_index: ch(8),
        }
        .into();

        let scroll_offset = ScrollOffset {
            col_index: ch(2),
            row_index: ch(3),
        };

        let raw_caret: RawCaret = (scroll_adjusted_caret, scroll_offset).into();

        assert_eq!(
            *raw_caret,
            Position {
                col_index: ch(5),
                row_index: ch(5)
            }
        );

        let back_to_scroll_adjusted_caret: ScrollAdjustedCaret =
            (raw_caret, scroll_offset).into();

        assert_eq!(*back_to_scroll_adjusted_caret, *scroll_adjusted_caret);
    }

    #[test]
    fn test_caret_conversion_to_scroll_adjusted() {
        let raw_caret: RawCaret = Position {
            col_index: ch(5),
            row_index: ch(5),
        }
        .into();

        let caret: Caret = raw_caret.into();

        let Caret::Raw(raw_caret) = caret else {
            panic!("Expected RawCaret");
        };

        let scroll_offset = ScrollOffset {
            col_index: ch(2),
            row_index: ch(3),
        };

        let scroll_adjusted_caret: ScrollAdjustedCaret =
            (raw_caret, scroll_offset).into();

        assert_eq!(
            *scroll_adjusted_caret,
            Position {
                col_index: ch(7),
                row_index: ch(8)
            }
        );
    }

    #[test]
    fn test_caret_conversion_to_raw() {
        let scroll_adjusted_caret: ScrollAdjustedCaret = Position {
            col_index: ch(7),
            row_index: ch(8),
        }
        .into();

        let caret: Caret = scroll_adjusted_caret.into();

        let Caret::ScrollAdjusted(scroll_adjusted_caret) = caret else {
            panic!("Expected ScrollAdjustedCaret");
        };

        let scroll_offset = ScrollOffset {
            col_index: ch(2),
            row_index: ch(3),
        };

        let raw_caret: RawCaret = (scroll_adjusted_caret, scroll_offset).into();

        assert_eq!(
            raw_caret.inner,
            Position {
                col_index: ch(5),
                row_index: ch(5)
            }
        );
    }
}
