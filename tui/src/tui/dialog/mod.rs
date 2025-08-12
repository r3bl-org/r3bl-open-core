// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach sources.
pub mod dialog_buffer;
pub mod dialog_component;
pub mod dialog_engine;

// Re-export.
pub use dialog_buffer::*;
pub use dialog_component::*;
pub use dialog_engine::*;

// Tests.
pub mod test_dialog;
