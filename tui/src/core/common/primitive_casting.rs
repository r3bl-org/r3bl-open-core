// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::DEBUG_TUI_BOUNDS_CHECK;

/// Extension trait for performing potentially lossy conversions from primitive types to
/// [`u8`]. Avoid triggering warnings from:
/// - `clippy::cast_sign_loss`
/// - `clippy::cast_lossless`
/// - `clippy::cast_possible_truncation`
///
/// The `as` keyword is the designated tool for primitive, potentially lossy conversions.
/// This trait provides a consistent interface for converting various numeric types to
/// [`u8`] with appropriate bounds checking where needed.
///
/// See also:
/// - [`u16`]: This is the type used for unit values in the TUI library.
/// - [`crate::ChUnit`]: This is the type used for unit values and conversions in the TUI
///   library.
pub trait LossyConvertToByte {
    /// Intentionally converts the value to a [`u8`] with direct casting, potentially
    /// losing precision or clamping values. Values outside the valid range may
    /// produce unexpected results.
    #[must_use]
    fn to_u8_lossy(self) -> u8;
}

// - The implementations intentionally perform primitive casts (such as `self as u8`).
// - These explicit lossy conversions are intended to truncate precision or drop sign
//   information, so Clippy warnings for truncation and sign loss are suppressed at the
//   module level.
// - XMARK: Intentional numeric casting using as.
#[allow(
    clippy::as_conversions,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation
)]
mod impl_lossy_convert_to_byte {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl LossyConvertToByte for f64 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for f32 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for i32 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for u32 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for usize {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for u64 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for u16 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for i16 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for i8 {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }

    impl LossyConvertToByte for char {
        fn to_u8_lossy(self) -> u8 { self as u8 }
    }
}

/// Extension trait for primitive numeric types allowing safe saturating conversion to
/// [`u16`].
///
/// This trait provides [`as_u16_narrowing`] for converting primitive integer types (such
/// as [`usize`], [`isize`], [`i32`], [`u32`], [`u64`], and [`i64`]) into a 16-bit
/// unsigned integer [`u16`].
///
/// ## Purpose
///
/// When mapping larger coordinate sizes or dynamic calculations to 16-bit screen
/// coordinates or terminal grid bounds, values exceeding [`u16::MAX`] (`65,535`) or
/// falling below `0` must be handled safely without crashing or wrapping around
/// unpredictably.
///
/// ## Behavior
///
/// - Values within `0..=65_535` convert directly to [`u16`].
/// - Values exceeding [`u16::MAX`] log a [`tracing::error!`] event and saturate to
///   [`u16::MAX`].
/// - Signed values that underflow `0` log a [`tracing::error!`] event and saturate to
///   `0`.
///
/// ## Example
///
/// ```rust
/// use r3bl_tui::NarrowingCastToU16;
///
/// let large_val: usize = 100_000;
/// assert_eq!(large_val.as_u16_narrowing(), u16::MAX);
///
/// let normal_val: usize = 42;
/// assert_eq!(normal_val.as_u16_narrowing(), 42u16);
/// ```
///
/// [`as_u16_narrowing`]: Self::as_u16_narrowing
pub trait NarrowingCastToU16 {
    /// Safely casts to [`u16`]. If the value overflows [`u16::MAX`], it logs a
    /// [`tracing::error!`] and saturates to [`u16::MAX`]. For signed integers, an
    /// underflow logs and saturates to `0`.
    #[allow(clippy::wrong_self_convention)]
    fn as_u16_narrowing(self) -> u16;
}

// - The implementations perform explicit bounds checks before performing raw primitive
//   casts (such as `self as u16`).
// - Clippy's static analysis does not evaluate the preceding bounds check guards, so it
//   would otherwise raise false-positive truncation and sign-loss warnings during checks.
// - XMARK: Intentional numeric casting using as.
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
mod impl_narrowing_cast_to_u16 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NarrowingCastToU16 for usize {
        fn as_u16_narrowing(self) -> u16 {
            if self > u16::MAX as usize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "usize",
                        value = %self,
                        issue = "overflowed u16::MAX",
                    };
                });
                u16::MAX
            } else {
                self as u16
            }
        }
    }

    impl NarrowingCastToU16 for isize {
        fn as_u16_narrowing(self) -> u16 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "isize",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else if self > u16::MAX as isize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "isize",
                        value = %self,
                        issue = "overflowed u16::MAX",
                    };
                });
                u16::MAX
            } else {
                self as u16
            }
        }
    }

    impl NarrowingCastToU16 for u16 {
        fn as_u16_narrowing(self) -> u16 { self }
    }

    impl NarrowingCastToU16 for i16 {
        fn as_u16_narrowing(self) -> u16 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "i16",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else {
                self as u16
            }
        }
    }

    impl NarrowingCastToU16 for u32 {
        fn as_u16_narrowing(self) -> u16 {
            if self > u16::MAX as u32 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "u32",
                        value = %self,
                        issue = "overflowed u16::MAX",
                    };
                });
                u16::MAX
            } else {
                self as u16
            }
        }
    }

    impl NarrowingCastToU16 for i32 {
        fn as_u16_narrowing(self) -> u16 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "i32",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else if self > u16::MAX as i32 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "i32",
                        value = %self,
                        issue = "overflowed u16::MAX",
                    };
                });
                u16::MAX
            } else {
                self as u16
            }
        }
    }

    impl NarrowingCastToU16 for u64 {
        fn as_u16_narrowing(self) -> u16 {
            if self > u16::MAX as u64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "u64",
                        value = %self,
                        issue = "overflowed u16::MAX",
                    };
                });
                u16::MAX
            } else {
                self as u16
            }
        }
    }

    impl NarrowingCastToU16 for i64 {
        fn as_u16_narrowing(self) -> u16 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "i64",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else if self > u16::MAX as i64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU16::as_u16_narrowing",
                        original_type = "i64",
                        value = %self,
                        issue = "overflowed u16::MAX",
                    };
                });
                u16::MAX
            } else {
                self as u16
            }
        }
    }
}

