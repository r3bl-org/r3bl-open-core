// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait for converting strongly-typed coordinate ranges into raw [`usize`]
//! ranges for slice indexing and type-safe iteration — see [`RangeExt`].

use crate::{ByteIndex, CCol, CRow, ChUnit, LengthOps, NumericConversions, SegIndex,
            VPCol, VPIndex, VPRow};
use std::ops::{Range, RangeBounds, RangeFrom, RangeInclusive, RangeTo};

/// Extension trait to convert strongly-typed index ranges into raw [`usize`] ranges for
/// slice indexing and type-safe iteration.
///
/// # Purpose
///
/// This trait answers the question: **"How do I convert strongly-typed index ranges into
/// raw [`usize`] ranges for slice indexing or iterate over them?"**
///
/// <div class="warning">
///
/// Strongly-typed coordinate indices (like [`VPRow`], [`VPCol`], or
/// [`SegIndex`]) cannot directly be used as range iterators in `for` loops on stable Rust
/// because [`std::iter::Step`] is a nightly-only feature. Additionally, standard Rust
/// slice indexing (e.g. `slice[range]`) requires raw [`usize`] range structs.
///
/// </div>
///
/// This trait provides two ergonomic solutions:
/// 1. [`as_index_iter()`]: Provides an [`Iterator`] yielding strongly-typed index values
///    (e.g. [`VPRow`]) directly in `for` loops.
/// 2. [`as_usize_range()`]: Converts strongly-typed range bounds into standard [`usize`]
///    ranges for slice indexing.
///
/// # Syntax Unlocked
///
/// ## 1. Iterating Over Coordinate Ranges with [`as_index_iter()`]
///
/// ```
/// use r3bl_tui::{VPCol, RangeExt, VPRow, vp_col, vp_height, vp_row};
///
/// // Half-open RangeTo over a Length (implicit start at index 0)
/// let max_rows = vp_height(5);
/// let rows: Vec<VPRow> = (..max_rows).as_index_iter().collect();
/// assert_eq!(rows, vec![vp_row(0), vp_row(1), vp_row(2), vp_row(3), vp_row(4)]);
///
/// // Exclusive Range over Indices
/// let cols: Vec<VPCol> = (vp_col(1)..vp_col(4)).as_index_iter().collect();
/// assert_eq!(cols, vec![vp_col(1), vp_col(2), vp_col(3)]);
///
/// // Inclusive Range over Indices
/// let cols_inc: Vec<VPCol> = (vp_col(1)..=vp_col(3)).as_index_iter().collect();
/// assert_eq!(cols_inc, vec![vp_col(1), vp_col(2), vp_col(3)]);
/// ```
///
/// ## 2. Converting Coordinate Ranges for Slice Indexing with [`as_usize_range()`]
///
/// ```
/// use r3bl_tui::{RangeExt, vp_col};
///
/// let slice = ["a", "b", "c", "d", "e"];
/// let range = vp_col(1)..vp_col(4);
///
/// // Convert strongly-typed Range<ColIndex> to Range<usize> for slice indexing
/// let sub_slice = &slice[range.as_usize_range()];
/// assert_eq!(sub_slice, &["b", "c", "d"]);
/// ```
///
/// [`as_index_iter()`]: Self::as_index_iter
/// [`as_usize_range()`]: Self::as_usize_range
/// [`SegIndex`]: crate::SegIndex
/// [`VPCol`]: crate::VPCol
/// [`VPRow`]: crate::VPRow
pub trait RangeExt {
    /// The equivalent raw [`usize`] range type corresponding to this coordinate range.
    ///
    /// Depending on the input range variant, this associated type resolves to a concrete
    /// type implementing [`RangeBounds<usize>`]:
    /// - [`Range<usize>`] for exclusive ranges ([`Range<I>`])
    /// - [`RangeInclusive<usize>`] for inclusive ranges ([`RangeInclusive<I>`])
    /// - [`RangeFrom<usize>`] for half-open start ranges ([`RangeFrom<I>`])
    /// - [`RangeTo<usize>`] for half-open end ranges ([`RangeTo<I>`])
    ///
    /// [`Range<I>`]: std::ops::Range
    /// [`Range<usize>`]: std::ops::Range
    /// [`RangeBounds<usize>`]: std::ops::RangeBounds
    /// [`RangeFrom<I>`]: std::ops::RangeFrom
    /// [`RangeFrom<usize>`]: std::ops::RangeFrom
    /// [`RangeInclusive<I>`]: std::ops::RangeInclusive
    /// [`RangeInclusive<usize>`]: std::ops::RangeInclusive
    /// [`RangeTo<I>`]: std::ops::RangeTo
    /// [`RangeTo<usize>`]: std::ops::RangeTo
    type TargetRange: RangeBounds<usize>;

