// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Module for range bounds checking, construction, conversion, and range extension traits.

#![rustfmt::skip]

// Attach.
pub(crate) mod range_bounds_check; // Linked from parent bounds_check module docs.
pub(crate) mod range_construct_ext; // Linked from parent bounds_check module docs.
pub(crate) mod range_convert_ext; // Linked from parent bounds_check module docs.
pub(crate) mod range_ext; // Linked from parent bounds_check module docs.

// Re-export.
pub use range_bounds_check::*;
pub use range_construct_ext::*;
pub use range_convert_ext::*;
pub use range_ext::*;
