// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach. Crate-wide (Intra-crate).
pub(crate) mod render_cache; // Crate-internal render cache types used by editor_engine (not re-exported).

// Attach. Folder-local (Intra-module).
mod buffer_config_struct;
mod buffer_struct;
mod caret_locate;
mod clipboard;
mod history;
mod selection;
mod sizing; // Not re-exported.

// Re-export.
pub use buffer_config_struct::*;
pub use buffer_struct::*;
pub use caret_locate::*;
pub use clipboard::*;
pub use history::*;
pub use selection::*;