    /// The index type produced by [`as_index_iter()`], matching [`LengthOps::IndexType`].
    ///
    /// [`as_index_iter()`]: Self::as_index_iter
    /// [`LengthOps::IndexType`]: crate::LengthOps::IndexType
    type IndexType: TryFrom<usize>;

    /// Converts this range into a [`usize`] range suitable for slice indexing
    /// (`slice[...]`).
    ///
    /// This provides a clean way to use coordinate ranges for accessing elements in
    /// standard Rust collections (like [`Vec`] or slices) without manually casting the
    /// endpoints.
    ///
    /// # Returns
    /// A range with [`usize`] boundaries that can be used directly as a [`SliceIndex`].
    ///
    /// [`SliceIndex`]: std::slice::SliceIndex
    #[must_use]
    fn as_usize_range(&self) -> Self::TargetRange;

    /// Returns an [`Iterator`] over this range that yields values converted into target
    /// index type `Self::IndexType`.
    ///
    /// The iterator struct returned is 100% owned data. It does not contain any internal
    /// references or pointers back to `&self`. It only holds:
    /// 1. Two `usize` numbers (the start and end of the range) that are copied from the
    ///    original range.
    /// 2. One function pointer. This the closure, or the pointer to the conversion
    ///    function that turns `usize` into `Self::IndexType`. It is used by [`map()`]
    ///    when the caller uses this iterator to convert each `usize` into the target
    ///    index type.
    ///
    /// See [trait-level examples].
    ///
    /// [`map()`]: std::iter::Iterator::map
    /// [trait-level examples]:
    ///     RangeExt#1-iterating-over-coordinate-ranges-with-as_index_iter
    fn as_index_iter(&self) -> impl Iterator<Item = Self::IndexType> + 'static;
}

/// Trait linking range bound types (lengths and indices) to their target [`IndexType`].
///
/// This is an internal bridge trait used exclusively by [`RangeExt`] to determine the
/// correct index type to yield during iteration, regardless of whether the range is
/// composed of Lengths (e.g., `0..vp_height(5)`) or Indices (e.g.,
/// `vp_row(0)..vp_row(5)`). The primary reason we need this is so that we can write `for`
/// loops using [`as_index_iter()`] for both Length and Index types seamlessly.
///
/// # Why not just use `LengthOps::IndexType`?
///
/// ## Problem
///
/// While [`LengthOps`] already defines an `IndexType` association (e.g., [`VPHeight`]
/// maps to [`VPRow`]), we cannot rely solely on it for [`RangeExt`] because:
///
/// 1. **Index types don't implement [`LengthOps`]**: If [`RangeExt`] required
///    [`LengthOps`], you could not iterate over ranges of indices (like
///    `vp_row(0)..vp_row(5)`).
/// 2. **Trait coherence ([`E0119`]) prevents multiple generic impls**: If we tried to
///    provide two separate implementations of [`RangeExt`], one for `T: LengthOps` and
///    another for `T: IndexOps`, then compiler would reject it, as it cannot guarantee a
///    type will never implement both traits.
///
/// ## Solution
///
/// By funneling all types through this single [`RangeIndexType`] trait, we can provide
/// exactly one generic implementation of [`RangeExt`] that covers both scenarios safely.
///
/// **Mappings:**
/// - [`LengthOps`] types delegate directly to their associated [`LengthOps::IndexType`].
/// - Index types and primitives ([`usize`], [`ChUnit`]) map directly to themselves.
///
/// [`as_index_iter()`]: crate::RangeExt::as_index_iter
/// [`ChUnit`]: crate::ChUnit
/// [`E0119`]: https://doc.rust-lang.org/error_codes/E0119.html
/// [`IndexType`]: Self::IndexType
/// [`LengthOps::IndexType`]: crate::LengthOps::IndexType
/// [`LengthOps`]: crate::LengthOps
/// [`RangeExt`]: RangeExt
/// [`RangeIndexType`]: Self
/// [`VPHeight`]: crate::VPHeight
/// [`VPRow`]: crate::VPRow
pub trait RangeIndexType {
    type IndexType: TryFrom<usize>;
}

