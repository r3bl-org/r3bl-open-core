// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CLength, c_len};
use crate::{VPLength, generate_canvas_index_type_impl};

/// Generic 0-based 1D index in the continuous storage buffer space (64-bit [`Canvas`]
/// domain).
///
/// Addresses an absolute index within a 1D sequence in document space (such as a grapheme
/// cluster position or 1D buffer offset). Valid indices start at 0 and go up to, but do
/// not include, the total length of the sequence.
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CIndex(pub usize);
generate_canvas_index_type_impl!(CIndex, CLength, VPLength, c_index, c_len);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IndexOps, NarrowingCastToU16, vp_len};

    #[test]
    fn test_get_and_set_methods() {
        let mut idx = c_index(5);
        assert_eq!(idx.get(), c_index(5));
        idx.set(c_index(10));
        assert_eq!(idx.get(), c_index(10));
    }

    #[test]
    fn test_canvas_index_constructors_and_conversions() {
        let idx1 = c_index(10usize);
        assert_eq!(idx1.0, 10);
        assert_eq!(idx1.as_usize(), 10);

        let idx2: CIndex = 10u16.into();
        assert_eq!(idx1, idx2);

        let idx3: CIndex = 10i32.into();
        assert_eq!(idx1, idx3);

        let u_val: usize = idx1.into();
        assert_eq!(u_val, 10);

        let mut idx_mut = c_index(5usize);
        idx_mut.0 = 20;
        assert_eq!(idx_mut.0, 20);

        let idx_cast = c_index(42usize);
        assert_eq!(idx_cast.as_u16_narrowing(), 42u16);
    }

    #[test]
    fn test_canvas_index_math_ops() {
        let idx1 = c_index(10usize);
        let idx2 = c_index(3usize);

        assert_eq!(idx1.as_usize(), 10);

        // CIndex - CIndex = CLength
        let len_diff: CLength = idx1 - idx2;
        assert_eq!(len_diff, c_len(7usize));

        // CIndex + CLength = CIndex
        let idx_add_len: CIndex = idx2 + c_len(4usize);
        assert_eq!(idx_add_len, c_index(7usize));

        // CIndex - CLength = CIndex
        let idx_sub_len: CIndex = idx1 - c_len(4usize);
        assert_eq!(idx_sub_len, c_index(6usize));

        // CIndex + CIndex = CIndex
        let idx_add_idx: CIndex = idx1 + idx2;
        assert_eq!(idx_add_idx, c_index(13usize));

        // Conversions between CIndex and CLength
        assert_eq!(c_index(0usize).convert_to_length(), c_len(1usize));
        assert_eq!(c_index(9usize).convert_to_length(), c_len(10usize));

        // Interactions with usize and i32
        assert_eq!(idx1 + 5usize, c_index(15usize));
        assert_eq!(idx1 - 4usize, c_index(6usize));
        assert_eq!(idx1 + 5i32, c_index(15usize));
        assert_eq!(idx1 + (-3i32), c_index(7usize));
        assert_eq!(idx1 - 3i32, c_index(7usize));
        assert_eq!(idx1 - (-5i32), c_index(15usize));

        let mut idx_mut = c_index(10usize);
        idx_mut += c_len(5usize);
        assert_eq!(idx_mut, c_index(15usize));
        idx_mut -= c_len(3usize);
        assert_eq!(idx_mut, c_index(12usize));
        idx_mut += 2usize;
        assert_eq!(idx_mut, c_index(14usize));
        idx_mut -= 4usize;
        assert_eq!(idx_mut, c_index(10usize));

        // Interactions with VPLength
        assert_eq!(idx1 + vp_len(5), c_index(15usize));
        assert_eq!(idx1 - vp_len(3), c_index(7usize));

        let mut idx_vp = c_index(10usize);
        idx_vp += vp_len(5);
        assert_eq!(idx_vp, c_index(15usize));
        idx_vp -= vp_len(3);
        assert_eq!(idx_vp, c_index(12usize));
    }
}
