// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Code generation macros for Canvas dimension types. See
//! [`crate::generate_canvas_index_type_impl!`] and
//! [`crate::generate_canvas_length_type_impl!`].

/// Generates complete implementation for Canvas index-like types (0-based positions).
#[macro_export]
macro_rules! generate_canvas_index_type_impl {
    (
        $idx_ty:ident,
        $assoc_len_ty:ident,
        $assoc_vp_len_ty:ident,
        $constr_fn:ident,
        $assoc_len_constr_fn:ident
    ) => {
        /// Helper constructor function.
        #[inline]
        pub fn $constr_fn(val: impl Into<$idx_ty>) -> $idx_ty { val.into() }

        impl $idx_ty {
            #[inline]
            pub fn new(val: impl Into<$idx_ty>) -> Self { val.into() }

            pub fn set(&mut self, value: impl Into<Self>) { *self = value.into(); }

            #[must_use]
            pub fn get(&self) -> Self { *self }

            #[must_use]
            pub fn as_usize(&self) -> usize { self.0 }
        }

        impl From<usize> for $idx_ty {
            fn from(val: usize) -> Self { $idx_ty(val) }
        }

        impl From<u16> for $idx_ty {
            fn from(val: u16) -> Self {
                use $crate::core::common::primitive_casting::WideningCastToUsize;
                $idx_ty(val.as_usize_widening())
            }
        }

        impl From<i32> for $idx_ty {
            fn from(val: i32) -> Self {
                // XMARK: Intentional numeric casting using as.
                #[allow(
                    clippy::as_conversions,
                    clippy::cast_sign_loss,
                    clippy::cast_possible_truncation
                )]
                $idx_ty(val as usize)
            }
        }

        impl From<$idx_ty> for usize {
            fn from(val: $idx_ty) -> usize { val.0 }
        }

        impl $crate::NarrowingCastToU16 for $idx_ty {
            fn as_u16_narrowing(self) -> u16 { self.0.as_u16_narrowing() }
        }

        impl $crate::NumericConversions for $idx_ty {
            fn as_usize(&self) -> usize { self.0 }
        }

        impl $crate::NumericValue for $idx_ty {}

        impl $crate::StorageCoordinate for $idx_ty {}

        impl $crate::IndexOps for $idx_ty {
            type LengthType = $assoc_len_ty;

            fn convert_to_length(&self) -> Self::LengthType {
                $assoc_len_ty(self.0.saturating_add(1))
            }
        }

        impl $crate::ArrayBoundsCheck<$assoc_len_ty> for $idx_ty {}

        // Self arithmetic
        impl ::std::ops::Add for $idx_ty {
            type Output = Self;
            fn add(self, rhs: Self) -> Self { $idx_ty(self.0.saturating_add(rhs.0)) }
        }

        impl ::std::ops::Sub for $idx_ty {
            type Output = $assoc_len_ty;
            fn sub(self, rhs: Self) -> Self::Output {
                $assoc_len_constr_fn(self.0.saturating_sub(rhs.0))
            }
        }

        impl ::std::ops::AddAssign for $idx_ty {
            fn add_assign(&mut self, rhs: Self) { self.0 = self.0.saturating_add(rhs.0); }
        }

        // Arithmetic with usize
        impl ::std::ops::Add<usize> for $idx_ty {
            type Output = Self;
            fn add(self, rhs: usize) -> Self { $idx_ty(self.0.saturating_add(rhs)) }
        }

        impl ::std::ops::Sub<usize> for $idx_ty {
            type Output = Self;
            fn sub(self, rhs: usize) -> Self { $idx_ty(self.0.saturating_sub(rhs)) }
        }

        impl ::std::ops::AddAssign<usize> for $idx_ty {
            fn add_assign(&mut self, rhs: usize) { self.0 = self.0.saturating_add(rhs); }
        }

        impl ::std::ops::SubAssign<usize> for $idx_ty {
            fn sub_assign(&mut self, rhs: usize) { self.0 = self.0.saturating_sub(rhs); }
        }

        // Arithmetic with i32
        impl ::std::ops::Add<i32> for $idx_ty {
            type Output = Self;
            fn add(self, rhs: i32) -> Self {
                use $crate::NarrowingCastToUsize;
                if rhs.is_negative() {
                    $idx_ty(
                        self.0
                            .saturating_sub(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                } else {
                    $idx_ty(
                        self.0
                            .saturating_add(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                }
            }
        }

        impl ::std::ops::Sub<i32> for $idx_ty {
            type Output = Self;
            fn sub(self, rhs: i32) -> Self::Output {
                use $crate::NarrowingCastToUsize;
                if rhs.is_negative() {
                    $idx_ty(
                        self.0
                            .saturating_add(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                } else {
                    $idx_ty(
                        self.0
                            .saturating_sub(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                }
            }
        }

        impl ::std::ops::AddAssign<i32> for $idx_ty {
            fn add_assign(&mut self, rhs: i32) { *self = *self + rhs; }
        }

        impl ::std::ops::SubAssign<i32> for $idx_ty {
            fn sub_assign(&mut self, rhs: i32) { *self = *self - rhs; }
        }

        // Arithmetic with associated Canvas length type
        impl ::std::ops::Add<$assoc_len_ty> for $idx_ty {
            type Output = Self;
            fn add(self, rhs: $assoc_len_ty) -> Self {
                $idx_ty(self.0.saturating_add(rhs.as_usize()))
            }
        }

        impl ::std::ops::Sub<$assoc_len_ty> for $idx_ty {
            type Output = Self;
            fn sub(self, rhs: $assoc_len_ty) -> Self {
                $idx_ty(self.0.saturating_sub(rhs.as_usize()))
            }
        }

        impl ::std::ops::AddAssign<$assoc_len_ty> for $idx_ty {
            fn add_assign(&mut self, rhs: $assoc_len_ty) {
                self.0 = self.0.saturating_add(rhs.as_usize());
            }
        }

        impl ::std::ops::SubAssign<$assoc_len_ty> for $idx_ty {
            fn sub_assign(&mut self, rhs: $assoc_len_ty) {
                self.0 = self.0.saturating_sub(rhs.as_usize());
            }
        }

        // Arithmetic with associated Viewport length type
        impl ::std::ops::Add<$assoc_vp_len_ty> for $idx_ty {
            type Output = Self;
            fn add(self, rhs: $assoc_vp_len_ty) -> Self {
                $idx_ty(self.0.saturating_add(rhs.as_usize()))
            }
        }

        impl ::std::ops::Sub<$assoc_vp_len_ty> for $idx_ty {
            type Output = Self;
            fn sub(self, rhs: $assoc_vp_len_ty) -> Self {
                $idx_ty(self.0.saturating_sub(rhs.as_usize()))
            }
        }

        impl ::std::ops::AddAssign<$assoc_vp_len_ty> for $idx_ty {
            fn add_assign(&mut self, rhs: $assoc_vp_len_ty) {
                self.0 = self.0.saturating_add(rhs.as_usize());
            }
        }

        impl ::std::ops::SubAssign<$assoc_vp_len_ty> for $idx_ty {
            fn sub_assign(&mut self, rhs: $assoc_vp_len_ty) {
                self.0 = self.0.saturating_sub(rhs.as_usize());
            }
        }
    };
}

/// Generates complete implementation for Canvas length-like types (1-based dimensions).
#[macro_export]
macro_rules! generate_canvas_length_type_impl {
    (
        $len_ty:ident,
        $assoc_idx_ty:ident,
        $assoc_vp_len_ty:ident,
        $constr_fn:ident,
        $assoc_idx_constr_fn:ident
    ) => {
        /// Helper constructor function.
        #[inline]
        pub fn $constr_fn(val: impl Into<$len_ty>) -> $len_ty { val.into() }

        impl $len_ty {
            #[inline]
            pub fn new(val: impl Into<$len_ty>) -> Self { val.into() }

            pub fn set(&mut self, value: impl Into<Self>) { *self = value.into(); }

            #[must_use]
            pub fn get(&self) -> Self { *self }

            #[must_use]
            pub fn as_usize(&self) -> usize { self.0 }

            #[must_use]
            pub fn is_empty(&self) -> bool { self.0 == 0 }
        }

        impl From<usize> for $len_ty {
            fn from(val: usize) -> Self { $len_ty(val) }
        }

        impl From<u16> for $len_ty {
            fn from(val: u16) -> Self {
                use $crate::core::common::primitive_casting::WideningCastToUsize;
                $len_ty(val.as_usize_widening())
            }
        }

        impl From<i32> for $len_ty {
            fn from(val: i32) -> Self {
                // XMARK: Intentional numeric casting using as.
                #[allow(
                    clippy::as_conversions,
                    clippy::cast_sign_loss,
                    clippy::cast_possible_truncation
                )]
                $len_ty(val as usize)
            }
        }

        impl From<$assoc_vp_len_ty> for $len_ty {
            fn from(val: $assoc_vp_len_ty) -> Self { $len_ty(val.as_usize()) }
        }

        impl From<$len_ty> for usize {
            fn from(val: $len_ty) -> usize { val.0 }
        }

        impl $crate::NarrowingCastToU16 for $len_ty {
            fn as_u16_narrowing(self) -> u16 { self.0.as_u16_narrowing() }
        }

        impl $crate::NumericConversions for $len_ty {
            fn as_usize(&self) -> usize { self.0 }
        }

        impl $crate::NumericValue for $len_ty {}

        impl $crate::StorageCoordinate for $len_ty {}

        impl $crate::LengthOps for $len_ty {
            type IndexType = $assoc_idx_ty;

            fn convert_to_index(&self) -> Self::IndexType {
                $assoc_idx_ty(self.0.saturating_sub(1))
            }
        }

        // Self arithmetic
        impl ::std::ops::Add for $len_ty {
            type Output = Self;
            fn add(self, rhs: Self) -> Self { $len_ty(self.0.saturating_add(rhs.0)) }
        }

        impl ::std::ops::Sub for $len_ty {
            type Output = Self;
            fn sub(self, rhs: Self) -> Self { $len_ty(self.0.saturating_sub(rhs.0)) }
        }

        impl ::std::ops::AddAssign for $len_ty {
            fn add_assign(&mut self, rhs: Self) { self.0 = self.0.saturating_add(rhs.0); }
        }

        impl ::std::ops::SubAssign for $len_ty {
            fn sub_assign(&mut self, rhs: Self) { self.0 = self.0.saturating_sub(rhs.0); }
        }

        // Arithmetic with usize
        impl ::std::ops::Add<usize> for $len_ty {
            type Output = Self;
            fn add(self, rhs: usize) -> Self { $len_ty(self.0.saturating_add(rhs)) }
        }

        impl ::std::ops::Sub<usize> for $len_ty {
            type Output = Self;
            fn sub(self, rhs: usize) -> Self { $len_ty(self.0.saturating_sub(rhs)) }
        }

        impl ::std::ops::AddAssign<usize> for $len_ty {
            fn add_assign(&mut self, rhs: usize) { self.0 = self.0.saturating_add(rhs); }
        }

        impl ::std::ops::SubAssign<usize> for $len_ty {
            fn sub_assign(&mut self, rhs: usize) { self.0 = self.0.saturating_sub(rhs); }
        }

        // Arithmetic with i32
        impl ::std::ops::Add<i32> for $len_ty {
            type Output = Self;
            fn add(self, rhs: i32) -> Self {
                use $crate::NarrowingCastToUsize;
                if rhs.is_negative() {
                    $len_ty(
                        self.0
                            .saturating_sub(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                } else {
                    $len_ty(
                        self.0
                            .saturating_add(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                }
            }
        }

        impl ::std::ops::Sub<i32> for $len_ty {
            type Output = Self;
            fn sub(self, rhs: i32) -> Self {
                use $crate::NarrowingCastToUsize;
                if rhs.is_negative() {
                    $len_ty(
                        self.0
                            .saturating_add(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                } else {
                    $len_ty(
                        self.0
                            .saturating_sub(rhs.unsigned_abs().as_usize_narrowing()),
                    )
                }
            }
        }

        impl ::std::ops::AddAssign<i32> for $len_ty {
            fn add_assign(&mut self, rhs: i32) { *self = *self + rhs; }
        }

        impl ::std::ops::SubAssign<i32> for $len_ty {
            fn sub_assign(&mut self, rhs: i32) { *self = *self - rhs; }
        }

        // Multiplication and Division with usize
        impl ::std::ops::Mul<usize> for $len_ty {
            type Output = Self;
            fn mul(self, rhs: usize) -> Self { $len_ty(self.0.saturating_mul(rhs)) }
        }

        impl ::std::ops::Div<usize> for $len_ty {
            type Output = Self;
            fn div(self, rhs: usize) -> Self {
                if rhs == 0 {
                    $len_ty(0)
                } else {
                    $len_ty(self.0 / rhs)
                }
            }
        }
    };
}