/// [`LengthOps`] types delegate directly to `LengthOps::IndexType`.
impl<T: LengthOps> RangeIndexType for T
where
    T::IndexType: TryFrom<usize>, /* Convert usize bounds to target index type */
{
    type IndexType = T::IndexType;
}

/// Implement [`RangeIndexType`] for types that act as their own index type.
///
/// # Syntax unlocked
///
/// Implementing [`RangeIndexType`] unlocks two key range features on index ranges:
///
/// 1. **`for` loop iteration yielding strongly-typed indices via [`.as_index_iter()`]**:
///
///    ```no_run
///    use r3bl_tui::{CRow, RangeExt, c_row};
///
///    let range = c_row(0_usize)..c_row(5_usize);
///    for row in range.as_index_iter() {
///        // `row` is strongly typed as `CRow`
///    }
///    ```
///
/// 2. **Slice indexing via [`.as_usize_range()`]**:
///
///    ```no_run
///    use r3bl_tui::{RangeExt, vp_col};
///
///    let slice = ["a", "b", "c", "d", "e"];
///    let range = vp_col(1_u16)..vp_col(4_u16);
///    let sub = &slice[range.as_usize_range()];
///    ```
///
/// # Why This Macro Exists
///
/// <div class="warning">
///
/// Rust trait coherence rules ([`E0119`]) prevent us from writing both of the following
/// because a type `T` could theoretically implement both traits:
/// - a blanket implementation `impl<T: IndexOps> RangeIndexType for T`
/// - a blanket implementation `impl<T: LengthOps> RangeIndexType for T`
///
/// Instead of writing manual, repetitive blocks for every index type, this helper macro
/// generates them cleanly. Here's the boilerplate it replaces:
/// - `impl RangeIndexType for MyIndex { type IndexType = MyIndex; }`
///
/// </div>
///
/// [`.as_index_iter()`]: RangeExt::as_index_iter
/// [`.as_usize_range()`]: RangeExt::as_usize_range
/// [`CRow`]: crate::CRow
/// [`E0119`]: https://doc.rust-lang.org/error_codes/E0119.html
/// [`LengthOps`]: crate::LengthOps
/// [`RangeIndexType`]: crate::RangeIndexType
/// [`VPRow`]: crate::VPRow
macro_rules! impl_range_index_type {
    ($type:ty) => {
        impl RangeIndexType for $type {
            type IndexType = $type;
        }
    };
}

impl_range_index_type!(VPRow);
impl_range_index_type!(VPCol);
impl_range_index_type!(SegIndex);
impl_range_index_type!(ByteIndex);
impl_range_index_type!(VPIndex);
impl_range_index_type!(usize);
impl_range_index_type!(ChUnit);
impl_range_index_type!(CRow);
impl_range_index_type!(CCol);

