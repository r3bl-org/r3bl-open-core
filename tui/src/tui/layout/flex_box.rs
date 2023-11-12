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

use std::fmt::{Debug, Display};

use r3bl_rs_utils_core::*;
use serde::{Deserialize, Serialize};

use crate::*;

/// Direction of the layout of the box.
#[non_exhaustive]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum LayoutDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// This works w/ the [int-enum](https://crates.io/crates/int-enum) crate in order to
/// allow for the definition of enums that are represented in memory as [u8]s.
#[derive(Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct FlexBoxId(pub u8);

mod flexbox_id_impl {
    use std::ops::Deref;

    use super::*;

    impl From<FlexBoxId> for u8 {
        fn from(id: FlexBoxId) -> Self { id.0 }
    }

    impl From<u8> for FlexBoxId {
        fn from(id: u8) -> Self { Self(id) }
    }

    impl Deref for FlexBoxId {
        type Target = u8;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl FlexBoxId {
        fn pretty_print(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "🔑┆id: {}┆", self.0)
        }
    }

    impl Debug for FlexBoxId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.pretty_print(f)
        }
    }

    impl Display for FlexBoxId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.pretty_print(f)
        }
    }
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
    pub maybe_computed_style: Option<Style>,
}

mod flex_box_impl {
    use super::*;

    impl FlexBox {
        pub fn get_computed_style(&self) -> Option<Style> { self.maybe_computed_style }
    }

    impl Debug for FlexBox {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FlexBox")
                .field("id", &self.id)
                .field("dir", &self.dir)
                .field("origin_pos", &self.origin_pos)
                .field("bounds_size", &self.bounds_size)
                .field("style_adjusted_origin_pos", &self.style_adjusted_origin_pos)
                .field(
                    "style_adjusted_bounds_size",
                    &self.style_adjusted_bounds_size,
                )
                .field("requested_size_percent", &self.requested_size_percent)
                .field(
                    "insertion_pos_for_next_box",
                    format_option!(&self.insertion_pos_for_next_box),
                )
                .field(
                    "maybe_computed_style",
                    format_option!(&self.maybe_computed_style),
                )
                .finish()
        }
    }
}

/// Holds a subset of the fields in [FlexBox] that are required by the editor and dialog
/// engines.
#[derive(Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialFlexBox {
    pub id: FlexBoxId,
    pub style_adjusted_origin_pos: Position,
    pub style_adjusted_bounds_size: Size,
    pub maybe_computed_style: Option<Style>,
}

mod partial_flex_box_impl {
    use super::*;

    impl PartialFlexBox {
        pub fn get_computed_style(&self) -> Option<Style> { self.maybe_computed_style }

        pub fn get_style_adjusted_position_and_size(&self) -> (Position, Size) {
            (
                self.style_adjusted_origin_pos,
                self.style_adjusted_bounds_size,
            )
        }
    }

    impl Debug for PartialFlexBox {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FlexBox")
                .field("id", &self.id)
                .field("style_adjusted_origin_pos", &self.style_adjusted_origin_pos)
                .field(
                    "style_adjusted_bounds_size",
                    &self.style_adjusted_bounds_size,
                )
                .field(
                    "maybe_computed_style",
                    format_option!(&self.maybe_computed_style),
                )
                .finish()
        }
    }

    impl From<PartialFlexBox> for FlexBox {
        fn from(engine_box: PartialFlexBox) -> Self {
            Self {
                id: engine_box.id,
                style_adjusted_origin_pos: engine_box.style_adjusted_origin_pos,
                style_adjusted_bounds_size: engine_box.style_adjusted_bounds_size,
                maybe_computed_style: engine_box.get_computed_style(),
                ..Default::default()
            }
        }
    }

    impl From<FlexBox> for PartialFlexBox {
        fn from(flex_box: FlexBox) -> Self { PartialFlexBox::from(&flex_box) }
    }

    impl From<&FlexBox> for PartialFlexBox {
        fn from(flex_box: &FlexBox) -> Self {
            Self {
                id: flex_box.id,
                style_adjusted_origin_pos: flex_box.style_adjusted_origin_pos,
                style_adjusted_bounds_size: flex_box.style_adjusted_bounds_size,
                maybe_computed_style: flex_box.get_computed_style(),
            }
        }
    }
}
