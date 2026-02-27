// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach source files.
#[macro_use]
pub mod layout_error;
#[macro_use]
pub mod surface;
pub mod flex_box;
pub mod flex_box_id;
pub mod layout_and_positioning_traits;
pub mod partial_flex_box;
pub mod props;

// Re-export the public items.
pub use flex_box::*;
pub use flex_box_id::*;
pub use layout_and_positioning_traits::*;
pub use layout_error::*;
pub use partial_flex_box::*;
pub use props::*;
pub use surface::*;

// Tests.
mod test_surface_2_col_complex;
mod test_surface_2_col_simple;
