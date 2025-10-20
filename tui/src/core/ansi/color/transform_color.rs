// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AnsiValue, RgbValue};

pub trait TransformColor {
    /// Returns a [`RgbValue`] representation of the `self` color.
    fn as_rgb(&self) -> RgbValue;

    /// Returns the index of a color in 256-color ANSI palette approximating the `self`
    /// color.
    fn as_ansi(&self) -> AnsiValue;

    /// Returns the index of a color in 256-color ANSI palette approximating the `self`
    /// color as grayscale.
    fn as_grayscale(&self) -> AnsiValue;
}
