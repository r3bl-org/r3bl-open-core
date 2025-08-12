// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach.
pub mod buffer_struct;
pub mod caret_locate;
pub mod clipboard_service;
pub mod clipboard_support;
pub mod cur_index; // Not re-exported.
pub mod history; // Not re-exported.
pub mod render_cache; // Not re-exported.
pub mod selection_list;
pub mod selection_range;
pub mod selection_support;
pub mod sizing; // Not re-exported.

// Re-export.
pub use buffer_struct::*;
pub use caret_locate::*;
pub use clipboard_service::*;
pub use clipboard_support::*;
pub use selection_list::*;
pub use selection_range::*;
pub use selection_support::*;
