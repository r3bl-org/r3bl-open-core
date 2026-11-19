// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod editor_macros;

// Attach. Crate-wide (Intra-crate).
pub(crate) mod caret_mut; // Cross-folder rustdoc link target for caret_mut::tests (scroll_editor_content).
pub(crate) mod content_mut; // Cross-folder rustdoc module link (buffer_struct).
pub(crate) mod engine_internal_api; // Crate-internal editor API used by editor_component, test_fixtures_editor, and tests.
pub(crate) mod engine_public_api; // Cross-folder editor API used by dialog, editor_component, render_cache, and docs.
pub(crate) mod validate_buffer_mut; // Cross-folder rustdoc links (buffer_struct, scroll_editor_content).
pub(crate) mod validate_scroll_on_resize; // Cross-folder rustdoc module link (buffer_struct).
pub(crate) mod scroll_editor_content; // Cross-folder rustdoc module link (viewport_coords/pos).

// Attach. Folder-local (Intra-module).
mod engine_struct;
mod select_mode;

// Re-export.
pub use engine_public_api::*;
pub use engine_struct::*;
pub use select_mode::*;
pub use validate_buffer_mut::*;
pub use validate_scroll_on_resize::*;