// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Module for range bounds checking, construction, conversion, and range extension traits.

#![rustfmt::skip]

// Attach.
#[cfg(any(test, doc))]
pub mod range_bounds_check;
#[cfg(not(any(test, doc)))]
mod range_bounds_check;

#[cfg(any(test, doc))]
pub mod range_construct_ext;
#[cfg(not(any(test, doc)))]
mod range_construct_ext;

#[cfg(any(test, doc))]
pub mod range_convert_ext;
#[cfg(not(any(test, doc)))]
mod range_convert_ext;

#[cfg(any(test, doc))]
pub mod range_ext;
#[cfg(not(any(test, doc)))]
mod range_ext;

// Re-export.
pub use range_bounds_check::*;
pub use range_construct_ext::*;
pub use range_convert_ext::*;
pub use range_ext::*;
