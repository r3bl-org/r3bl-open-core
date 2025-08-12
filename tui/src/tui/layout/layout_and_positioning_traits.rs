// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{FlexBox, FlexBoxProps, SurfaceProps};
use crate::{CommonResult, Pos, Size};

/// Public API interface to create nested & responsive layout based UIs.
pub trait LayoutManagement {
    /// Set the origin pos (x, y) & surface size (width, height) of our box (container).
    ///
    /// # Errors
    ///
    /// Returns `LayoutErrorType::MismatchedSurfaceStart` if the stack of boxes is not
    /// empty when this method is called.
    fn surface_start(&mut self, bounds_props: SurfaceProps) -> CommonResult<()>;

    /// # Errors
    ///
    /// Returns `LayoutErrorType::MismatchedSurfaceEnd` if the stack of boxes is not empty
    /// when this method is called.
    fn surface_end(&mut self) -> CommonResult<()>;

    /// Add a new layout on the stack w/ the direction & (width, height) percentages.
    ///
    /// # Errors
    ///
    /// Returns an error if adding the box fails due to invalid layout configuration.
    fn box_start(&mut self, flex_box_props: FlexBoxProps) -> CommonResult<()>;

    /// # Errors
    ///
    /// Returns `LayoutErrorType::MismatchedBoxEnd` if the stack of boxes is empty
    /// when this method is called.
    fn box_end(&mut self) -> CommonResult<()>;
}

/// Methods that actually perform the layout and positioning.
pub trait PerformPositioningAndSizing {
    /// Update `box_cursor_pos`. This needs to be called before adding a new [`FlexBox`].
    ///
    /// # Errors
    ///
    /// Returns `LayoutErrorType::ErrorCalculatingNextBoxPos` if the current insertion
    /// position is undefined.
    fn update_insertion_pos_for_next_box(
        &mut self,
        allocated_size: Size,
    ) -> CommonResult<Pos>;

    /// Get the [`FlexBox`] at the "top" of the `stack`.
    ///
    /// # Errors
    ///
    /// Returns `LayoutErrorType::StackOfBoxesShouldNotBeEmpty` if the stack is empty.
    fn current_box(&mut self) -> CommonResult<&mut FlexBox>;

    fn no_boxes_added(&self) -> bool;

    /// Add the first [`FlexBox`] to the [`crate::Surface`].
    /// 1. This one is explicitly sized.
    /// 2. there can be only one.
    ///
    /// # Errors
    ///
    /// Returns an error if the box properties are invalid or if adding the root box
    /// fails.
    fn add_root_box(&mut self, props: FlexBoxProps) -> CommonResult<()>;

    /// Add non-root [`FlexBox`].
    ///
    /// # Errors
    ///
    /// Returns `LayoutErrorType::BoxCursorPositionUndefined` if the container box's
    /// insertion position is undefined, or other errors if adding the box fails.
    fn add_non_root_box(&mut self, props: FlexBoxProps) -> CommonResult<()>;
}