/// Extension trait for primitive numeric types allowing safe saturating conversion to
/// [`i16`].
///
/// This trait provides [`as_i16_narrowing`] for converting primitive integer types into
/// a 16-bit signed integer [`i16`].
///
/// [`as_i16_narrowing`]: Self::as_i16_narrowing
pub trait NarrowingCastToI16 {
    /// Safely casts to [`i16`]. If the value overflows [`i16::MAX`], it logs a
    /// [`tracing::error!`] and saturates to [`i16::MAX`]. For signed integers, an
    /// underflow logs and saturates to [`i16::MIN`].
    #[allow(clippy::wrong_self_convention)]
    fn as_i16_narrowing(self) -> i16;
}

#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
mod impl_narrowing_cast_to_i16 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NarrowingCastToI16 for u16 {
        fn as_i16_narrowing(self) -> i16 {
            if self > i16::MAX as u16 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToI16::as_i16_narrowing",
                        original_type = "u16",
                        value = %self,
                        issue = "overflowed i16::MAX",
                    };
                });
                i16::MAX
            } else {
                self as i16
            }
        }
    }

    impl NarrowingCastToI16 for usize {
        fn as_i16_narrowing(self) -> i16 {
            if self > i16::MAX as usize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToI16::as_i16_narrowing",
                        original_type = "usize",
                        value = %self,
                        issue = "overflowed i16::MAX",
                    };
                });
                i16::MAX
            } else {
                self as i16
            }
        }
    }

    impl NarrowingCastToI16 for isize {
        fn as_i16_narrowing(self) -> i16 {
            if self < i16::MIN as isize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToI16::as_i16_narrowing",
                        original_type = "isize",
                        value = %self,
                        issue = "underflowed i16::MIN",
                    };
                });
                i16::MIN
            } else if self > i16::MAX as isize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToI16::as_i16_narrowing",
                        original_type = "isize",
                        value = %self,
                        issue = "overflowed i16::MAX",
                    };
                });
                i16::MAX
            } else {
                self as i16
            }
        }
    }

    impl NarrowingCastToI16 for u32 {
        fn as_i16_narrowing(self) -> i16 {
            if self > i16::MAX as u32 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToI16::as_i16_narrowing",
                        original_type = "u32",
                        value = %self,
                        issue = "overflowed i16::MAX",
                    };
                });
                i16::MAX
            } else {
                self as i16
            }
        }
    }

    impl NarrowingCastToI16 for i32 {
        fn as_i16_narrowing(self) -> i16 {
            if self < i16::MIN as i32 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToI16::as_i16_narrowing",
                        original_type = "i32",
                        value = %self,
                        issue = "underflowed i16::MIN",
                    };
                });
                i16::MIN
            } else if self > i16::MAX as i32 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToI16::as_i16_narrowing",
                        original_type = "i32",
                        value = %self,
                        issue = "overflowed i16::MAX",
                    };
                });
                i16::MAX
            } else {
                self as i16
            }
        }
    }
}

