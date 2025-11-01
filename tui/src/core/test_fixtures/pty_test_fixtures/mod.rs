// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach private modules.
mod deadline;

// Macro module - #[macro_export] makes it available at crate root.
mod generate_pty_test;

// Export flat public API.
pub use deadline::*;
