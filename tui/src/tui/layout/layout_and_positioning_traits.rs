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

use r3bl_core::{CommonResult, Pos, Size};

use super::{FlexBox, FlexBoxProps, SurfaceProps};

/// Public API interface to create nested & responsive layout based UIs.
pub trait LayoutManagement {
    /// Set the origin pos (x, y) & surface size (width, height) of our box (container).
    fn surface_start(&mut self, bounds_props: SurfaceProps) -> CommonResult<()>;

    fn surface_end(&mut self) -> CommonResult<()>;

    /// Add a new layout on the stack w/ the direction & (width, height) percentages.
    fn box_start(&mut self, flex_box_props: FlexBoxProps) -> CommonResult<()>;

    fn box_end(&mut self) -> CommonResult<()>;
}

/// Methods that actually perform the layout and positioning.
pub trait PerformPositioningAndSizing {
    /// Update `box_cursor_pos`. This needs to be called before adding a new [FlexBox].
    fn update_insertion_pos_for_next_box(
        &mut self,
        allocated_size: Size,
    ) -> CommonResult<Pos>;

    /// Get the [FlexBox] at the "top" of the `stack`.
    fn current_box(&mut self) -> CommonResult<&mut FlexBox>;

    fn no_boxes_added(&self) -> bool;

    /// Add the first [FlexBox] to the [crate::Surface].
    /// 1. This one is explicitly sized.
    /// 2. there can be only one.
    fn add_root_box(&mut self, props: FlexBoxProps) -> CommonResult<()>;

    /// Add non-root [FlexBox].
    fn add_non_root_box(&mut self, props: FlexBoxProps) -> CommonResult<()>;
}
