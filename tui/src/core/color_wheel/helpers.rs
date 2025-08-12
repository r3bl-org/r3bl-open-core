// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Helper functions for color calculations and conversions.
//!
//! This module provides utility functions for color wheel operations:
//! - `calc_fg_color()` - Calculates appropriate foreground color for contrast
//! - `get_color_tuple()` - Generates RGB color values from `ColorWheelControl`
//! - `clamp()` - Safely converts f64 to u8 values
//!
//! These functions handle the mathematical calculations for color generation
//! and ensure proper contrast ratios. Previously located in
//! `color_wheel_core/color_helpers.rs`.

use super::types::ColorWheelControl;
use crate::LossyConvertToByte;

/// More info on luminance:
/// - <https://stackoverflow.com/a/49092130/2085356>
/// - <https://stackoverflow.com/a/3118280/2085356>
#[must_use]
pub fn calc_fg_color(bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let luminance =
        0.2126 * f32::from(bg.0) + 0.7152 * f32::from(bg.1) + 0.0722 * f32::from(bg.2);
    if luminance < 140.0 {
        (255, 255, 255)
    } else {
        (0, 0, 0)
    }
}

/// Safely convert a [`f64`] to [`u8`] by clamping to the range `[0, 255]`.
#[must_use]
fn clamp(value: f64) -> u8 {
    let val_f64 = value.clamp(0.0, 255.0);
    val_f64.to_u8_lossy()
}

/// Generate an RGB color tuple based on the `ColorWheelControl` parameters.
///
/// This function uses trigonometric calculations to generate smooth color transitions.
/// The seed value determines the starting point in the color cycle.
#[must_use]
pub fn get_color_tuple(c: &ColorWheelControl) -> (u8, u8, u8) {
    // Calculate the angle for the sine functions
    let i = *c.frequency * *c.seed / *c.spread;

    // Calculate RGB components using sine waves offset by 120° (2π/3) each
    // This creates a smooth transition through the color spectrum
    let red = i.sin() * 127.00 + 128.00;
    let green = (i + (std::f64::consts::PI * 2.00 / 3.00)).sin() * 127.00 + 128.00;
    let blue = (i + (std::f64::consts::PI * 4.00 / 3.00)).sin() * 127.00 + 128.00;

    // Clamp values to valid RGB range (0-255)
    (clamp(red), clamp(green), clamp(blue))
}