/// Extension trait for primitive numeric types allowing safe saturating conversion to
/// [`isize`].
///
/// This trait provides [`as_isize_narrowing`] for converting primitive integer types into
/// [`isize`].
///
/// [`as_isize_narrowing`]: Self::as_isize_narrowing
pub trait NarrowingCastToIsize {
    /// Safely casts to [`isize`]. If the value overflows [`isize::MAX`], it logs a
    /// [`tracing::error!`] and saturates to [`isize::MAX`]. For signed integers, an
    /// underflow logs and saturates to [`isize::MIN`].
    #[allow(clippy::wrong_self_convention)]
    fn as_isize_narrowing(self) -> isize;
}

#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
mod impl_narrowing_cast_to_isize {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NarrowingCastToIsize for u16 {
        fn as_isize_narrowing(self) -> isize { self as isize }
    }

    impl NarrowingCastToIsize for usize {
        fn as_isize_narrowing(self) -> isize {
            if self > isize::MAX as usize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToIsize::as_isize_narrowing",
                        original_type = "usize",
                        value = %self,
                        issue = "overflowed isize::MAX",
                    };
                });
                isize::MAX
            } else {
                self as isize
            }
        }
    }

    impl NarrowingCastToIsize for u64 {
        fn as_isize_narrowing(self) -> isize {
            if self > isize::MAX as u64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToIsize::as_isize_narrowing",
                        original_type = "u64",
                        value = %self,
                        issue = "overflowed isize::MAX",
                    };
                });
                isize::MAX
            } else {
                self as isize
            }
        }
    }

    impl NarrowingCastToIsize for i64 {
        fn as_isize_narrowing(self) -> isize {
            if self < isize::MIN as i64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToIsize::as_isize_narrowing",
                        original_type = "i64",
                        value = %self,
                        issue = "underflowed isize::MIN",
                    };
                });
                isize::MIN
            } else if self > isize::MAX as i64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToIsize::as_isize_narrowing",
                        original_type = "i64",
                        value = %self,
                        issue = "overflowed isize::MAX",
                    };
                });
                isize::MAX
            } else {
                self as isize
            }
        }
    }
}

/// Extension trait for primitive numeric types allowing safe saturating conversion to
/// [`u8`].
///
/// This trait provides [`as_u8_narrowing`] for converting primitive integer types (such
/// as [`usize`], [`isize`], [`i32`], [`u32`], and [`u16`]) into an 8-bit unsigned integer
/// [`u8`].
///
/// ## Purpose
///
/// When mapping values to 8-bit byte bounds or compact byte masks, values exceeding
/// [`u8::MAX`] (`255`) or falling below `0` must be handled safely without crashing or
/// wrapping around unpredictably.
///
/// ## Behavior
///
/// - Values within `0..=255` convert directly to [`u8`].
/// - Values exceeding [`u8::MAX`] log a [`tracing::error!`] event and saturate to
///   [`u8::MAX`].
/// - Signed values that underflow `0` log a [`tracing::error!`] event and saturate to
///   `0`.
///
/// ## Example
///
/// ```rust
/// use r3bl_tui::NarrowingCastToU8;
///
/// let large_val: usize = 1_000;
/// assert_eq!(large_val.as_u8_narrowing(), u8::MAX);
///
/// let normal_val: usize = 128;
/// assert_eq!(normal_val.as_u8_narrowing(), 128u8);
/// ```
///
/// [`as_u8_narrowing`]: Self::as_u8_narrowing
pub trait NarrowingCastToU8 {
    /// Safely casts to [`u8`]. If the value overflows [`u8::MAX`], it logs a
    /// [`tracing::error!`] and saturates to [`u8::MAX`]. For signed integers, an
    /// underflow logs and saturates to `0`.
    #[allow(clippy::wrong_self_convention)]
    fn as_u8_narrowing(self) -> u8;
}

// - The implementations perform explicit bounds checks before performing raw primitive
//   casts (such as `self as u8`).
// - Clippy's static analysis does not evaluate the preceding bounds check guards, so it
//   would otherwise raise false-positive truncation and sign-loss warnings during checks.
// - XMARK: Intentional numeric casting using as.
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
mod impl_narrowing_cast_to_u8 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NarrowingCastToU8 for usize {
        fn as_u8_narrowing(self) -> u8 {
            if self > u8::MAX as usize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "usize",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }

