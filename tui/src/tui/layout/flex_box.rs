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

use std::fmt::Debug;

use r3bl_core::{Position, RequestedSizePercent, Size, TuiStyle};
use serde::{Deserialize, Serialize};

use super::FlexBoxId;

/// Direction of the layout of the box.
#[non_exhaustive]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum LayoutDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// A box is a rectangle with a position and size. The direction of the box determines how
/// it's contained elements are positioned.
#[derive(Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct FlexBox {
    pub id: FlexBoxId,
    pub dir: LayoutDirection,
    pub origin_pos: Position,
    pub bounds_size: Size,
    pub style_adjusted_origin_pos: Position,
    pub style_adjusted_bounds_size: Size,
    pub requested_size_percent: RequestedSizePercent,
    pub insertion_pos_for_next_box: Option<Position>,
    pub maybe_computed_style: Option<TuiStyle>,
}

impl FlexBox {
    pub fn get_computed_style(&self) -> Option<TuiStyle> { self.maybe_computed_style }
}

impl Debug for FlexBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut vec_lines = vec![];

        // Require fields.
        vec_lines.push(format!("FlexBox id: {:?}", self.id));
        vec_lines.push(format!("dir: {:?}", self.dir));
        vec_lines.push(format!("origin_pos: {:?}", self.origin_pos));
        vec_lines.push(format!("bounds_size: {:?}", self.bounds_size));
        vec_lines.push(format!(
            "style_adjusted_origin_pos: {:?}",
            self.style_adjusted_origin_pos
        ));
        vec_lines.push(format!(
            "style_adjusted_bounds_size: {:?}",
            self.style_adjusted_bounds_size
        ));
        vec_lines.push(format!(
            "requested_size_percent: {:?}",
            self.requested_size_percent
        ));

        // Optional fields.
        match self.insertion_pos_for_next_box {
            Some(pos) => {
                vec_lines.push(format!("insertion_pos_for_next_box: {:?}", pos));
            }
            None => {
                vec_lines.push("insertion_pos_for_next_box: None".to_string());
            }
        }
        match self.maybe_computed_style {
            Some(style) => {
                vec_lines.push(format!("maybe_computed_style: {:?}", style));
            }
            None => {
                vec_lines.push("maybe_computed_style: None".to_string());
            }
        }
        write!(f, "{}", vec_lines.join("\n  - "))
    }
}

impl std::fmt::Display for FlexBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use r3bl_core::{ok, position, requested_size_percent, size, CommonResult};

    use super::*;

    #[test]
    fn test_flex_box_default() {
        let flex_box = FlexBox::default();
        assert_eq!(flex_box.id, FlexBoxId::default());
        assert_eq!(flex_box.dir, LayoutDirection::Horizontal);
        assert_eq!(flex_box.origin_pos, Position::default());
        assert_eq!(flex_box.bounds_size, Size::default());
        assert_eq!(flex_box.style_adjusted_origin_pos, Position::default());
        assert_eq!(flex_box.style_adjusted_bounds_size, Size::default());
        assert_eq!(
            flex_box.requested_size_percent,
            RequestedSizePercent::default()
        );
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
            origin_pos: position! { col_index: 1, row_index: 2 },
            bounds_size: size! { col_count: 3, row_count: 4 },
            style_adjusted_origin_pos: position! { col_index: 5, row_index: 6 },
            style_adjusted_bounds_size: size! { col_count: 7, row_count: 8 },
            requested_size_percent: requested_size_percent!(
                width: 50,
                height: 50
            ),
            insertion_pos_for_next_box: position! { col_index: 9, row_index: 10 }.into(),
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
