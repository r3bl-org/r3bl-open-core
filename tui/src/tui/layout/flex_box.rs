// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::FlexBoxId;
use crate::{Pos, ReqSizePc, Size, TuiStyle, ok};
use std::fmt::Debug;

/// Direction of the layout of the box.
#[non_exhaustive]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LayoutDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// A box is a rectangle with a position and size. The direction of the box determines how
/// it's contained elements are positioned.
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
pub struct FlexBox {
    pub id: FlexBoxId,
    pub dir: LayoutDirection,
    pub origin_pos: Pos,
    pub bounds_size: Size,
    pub style_adjusted_origin_pos: Pos,
    pub style_adjusted_bounds_size: Size,
    pub requested_size_percent: ReqSizePc,
    pub insertion_pos_for_next_box: Option<Pos>,
    pub maybe_computed_style: Option<TuiStyle>,
}

impl FlexBox {
    #[must_use]
    pub fn get_computed_style(&self) -> Option<TuiStyle> { self.maybe_computed_style }
}

impl Debug for FlexBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const EOL: &str = "\n  - ";

        // Require fields.
        write!(f, "FlexBox id: {:?}{EOL}", self.id)?;
        write!(f, "dir: {:?}{EOL}", self.dir)?;
        write!(f, "origin_pos: {:?}{EOL}", self.origin_pos)?;
        write!(f, "bounds_size: {:?}{EOL}", self.bounds_size)?;
        write!(
            f,
            "style_adjusted_origin_pos: {:?}{EOL}",
            self.style_adjusted_origin_pos
        )?;
        write!(
            f,
            "style_adjusted_bounds_size: {:?}{EOL}",
            self.style_adjusted_bounds_size
        )?;
        write!(
            f,
            "requested_size_percent: {:?}{EOL}",
            self.requested_size_percent
        )?;

        // Optional fields.
        match self.insertion_pos_for_next_box {
            Some(pos) => {
                write!(f, "insertion_pos_for_next_box: {pos:?}{EOL}")?;
            }
            None => {
                write!(f, "insertion_pos_for_next_box: None{EOL}")?;
            }
        }
        // Last line.
        match self.maybe_computed_style {
            Some(style) => {
                write!(f, "maybe_computed_style: {style:?}")?;
            }
            None => {
                write!(f, "maybe_computed_style: None")?;
            }
        }

        ok!()
    }
}

impl std::fmt::Display for FlexBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flex_box_default() {
        let flex_box = FlexBox::default();
        assert_eq!(flex_box.id, FlexBoxId::default());
        assert_eq!(flex_box.dir, LayoutDirection::Horizontal);
        assert_eq!(flex_box.origin_pos, Pos::default());
        assert_eq!(flex_box.bounds_size, Size::default());
        assert_eq!(flex_box.style_adjusted_origin_pos, Pos::default());
        assert_eq!(flex_box.style_adjusted_bounds_size, Size::default());
        assert_eq!(flex_box.requested_size_percent, ReqSizePc::default());
        assert!(flex_box.insertion_pos_for_next_box.is_none());
        assert!(flex_box.maybe_computed_style.is_none());
    }

    #[test]
    fn test_flex_box_get_computed_style() {
        let mut flex_box = FlexBox::default();
        assert!(flex_box.get_computed_style().is_none());

        let style = TuiStyle::default();
        flex_box.maybe_computed_style = Some(style);
        assert_eq!(flex_box.get_computed_style(), Some(style));
    }

    #[test]
    fn test_layout_direction_default() {
        let direction = LayoutDirection::default();
        assert_eq!(direction, LayoutDirection::Horizontal);
    }
}
