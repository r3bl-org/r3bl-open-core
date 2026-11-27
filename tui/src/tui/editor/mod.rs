// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod editor_engine; // Cross-folder internal API and rustdoc links.
pub(crate) mod zero_copy_gap_buffer; // Cross-folder rustdoc module link (lib.rs).

// Attach. Folder-local (Intra-module).
mod default_content;
mod editor_buffer;
mod editor_component;

// Re-export.
pub use default_content::*;
pub use editor_buffer::*;
pub use editor_component::*;
pub use editor_engine::*;
pub use zero_copy_gap_buffer::*;

// Tests.
#[cfg(any(test, doc))]
pub(crate) mod test_fixtures_editor; // Shared editor test mocks used by dialog tests.
