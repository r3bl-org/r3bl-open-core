// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait for constructing [`RangeExclusive`] and [`RangeInclusive`] objects
//! directly from `(Index, Length)` coordinate tuple pairs (see [`RangeConstructExt`]).

use crate::{
    byte_index, c_col, c_row, vp_col, vp_idx, vp_row, ByteIndex, ByteLength, CCol,
    CHeight, CRow, CWidth, VPCol, VPHeight, VPIndex, VPLength, VPRow, VPWidth,
};
use std::ops::{Range, RangeInclusive};

/// Type alias for [`Range`] to explicitly signify exclusive interval semantics `[start,
/// end)` alongside [`RangeInclusive`].
pub type RangeExclusive<Idx> = Range<Idx>;

/// Extension trait for constructing [`RangeExclusive`] (`[start, start+len)`) and
/// [`RangeInclusive`] (`[start, start+len-1]`) instances directly from coordinate
/// `(start_index, length)` 2-tuples like `(`[`VPRow`]`, `[`VPHeight`]`)` or
/// `(`[`VPCol`]`, `[`VPWidth`]`)`.
///
/// `Self` represents a `(start, length)` tuple where:
/// - `self.0` is the starting index (e.g. [`VPRow`], [`CCol`], [`ByteIndex`]).
/// - `self.1` is the length / span (e.g. [`VPHeight`], [`CWidth`], [`ByteLength`]).
///
/// <div class="warning">
///
/// We cannot implement [`From`] directly for coordinate tuples to [`RangeExclusive`]
/// due to Rust's orphan rules (both [`Range`] and standard tuple types are in [`std`]),
/// so we use this extension trait.
///
/// </div>
///
/// # Purpose
///
/// This trait answers the question: **"How do I construct a [`RangeExclusive`] or
/// [`RangeInclusive`] directly from an origin index and a length?"**
///
/// This trait eliminates repetitive manual conversion logic and off-by-one arithmetic
/// errors by providing explicit, type-safe range construction methods.
///
/// # When to Use `RangeExclusive` vs `RangeInclusive`
///
/// Although both `.to_exclusive_range()` and `.to_inclusive_range()` cover the exact same
/// set of underlying indices when the length is greater than 0 (e.g. `2..6` and `2..=5`
/// both yield `2, 3, 4, 5`), choosing between them is driven by domain specifications and
/// Rust language semantics:
///
/// | Aspect                    | [`RangeExclusive`] (`.to_exclusive_range()`)                           | [`RangeInclusive`] (`.to_inclusive_range()`)                               |
/// | :------------------------ | :--------------------------------------------------------------------- | :------------------------------------------------------------------------- |
/// | **Domain Fit**            | Rust slice indexing (`slice[start..end]`), vector buffers, `for` loops | External specs & [`VT-100`] escape sequences (e.g. scroll margin `1..=24`) |
/// | **End Bound Semantics**   | Upper bound is excluded (`start + len`)                                | Upper bound is included (`start + len - 1`)                                |
/// | **Zero Length (`len=0`)** | Supports empty ranges at any position (`2..2` is empty)                | Returns [`None`] (Rust cannot express empty `RangeInclusive` at `N`)       |
/// | **Rust Syntax**           | `2..6`                                                                 | `2..=5`                                                                    |
///
/// # Quick Comparison
///
/// | Goal                                       | Imperative Approach (Error-prone)                         | `RangeConstructExt` Approach (Clean & Type-safe) |
/// | :----------------------------------------- | :-------------------------------------------------------- | :----------------------------------------------- |
/// | **Exclusive Range** `[start, start+len)`   | `(start..=start + len.convert_to_index()).to_exclusive()` | `(start, len).to_exclusive_range()`              |
/// | **Inclusive Range** `[start, start+len-1]` | `start..=start + (len - 1)`                               | `(start, len).to_inclusive_range()`              |
///
/// # Visualizing Range Construction
///
/// Given pair `(start = vp_row(2), length = vp_height(4))`:
///
/// ```text
/// Exclusive Range via .to_exclusive_range():
/// - 2..6
/// - [2, 6)
/// - covers 4 indices: 2, 3, 4, 5
///
/// Index:      0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │
///          └───┴───┴───┴───┴───┴───┴───┴───┘
///                    ▲               ▲
///                 start=2         end=6 (exclusive)
///
/// Inclusive Range via .to_inclusive_range():
/// - 2..=5
/// - [2, 5]
/// - covers 4 indices: 2, 3, 4, 5
///
/// Index:      0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │
///          └───┴───┴───┴───┴───┴───┴───┴───┘
///                    ▲           ▲
///                 start=2     end=5 (inclusive)
/// ```
///
/// For details on `[start, end)` vs `[start, end]` notation, see [Interval Notation].
///
/// # Supported Tuple Pairs
///
/// This trait is implemented for all coordinate index and length pairs:
/// - Primitives:
///   - ([`VPRow`], [`VPHeight`])
///   - ([`VPCol`], [`VPWidth`])
///   - ([`VPIndex`], [`VPLength`])
///   - ([`ByteIndex`], [`ByteLength`])
/// - Viewport Decorators:
///   - ([`VPRow`], [`VPHeight`])
///   - ([`VPCol`], [`VPWidth`])
/// - [`Canvas`] Newtypes:
///   - ([`CRow`], [`CHeight`]),
///   - ([`CCol`], [`CWidth`])
///
/// # Example Usage
///
/// ```rust
/// use r3bl_tui::{
///     CanvasRangeExt, RangeBoundsExt, RangeBoundsResult, RangeConstructExt, VPHeight,
///     VPRow, vp_height, vp_row,
/// };
///
/// let start = vp_row(0);
/// let height = vp_height(24);
///
/// // 1. Construct exclusive viewport iteration range [0..24)
/// let range = (start, height).to_exclusive_range();
/// assert_eq!(range, vp_row(0)..vp_row(24));
///
/// // 2. Perform range bounds check directly via underlying raw range
/// assert_eq!(
///     range.to_raw().check_index_is_within(vp_row(5)),
///     RangeBoundsResult::Within
/// );
/// ```
///
/// [`ByteIndex`]: crate::ByteIndex
/// [`ByteLength`]: crate::ByteLength
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [`CCol`]: crate::CCol
/// [`CHeight`]: crate::CHeight
/// [`CRow`]: crate::CRow
/// [`CWidth`]: crate::CWidth
/// [`Interval Notation`]: mod@crate::bounds_check#interval-notation
/// [`Range`]: std::ops::Range
/// [`std`]: std
/// [`VPCol`]: crate::VPCol
/// [`VPHeight`]: crate::VPHeight
/// [`VPIndex`]: crate::VPIndex
/// [`VPLength`]: crate::VPLength
/// [`VPRow`]: crate::VPRow
/// [`VPWidth`]: crate::VPWidth
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
pub trait RangeConstructExt {
    /// The index type contained in the resulting range.
    type IndexType;

