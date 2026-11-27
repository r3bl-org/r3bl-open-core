// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach.
mod spinner_constants;
mod spinner_print;
mod spinner_render;
mod spinner_style;

// Re-export.
pub use spinner_constants::*;
pub use spinner_print::*;
pub use spinner_render::*;
pub use spinner_style::*;

// Integration tests (conditional visibility).
#[cfg(any(test, doc))]
mod spinner_integration_tests;
