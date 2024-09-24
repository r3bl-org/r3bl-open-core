/*
 *   Copyright (c) 2024 R3BL LLC
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

use r3bl_rs_utils_core::{Position, RequestedSizePercent, Size, TuiStyle};

use super::{FlexBoxId, LayoutDirection};

/// Properties that are needed to create a [FlexBox].
#[derive(Clone, Debug, Default)]
pub struct FlexBoxProps {
    pub id: FlexBoxId,
    pub dir: LayoutDirection,
    pub requested_size_percent: RequestedSizePercent,
    pub maybe_styles: Option<Vec<TuiStyle>>,
}

/// Properties that are needed to create a [crate::Surface].
#[derive(Clone, Debug, Default)]
pub struct SurfaceProps {
    pub pos: Position,
    pub size: Size,
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::{ok, position, requested_size_percent, size, CommonResult};

    use super::*;
    use crate::tui::layout::{FlexBoxId, LayoutDirection};

    #[test]
    fn test_flex_box_props_default() {
        let props = FlexBoxProps::default();
        assert_eq!(props.id, FlexBoxId::default());
        assert_eq!(props.dir, LayoutDirection::default());
        assert_eq!(
            props.requested_size_percent,
            RequestedSizePercent::default()
        );
        assert_eq!(props.maybe_styles, None);
    }

    #[test]
    fn test_flex_box_props_custom() -> CommonResult<()> {
        let props = FlexBoxProps {
            id: FlexBoxId::from(10),
            dir: LayoutDirection::Horizontal,
            requested_size_percent: requested_size_percent!(width: 50, height: 50),
            maybe_styles: Some(vec![TuiStyle::default()]),
        };
        assert_eq!(props.id.0, 10);
        assert_eq!(props.dir, LayoutDirection::Horizontal);
        assert_eq!(
            props.requested_size_percent,
            requested_size_percent!(width: 50, height: 50)
        );
        assert_eq!(props.maybe_styles.unwrap().len(), 1);

        ok!()
    }

    #[test]
    fn test_surface_props_default() {
        let props = SurfaceProps::default();
        assert_eq!(props.pos, Position::default());
        assert_eq!(props.size, Size::default());
    }

    #[test]
    fn test_surface_props_custom() {
        let props = SurfaceProps {
            pos: position!(col_index:10, row_index:20),
            size: size!(col_count:30, row_count:40),
        };
        assert_eq!(props.pos, position!(col_index:10, row_index:20));
        assert_eq!(props.size, size!(col_count:30, row_count:40));
    }
}
