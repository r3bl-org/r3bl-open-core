/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use super::{FlexBoxId, LayoutDirection};
use crate::{InlineVec, Pos, ReqSizePc, Size, TuiStyle};

/// Properties that are needed to create a [`crate::FlexBox`].
#[derive(Clone, Debug, Default)]
pub struct FlexBoxProps {
    pub id: FlexBoxId,
    pub dir: LayoutDirection,
    pub requested_size_percent: ReqSizePc,
    pub maybe_styles: Option<InlineVec<TuiStyle>>,
}

/// Properties that are needed to create a [`crate::Surface`].
#[derive(Clone, Debug, Default)]
pub struct SurfaceProps {
    pub pos: Pos,
    pub size: Size,
}

#[cfg(test)]
mod tests {
    use smallvec::smallvec;

    use super::*;
    use crate::{col, height, ok, req_size_pc, row, width, CommonResult};

    #[test]
    fn test_flex_box_props_default() {
        let props = FlexBoxProps::default();
        assert_eq!(props.id, FlexBoxId::default());
        assert_eq!(props.dir, LayoutDirection::default());
        assert_eq!(props.requested_size_percent, ReqSizePc::default());
        assert_eq!(props.maybe_styles, None);
    }

    #[test]
    fn test_flex_box_props_custom() -> CommonResult<()> {
        let props = FlexBoxProps {
            id: FlexBoxId::from(10),
            dir: LayoutDirection::Horizontal,
            requested_size_percent: req_size_pc!(width: 50, height: 50),
            maybe_styles: Some(smallvec![TuiStyle::default()]),
        };
        assert_eq!(props.id.inner, 10);
        assert_eq!(props.dir, LayoutDirection::Horizontal);
        assert_eq!(
            props.requested_size_percent,
            req_size_pc!(width: 50, height: 50)
        );
        assert_eq!(props.maybe_styles.unwrap().len(), 1);

        ok!()
    }

    #[test]
    fn test_surface_props_default() {
        let props = SurfaceProps::default();
        assert_eq!(props.pos, Pos::default());
        assert_eq!(props.size, Size::default());
    }

    #[test]
    fn test_surface_props_custom() {
        let props = SurfaceProps {
            pos: col(10) + row(20),
            size: width(30) + height(40),
        };
        assert_eq!(props.pos, col(10) + row(20));
        assert_eq!(props.size, width(30) + height(40));
    }
}
