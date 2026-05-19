// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach.
pub mod detect_color_support;

// Re-export.
pub use detect_color_support::*;

// Tests.
#[cfg(any(test, doc))]
pub use detect_color_integration_tests::*;
#[cfg(any(test, doc))]
pub mod detect_color_integration_tests;