    impl NarrowingCastToU8 for isize {
        fn as_u8_narrowing(self) -> u8 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "isize",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else if self > u8::MAX as isize {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "isize",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }

    impl NarrowingCastToU8 for u32 {
        fn as_u8_narrowing(self) -> u8 {
            if self > u8::MAX as u32 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "u32",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }

    impl NarrowingCastToU8 for i32 {
        fn as_u8_narrowing(self) -> u8 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "i32",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else if self > u8::MAX as i32 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "i32",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }

    impl NarrowingCastToU8 for u64 {
        fn as_u8_narrowing(self) -> u8 {
            if self > u8::MAX as u64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "u64",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }

    impl NarrowingCastToU8 for i64 {
        fn as_u8_narrowing(self) -> u8 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "i64",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else if self > u8::MAX as i64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "i64",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }

    impl NarrowingCastToU8 for u16 {
        fn as_u8_narrowing(self) -> u8 {
            if self > u8::MAX as u16 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "u16",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }

    impl NarrowingCastToU8 for i16 {
        fn as_u8_narrowing(self) -> u8 {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "i16",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else if self > u8::MAX as i16 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToU8::as_u8_narrowing",
                        original_type = "i16",
                        value = %self,
                        issue = "overflowed u8::MAX",
                    };
                });
                u8::MAX
            } else {
                self as u8
            }
        }
    }
}

/// Extension trait for primitive numeric types allowing safe saturating conversion to
/// [`usize`].
///
/// This trait provides [`as_usize_narrowing`] for converting signed integer types (such
/// as [`i32`]) into a machine-width unsigned integer [`usize`].
///
/// ## Purpose
///
/// When mapping signed integer calculations or offset differences to pointer-sized index
/// space, negative values that underflow `0` must be handled safely without crashing or
/// causing unsigned overflow wrapping.
///
/// ## Behavior
///
/// - Non-negative values convert directly to [`usize`].
/// - Signed values that underflow `0` log a [`tracing::error!`] event and saturate to
///   `0`.
///
/// ## Example
///
/// ```rust
/// use r3bl_tui::NarrowingCastToUsize;
///
/// let negative_val: i32 = -10;
/// assert_eq!(negative_val.as_usize_narrowing(), 0usize);
///
/// let positive_val: i32 = 42;
/// assert_eq!(positive_val.as_usize_narrowing(), 42usize);
/// ```
///
/// [`as_usize_narrowing`]: Self::as_usize_narrowing
pub trait NarrowingCastToUsize {
    /// Safely casts to [`usize`]. If the value overflows [`usize::MAX`], it logs a
    /// [`tracing::error!`] and saturates to [`usize::MAX`]. For signed integers, an
    /// underflow logs and saturates to `0`.
    #[allow(clippy::wrong_self_convention)]
    fn as_usize_narrowing(self) -> usize;
}

// - The implementations perform explicit bounds checks before performing raw primitive
//   casts (such as `self as usize`).
// - Clippy's static analysis does not evaluate the preceding bounds check guards, so it
//   would otherwise raise false-positive truncation and sign-loss warnings during checks.
// - XMARK: Intentional numeric casting using as.
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
mod impl_narrowing_cast_to_usize {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl NarrowingCastToUsize for i32 {
        fn as_usize_narrowing(self) -> usize {
            if self < 0 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    // % is Display, ? is Debug.
                    tracing::error! {
                        message = "NarrowingCastToUsize::as_usize_narrowing",
                        original_type = "i32",
                        value = %self,
                        issue = "underflowed 0",
                    };
                });
                0
            } else {
                self as usize
            }
        }
    }

    impl NarrowingCastToUsize for u32 {
        fn as_usize_narrowing(self) -> usize { self as usize }
    }

    impl NarrowingCastToUsize for u64 {
        fn as_usize_narrowing(self) -> usize {
            if self > usize::MAX as u64 {
                DEBUG_TUI_BOUNDS_CHECK.then(|| {
                    tracing::error! {
                        message = "NarrowingCastToUsize::as_usize_narrowing",
                        original_type = "u64",
                        value = %self,
                        issue = "overflowed usize::MAX",
                    };
                });
                usize::MAX
            } else {
                self as usize
            }
        }
    }
}

