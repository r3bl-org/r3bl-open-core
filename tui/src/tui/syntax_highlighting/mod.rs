// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach sources.
pub mod convert_syntect_to_styled_text;
pub mod global_syntax_resources;
pub mod intermediate_types;
pub mod md_parser_syn_hi;
pub mod pattern_matcher;
pub mod r3bl_syntect_theme;

// Re-export
pub use convert_syntect_to_styled_text::*;
pub use global_syntax_resources::*;
pub use intermediate_types::*;
pub use md_parser_syn_hi::*;
pub use pattern_matcher::*;
pub use r3bl_syntect_theme::*;