    /// Constructs an exclusive [`RangeExclusive`] (`[start, start+len)`) from `self`.
    ///
    /// Here, `self` is a `(start, length)` coordinate 2-tuple where:
    /// - `self.0` is the starting index (e.g. [`VPRow`], [`CCol`], [`ByteIndex`]).
    /// - `self.1` is the length / span (e.g. [`VPHeight`], [`CWidth`], [`ByteLength`]).
    ///
    /// Both [`Self::to_exclusive_range()`] and [`Self::to_inclusive_range()`] cover the
    /// exact same set of underlying indices when the length (`self.1`) is greater than 0
    /// and are semantically equivalent.
    ///
    /// # Visualizing `(start = vp_row(2), length = vp_height(4))`
    ///
    /// ```text
    /// Exclusive Range via .to_exclusive_range():
    /// - 2..6
    /// - [2, 6)
    /// - covers 4 indices: 2, 3, 4, 5
    ///
    /// Index:      0   1   2   3   4   5   6   7
    ///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
    ///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │
    ///          └───┴───┴───┴───┴───┴───┴───┴───┘
    ///                    ▲               ▲
    ///                 start=2         end=6 (exclusive)
    /// ```
    ///
    /// [`ByteIndex`]: crate::ByteIndex
    /// [`ByteLength`]: crate::ByteLength
    /// [`CCol`]: crate::CCol
    /// [`CWidth`]: crate::CWidth
    /// [`RangeExclusive`]: crate::RangeExclusive
    /// [`Self::to_exclusive_range()`]: RangeConstructExt::to_exclusive_range
    /// [`Self::to_inclusive_range()`]: RangeConstructExt::to_inclusive_range
    /// [`VPHeight`]: crate::VPHeight
    /// [`VPRow`]: crate::VPRow
    fn to_exclusive_range(&self) -> RangeExclusive<Self::IndexType>;