/// Extension trait for safe, lossless widening numeric conversions to `usize`.
///
/// The `as` keyword can silently truncate values if types change during refactoring.
/// By explicitly using `as_usize_widening()`, the compiler guarantees that the
/// conversion remains a lossless widening operation.
pub trait WideningCastToUsize {
    #[allow(clippy::wrong_self_convention)]
    fn as_usize_widening(self) -> usize;
}

mod impl_widening_cast_to_usize {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl WideningCastToUsize for u8 {
        fn as_usize_widening(self) -> usize { usize::from(self) }
    }

    impl WideningCastToUsize for u16 {
        fn as_usize_widening(self) -> usize { usize::from(self) }
    }

    impl WideningCastToUsize for u32 {
        fn as_usize_widening(self) -> usize {
            // XMARK: Intentional numeric casting using as.
            #[allow(clippy::as_conversions)]
            {
                self as usize
            }
        }
    }
}

/// Extension trait for safe, lossless widening numeric conversions to `u32`.
///
/// The `as` keyword can silently truncate values if types change during refactoring.
/// By explicitly using `as_u32_widening()`, the compiler guarantees that the
/// conversion remains a lossless widening operation.
pub trait WideningCastToU32 {
    #[allow(clippy::wrong_self_convention)]
    fn as_u32_widening(self) -> u32;
}

mod impl_widening_cast_to_u32 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl WideningCastToU32 for char {
        fn as_u32_widening(self) -> u32 { u32::from(self) }
    }

    impl WideningCastToU32 for u8 {
        fn as_u32_widening(self) -> u32 { u32::from(self) }
    }

    impl WideningCastToU32 for u16 {
        fn as_u32_widening(self) -> u32 { u32::from(self) }
    }
}

/// Extension trait for safe, lossless widening numeric conversions to `u16`.
///
/// The `as` keyword can silently truncate values if types change during refactoring.
/// By explicitly using `as_u16_widening()`, the compiler guarantees that the
/// conversion remains a lossless widening operation.
pub trait WideningCastToU16 {
    #[allow(clippy::wrong_self_convention)]
    fn as_u16_widening(self) -> u16;
}

mod impl_widening_cast_to_u16 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl WideningCastToU16 for u8 {
        fn as_u16_widening(self) -> u16 { u16::from(self) }
    }
}

/// Extension trait for safe, lossless widening numeric conversions to `u8`.
/// Designed primarily for explicit `#[repr(u8)]` enum discriminants.
///
/// The `as` keyword can silently truncate values if types change during refactoring.
/// By explicitly using `as_u8_widening()`, the compiler guarantees that the
/// conversion remains a lossless widening operation.
pub trait WideningCastToU8 {
    #[allow(clippy::wrong_self_convention)]
    fn as_u8_widening(self) -> u8;
}

mod impl_widening_cast_to_u8 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl WideningCastToU8 for u8 {
        fn as_u8_widening(self) -> u8 { self }
    }
}

/// Extension trait for safe, lossless widening numeric conversions to `i32`.
///
/// The `as` keyword can silently truncate values if types change during refactoring.
/// By explicitly using `as_i32_widening()`, the compiler guarantees that the
/// conversion remains a lossless widening operation.
pub trait WideningCastToI32 {
    #[allow(clippy::wrong_self_convention)]
    fn as_i32_widening(self) -> i32;
}

mod impl_widening_cast_to_i32 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl WideningCastToI32 for u8 {
        fn as_i32_widening(self) -> i32 { i32::from(self) }
    }

    impl WideningCastToI32 for u16 {
        fn as_i32_widening(self) -> i32 { i32::from(self) }
    }

    impl WideningCastToI32 for i8 {
        fn as_i32_widening(self) -> i32 { i32::from(self) }
    }

    impl WideningCastToI32 for i16 {
        fn as_i32_widening(self) -> i32 { i32::from(self) }
    }
}

/// Extension trait for safe, lossless widening numeric conversions to `u64`.
///
/// The `as` keyword can silently truncate values if types change during refactoring.
/// By explicitly using `as_u64_widening()`, the compiler guarantees that the
/// conversion remains a lossless widening operation.
pub trait WideningCastToU64 {
    #[allow(clippy::wrong_self_convention)]
    fn as_u64_widening(self) -> u64;
}

