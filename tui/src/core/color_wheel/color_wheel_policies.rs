// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Policy enums for controlling color wheel behavior.
//!
//! This module defines strategy patterns for color wheel operations:
//! - `GradientGenerationPolicy` - Controls how gradients are generated and reused
//! - `TextColorizationPolicy` - Controls how text is colorized (per character vs per
//!   word)
//!
//! These policies allow fine-grained control over performance vs quality tradeoffs
//! and different styling approaches. Previously located in
//! `color_wheel_core/policies.rs`.

use crate::TuiStyle;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum GradientGenerationPolicy {
    /// The first time this method is called it will generate a gradient w/ the number
    /// of steps. Subsequent calls will use the same gradient and index **if** the
    /// number of steps is the same. However, if the number of steps are different,
    /// then a new gradient will be generated & the index reset.
    RegenerateGradientAndIndexBasedOnTextLength,
    /// The first time this method is called it will generate a gradient w/ the number
    /// of steps. Subsequent calls will use the same gradient and index.
    ReuseExistingGradientAndIndex,
    ReuseExistingGradientAndResetIndex,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum TextColorizationPolicy {
    ColorEachCharacter(Option<TuiStyle>),
    ColorEachWord(Option<TuiStyle>),
}
