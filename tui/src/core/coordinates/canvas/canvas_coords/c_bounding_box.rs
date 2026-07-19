// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CPos, CSize};

/// Represents a 2D rectangular spatial boundary on the [`Canvas`] defined by:
/// 1. an `origin_pos` ([`CPos`]) and
/// 2. a `bounds_size` ([`CSize`]).
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CBoundingBox {
    pub origin_pos: CPos,
    pub bounds_size: CSize,
}

mod impl_canvas_bounding_box {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CBoundingBox {
        #[must_use]
        pub fn new(origin_pos: CPos, bounds_size: CSize) -> Self {
            Self {
                origin_pos,
                bounds_size,
            }
        }

        /// Checks if a given absolute canvas position falls inside this bounding box.
        #[must_use]
        pub fn contains_pos(&self, pos: CPos) -> bool {
            let col_range = self.origin_pos.col_index
                ..(self.origin_pos.col_index + self.bounds_size.col_width);
            let row_range = self.origin_pos.row_index
                ..(self.origin_pos.row_index + self.bounds_size.row_height);

            col_range.contains(&pos.col_index) && row_range.contains(&pos.row_index)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{c_pos, c_size};

    #[test]
    fn test_canvas_bounding_box() {
        let bbox = CBoundingBox::new(c_pos(5usize, 10usize), c_size(20usize, 15usize));

        // Inside
        assert!(bbox.contains_pos(c_pos(5usize, 10usize))); // Top-left origin
        assert!(bbox.contains_pos(c_pos(10usize, 15usize))); // Center
        assert!(bbox.contains_pos(c_pos(24usize, 24usize))); // Bottom-right inclusive edge

        // Outside
        assert!(!bbox.contains_pos(c_pos(4usize, 10usize))); // Left of bbox
        assert!(!bbox.contains_pos(c_pos(5usize, 9usize))); // Above bbox
        assert!(!bbox.contains_pos(c_pos(25usize, 15usize))); // Right of bbox (col 5 + 20 = 25 is exclusive)
        assert!(!bbox.contains_pos(c_pos(15usize, 25usize))); // Below bbox (row 10 + 15 = 25 is exclusive)
    }
}
