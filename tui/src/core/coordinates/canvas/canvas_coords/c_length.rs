// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::CIndex;
use crate::{VPLength, generate_canvas_length_type_impl};

/// Generic 1-based 1D length or count in the continuous storage buffer space (64-bit
/// [`Canvas`] domain).
///
/// Represents the total number of elements or total size of a 1D sequence in document
/// space (such as total grapheme cluster count or 1D buffer length).
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CLength(pub usize);
generate_canvas_length_type_impl!(CLength, CIndex, VPLength, c_len, c_index);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, NarrowingCastToU16, c_index, vp_len};

    #[test]
    fn test_get_and_set_methods() {
        let mut len = c_len(5);
        assert_eq!(len.get(), c_len(5));
        len.set(c_len(10));
        assert_eq!(len.get(), c_len(10));
    }

    #[test]
    fn test_canvas_length_constructors_and_conversions() {
        let len1 = c_len(10usize);
        assert_eq!(len1.0, 10);
        assert_eq!(len1.as_usize(), 10);

        let len2: CLength = 10u16.into();
        assert_eq!(len1, len2);

        let len3: CLength = 10i32.into();
        assert_eq!(len1, len3);

        let len4: CLength = vp_len(10).into();
        assert_eq!(len1, len4);

        let u_val: usize = len1.into();
        assert_eq!(u_val, 10);

        let mut len_mut = c_len(5usize);
        len_mut.0 = 20;
        assert_eq!(len_mut.0, 20);

        let len_cast = c_len(42usize);
        assert_eq!(len_cast.as_u16_narrowing(), 42u16);
    }

    #[test]
    fn test_canvas_length_math_ops() {
        assert_eq!(c_len(5usize).as_usize(), 5);
        assert!(c_len(0usize).is_empty());

        // CLength + CLength = CLength
        let len1 = c_len(10usize);
        let len2 = c_len(5usize);
        assert_eq!(len1 + len2, c_len(15usize));

        // CLength - CLength = CLength
        assert_eq!(len1 - len2, c_len(5usize));
        assert_eq!(len2 - len1, c_len(0usize)); // Saturating underflow

        let mut len_assign = c_len(10usize);
        len_assign += c_len(5usize);
        assert_eq!(len_assign, c_len(15usize));
        len_assign -= c_len(20usize);
        assert_eq!(len_assign, c_len(0usize));

        // CLength * usize = CLength
        assert_eq!(len2 * 3usize, c_len(15usize));

        // CLength / usize = CLength
        assert_eq!(len1 / 2usize, c_len(5usize));
        assert_eq!(len1 / 0usize, c_len(0usize)); // Div by zero edge case

        // Conversions from CLength to CIndex
        assert_eq!(c_len(10usize).convert_to_index(), c_index(9usize));
        assert_eq!(c_len(0usize).convert_to_index(), c_index(0usize));

        // Arithmetic with usize
        assert_eq!(len1 + 5usize, c_len(15usize));
        assert_eq!(len1 - 3usize, c_len(7usize));
        assert_eq!(len1 - 20usize, c_len(0usize));

        let mut len_assign_u = c_len(10usize);
        len_assign_u += 5usize;
        assert_eq!(len_assign_u, c_len(15usize));
        len_assign_u -= 20usize;
        assert_eq!(len_assign_u, c_len(0usize));

        // Arithmetic with i32
        assert_eq!(len1 + 5i32, c_len(15usize));
        assert_eq!(len1 + (-3i32), c_len(7usize));
        assert_eq!(len1 + (-20i32), c_len(0usize));
        assert_eq!(len1 - 3i32, c_len(7usize));
        assert_eq!(len1 - (-5i32), c_len(15usize));
        assert_eq!(len1 - 20i32, c_len(0usize));

        let mut len_assign_i32 = c_len(10usize);
        len_assign_i32 += 5i32;
        assert_eq!(len_assign_i32, c_len(15usize));
        len_assign_i32 -= -5i32;
        assert_eq!(len_assign_i32, c_len(20usize));
    }
}
