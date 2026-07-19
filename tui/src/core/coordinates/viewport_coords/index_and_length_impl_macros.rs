// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Code generation macros for dimension types. See [`crate::generate_index_type_impl!`]
//! and [`crate::generate_length_type_impl!`].

/// Generates complete implementation for index-like types (0-based positions).
///
/// This macro reduces boilerplate for types like [`VPIndex`], [`VPRow`], and
/// [`VPCol`] by generating all common implementations in one place.
///
/// # Arguments
/// - `$idx_ty`: The index type being implemented (e.g., [`VPRow`], [`VPCol`],
///   [`VPIndex`])
/// - `$assoc_len_ty`: The associated length type (e.g., [`VPHeight`], [`VPWidth`],
///   [`VPLength`])
/// - `$constr_fn`: Constructor function name (e.g., `vp_row`, `vp_col`, `vp_idx`)
/// - `$assoc_len_constr_fn`: Length constructor function (e.g., `vp_height`, `vp_width`,
///   `vp_len`)
/// - `$alias_fn`: Optional alias constructor function name(s) (e.g., `vp_row`, `vp_col`,
///   `vp_index`)
///
/// # Generated Code
/// The macro generates:
/// - [`Debug`] trait implementation
/// - Constructor helper function (`vp_row()`, `vp_col()`, `vp_idx()`)
/// - Core methods: `new()`, `as_usize()`, `as_u16()`
/// - Conversion method: `convert_to_length()` / `convert_to_height()` /
///   `convert_to_width()`
/// - [`From`] trait implementations for: [`ChUnit`], `usize`, `u16`, `i32`
/// - [`From`] trait to convert to `usize` and `u16`
/// - [`Deref`] and [`DerefMut`] to [`ChUnit`]
/// - Arithmetic operators: [`Add`], [`AddAssign`], [`Sub`], [`SubAssign`] (for self and
///   paired length type)
/// - [`Mul`] with paired length type
/// - Numeric arithmetic via `generate_numeric_arithmetic_ops_impl!` macro
/// - Trait implementations: [`NumericConversions`], [`NumericValue`], [`IndexOps`]
/// - [`ArrayBoundsCheck`] implementation
///
/// # Usage
///
/// See the implementations in [`VPCol`], [`VPRow`], and [`VPIndex`] for
/// real-world examples of how to use this macro.
///
/// [`Add`]: std::ops::Add
/// [`AddAssign`]: std::ops::AddAssign
/// [`ArrayBoundsCheck`]: crate::core::ArrayBoundsCheck
/// [`ChUnit`]: crate::ChUnit
/// [`Debug`]: std::fmt::Debug
/// [`Deref`]: std::ops::Deref
/// [`DerefMut`]: std::ops::DerefMut
/// [`From`]: std::convert::From
/// [`IndexOps`]: crate::IndexOps
/// [`Mul`]: std::ops::Mul
/// [`NumericConversions`]: crate::NumericConversions
/// [`NumericValue`]: crate::NumericValue
/// [`Sub`]: std::ops::Sub
/// [`SubAssign`]: std::ops::SubAssign
/// [`VPCol`]: crate::VPCol
/// [`VPHeight`]: crate::VPHeight
/// [`VPIndex`]: crate::VPIndex
/// [`VPLength`]: crate::VPLength
/// [`VPRow`]: crate::VPRow
/// [`VPWidth`]: crate::VPWidth
#[macro_export]
macro_rules! generate_index_type_impl {
    (
        /* Make this */ $idx_ty:ident,
        /* Use this */ $assoc_len_ty:ident,
        /* Make this */ $constr_fn:ident,
        /* Use this */ $assoc_len_constr_fn:ident
    ) => {
        // Constructor helper function
        #[doc = concat!("Creates a new [`", stringify!($idx_ty), "`] from any type that can be converted into it.")]
        #[inline]
        pub fn $constr_fn(arg_index: impl Into<$idx_ty>) -> $idx_ty {
            arg_index.into()
        }

        impl ::std::fmt::Debug for $idx_ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({:?})", stringify!($idx_ty), self.0)
            }
        }

        mod impl_core {
            #![allow(clippy::wildcard_imports)]
            use super::*;

            impl $idx_ty {
                #[doc = concat!("Creates a new [`", stringify!($idx_ty), "`] from any type that can be converted into it.")]
                #[inline]
                pub fn new(arg_index: impl Into<$idx_ty>) -> Self {
                    arg_index.into()
                }

                #[doc = concat!("Returns the value of this [`", stringify!($idx_ty), "`] as a `usize`.")]
                #[inline]
                #[must_use]
                pub fn as_usize(&self) -> usize {
                    $crate::usize(self.0)
                }

                #[doc = concat!("Returns the value of this [`", stringify!($idx_ty), "`] as a `u16`.")]
                #[inline]
                #[must_use]
                pub fn as_u16(&self) -> u16 {
                    self.0.into()
                }

                #[doc = concat!("Add 1 to the index to convert it to a ", stringify!($assoc_len_ty), ".")]
                #[inline]
                #[must_use]
                pub fn convert_to_length(&self) -> $assoc_len_ty {
                    $assoc_len_constr_fn(self.0.saturating_add(1))
                }

                #[doc = concat!("Calculates the distance between two positions.\n\n")]
                #[doc = concat!("Returns the number of positions from `other` to `self` as a [`", stringify!($assoc_len_ty), "`].\n")]
                #[doc = concat!("For example, `", stringify!($constr_fn), "(10).distance_from(", stringify!($constr_fn), "(3))` returns `", stringify!($assoc_len_constr_fn), "(7)` (7 units apart).\n\n")]
                #[doc = concat!("# Use Cases\n\n")]
                #[doc = concat!("- Measuring how many units to scroll:\n")]
                #[doc = concat!("  `desired_pos.distance_from(current_pos)`.\n")]
                #[doc = concat!("- Calculating viewport spans: `end_index.distance_from(start_index)`.\n\n")]
                #[doc = concat!("# Panics\n\n")]
                #[doc = concat!("Panics if `other > self` (negative distance).\n\n")]
                #[doc = concat!("# See Also\n\n")]
                #[doc = concat!("For **navigating** backward by an offset (returning a position), use the\n")]
                #[doc = concat!("subtraction operator (`-`) instead: `position - offset`.")]
                #[inline]
                #[must_use]
                pub fn distance_from(self, other: Self) -> $assoc_len_ty {
                    $assoc_len_constr_fn(self.as_u16() - other.as_u16())
                }
            }
        }

        mod impl_from_numeric {
            #![allow(clippy::wildcard_imports)]
            use super::*;

            impl From<$crate::ChUnit> for $idx_ty {
                #[inline]
                fn from(ch_unit: $crate::ChUnit) -> Self {
                    $idx_ty(ch_unit)
                }
            }

            impl From<usize> for $idx_ty {
                #[inline]
                fn from(val: usize) -> Self {
                    $idx_ty(val.into())
                }
            }

            impl From<$idx_ty> for usize {
                #[inline]
                fn from(index: $idx_ty) -> Self {
                    index.as_usize()
                }
            }

            impl From<u16> for $idx_ty {
                #[inline]
                fn from(val: u16) -> Self {
                    $idx_ty(val.into())
                }
            }

            impl From<i32> for $idx_ty {
                #[inline]
                fn from(val: i32) -> Self {
                    $idx_ty(val.into())
                }
            }

            impl From<$idx_ty> for u16 {
                #[inline]
                fn from(index: $idx_ty) -> Self {
                    index.as_u16()
                }
            }
        }

        mod impl_deref {
            #![allow(clippy::wildcard_imports)]
            use super::*;

            impl ::std::ops::Deref for $idx_ty {
                type Target = $crate::ChUnit;

                #[inline]
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::std::ops::DerefMut for $idx_ty {
                #[inline]
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
        }

        mod dimension_arithmetic_operators {
            #![allow(clippy::wildcard_imports)]
            use super::*;

            // Self + Self operations
            impl ::std::ops::Add<$idx_ty> for $idx_ty {
                type Output = $idx_ty;

                fn add(self, rhs: $idx_ty) -> Self::Output {
                    let mut self_copy = self;
                    *self_copy += *rhs;
                    self_copy
                }
            }

            impl ::std::ops::AddAssign<$idx_ty> for $idx_ty {
                fn add_assign(&mut self, rhs: $idx_ty) {
                    *self = *self + rhs;
                }
            }

            #[doc = concat!("Navigates backward by subtracting an offset from a position.\n\n")]
            #[doc = concat!("This operation moves backward within an index space, returning a new position.\n")]
            #[doc = concat!("For example, `", stringify!($constr_fn), "(5) - ", stringify!($constr_fn), "(2)` returns `", stringify!($constr_fn), "(3)` (the position 2 steps before ", stringify!($constr_fn), " 5).\n\n")]
            #[doc = concat!("Uses saturating subtraction: if the offset exceeds the position, returns\n")]
            #[doc = concat!("position 0 rather than overflowing.\n\n")]
            #[doc = concat!("# Use Cases\n\n")]
            #[doc = concat!("- Moving cursor backward: `cursor_pos - ", stringify!($constr_fn), "(1)`.\n")]
            #[doc = concat!("- Calculating previous position: `current_index - offset`.\n\n")]
            #[doc = concat!("# See Also\n\n")]
            #[doc = concat!("For calculating the **distance** between two positions (returning a length),\n")]
            #[doc = concat!("use [`distance_from()`] instead.\n\n")]
            #[doc = concat!("[`distance_from()`]: #method.distance_from")]
            impl ::std::ops::Sub<$idx_ty> for $idx_ty {
                type Output = $idx_ty;

                fn sub(self, rhs: $idx_ty) -> Self::Output {
                    $constr_fn(self.as_u16().saturating_sub(rhs.as_u16()))
                }
            }

            impl ::std::ops::SubAssign<$idx_ty> for $idx_ty {
                fn sub_assign(&mut self, rhs: $idx_ty) {
                    *self = *self - rhs;
                }
            }

            // Operations with paired length type
            impl ::std::ops::Add<$assoc_len_ty> for $idx_ty {
                type Output = $idx_ty;

                fn add(self, rhs: $assoc_len_ty) -> Self::Output {
                    let mut self_copy = self;
                    *self_copy += *rhs;
                    self_copy
                }
            }

            impl ::std::ops::AddAssign<$assoc_len_ty> for $idx_ty {
                fn add_assign(&mut self, rhs: $assoc_len_ty) {
                    *self = *self + rhs;
                }
            }

            impl ::std::ops::Sub<$assoc_len_ty> for $idx_ty {
                type Output = $idx_ty;

                fn sub(self, rhs: $assoc_len_ty) -> Self::Output {
                    let mut self_copy = self;
                    *self_copy -= *rhs;
                    self_copy
                }
            }

            impl ::std::ops::SubAssign<$assoc_len_ty> for $idx_ty {
                fn sub_assign(&mut self, rhs: $assoc_len_ty) {
                    **self -= *rhs;
                }
            }

            impl ::std::ops::Mul<$assoc_len_ty> for $idx_ty {
                type Output = $idx_ty;

                fn mul(self, rhs: $assoc_len_ty) -> Self::Output {
                    let mut self_copy = self;
                    *self_copy *= *rhs;
                    self_copy
                }
            }
        }

        mod numeric_arithmetic_operators {
            #![allow(clippy::wildcard_imports)]
            use super::*;

            // Numeric operations for usize
            impl ::std::ops::Sub<usize> for $idx_ty {
                type Output = $idx_ty;
                fn sub(self, rhs: usize) -> Self::Output {
                    $constr_fn(self.as_usize().saturating_sub(rhs))
                }
            }

            impl ::std::ops::SubAssign<usize> for $idx_ty {
                fn sub_assign(&mut self, rhs: usize) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<usize> for $idx_ty {
                type Output = $idx_ty;
                fn add(self, rhs: usize) -> Self::Output {
                    $constr_fn(self.as_usize() + rhs)
                }
            }

            impl ::std::ops::AddAssign<usize> for $idx_ty {
                fn add_assign(&mut self, rhs: usize) {
                    *self = *self + rhs;
                }
            }

            // Numeric operations for u16
            impl ::std::ops::Sub<u16> for $idx_ty {
                type Output = $idx_ty;
                fn sub(self, rhs: u16) -> Self::Output {
                    $constr_fn(self.as_u16().saturating_sub(rhs))
                }
            }

            impl ::std::ops::SubAssign<u16> for $idx_ty {
                fn sub_assign(&mut self, rhs: u16) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<u16> for $idx_ty {
                type Output = $idx_ty;
                fn add(self, rhs: u16) -> Self::Output {
                    $constr_fn(self.as_u16() + rhs)
                }
            }

            impl ::std::ops::AddAssign<u16> for $idx_ty {
                fn add_assign(&mut self, rhs: u16) {
                    *self = *self + rhs;
                }
            }

            // Numeric operations for i32
            impl ::std::ops::Sub<i32> for $idx_ty {
                type Output = $idx_ty;
                fn sub(self, rhs: i32) -> Self::Output {
                    use $crate::NarrowingCastToUsize;
                    $constr_fn(self.as_usize().saturating_sub(rhs.as_usize_narrowing()))
                }
            }

            impl ::std::ops::SubAssign<i32> for $idx_ty {
                fn sub_assign(&mut self, rhs: i32) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<i32> for $idx_ty {
                type Output = $idx_ty;
                fn add(self, rhs: i32) -> Self::Output {
                    use $crate::NarrowingCastToUsize;
                    $constr_fn(self.as_usize() + rhs.as_usize_narrowing())
                }
            }

            impl ::std::ops::AddAssign<i32> for $idx_ty {
                fn add_assign(&mut self, rhs: i32) {
                    *self = *self + rhs;
                }
            }
        }

        mod bounds_check_trait_impls {
            #[allow(clippy::wildcard_imports)]
            use super::*;
            use $crate::{IndexOps, NumericConversions, NumericValue, ScreenCoordinate};

            impl NumericConversions for $idx_ty {
                fn as_usize(&self) -> usize {
                    self.0.as_usize()
                }
            }

            impl NumericValue for $idx_ty {}

            impl ScreenCoordinate for $idx_ty {
                fn as_u16(&self) -> u16 {
                    self.0.as_u16()
                }
            }

            impl IndexOps for $idx_ty {
                type LengthType = $assoc_len_ty;

                #[doc = concat!("Add 1 to the index to convert it to a ", stringify!($assoc_len_ty), ".")]
                fn convert_to_length(&self) -> Self::LengthType {
                    self.convert_to_length()
                }
            }
        }

        // ArrayBoundsCheck implementation for type-safe bounds checking
        impl $crate::core::ArrayBoundsCheck<$assoc_len_ty> for $idx_ty {}
    };
}

/// Generates complete implementation for length-like types (1-based sizes).
///
/// This macro reduces boilerplate for types like [`VPLength`], [`VPHeight`], and
/// [`VPWidth`] by generating all common implementations in one place.
///
/// # Arguments
/// - `$len_ty`: The length type being implemented (e.g., [`VPHeight`], [`VPWidth`],
///   [`VPLength`])
/// - `$assoc_idx_ty`: The associated index type (e.g., [`VPRow`], [`VPCol`], [`VPIndex`])
/// - `$constr_fn`: Constructor function name (e.g., `vp_height`, `vp_width`, `vp_len`)
/// - `$assoc_idx_constr_fn`: Index constructor function (e.g., `vp_row`, `vp_col`,
///   `vp_idx`)
///
/// # Generated Code
/// The macro generates:
/// - [`Debug`] trait implementation
/// - Constructor helper function (`vp_height()`, `vp_width()`, `vp_len()`)
/// - Core methods: `new()`, `as_usize()`, `as_u16()`
/// - [`From`] trait implementations for: [`ChUnit`], `usize`, `u16`, `i32`, `u8`
/// - [`From`] trait to convert to `u16`
/// - [`Deref`] and [`DerefMut`] to [`ChUnit`]
/// - Arithmetic operators: [`Add`], [`AddAssign`], [`Sub`], [`SubAssign`] (for self type)
/// - [`Div`]`<`[`ChUnit`]`>` operation
/// - Numeric arithmetic via `generate_numeric_arithmetic_ops_impl!` macro
/// - Trait implementations: [`NumericConversions`], [`NumericValue`], [`LengthOps`]
///
/// # Usage
///
/// See the implementations in [`VPWidth`], [`VPHeight`], and [`VPLength`] for
/// real-world examples of how to use this macro.
///
/// [`Add`]: std::ops::Add
/// [`AddAssign`]: std::ops::AddAssign
/// [`ChUnit`]: crate::ChUnit
/// [`Debug`]: std::fmt::Debug
/// [`Deref`]: std::ops::Deref
/// [`DerefMut`]: std::ops::DerefMut
/// [`Div`]: std::ops::Div
/// [`From`]: std::convert::From
/// [`LengthOps`]: crate::LengthOps
/// [`NumericConversions`]: crate::NumericConversions
/// [`NumericValue`]: crate::NumericValue
/// [`Sub`]: std::ops::Sub
/// [`SubAssign`]: std::ops::SubAssign
/// [`VPCol`]: crate::VPCol
/// [`VPHeight`]: crate::VPHeight
/// [`VPIndex`]: crate::VPIndex
/// [`VPLength`]: crate::VPLength
/// [`VPRow`]: crate::VPRow
/// [`VPWidth`]: crate::VPWidth
#[macro_export]
macro_rules! generate_length_type_impl {
    (
        /* Make this */ $len_ty:ident,
        /* Use this */ $assoc_idx_ty:ident,
        /* Make this */ $constr_fn:ident,
        /* Use this */ $assoc_idx_constr_fn:ident
    ) => {
        // Constructor helper function
        #[doc = concat!("Creates a new [`", stringify!($len_ty), "`] from any type that can be converted into it.")]
        #[inline]
        pub fn $constr_fn(arg_length: impl Into<$len_ty>) -> $len_ty {
            arg_length.into()
        }

        impl ::std::fmt::Debug for $len_ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}({:?})", stringify!($len_ty), self.0)
            }
        }

        mod impl_core {
            #[allow(clippy::wildcard_imports)]
            use super::*;

            impl $len_ty {
                #[doc = concat!("Creates a new [`", stringify!($len_ty), "`] from any type that can be converted into it.")]
                #[inline]
                pub fn new(arg_length: impl Into<$len_ty>) -> Self {
                    arg_length.into()
                }

                #[doc = concat!("Returns the value of this [`", stringify!($len_ty), "`] as a `u16`.")]
                #[inline]
                #[must_use]
                pub fn as_u16(&self) -> u16 {
                    self.0.into()
                }

                #[doc = concat!("Returns the value of this [`", stringify!($len_ty), "`] as a `usize`.")]
                #[inline]
                #[must_use]
                pub fn as_usize(&self) -> usize {
                    self.0.into()
                }

                /// Returns `true` if this length has 0 size.
                ///
                /// Length types represent dimensions, spans, or quantities of items.
                /// Semantically, checking if a dimension has no size means checking
                /// whether it is empty (`length.is_empty()`), matching standard Rust
                /// conventions where types with a `len()` method also provide
                /// `is_empty()`.
                ///
                /// For checking if a coordinate index is at origin `0`, see
                /// [`NumericValue::is_zero()`].
                ///
                /// [`NumericValue::is_zero()`]: crate::NumericValue::is_zero
                #[inline]
                #[must_use]
                pub fn is_empty(&self) -> bool {
                    self.0.value == 0
                }
            }
        }

        mod impl_from_numeric {
            #[allow(clippy::wildcard_imports)]
            use super::*;

            impl From<$crate::ChUnit> for $len_ty {
                #[inline]
                fn from(ch_unit: $crate::ChUnit) -> Self {
                    $len_ty(ch_unit)
                }
            }

            impl From<usize> for $len_ty {
                #[inline]
                fn from(length: usize) -> Self {
                    $len_ty($crate::ch(length))
                }
            }

            impl From<u16> for $len_ty {
                #[inline]
                fn from(val: u16) -> Self {
                    $len_ty(val.into())
                }
            }

            impl From<i32> for $len_ty {
                #[inline]
                fn from(val: i32) -> Self {
                    $len_ty(val.into())
                }
            }

            impl From<u8> for $len_ty {
                #[inline]
                fn from(val: u8) -> Self {
                    $len_ty(val.into())
                }
            }

            impl From<$len_ty> for u16 {
                #[inline]
                fn from(length: $len_ty) -> Self {
                    length.0.into()
                }
            }
        }

        mod impl_deref {
            #[allow(clippy::wildcard_imports)]
            use super::*;

            impl ::std::ops::Deref for $len_ty {
                type Target = $crate::ChUnit;

                #[inline]
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::std::ops::DerefMut for $len_ty {
                #[inline]
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
        }

        mod dimension_arithmetic_operators {
            #[allow(clippy::wildcard_imports)]
            use super::*;

            impl ::std::ops::Add<$len_ty> for $len_ty {
                type Output = $len_ty;

                fn add(self, rhs: $len_ty) -> Self::Output {
                    let mut self_copy = self;
                    *self_copy += *rhs;
                    self_copy
                }
            }

            impl ::std::ops::AddAssign<$len_ty> for $len_ty {
                fn add_assign(&mut self, rhs: $len_ty) {
                    **self += *rhs;
                }
            }

            impl ::std::ops::Sub<$len_ty> for $len_ty {
                type Output = $len_ty;

                fn sub(self, rhs: $len_ty) -> Self::Output {
                    let mut self_copy = self;
                    *self_copy -= *rhs;
                    self_copy
                }
            }

            impl ::std::ops::SubAssign<$len_ty> for $len_ty {
                fn sub_assign(&mut self, rhs: $len_ty) {
                    **self -= *rhs;
                }
            }

            #[doc = concat!("Dividing a length by a length yields a dimensionless count.\n\n")]
            #[doc = concat!("For example, `", stringify!($len_ty), "(240) / ", stringify!($len_ty), "(80)` returns `3` (the number of\n")]
            #[doc = concat!("80-unit lengths that fit in 240 units).")]
            impl ::std::ops::Div<$len_ty> for $len_ty {
                type Output = u16;

                fn div(self, rhs: $len_ty) -> Self::Output {
                    self.as_u16() / rhs.as_u16()
                }
            }

            #[doc = concat!("Remainder after dividing a length by a length yields a dimensionless offset.\n\n")]
            #[doc = concat!("For example, `", stringify!($len_ty), "(245) % ", stringify!($len_ty), "(80)` returns `5` (the remainder\n")]
            #[doc = concat!("after fitting 80-unit lengths into 245 units).")]
            impl ::std::ops::Rem<$len_ty> for $len_ty {
                type Output = u16;

                fn rem(self, rhs: $len_ty) -> Self::Output {
                    self.as_u16() % rhs.as_u16()
                }
            }

            #[doc = concat!("Dividing a length by a scalar scales the length down.\n\n")]
            #[doc = concat!("For example, `", stringify!($len_ty), "(80) / 2u16` returns `", stringify!($len_ty), "(40)`.")]
            impl ::std::ops::Div<u16> for $len_ty {
                type Output = $len_ty;

                fn div(self, rhs: u16) -> Self::Output {
                    $constr_fn(self.as_u16() / rhs)
                }
            }

            impl ::std::ops::Div<$crate::ChUnit> for $len_ty {
                type Output = $len_ty;

                fn div(self, rhs: $crate::ChUnit) -> Self::Output {
                    let value = *self / rhs;
                    $constr_fn(value)
                }
            }
        }

        mod numeric_arithmetic_operators {
            #![allow(clippy::wildcard_imports)]
            use super::*;

            // Inline numeric operations for usize
            impl ::std::ops::Sub<usize> for $len_ty {
                type Output = $len_ty;
                fn sub(self, rhs: usize) -> Self::Output {
                    $constr_fn(self.as_usize().saturating_sub(rhs))
                }
            }

            impl ::std::ops::SubAssign<usize> for $len_ty {
                fn sub_assign(&mut self, rhs: usize) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<usize> for $len_ty {
                type Output = $len_ty;
                fn add(self, rhs: usize) -> Self::Output {
                    $constr_fn(self.as_usize() + rhs)
                }
            }

            impl ::std::ops::AddAssign<usize> for $len_ty {
                fn add_assign(&mut self, rhs: usize) {
                    *self = *self + rhs;
                }
            }

            // Numeric operations for u16
            impl ::std::ops::Sub<u16> for $len_ty {
                type Output = $len_ty;
                fn sub(self, rhs: u16) -> Self::Output {
                    $constr_fn(self.as_u16().saturating_sub(rhs))
                }
            }

            impl ::std::ops::SubAssign<u16> for $len_ty {
                fn sub_assign(&mut self, rhs: u16) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<u16> for $len_ty {
                type Output = $len_ty;
                fn add(self, rhs: u16) -> Self::Output {
                    $constr_fn(self.as_u16() + rhs)
                }
            }

            impl ::std::ops::AddAssign<u16> for $len_ty {
                fn add_assign(&mut self, rhs: u16) {
                    *self = *self + rhs;
                }
            }

            // Numeric operations for i32
            impl ::std::ops::Sub<i32> for $len_ty {
                type Output = $len_ty;
                fn sub(self, rhs: i32) -> Self::Output {
                    use $crate::NarrowingCastToUsize;
                    $constr_fn(self.as_usize().saturating_sub(rhs.as_usize_narrowing()))
                }
            }

            impl ::std::ops::SubAssign<i32> for $len_ty {
                fn sub_assign(&mut self, rhs: i32) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<i32> for $len_ty {
                type Output = $len_ty;
                fn add(self, rhs: i32) -> Self::Output {
                    use $crate::NarrowingCastToUsize;
                    $constr_fn(self.as_usize() + rhs.as_usize_narrowing())
                }
            }

            impl ::std::ops::AddAssign<i32> for $len_ty {
                fn add_assign(&mut self, rhs: i32) {
                    *self = *self + rhs;
                }
            }
        }

        mod bounds_check_trait_impls {
            #[allow(clippy::wildcard_imports)]
            use super::*;
            use $crate::{LengthOps, NumericConversions, NumericValue, ScreenCoordinate};

            impl NumericConversions for $len_ty {
                fn as_usize(&self) -> usize {
                    self.0.as_usize()
                }
            }

            impl NumericValue for $len_ty {}

            impl ScreenCoordinate for $len_ty {
                fn as_u16(&self) -> u16 {
                    self.0.as_u16()
                }
            }

            impl LengthOps for $len_ty {
                type IndexType = $assoc_idx_ty;

                #[doc = concat!("Subtract 1 from the length to convert it to a ", stringify!($assoc_idx_ty), ".")]
                fn convert_to_index(&self) -> Self::IndexType {
                    $assoc_idx_ty::new(self.0.value.saturating_sub(1))
                }
            }
        }
    };
}