/// Blanket implementations of [`RangeExt`] for standard library range variants.
///
/// # Syntax Unlocked
///
/// Implementing [`RangeExt`] for standard library range variants unlocks two key
/// syntaxes:
///
/// 1. **Slice Indexing via [`.as_usize_range()`]**:
///
///    ```no_run
///    use r3bl_tui::{RangeExt, vp_col};
///    let slice = ["a", "b", "c", "d", "e"];
///    // Range<ColIndex>
///    let range = vp_col(1)..vp_col(4);
///    // Converts Range<ColIndex> to Range<usize> (1..4)
///    let sub = &slice[range.as_usize_range()];
///    ```
///
/// 2. **Type-Safe `for` Loop Iteration via [`.as_index_iter()`]**:
///
///    ```no_run
///    use r3bl_tui::{RangeExt, vp_row};
///    // Range<VPRow>
///    let rows = vp_row(0)..vp_row(5);
///    for r in rows.as_index_iter() {
///        // `r` is strongly typed as `VPRow`
///    }
///    ```
///
/// # Why These Implementations Exist
///
/// <div class="warning">
///
/// Standard Rust has two limitations when working with custom strongly-typed coordinate
/// types (like [`VPRow`], [`VPCol`], or [`CRow`]):
/// 1. **Slice Indexing Restriction**: Rust slices (`slice[range]`) only accept raw
///    [`usize`] ranges (e.g. [`Range<usize>`]). Passing a strongly-typed range like
///    `vp_col(1)..vp_col(4)` fails to compile.
/// 2. **No `for` Loop Iteration**: On stable Rust, `std::ops::Range<T>` only implements
///    [`Iterator`] if `T` implements `std::iter::Step` (a nightly-only feature). You
///    cannot write `for r in vp_row(0)..vp_row(5)` directly.
///
/// </div>
///
/// By providing explicit blanket implementations of [`RangeExt`] for the 4 standard
/// library range types (`Range<I>`, `RangeInclusive<I>`, `RangeFrom<I>`, `RangeTo<I>`),
/// we attach [`RangeExt`] methods directly to standard Rust range syntax.
///
/// These are implemented as separate `impl` blocks rather than via a macro because each
/// std range variant has unique syntax for accessing bounds (fields `.start`/`.end` vs
/// methods `.start()`/`.end()`, and distinct range construction expressions). Explicit
/// `impl` blocks also preserve detailed, variant-specific rustdoc documentation for each
/// range type.
///
/// [`.as_index_iter()`]: RangeExt::as_index_iter
/// [`.as_usize_range()`]: RangeExt::as_usize_range
/// [`ByteIndex`]: crate::ByteIndex
/// [`CCol`]: crate::CCol
/// [`CRow`]: crate::CRow
/// [`Iterator`]: std::iter::Iterator
/// [`Range<usize>`]: std::ops::Range
/// [`RangeExt`]: RangeExt
/// [`VPCol`]: crate::VPCol
/// [`VPIndex`]: crate::VPIndex
/// [`VPRow`]: crate::VPRow
mod blanket_impls {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Blanket implementation of [`RangeExt`] for exclusive ranges (`start..end`).
    ///
    /// Converts a strongly-typed exclusive range into a raw [`Range<usize>`] for slice
    /// indexing, and provides an iterator yielding the exact `IndexType`.
    ///
    /// [`Range<usize>`]: std::ops::Range
    /// [`RangeExt`]: crate::RangeExt
    impl<I: RangeIndexType + NumericConversions> RangeExt for Range<I> {
        type TargetRange = Range<usize>;
        type IndexType = I::IndexType;

        fn as_usize_range(&self) -> Self::TargetRange {
            self.start.as_usize()..self.end.as_usize()
        }

        fn as_index_iter(&self) -> impl Iterator<Item = Self::IndexType> + 'static {
            self.as_usize_range()
                .map(|v| Self::IndexType::try_from(v).ok().expect("conversion error"))
        }
    }

    /// Blanket implementation of [`RangeExt`] for inclusive ranges (`start..=end`).
    ///
    /// Converts a strongly-typed inclusive range into a raw [`RangeInclusive<usize>`] for
    /// slice indexing, and provides an iterator yielding the exact `IndexType`.
    ///
    /// [`RangeExt`]: crate::RangeExt
    /// [`RangeInclusive<usize>`]: std::ops::RangeInclusive
    impl<I: RangeIndexType + NumericConversions> RangeExt for RangeInclusive<I> {
        type TargetRange = RangeInclusive<usize>;
        type IndexType = I::IndexType;

        fn as_usize_range(&self) -> Self::TargetRange {
            self.start().as_usize()..=self.end().as_usize()
        }

        fn as_index_iter(&self) -> impl Iterator<Item = Self::IndexType> + 'static {
            self.as_usize_range()
                .map(|v| Self::IndexType::try_from(v).ok().expect("conversion error"))
        }
    }

    /// Blanket implementation of [`RangeExt`] for half-open start ranges (`start..`).
    ///
    /// Converts a strongly-typed half-open start range into a raw [`RangeFrom<usize>`]
    /// for slice indexing, and provides an iterator yielding the exact `IndexType`.
    ///
    /// [`RangeExt`]: crate::RangeExt
    /// [`RangeFrom<usize>`]: std::ops::RangeFrom
    impl<I: RangeIndexType + NumericConversions> RangeExt for RangeFrom<I> {
        type TargetRange = RangeFrom<usize>;
        type IndexType = I::IndexType;

        fn as_usize_range(&self) -> Self::TargetRange { self.start.as_usize().. }

        fn as_index_iter(&self) -> impl Iterator<Item = Self::IndexType> + 'static {
            self.as_usize_range()
                .map(|v| Self::IndexType::try_from(v).ok().expect("conversion error"))
        }
    }

    /// Blanket implementation of [`RangeExt`] for half-open end ranges (`..end`).
    ///
    /// Converts a strongly-typed half-open end range into a raw [`RangeTo<usize>`] for
    /// slice indexing, and provides an iterator yielding the exact `IndexType`.
    ///
    /// [`RangeExt`]: crate::RangeExt
    /// [`RangeTo<usize>`]: std::ops::RangeTo
    impl<I: RangeIndexType + NumericConversions> RangeExt for RangeTo<I> {
        type TargetRange = RangeTo<usize>;
        type IndexType = I::IndexType;

        fn as_usize_range(&self) -> Self::TargetRange { ..self.end.as_usize() }

        fn as_index_iter(&self) -> impl Iterator<Item = Self::IndexType> + 'static {
            (0..self.end.as_usize())
                .map(|v| Self::IndexType::try_from(v).ok().expect("conversion error"))
        }
    }
}

