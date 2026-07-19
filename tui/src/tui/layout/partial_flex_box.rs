// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{FlexBox, FlexBoxId};
use crate::{TuiStyle, VPBoundingBox};
use std::fmt::Debug;

/// Holds a subset of the fields in [`FlexBox`] that are required by the editor and dialog
/// engines.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct PartialFlexBox {
    pub id: FlexBoxId,
    pub style_adjusted_bounds: VPBoundingBox,
    pub maybe_computed_style: Option<TuiStyle>,
}

impl PartialFlexBox {
    #[must_use]
    pub fn get_computed_style(&self) -> Option<TuiStyle> { self.maybe_computed_style }

    #[must_use]
    pub fn get_style_adjusted_pos_and_dim(&self) -> VPBoundingBox {
        self.style_adjusted_bounds
    }
}

impl From<PartialFlexBox> for FlexBox {
    fn from(partial_flex_box: PartialFlexBox) -> FlexBox {
        Self {
            id: partial_flex_box.id,
            style_adjusted_bounds: partial_flex_box.style_adjusted_bounds,
            maybe_computed_style: partial_flex_box.get_computed_style(),
            ..FlexBox::default()
        }
    }
}

impl From<FlexBox> for PartialFlexBox {
    fn from(flex_box: FlexBox) -> PartialFlexBox { PartialFlexBox::from(&flex_box) }
}

impl From<&FlexBox> for PartialFlexBox {
    fn from(flex_box: &FlexBox) -> PartialFlexBox {
        PartialFlexBox {
            id: flex_box.id,
            style_adjusted_bounds: flex_box.style_adjusted_bounds,
            maybe_computed_style: flex_box.get_computed_style(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VPPos, VPSize, vp_col, vp_height, vp_row, vp_width};

    #[test]
    fn test_partial_flex_box_default() {
        let partial_flex_box = PartialFlexBox::default();
        assert_eq!(partial_flex_box.id, FlexBoxId::default());
        assert_eq!(
            partial_flex_box.style_adjusted_bounds.origin_pos,
            VPPos::default()
        );
        assert_eq!(
            partial_flex_box.style_adjusted_bounds.bounds_size,
            VPSize::default()
        );
        assert_eq!(partial_flex_box.maybe_computed_style, None);
    }

    #[test]
    fn test_partial_flex_box_get_computed_style() {
        let style = TuiStyle::default();
        let partial_flex_box = PartialFlexBox {
            maybe_computed_style: Some(style),
            ..Default::default()
        };
        assert_eq!(partial_flex_box.get_computed_style(), Some(style));
    }

    #[test]
    fn test_partial_flex_box_get_style_adjusted_pos_and_dim() {
        let position = vp_col(1) + vp_row(2);
        let size = vp_width(3) + vp_height(4);
        let partial_flex_box = PartialFlexBox {
            style_adjusted_bounds: VPBoundingBox {
                origin_pos: position,
                bounds_size: size,
            },
            ..Default::default()
        };
        assert_eq!(
            partial_flex_box.get_style_adjusted_pos_and_dim(),
            VPBoundingBox {
                origin_pos: position,
                bounds_size: size,
            }
        );
    }

    #[test]
    fn test_partial_flex_box_from_flex_box() {
        let style = TuiStyle::default();
        let flex_box = FlexBox {
            id: FlexBoxId::from(42),
            style_adjusted_bounds: VPBoundingBox {
                origin_pos: vp_col(5) + vp_row(10),
                bounds_size: vp_width(20) + vp_height(30),
            },
            maybe_computed_style: Some(style),
            ..Default::default()
        };

        // Test From<&FlexBox> for PartialFlexBox
        let partial_flex_box_ref = PartialFlexBox::from(&flex_box);
        assert_eq!(partial_flex_box_ref.id, FlexBoxId::from(42));
        assert_eq!(
            partial_flex_box_ref.style_adjusted_bounds,
            VPBoundingBox {
                origin_pos: vp_col(5) + vp_row(10),
                bounds_size: vp_width(20) + vp_height(30),
            }
        );
        assert_eq!(partial_flex_box_ref.maybe_computed_style, Some(style));

        // Test From<FlexBox> for PartialFlexBox (owned)
        let partial_flex_box_owned: PartialFlexBox = flex_box.into();
        assert_eq!(partial_flex_box_owned.id, FlexBoxId::from(42));
        assert_eq!(
            partial_flex_box_owned.style_adjusted_bounds,
            VPBoundingBox {
                origin_pos: vp_col(5) + vp_row(10),
                bounds_size: vp_width(20) + vp_height(30),
            }
        );
        assert_eq!(partial_flex_box_owned.maybe_computed_style, Some(style));
    }

    #[test]
    fn test_flex_box_from_partial_flex_box() {
        let style = TuiStyle::default();
        let partial_flex_box = PartialFlexBox {
            id: FlexBoxId::from(99),
            style_adjusted_bounds: VPBoundingBox {
                origin_pos: vp_col(7) + vp_row(14),
                bounds_size: vp_width(50) + vp_height(60),
            },
            maybe_computed_style: Some(style),
        };

        let flex_box: FlexBox = partial_flex_box.into();
        assert_eq!(flex_box.id, FlexBoxId::from(99));
        assert_eq!(
            flex_box.style_adjusted_bounds,
            VPBoundingBox {
                origin_pos: vp_col(7) + vp_row(14),
                bounds_size: vp_width(50) + vp_height(60),
            }
        );
        assert_eq!(flex_box.maybe_computed_style, Some(style));
    }
}
