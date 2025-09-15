// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::fmt::Debug;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum ZOrder {
    #[default]
    Normal,
    High,
    Glass,
}

impl ZOrder {
    /// Contains the priority that is used to paint the different groups of
    /// [`crate::RenderOp`] items.
    #[must_use]
    pub fn get_render_order() -> [ZOrder; 3] {
        [ZOrder::Normal, ZOrder::High, ZOrder::Glass]
    }
}
