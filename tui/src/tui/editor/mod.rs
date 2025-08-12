// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach.
pub mod default_content;
pub mod editor_buffer;
pub mod editor_component;
pub mod editor_engine;
pub mod zero_copy_gap_buffer;

// Re-export.
pub use default_content::*;
pub use editor_buffer::*;
pub use editor_component::*;
pub use editor_engine::*;
pub use zero_copy_gap_buffer::*;

// Tests.
pub mod editor_test_fixtures;
