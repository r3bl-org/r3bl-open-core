// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{VPPos, VPSize};

/// Represents a 2D rectangular spatial boundary defined by:
/// 1. an `origin_pos` ([`VPPos`]) and
/// 2. a `bounds_size` ([`VPSize`]).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct VPBoundingBox {
    pub origin_pos: VPPos,
    pub bounds_size: VPSize,
}

impl VPBoundingBox {
    #[must_use]
    pub fn new(origin_pos: VPPos, bounds_size: VPSize) -> Self {
        Self {
            origin_pos,
            bounds_size,
        }
    }
}

impl From<(VPPos, VPSize)> for VPBoundingBox {
    fn from((origin_pos, bounds_size): (VPPos, VPSize)) -> VPBoundingBox {
        VPBoundingBox::new(origin_pos, bounds_size)
    }
}

impl From<(VPSize, VPPos)> for VPBoundingBox {
    fn from((bounds_size, origin_pos): (VPSize, VPPos)) -> VPBoundingBox {
        VPBoundingBox::new(origin_pos, bounds_size)
    }
}
