// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Base traits for numeric conversions - see [`NumericValue`], [`NumericConversions`],
//! [`ScreenCoordinate`], [`StorageCoordinate`].

/// Base trait for reading numeric values from wrapper types.
///
/// [`NumericConversions`] provides the foundational conversion methods that enable all
/// numeric types in the bounds checking system to convert to standard Rust integer types.
/// It separates the concern of "reading values" from "constructing values".
///
/// ## Purpose
///
/// This trait serves as the minimal interface for types that wrap numeric values and need
/// to expose those values as [`usize`] or [`u16`]. This trait is extended by
/// [`NumericValue`] (which adds arithmetic, ordering, and zero-checking) and is also used
/// by types that cannot be constructed from arbitrary integers (like terminal coordinates
/// that must be non-zero).
///
/// ## Implementing Types
///
/// This trait is implemented by:
/// - All index and length types (via [`NumericValue`])
/// - Terminal coordinate types ([`TermRow`], [`TermCol`], [`CsiCount`]) that wrap
///   [`NonZeroU16`]
///
/// ## Design Rationale
///
/// By separating reading ([`as_usize`], [`try_as_u16`]) from construction
/// ([`From<usize>`], [`From<u16>`]), we allow types with construction constraints (like
/// non-zero values) to participate in generic numeric operations without violating their
/// invariants.
///
/// See [Trait Hierarchy of Coordinate Types] for a visual overview of how these traits
/// relate.
///
/// [`as_usize`]: Self::as_usize
/// [`CsiCount`]: crate::CsiCount
/// [`From<u16>`]: std::convert::From
/// [`From<usize>`]: std::convert::From
/// [`NonZeroU16`]: std::num::NonZeroU16
/// [`TermCol`]: crate::TermCol
/// [`TermRow`]: crate::TermRow
/// [`try_as_u16`]: Self::try_as_u16
/// [Trait Hierarchy of Coordinate Types]:
///     mod@crate::bounds_check#trait-hierarchy-of-coordinate-types
pub trait NumericConversions: Copy + Sized {
    /// Converts to a [`usize`] value for array indexing, length calculations, and generic
    /// numeric operations across all coordinate types.
    ///
    /// This is the preferred conversion method for most operations due to its flexibility
    /// and compatibility with Rust's standard library.
    fn as_usize(&self) -> usize;

    /// Attempts to convert to a [`u16`] value for screen operations or downcasting
    /// [`StorageCoordinate`] types to [`ScreenCoordinate`] types.
    ///
    /// This is a fallible conversion since types that implement this trait (such as
    /// [`usize`]-backed storage types) may wrap values that exceed [`u16::MAX`].
    ///
    /// Returns [`None`] if the value exceeds [`u16::MAX`].
    ///
    /// [`ScreenCoordinate`]: ScreenCoordinate
    /// [`StorageCoordinate`]: StorageCoordinate
    fn try_as_u16(&self) -> Option<u16> { u16::try_from(self.as_usize()).ok() }
}