mod impl_widening_cast_to_u64 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl WideningCastToU64 for u8 {
        fn as_u64_widening(self) -> u64 { u64::from(self) }
    }

    impl WideningCastToU64 for u16 {
        fn as_u64_widening(self) -> u64 { u64::from(self) }
    }

    impl WideningCastToU64 for u32 {
        fn as_u64_widening(self) -> u64 { u64::from(self) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NumericConversions, NumericValue, ScreenCoordinate};
    use std::ops::{Add, Sub};

    // Test implementation for unit testing
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    struct TestUnit(usize);

    impl From<usize> for TestUnit {
        fn from(value: usize) -> TestUnit { TestUnit(value) }
    }

    impl From<u16> for TestUnit {
        fn from(value: u16) -> TestUnit { TestUnit(usize::from(value)) }
    }

    impl Add for TestUnit {
        type Output = Self;
        fn add(self, other: Self) -> Self { TestUnit(self.0.saturating_add(other.0)) }
    }

    impl Sub for TestUnit {
        type Output = Self;
        fn sub(self, other: Self) -> Self { TestUnit(self.0.saturating_sub(other.0)) }
    }

    impl NumericConversions for TestUnit {
        fn as_usize(&self) -> usize { self.0 }
    }

    impl NumericValue for TestUnit {}

    impl ScreenCoordinate for TestUnit {
        fn as_u16(&self) -> u16 { self.0.as_u16_narrowing() }
    }

    #[test]
    fn test_as_usize_conversion() {
        let unit = TestUnit::from(42u16);
        assert_eq!(unit.as_usize(), 42);
    }

    #[test]
    fn test_as_u16_conversion() {
        let unit = TestUnit::from(42u16);
        assert_eq!(unit.as_u16(), 42u16);
    }

    #[test]
    fn test_from_usize() {
        let unit = TestUnit::from(123u16);
        assert_eq!(unit.as_usize(), 123);
    }

    #[test]
    fn test_from_u16() {
        let unit = TestUnit::from(456u16);
        assert_eq!(unit.as_usize(), 456);
    }

    #[test]
    fn test_is_zero_default_implementation() {
        let zero_unit = TestUnit::from(0u16);
        let non_zero_unit = TestUnit::from(42u16);

        assert!(zero_unit.is_zero());
        assert!(!non_zero_unit.is_zero());
    }

    #[test]
    fn test_zero_edge_cases() {
        // Test conversion edge cases for zero
        let zero_from_usize = TestUnit::from(0u16);
        let zero_from_u16 = TestUnit::from(0u16);

        assert!(zero_from_usize.is_zero());
        assert!(zero_from_u16.is_zero());
        assert_eq!(zero_from_usize.as_usize(), 0);
        assert_eq!(zero_from_u16.as_u16(), 0);
    }

    #[test]
    fn test_large_values() {
        // Test with larger values to ensure conversion stability
        let large_value = 65535usize;
        let unit = TestUnit::from(large_value);

        assert_eq!(unit.as_usize(), large_value);
        let expected_u16 = large_value.as_u16_narrowing();
        assert_eq!(unit.as_u16(), expected_u16);
        assert!(!unit.is_zero());
    }

    #[test]
    fn test_u16_overflow_edge_case() {
        // Test what happens when usize value exceeds u16 range
        let large_value = 70000usize; // Exceeds u16::MAX (65535)
        let unit = TestUnit::from(large_value);

        assert_eq!(unit.as_usize(), large_value);
        // This should truncate to fit in u16
        let expected_u16 = large_value.as_u16_narrowing();
        assert_eq!(unit.as_u16(), expected_u16);
        assert!(!unit.is_zero());
    }

    #[test]
    fn test_saturating_cast_to_u16() {
        // Test usize
        assert_eq!(0usize.as_u16_narrowing(), 0u16);
        assert_eq!(65535usize.as_u16_narrowing(), 65535u16);
        assert_eq!(100_000usize.as_u16_narrowing(), u16::MAX);

        // Test isize
        assert_eq!(0isize.as_u16_narrowing(), 0u16);
        assert_eq!(65535isize.as_u16_narrowing(), 65535u16);
        assert_eq!(100_000isize.as_u16_narrowing(), u16::MAX);
        assert_eq!((-10isize).as_u16_narrowing(), 0u16);

        // Test i32
        assert_eq!(0i32.as_u16_narrowing(), 0u16);
        assert_eq!(65535i32.as_u16_narrowing(), 65535u16);
        assert_eq!(100_000i32.as_u16_narrowing(), u16::MAX);
        assert_eq!((-10i32).as_u16_narrowing(), 0u16);

        // Test u32
        assert_eq!(0u32.as_u16_narrowing(), 0u16);
        assert_eq!(65535u32.as_u16_narrowing(), 65535u16);
        assert_eq!(100_000u32.as_u16_narrowing(), u16::MAX);

        // Test u16
        assert_eq!(0u16.as_u16_narrowing(), 0u16);
        assert_eq!(65535u16.as_u16_narrowing(), 65535u16);

        // Test i16
        assert_eq!(0i16.as_u16_narrowing(), 0u16);
        assert_eq!(32767i16.as_u16_narrowing(), 32767u16);
        assert_eq!((-10i16).as_u16_narrowing(), 0u16);

        // Test u64
        assert_eq!(0u64.as_u16_narrowing(), 0u16);
        assert_eq!(65535u64.as_u16_narrowing(), 65535u16);
        assert_eq!(100_000u64.as_u16_narrowing(), u16::MAX);

        // Test i64
        assert_eq!(0i64.as_u16_narrowing(), 0u16);
        assert_eq!(65535i64.as_u16_narrowing(), 65535u16);
        assert_eq!(100_000i64.as_u16_narrowing(), u16::MAX);
        assert_eq!((-10i64).as_u16_narrowing(), 0u16);
    }

    #[test]
    fn test_try_as_u16_conversion() {
        let valid_unit = TestUnit::from(500usize);
        assert_eq!(valid_unit.try_as_u16(), Some(500u16));

        let invalid_unit = TestUnit::from(70_000usize);
        assert_eq!(invalid_unit.try_as_u16(), None);
    }

    #[test]
    fn test_saturating_cast_to_u8() {
        // Test usize
        assert_eq!(0usize.as_u8_narrowing(), 0u8);
        assert_eq!(255usize.as_u8_narrowing(), 255u8);
        assert_eq!(1000usize.as_u8_narrowing(), u8::MAX);

        // Test isize
        assert_eq!(0isize.as_u8_narrowing(), 0u8);
        assert_eq!(255isize.as_u8_narrowing(), 255u8);
        assert_eq!(1000isize.as_u8_narrowing(), u8::MAX);
        assert_eq!((-10isize).as_u8_narrowing(), 0u8);

        // Test u16
        assert_eq!(0u16.as_u8_narrowing(), 0u8);
        assert_eq!(255u16.as_u8_narrowing(), 255u8);
        assert_eq!(1000u16.as_u8_narrowing(), u8::MAX);

        // Test i16
        assert_eq!(0i16.as_u8_narrowing(), 0u8);
        assert_eq!(255i16.as_u8_narrowing(), 255u8);
        assert_eq!(1000i16.as_u8_narrowing(), u8::MAX);
        assert_eq!((-10i16).as_u8_narrowing(), 0u8);

        // Test u32
        assert_eq!(0u32.as_u8_narrowing(), 0u8);
        assert_eq!(255u32.as_u8_narrowing(), 255u8);
        assert_eq!(1000u32.as_u8_narrowing(), u8::MAX);

        // Test i32
        assert_eq!(0i32.as_u8_narrowing(), 0u8);
        assert_eq!(255i32.as_u8_narrowing(), 255u8);
        assert_eq!(1000i32.as_u8_narrowing(), u8::MAX);
        assert_eq!((-10i32).as_u8_narrowing(), 0u8);

        // Test u64
        assert_eq!(0u64.as_u8_narrowing(), 0u8);
        assert_eq!(255u64.as_u8_narrowing(), 255u8);
        assert_eq!(1000u64.as_u8_narrowing(), u8::MAX);

        // Test i64
        assert_eq!(0i64.as_u8_narrowing(), 0u8);
        assert_eq!(255i64.as_u8_narrowing(), 255u8);
        assert_eq!(1000i64.as_u8_narrowing(), u8::MAX);
        assert_eq!((-10i64).as_u8_narrowing(), 0u8);
    }

    #[test]
    fn test_lossy_convert_to_byte() {
        assert_eq!(255.5f64.to_u8_lossy(), 255u8);
        assert_eq!(42.0f32.to_u8_lossy(), 42u8);
        assert_eq!(300i32.to_u8_lossy(), 44u8);
        assert_eq!(300u32.to_u8_lossy(), 44u8);
        assert_eq!(300usize.to_u8_lossy(), 44u8);
        assert_eq!(300u64.to_u8_lossy(), 44u8);
        assert_eq!(300u16.to_u8_lossy(), 44u8);
        assert_eq!(300i16.to_u8_lossy(), 44u8);
        assert_eq!((-5i8).to_u8_lossy(), 251u8);
        assert_eq!('A'.to_u8_lossy(), 65u8);
    }

    #[test]
    fn test_saturating_cast_to_usize() {
        assert_eq!(0i32.as_usize_narrowing(), 0usize);
        assert_eq!(42i32.as_usize_narrowing(), 42usize);
        assert_eq!((-10i32).as_usize_narrowing(), 0usize);

        assert_eq!(0u32.as_usize_narrowing(), 0usize);
        assert_eq!(42u32.as_usize_narrowing(), 42usize);

        assert_eq!(0u64.as_usize_narrowing(), 0usize);
        assert_eq!(42u64.as_usize_narrowing(), 42usize);
        #[cfg(target_pointer_width = "32")]
        assert_eq!((u64::MAX).as_usize_narrowing(), usize::MAX);
    }

    #[test]
    fn test_widening_cast_to_usize() {
        assert_eq!(255u8.as_usize_widening(), 255usize);
        assert_eq!(65535u16.as_usize_widening(), 65535usize);
        assert_eq!(42u32.as_usize_widening(), 42usize);
    }

    #[test]
    fn test_widening_cast_to_u32() {
        assert_eq!('a'.as_u32_widening(), 97u32);
        assert_eq!(255u8.as_u32_widening(), 255u32);
        assert_eq!(65535u16.as_u32_widening(), 65535u32);
    }

    #[test]
    fn test_widening_cast_to_u16() {
        assert_eq!(255u8.as_u16_widening(), 255u16);
    }

    #[test]
    fn test_widening_cast_to_u8() {
        assert_eq!(255u8.as_u8_widening(), 255u8);
    }

    #[test]
    fn test_widening_cast_to_i32() {
        assert_eq!(255u8.as_i32_widening(), 255i32);
        assert_eq!(65535u16.as_i32_widening(), 65535i32);
        assert_eq!((-128i8).as_i32_widening(), -128i32);
        assert_eq!((-32768i16).as_i32_widening(), -32768i32);
    }

    #[test]
    fn test_widening_cast_to_u64() {
        assert_eq!(255u8.as_u64_widening(), 255u64);
        assert_eq!(65535u16.as_u64_widening(), 65535u64);
        assert_eq!(4_294_967_295u32.as_u64_widening(), 4_294_967_295u64);
    }

    #[test]
    fn test_saturating_cast_to_i16() {
        assert_eq!(0u16.as_i16_narrowing(), 0i16);
        assert_eq!(32767u16.as_i16_narrowing(), 32767i16);
        assert_eq!(40000u16.as_i16_narrowing(), i16::MAX);

        assert_eq!(0usize.as_i16_narrowing(), 0i16);
        assert_eq!(32767usize.as_i16_narrowing(), 32767i16);
        assert_eq!(40000usize.as_i16_narrowing(), i16::MAX);

        assert_eq!(0isize.as_i16_narrowing(), 0i16);
        assert_eq!(32767isize.as_i16_narrowing(), 32767i16);
        assert_eq!(40000isize.as_i16_narrowing(), i16::MAX);
        assert_eq!((-40000isize).as_i16_narrowing(), i16::MIN);

        assert_eq!(0u32.as_i16_narrowing(), 0i16);
        assert_eq!(32767u32.as_i16_narrowing(), 32767i16);
        assert_eq!(40000u32.as_i16_narrowing(), i16::MAX);

        assert_eq!(0i32.as_i16_narrowing(), 0i16);
        assert_eq!(32767i32.as_i16_narrowing(), 32767i16);
        assert_eq!(40000i32.as_i16_narrowing(), i16::MAX);
        assert_eq!((-40000i32).as_i16_narrowing(), i16::MIN);
    }

    #[test]
    fn test_saturating_cast_to_isize() {
        assert_eq!(0u16.as_isize_narrowing(), 0isize);
        assert_eq!(65535u16.as_isize_narrowing(), 65535isize);

        assert_eq!(0usize.as_isize_narrowing(), 0isize);
        assert_eq!(1000usize.as_isize_narrowing(), 1000isize);

        assert_eq!(0u64.as_isize_narrowing(), 0isize);
        assert_eq!(1000u64.as_isize_narrowing(), 1000isize);

        assert_eq!(0i64.as_isize_narrowing(), 0isize);
        assert_eq!(1000i64.as_isize_narrowing(), 1000isize);
        assert_eq!((-1000i64).as_isize_narrowing(), -1000isize);
    }
}
