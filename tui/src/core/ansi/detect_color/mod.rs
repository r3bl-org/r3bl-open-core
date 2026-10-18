// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach.
mod detect_color_support;

#[cfg(any(test, doc))]
mod detect_color_integration_tests;

// Re-export.
pub use detect_color_support::*;
