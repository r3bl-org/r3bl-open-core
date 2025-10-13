// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Code generation macros for dimension types. See [`crate::generate_index_type_impl!`]
//! and [`crate::generate_length_type_impl!`].

/// Generates complete implementation for index-like types (0-based positions).
///
/// This macro reduces boilerplate for types like [`Index`], [`RowIndex`], and
/// [`ColIndex`] by generating all common implementations in one place.
///
/// # Parameters
/// - `$idx_ty`: The index type being implemented (e.g., [`RowIndex`], [`ColIndex`],
///   [`Index`])
/// - `$assoc_len_ty`: The associated length type (e.g., [`RowHeight`], [`ColWidth`],
///   [`Length`])
/// - `$constr_fn`: Constructor function name (e.g., `row`, `col`, `idx`)
/// - `$assoc_len_constr_fn`: Length constructor function (e.g., `height`, `width`, `len`)
///
/// # Generated Code
/// The macro generates:
/// - [`Debug`] trait implementation
/// - Constructor helper function (`row()`, `col()`, `idx()`)
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
/// See the implementations in [`ColIndex`], [`RowIndex`], and [`Index`] for real-world
/// examples of how to use this macro.
///
/// [`Index`]: crate::Index
/// [`RowIndex`]: crate::RowIndex
/// [`ColIndex`]: crate::ColIndex
/// [`Length`]: crate::Length
/// [`RowHeight`]: crate::RowHeight
/// [`ColWidth`]: crate::ColWidth
/// [`ChUnit`]: crate::ChUnit
/// [`NumericConversions`]: crate::NumericConversions
/// [`NumericValue`]: crate::NumericValue
/// [`IndexOps`]: crate::IndexOps
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`Debug`]: ::std::fmt::Debug
/// [`From`]: ::std::convert::From
/// [`Deref`]: ::std::ops::Deref
/// [`DerefMut`]: ::std::ops::DerefMut
/// [`Add`]: ::std::ops::Add
/// [`AddAssign`]: ::std::ops::AddAssign
/// [`Sub`]: ::std::ops::Sub
/// [`SubAssign`]: ::std::ops::SubAssign
/// [`Mul`]: ::std::ops::Mul
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
                pub fn new(arg_index: impl Into<$idx_ty>) -> Self {
                    arg_index.into()
                }

                #[must_use]
                pub fn as_usize(&self) -> usize {
                    $crate::usize(self.0)
                }

                #[must_use]
                pub fn as_u16(&self) -> u16 {
                    self.0.into()
                }

                #[doc = concat!("Add 1 to the index to convert it to a ", stringify!($assoc_len_ty), ".")]
                #[must_use]
                pub fn convert_to_length(&self) -> $assoc_len_ty {
                    $assoc_len_constr_fn(self.0 + 1)
                }
            }
        }

        mod impl_from_numeric {
            #![allow(clippy::wildcard_imports)]
            use super::*;

            impl From<$crate::ChUnit> for $idx_ty {
                fn from(ch_unit: $crate::ChUnit) -> Self {
                    $idx_ty(ch_unit)
                }
            }

            impl From<usize> for $idx_ty {
                fn from(val: usize) -> Self {
                    $idx_ty(val.into())
                }
            }

            impl From<$idx_ty> for usize {
                fn from(index: $idx_ty) -> Self {
                    index.as_usize()
                }
            }

            impl From<u16> for $idx_ty {
                fn from(val: u16) -> Self {
                    $idx_ty(val.into())
                }
            }

            impl From<i32> for $idx_ty {
                fn from(val: i32) -> Self {
                    $idx_ty(val.into())
                }
            }

            impl From<$idx_ty> for u16 {
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

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::std::ops::DerefMut for $idx_ty {
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

            impl ::std::ops::Sub<$idx_ty> for $idx_ty {
                type Output = $idx_ty;

                fn sub(self, rhs: $idx_ty) -> Self::Output {
                    $constr_fn(*self - *rhs)
                }
            }

            impl ::std::ops::SubAssign<$idx_ty> for $idx_ty {
                fn sub_assign(&mut self, rhs: $idx_ty) {
                    let diff = **self - *rhs;
                    *self = $constr_fn(diff);
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
                #[allow(clippy::cast_sign_loss)]
                fn sub(self, rhs: i32) -> Self::Output {
                    $constr_fn(self.as_usize().saturating_sub(rhs.max(0) as usize))
                }
            }

            impl ::std::ops::SubAssign<i32> for $idx_ty {
                fn sub_assign(&mut self, rhs: i32) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<i32> for $idx_ty {
                type Output = $idx_ty;
                #[allow(clippy::cast_sign_loss)]
                fn add(self, rhs: i32) -> Self::Output {
                    $constr_fn(self.as_usize() + rhs.max(0) as usize)
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

            impl $crate::NumericConversions for $idx_ty {
                fn as_usize(&self) -> usize {
                    self.0.as_usize()
                }

                fn as_u16(&self) -> u16 {
                    self.0.as_u16()
                }
            }

            impl $crate::NumericValue for $idx_ty {}

            impl $crate::IndexOps for $idx_ty {
                type LengthType = $assoc_len_ty;
            }
        }

        // ArrayBoundsCheck implementation for type-safe bounds checking
        impl $crate::ArrayBoundsCheck<$assoc_len_ty> for $idx_ty {}
    };
}

/// Generates complete implementation for length-like types (1-based sizes).
///
/// This macro reduces boilerplate for types like [`Length`], [`RowHeight`], and
/// [`ColWidth`] by generating all common implementations in one place.
///
/// # Parameters
/// - `$len_ty`: The length type being implemented (e.g., [`RowHeight`], [`ColWidth`],
///   [`Length`])
/// - `$assoc_idx_ty`: The associated index type (e.g., [`RowIndex`], [`ColIndex`],
///   [`Index`])
/// - `$constr_fn`: Constructor function name (e.g., `height`, `width`, `len`)
/// - `$assoc_idx_constr_fn`: Index constructor function (e.g., `row`, `col`, `idx`)
///
/// # Generated Code
/// The macro generates:
/// - [`Debug`] trait implementation
/// - Constructor helper function (`height()`, `width()`, `len()`)
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
/// See the implementations in [`ColWidth`], [`RowHeight`], and [`Length`] for real-world
/// examples of how to use this macro.
///
/// [`Length`]: crate::Length
/// [`RowHeight`]: crate::RowHeight
/// [`ColWidth`]: crate::ColWidth
/// [`Index`]: crate::Index
/// [`RowIndex`]: crate::RowIndex
/// [`ColIndex`]: crate::ColIndex
/// [`ChUnit`]: crate::ChUnit
/// [`NumericConversions`]: crate::NumericConversions
/// [`NumericValue`]: crate::NumericValue
/// [`LengthOps`]: crate::LengthOps
/// [`Debug`]: ::std::fmt::Debug
/// [`From`]: ::std::convert::From
/// [`Deref`]: ::std::ops::Deref
/// [`DerefMut`]: ::std::ops::DerefMut
/// [`Add`]: ::std::ops::Add
/// [`AddAssign`]: ::std::ops::AddAssign
/// [`Sub`]: ::std::ops::Sub
/// [`SubAssign`]: ::std::ops::SubAssign
/// [`Div`]: ::std::ops::Div
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
                pub fn new(arg_length: impl Into<$len_ty>) -> Self {
                    arg_length.into()
                }

                #[must_use]
                pub fn as_u16(&self) -> u16 {
                    self.0.into()
                }

                #[must_use]
                pub fn as_usize(&self) -> usize {
                    self.0.into()
                }
            }
        }

        mod impl_from_numeric {
            #[allow(clippy::wildcard_imports)]
            use super::*;

            impl From<$crate::ChUnit> for $len_ty {
                fn from(ch_unit: $crate::ChUnit) -> Self {
                    $len_ty(ch_unit)
                }
            }

            impl From<usize> for $len_ty {
                fn from(length: usize) -> Self {
                    $len_ty($crate::ch(length))
                }
            }

            impl From<u16> for $len_ty {
                fn from(val: u16) -> Self {
                    $len_ty(val.into())
                }
            }

            impl From<i32> for $len_ty {
                fn from(val: i32) -> Self {
                    $len_ty(val.into())
                }
            }

            impl From<u8> for $len_ty {
                fn from(val: u8) -> Self {
                    $len_ty(val.into())
                }
            }

            impl From<$len_ty> for u16 {
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

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::std::ops::DerefMut for $len_ty {
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

            impl ::std::ops::Div<$len_ty> for $len_ty {
                type Output = $len_ty;

                fn div(self, rhs: $len_ty) -> Self::Output {
                    $len_ty(self.0 / rhs.0)
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
                #[allow(clippy::cast_sign_loss)]
                fn sub(self, rhs: i32) -> Self::Output {
                    $constr_fn(self.as_usize().saturating_sub(rhs.max(0) as usize))
                }
            }

            impl ::std::ops::SubAssign<i32> for $len_ty {
                fn sub_assign(&mut self, rhs: i32) {
                    *self = *self - rhs;
                }
            }

            impl ::std::ops::Add<i32> for $len_ty {
                type Output = $len_ty;
                #[allow(clippy::cast_sign_loss)]
                fn add(self, rhs: i32) -> Self::Output {
                    $constr_fn(self.as_usize() + rhs.max(0) as usize)
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

            impl $crate::NumericConversions for $len_ty {
                fn as_usize(&self) -> usize {
                    self.0.as_usize()
                }

                fn as_u16(&self) -> u16 {
                    self.0.as_u16()
                }
            }

            impl $crate::NumericValue for $len_ty {}

            impl $crate::LengthOps for $len_ty {
                type IndexType = $assoc_idx_ty;

                fn convert_to_index(&self) -> Self::IndexType {
                    if self.0.value == 0 {
                        $assoc_idx_ty::new(0)
                    } else {
                        $assoc_idx_ty::new(self.0.value - 1)
                    }
                }
            }
        }
    };
}
