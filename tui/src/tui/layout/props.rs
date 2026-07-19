// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{FlexBoxId, LayoutDirection};
use crate::{InlineVec, ReqSizePc, TuiStyle, VPPos, VPSize};

/// Properties that are needed to create a [`FlexBox`].
///
/// [`FlexBox`]: crate::tui::FlexBox
#[derive(Clone, Debug, Default)]
pub struct FlexBoxProps {
    pub id: FlexBoxId,
    pub dir: LayoutDirection,
    pub requested_size_percent: ReqSizePc,
    pub maybe_styles: Option<InlineVec<TuiStyle>>,
}

/// Properties that are needed to create a [`Surface`].
///
/// [`Surface`]: crate::tui::Surface
#[derive(Clone, Debug, Default)]
pub struct SurfaceProps {
    pub pos: VPPos,
    pub size: VPSize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommonResult, VPPos, ok, req_size_pc, vp_height, vp_pos, vp_width};
    use smallvec::smallvec;

    #[test]
    fn test_flex_box_props_default() {
        let props = FlexBoxProps::default();
        assert_eq!(props.id, FlexBoxId::default());
        assert_eq!(props.dir, LayoutDirection::default());
        assert_eq!(props.requested_size_percent, ReqSizePc::default());
        assert_eq!(props.maybe_styles, None);
    }

    #[test]
    fn test_flex_box_props_custom() -> CommonResult {
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
        assert_eq!(props.maybe_styles.expect("conversion error").len(), 1);

        ok!()
    }

    #[test]
    fn test_surface_props_default() {
        let props = SurfaceProps::default();
        assert_eq!(props.pos, VPPos::default());
        assert_eq!(props.size, VPSize::default());
    }

    #[test]
    fn test_surface_props_custom() {
        let props = SurfaceProps {
            pos: vp_pos(10, 20),
            size: vp_width(30) + vp_height(40),
        };
        assert_eq!(props.pos, vp_pos(10, 20));
        assert_eq!(props.size, vp_width(30) + vp_height(40));
    }
}
