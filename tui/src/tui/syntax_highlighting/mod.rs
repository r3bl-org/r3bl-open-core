// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod convert_syntect_to_styled_text; // Cross-folder rustdoc module link (buffer_struct).

// Attach. Folder-local (Intra-module).
mod global_syntax_resources;
mod intermediate_types;
mod md_parser_syn_hi;
mod pattern_matcher;
mod r3bl_syntect_theme;

// Re-export.
pub use convert_syntect_to_styled_text::*;
pub use global_syntax_resources::*;
pub use intermediate_types::*;
pub use md_parser_syn_hi::*;
pub use pattern_matcher::*;
pub use r3bl_syntect_theme::*;