#[cfg(test)]
mod tests_range_ext {
    use super::*;
    use crate::{CCol, CRow, VPCol, VPRow, c_col, c_row, vp_col, vp_height, vp_idx,
                vp_row};

    #[test]
    fn test_as_usize_range() {
        let range = vp_idx(2u16)..vp_idx(5u16);
        assert_eq!(range.as_usize_range(), 2..5);

        let range_inclusive = vp_idx(2u16)..=vp_idx(5u16);
        assert_eq!(range_inclusive.as_usize_range(), 2..=5);

        let range_from = vp_col(2)..;
        assert_eq!(range_from.as_usize_range(), 2..);

        let range_to = ..vp_col(5);
        assert_eq!(range_to.as_usize_range(), ..5);
    }

    #[test]
    fn test_as_index_iter() {
        let max_rows = vp_height(5);
        let rows: Vec<VPRow> = (..max_rows).as_index_iter().collect();
        assert_eq!(
            rows,
            vec![vp_row(0), vp_row(1), vp_row(2), vp_row(3), vp_row(4)]
        );

        let cols: Vec<VPCol> = (vp_col(1)..vp_col(4)).as_index_iter().collect();
        assert_eq!(cols, vec![vp_col(1), vp_col(2), vp_col(3)]);

        let cols_inc: Vec<VPCol> = (vp_col(1)..=vp_col(3)).as_index_iter().collect();
        assert_eq!(cols_inc, vec![vp_col(1), vp_col(2), vp_col(3)]);
    }

    #[test]
    fn test_canvas_and_viewport_range_ext() {
        // Canvas row & col ranges
        let c_rows: Vec<CRow> =
            (c_row(0_usize)..c_row(3_usize)).as_index_iter().collect();
        assert_eq!(c_rows, vec![c_row(0_usize), c_row(1_usize), c_row(2_usize)]);

        let c_cols_range = c_col(1_usize)..c_col(4_usize);
        assert_eq!(c_cols_range.as_usize_range(), 1..4);

        let c_cols: Vec<CCol> = c_cols_range.as_index_iter().collect();
        assert_eq!(c_cols, vec![c_col(1_usize), c_col(2_usize), c_col(3_usize)]);

        // Viewport row & col ranges
        let vp_rows: Vec<VPRow> =
            (vp_row(0_u16)..=vp_row(2_u16)).as_index_iter().collect();
        assert_eq!(vp_rows, vec![vp_row(0_u16), vp_row(1_u16), vp_row(2_u16)]);

        let vp_cols_range = vp_col(2_u16)..=vp_col(4_u16);
        assert_eq!(vp_cols_range.as_usize_range(), 2..=4);

        let vp_cols: Vec<VPCol> = vp_cols_range.as_index_iter().collect();
        assert_eq!(vp_cols, vec![vp_col(2_u16), vp_col(3_u16), vp_col(4_u16)]);
    }
}
