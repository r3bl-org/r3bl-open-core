/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use std::fmt::Debug;

use super::FlexBoxId;
use crate::{ok, Pos, ReqSizePc, Size, TuiStyle};

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
                write!(f, "insertion_pos_for_next_box: {:?}{EOL}", pos)?;
            }
            None => {
                write!(f, "insertion_pos_for_next_box: None{EOL}")?;
            }
        }
        // Last line.
        match self.maybe_computed_style {
            Some(style) => {
                write!(f, "maybe_computed_style: {:?}", style)?;
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
    use crate::{col, height, ok, req_size_pc, row, width, CommonResult, Pos, Size};

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

    #[test]
    fn test_flex_box_debug() -> CommonResult<()> {
        let flex_box = FlexBox {
            id: FlexBoxId::default(),
            dir: LayoutDirection::Vertical,
            origin_pos: col(1) + row(2),
            bounds_size: width(3) + height(4),
            style_adjusted_origin_pos: col(5) + row(6),
            style_adjusted_bounds_size: width(7) + height(8),
            requested_size_percent: req_size_pc!(
                width: 50,
                height: 50
            ),
            insertion_pos_for_next_box: Some(col(9) + row(10)),
            maybe_computed_style: TuiStyle::default().into(),
        };

        let debug_str = format!("{:?}", flex_box);
        assert!(debug_str.contains("FlexBox"));
        assert!(debug_str.contains("id"));
        assert!(debug_str.contains("dir"));
        assert!(debug_str.contains("origin_pos"));
        assert!(debug_str.contains("bounds_size"));
        assert!(debug_str.contains("style_adjusted_origin_pos"));
        assert!(debug_str.contains("style_adjusted_bounds_size"));
        assert!(debug_str.contains("requested_size_percent"));
        assert!(debug_str.contains("insertion_pos_for_next_box"));
        assert!(debug_str.contains("maybe_computed_style"));

        ok!()
    }
}
