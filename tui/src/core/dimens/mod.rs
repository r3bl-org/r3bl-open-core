// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[macro_use] // Module-private macro used locally. [macro_export] makes it available crate-wide.
pub mod shared_macros;

// Attach source files.
pub mod caret;
pub mod col_index;
pub mod col_width;
pub mod dim;
pub mod dimens_bounds_check_impl; // Don't re-export.
pub mod pc;
pub mod pos;
pub mod req_size_pc;
pub mod row_height;
pub mod row_index;
pub mod scr_ofs;

// Re-export.
pub use caret::*;
pub use col_index::*;
pub use col_width::*;
pub use dim::*;
pub use pc::*;
pub use pos::*;
pub use req_size_pc::*;
pub use row_height::*;
pub use row_index::*;
pub use scr_ofs::*;