    /// Constructs an inclusive [`RangeInclusive`] (`[start, start+len-1]`) from `self`.
    ///
    /// Here, `self` is a `(start, length)` coordinate 2-tuple where:
    /// - `self.0` is the starting index (e.g. [`VPRow`], [`CCol`], [`ByteIndex`]).
    /// - `self.1` is the length / span (e.g. [`VPHeight`], [`CWidth`], [`ByteLength`]).
    ///
    /// Both [`Self::to_inclusive_range()`] and [`Self::to_exclusive_range()`] cover the
    /// exact same set of underlying indices when the length (`self.1`) is greater than 0
    /// and are semantically equivalent.
    ///
    /// Returns [`None`] if the length (`self.1`) is zero (since an empty inclusive range
    /// cannot be represented in Rust).
    ///
    /// # Visualizing `(start = vp_row(2), length = vp_height(4))`
    ///
    /// ```text
    /// Inclusive Range via .to_inclusive_range():
    /// - 2..=5
    /// - [2, 5]
    /// - covers 4 indices: 2, 3, 4, 5
    ///
    /// Index:      0   1   2   3   4   5   6   7
    ///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
    ///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │
    ///          └───┴───┴───┴───┴───┴───┴───┴───┘
    ///                    ▲           ▲
    ///                 start=2     end=5 (inclusive)
    /// ```
    ///
    /// [`ByteIndex`]: crate::ByteIndex
    /// [`ByteLength`]: crate::ByteLength
    /// [`CCol`]: crate::CCol
    /// [`CWidth`]: crate::CWidth
    /// [`None`]: Option::None
    /// [`RangeInclusive`]: std::ops::RangeInclusive
    /// [`Self::to_exclusive_range()`]: RangeConstructExt::to_exclusive_range
    /// [`Self::to_inclusive_range()`]: RangeConstructExt::to_inclusive_range
    /// [`VPHeight`]: crate::VPHeight
    /// [`VPRow`]: crate::VPRow
    fn to_inclusive_range(&self) -> Option<RangeInclusive<Self::IndexType>>;
}

/// Macro to implement [`RangeConstructExt`] for coordinate `(`[`Index`]`, `[`Length`]`)`
/// pairs.
///
/// # Rationale: Declarative Macro vs. Generic Blanket Impl
///
/// A declarative macro is used here rather than a generic blanket implementation
/// (`impl<I, L> RangeConstructExt for (I, L)`) for two critical design reasons:
///
/// **1. IDE Ergonomics & Autocomplete**
///
/// A blanket impl on generic 2-tuples `(I, L)` causes `rust-analyzer` and IDEs to
/// register `.to_exclusive_range()` and `.to_inclusive_range()` on *every* 2-tuple in the
/// workspace (such as `("foo", 42)` or `(Pos, Size)`), cluttering autocomplete and
/// producing obscure trait failure compiler errors on unsupported tuples.
///
/// **2. Decorator Pattern Compatibility**
///
/// Decorator types (e.g. [`VPRow`]) use [`Deref`] forwarding for method calls
/// (`.as_usize()`), but Rust's [`Deref`] trait does not auto-forward binary operators
/// like `+` (`Add`), nor does Rust allow a single trait bound matching both direct
/// [`NumericValue`] types (e.g. [`VPRow`]) and `Deref` wrappers without custom trait
/// scaffolding. (See the [`decorator pattern`] docs for more on this pattern).
///
/// # Defensive Arithmetic
///
/// Implementations generated by this macro use [`saturating_add`] and [`saturating_sub`]
/// to guarantee that bounds calculations never panic from integer underflow or overflow.
///
/// [`CCol`]: crate::CCol
/// [`CRow`]: crate::CRow
/// [`decorator pattern`]:
///     mod@crate::core::canvas#design-decision-decorator-vs-newtype-pattern-rationale
/// [`Deref`]: std::ops::Deref
/// [`Index`]: crate::Index
/// [`Length`]: crate::Length
/// [`NumericValue`]: crate::NumericValue
/// [`saturating_add`]: usize::saturating_add
/// [`saturating_sub`]: usize::saturating_sub
/// [`VPCol`]: crate::VPCol
/// [`VPHeight`]: crate::VPHeight
/// [`VPRow`]: crate::VPRow
/// [`VPWidth`]: crate::VPWidth
macro_rules! impl_range_construct_ext {
    ($index_type:ty, $length_type:ty, $constructor:expr, $primitive_type:ty) => {
        impl RangeConstructExt for ($index_type, $length_type) {
            type IndexType = $index_type;

            fn to_exclusive_range(&self) -> RangeExclusive<Self::IndexType> {
                let (start, len) = *self;
                let start_val = start.as_usize();
                let len_val = len.as_usize();
                let end_val = start_val.saturating_add(len_val);
                start
                    ..$constructor(
                        <$primitive_type>::try_from(end_val)
                            .unwrap_or(<$primitive_type>::MAX),
                    )
            }

            fn to_inclusive_range(&self) -> Option<RangeInclusive<Self::IndexType>> {
                let (start, len) = *self;
                let len_val = len.as_usize();
                if len_val == 0 {
                    None
                } else {
                    let start_val = start.as_usize();
                    let end_val = start_val.saturating_add(len_val.saturating_sub(1));
                    Some(
                        start
                            ..=$constructor(
                                <$primitive_type>::try_from(end_val)
                                    .expect("conversion error"),
                            ),
                    )
                }
            }
        }
    };
}

