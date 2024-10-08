/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

// Attach source files.
pub mod flex_box;
pub mod flex_box_id;
pub mod layout_and_positioning_traits;
pub mod layout_error;
pub mod partial_flex_box;
pub mod props;
pub mod surface;

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