/// Base trait for arithmetic, ordering, and zero-checking on coordinate values.
///
/// [`NumericValue`] provides standardized arithmetic, ordering, and zero-checking
/// capabilities for any type that represents a numeric coordinate value.
///
/// ## Purpose
///
/// This trait extends [`NumericConversions`] and serves as the foundational super-trait
/// for all index and length types in the system, requiring basic arithmetic
/// ([`std::ops::Add`], [`std::ops::Sub`]), total ordering ([`Ord`]), and zero checking
/// ([`is_zero()`]).
///
/// ## Key Trait Capabilities
///
/// - **Inherited Conversions**: Read machine-width integer representations via
///   [`as_usize()`] and [`try_as_u16()`]
/// - **Core Arithmetic & Ordering**: Perform addition, subtraction, and comparison
///   operations
/// - **Zero Checking**: Test if a position or value represents zero via [`is_zero()`]
/// - **Generic Foundation**: Enables type-safe generic implementations across coordinate
///   types
///
/// ## Implementing Types
///
/// While this trait is general-purpose, it is implemented by all screen and storage
/// coordinate types in the system:
///
/// **Screen Coordinates** (16-bit grid bounds):
/// - [`VPRow`], [`VPCol`], [`VPHeight`], [`VPWidth`], [`VPLength`], [`VPIndex`]
///
/// **Storage Coordinates** (64-bit continuous bounds):
/// - [`CRow`], [`CCol`], [`ScrollbackAmount`], [`ByteIndex`], [`ByteLength`]
///
/// ## Examples
///
/// The [`NumericValue`] trait provides standardized numeric conversions and zero
/// checking:
///
/// ```rust
/// use r3bl_tui::{NumericValue, ScreenCoordinate, VPCol, VPWidth, vp_col, vp_width};
///
/// let index = vp_col(42);
/// let length = vp_width(100);
///
/// // Convert to numeric types
/// let buffer_pos: usize = index.as_usize(); // Inherited from NumericConversions
/// let terminal_col: u16 = index.as_u16();   // From ScreenCoordinate
/// assert_eq!(buffer_pos, 42);
/// assert_eq!(terminal_col, 42);
///
/// // Create screen coordinates from u16
/// let from_u16 = VPCol::from(42);
/// assert_eq!(index, from_u16);
///
/// // Check for zero values
/// let zero_length = vp_width(0);
/// let non_zero_length = vp_width(10);
/// assert!(zero_length.is_zero());
/// assert!(!non_zero_length.is_zero());
/// ```
///
/// See [Trait Hierarchy of Coordinate Types] for a visual overview of how these traits
/// relate.
///
/// [`as_u16()`]: ScreenCoordinate::as_u16
/// [`as_usize()`]: NumericConversions::as_usize
/// [`ByteIndex`]: crate::ByteIndex
/// [`ByteLength`]: crate::ByteLength
/// [`CCol`]: crate::CCol
/// [`ChUnit`]: crate::ChUnit
/// [`CRow`]: crate::CRow
/// [`is_zero()`]: Self::is_zero
/// [`ScrollbackAmount`]: crate::ScrollbackAmount
/// [`SegIndex`]: crate::SegIndex
/// [`SegLength`]: crate::SegLength
/// [`try_as_u16()`]: NumericConversions::try_as_u16
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`VPCol`]: crate::VPCol
/// [`VPHeight`]: crate::VPHeight
/// [`VPIndex`]: crate::VPIndex
/// [`VPLength`]: crate::VPLength
/// [`VPRow`]: crate::VPRow
/// [`VPWidth`]: crate::VPWidth
/// [Trait Hierarchy of Coordinate Types]:
///     mod@crate::bounds_check#trait-hierarchy-of-coordinate-types
pub trait NumericValue: NumericConversions + Ord {
    /// Checks if this numeric value or coordinate index is 0.
    ///
    /// Index types ([`VPRow`], [`VPCol`]) represent 0-based coordinate
    /// positions (where `0` is the origin/start). Coordinates are not containers that
    /// can be "empty"; semantically, an index is tested for whether its position
    /// value is zero (`index.is_zero()`).
    ///
    /// For checking if a length or dimension has 0 size, see [`LengthOps::is_empty()`].
    ///
    /// [`LengthOps::is_empty()`]: crate::LengthOps::is_empty
    /// [`VPCol`]: crate::VPCol
    /// [`VPRow`]: crate::VPRow
    fn is_zero(&self) -> bool { self.as_usize() == 0 }
}

/// A coordinate type backed by a 16-bit integer, used for screen / display dimensions.
///
/// Types implementing this trait (such as [`VPRow`] and [`VPCol`]) represent
/// screen / display terminal grid coordinates bounded by 16-bit integer space (up to
/// 65,535). They provide infallible extraction to [`u16`] via [`as_u16`] and construction
/// from [`u16`].
///
/// See [Trait Hierarchy of Coordinate Types] for a visual overview of how these traits
/// relate.
///
/// [`as_u16`]: Self::as_u16
/// [`VPCol`]: crate::VPCol
/// [`VPRow`]: crate::VPRow
/// [Trait Hierarchy of Coordinate Types]:
///     mod@crate::bounds_check#trait-hierarchy-of-coordinate-types
pub trait ScreenCoordinate: NumericValue + From<u16> {
    fn as_u16(&self) -> u16;
}

/// A coordinate type backed by a [`usize`] integer, used for storage and document space.
/// This is a marker trait that captures the more restrictive trait bounds for types that
/// represent continuous in-memory canvas space, document [`UTF-8`] byte offsets, and
/// scrollback storage history.
///
/// See [Trait Hierarchy of Coordinate Types] for a visual overview of how these traits
/// relate.
///
/// ## Purpose
///
/// This trait marks types that represent continuous in-memory canvas space, document
/// [`UTF-8`] byte offsets, and scrollback storage history (such as [`CRow`],
/// [`ScrollbackAmount`], and [`ByteIndex`]).
///
/// Unlike [`ScreenCoordinate`] types (which are bounded by 16-bit screen / display
/// terminal grid dimensions), [`StorageCoordinate`] types require machine-width integer
/// addressability to handle large buffers (such as documents exceeding 65,535 lines or 64
/// KB).
///
/// ## Capabilities and Bounds
///
/// As a marker trait, [`StorageCoordinate`] defines no additional methods beyond those
/// inherited from [`NumericValue`] (such as [`as_usize`]). It enforces the super-trait
/// bound [`From<usize>`], guaranteeing that any implementing type can be infallibly
/// constructed from a [`usize`].
///
/// [`as_usize`]: NumericConversions::as_usize
/// [`ByteIndex`]: crate::ByteIndex
/// [`CRow`]: crate::CRow
/// [`ScreenCoordinate`]: crate::ScreenCoordinate
/// [`ScrollbackAmount`]: crate::ScrollbackAmount
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [Trait Hierarchy of Coordinate Types]:
///     mod@crate::bounds_check#trait-hierarchy-of-coordinate-types
pub trait StorageCoordinate: NumericValue + From<usize> {}