impl_range_construct_ext!(VPRow, VPHeight, vp_row, u16);
impl_range_construct_ext!(VPCol, VPWidth, vp_col, u16);
impl_range_construct_ext!(VPIndex, VPLength, vp_idx, u16);

impl_range_construct_ext!(CRow, CHeight, c_row, usize);
impl_range_construct_ext!(CCol, CWidth, c_col, usize);
impl_range_construct_ext!(ByteIndex, ByteLength, byte_index, usize);

#[cfg(test)]
mod tests_range_construct {
    use super::*;
    use crate::{c_col, c_height, c_row, c_width, vp_col, vp_height, vp_row, vp_width};

    #[test]
    fn test_row_range_construct() {
        let pair = (c_row(2), c_height(5));
        assert_eq!(pair.to_exclusive_range(), c_row(2)..c_row(7));
        assert_eq!(pair.to_inclusive_range(), Some(c_row(2)..=c_row(6)));

        let zero_pair = (c_row(2), c_height(0));
        assert_eq!(zero_pair.to_exclusive_range(), c_row(2)..c_row(2));
        assert_eq!(zero_pair.to_inclusive_range(), None);
    }

    #[test]
    fn test_col_range_construct() {
        let pair = (c_col(10), c_width(20));
        assert_eq!(pair.to_exclusive_range(), c_col(10)..c_col(30));
        assert_eq!(pair.to_inclusive_range(), Some(c_col(10)..=c_col(29)));

        let zero_pair = (c_col(10), c_width(0usize));
        assert_eq!(zero_pair.to_exclusive_range(), c_col(10)..c_col(10));
        assert_eq!(zero_pair.to_inclusive_range(), None);
    }

    #[test]
    fn test_viewport_decorator_range_construct() {
        let vp_r_pair = (vp_row(0), vp_height(24));
        assert_eq!(vp_r_pair.to_exclusive_range(), vp_row(0)..vp_row(24));
        assert_eq!(vp_r_pair.to_inclusive_range(), Some(vp_row(0)..=vp_row(23)));

        let vp_c_pair = (vp_col(5), vp_width(10));
        assert_eq!(vp_c_pair.to_exclusive_range(), vp_col(5)..vp_col(15));
        assert_eq!(vp_c_pair.to_inclusive_range(), Some(vp_col(5)..=vp_col(14)));
    }

    #[test]
    fn test_canvas_newtype_range_construct() {
        let c_r_pair = (c_row(10usize), c_height(100));
        assert_eq!(
            c_r_pair.to_exclusive_range(),
            c_row(10usize)..c_row(110usize)
        );
        assert_eq!(
            c_r_pair.to_inclusive_range(),
            Some(c_row(10usize)..=c_row(109usize))
        );

        let c_c_pair = (c_col(0usize), c_width(80));
        assert_eq!(c_c_pair.to_exclusive_range(), c_col(0usize)..c_col(80usize));
        assert_eq!(
            c_c_pair.to_inclusive_range(),
            Some(c_col(0usize)..=c_col(79usize))
        );
    }

    #[test]
    fn test_byte_range_construct() {
        use crate::{ByteIndex, ByteLength};

        let byte_pair = (ByteIndex::from(10usize), ByteLength::from(5usize));
        assert_eq!(
            byte_pair.to_exclusive_range(),
            ByteIndex::from(10usize)..ByteIndex::from(15usize)
        );
        assert_eq!(
            byte_pair.to_inclusive_range(),
            Some(ByteIndex::from(10usize)..=ByteIndex::from(14usize))
        );
    }
}
