// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{fmt::Debug,
          ops::{Index, IndexMut}};
use strum::EnumCount;
use strum_macros::EnumCount; // This allows ZOrder::COUNT to be written below.

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, EnumCount)]
pub enum ZOrder {
    #[default]
    Normal,
    High,
    Glass,
}

impl ZOrder {
    /// Contains the priority that is used to paint the different groups of render
    /// operations: [`RenderOpCommon`], [`RenderOpIR`] and [`RenderOpOutput`] (operations
    /// at different Z orders).
    ///
    /// [`RenderOpCommon`]: crate::RenderOpCommon
    /// [`RenderOpIR`]: crate::RenderOpIR
    /// [`RenderOpOutput`]: crate::RenderOpOutput
    #[must_use]
    pub fn get_render_order() -> [ZOrder; ZOrder::COUNT] {
        [ZOrder::Normal, ZOrder::High, ZOrder::Glass]
    }
}

impl<T> Index<ZOrder> for [T; ZOrder::COUNT] {
    type Output = T;

    fn index(&self, index: ZOrder) -> &Self::Output { &self[index as usize] }
}

impl<T> IndexMut<ZOrder> for [T; ZOrder::COUNT] {
    fn index_mut(&mut self, index: ZOrder) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z_order_discriminants() {
        assert_eq!(ZOrder::Normal as usize, 0);
        assert_eq!(ZOrder::High as usize, 1);
        assert_eq!(ZOrder::Glass as usize, 2);
        assert_eq!(ZOrder::COUNT, 3);
    }

    #[test]
    fn test_z_order_index() {
        let array: [&str; ZOrder::COUNT] = ["normal_layer", "high_layer", "glass_layer"];

        // Test immutable Index
        assert_eq!(array[ZOrder::Normal], "normal_layer");
        assert_eq!(array[ZOrder::High], "high_layer");
        assert_eq!(array[ZOrder::Glass], "glass_layer");
    }

    #[test]
    fn test_z_order_index_mut() {
        let mut array: [i32; ZOrder::COUNT] = [10, 20, 30];

        // Test mutable IndexMut
        array[ZOrder::Normal] += 5;
        array[ZOrder::High] = 99;
        array[ZOrder::Glass] -= 10;

        assert_eq!(array[ZOrder::Normal], 15);
        assert_eq!(array[ZOrder::High], 99);
        assert_eq!(array[ZOrder::Glass], 20);
    }
}
