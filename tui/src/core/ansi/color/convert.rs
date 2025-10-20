// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! More info:
//! - <https://tintin.mudhalla.net/info/256color/>
//! - <https://talyian.github.io/ansicolors/>

use crate::{AnsiValue, RgbValue};
use std::cmp::Ordering::Less;

#[must_use]
pub fn convert_rgb_into_ansi256(rgb_color: RgbValue) -> AnsiValue {
    use ansi_constants::{ANSI_COLOR_PALETTE, ANSI256_FROM_GRAY};
    use cube_mapping::{CubeMappingResult, calculate_cube_mapping_for_rgb_color,
                       calculate_luminance, calculate_relative_diff_between_colors};

    let luminance_approximation: usize = calculate_luminance(rgb_color).into();
    let gray_ansi256_index: u8 = ANSI256_FROM_GRAY[luminance_approximation];

    let approximate_difference_to_grayscale = {
        let gray_ansi256_index: usize = gray_ansi256_index.into();
        let rgb_value_encoded_in_u32: u32 = ANSI_COLOR_PALETTE[gray_ansi256_index];
        let gray_color = RgbValue::from(rgb_value_encoded_in_u32);
        calculate_relative_diff_between_colors(rgb_color, gray_color)
    };

    let CubeMappingResult {
        cube_ansi256_index,
        cube_rgb_color,
    } = calculate_cube_mapping_for_rgb_color(rgb_color);

    let approximate_difference_to_cube_mapped_color =
        calculate_relative_diff_between_colors(rgb_color, cube_rgb_color);

    if let Less = approximate_difference_to_cube_mapped_color
        .cmp(&approximate_difference_to_grayscale)
    {
        cube_ansi256_index.into()
    } else {
        gray_ansi256_index.into()
    }
}

#[must_use]
pub fn convert_rgb_into_grayscale(rgb_color: RgbValue) -> RgbValue {
    let (r, g, b) = (rgb_color.red, rgb_color.green, rgb_color.blue);
    let (gray_r, gray_g, gray_b) = color_utils::convert_grayscale((r, g, b));
    RgbValue {
        red: gray_r,
        green: gray_g,
        blue: gray_b,
    }
}

mod color_utils {
    #[must_use]
    pub fn linear_to_srgb(intensity: f64) -> f64 {
        if intensity <= 0.003_130_8 {
            12.92 * intensity
        } else {
            1.055f64.mul_add(intensity.powf(1.0 / 2.4), -0.055)
        }
    }

    #[must_use]
    pub fn srgb_to_linear(intensity: f64) -> f64 {
        if intensity < 0.04045 {
            intensity / 12.92
        } else {
            ((intensity + 0.055) / 1.055).powf(2.4)
        }
    }

    /// More info: <https://goodcalculators.com/rgb-to-grayscale-conversion-calculator/>
    #[must_use]
    pub fn convert_grayscale(color: (u8, u8, u8)) -> (u8, u8, u8) {
        // See https://en.wikipedia.org/wiki/Grayscale#Converting_color_to_grayscale
        const SCALE: f64 = 256.0;

        // Changing SRGB to Linear for gamma correction.
        let red = srgb_to_linear(f64::from(color.0) / SCALE);
        let green = srgb_to_linear(f64::from(color.1) / SCALE);
        let blue = srgb_to_linear(f64::from(color.2) / SCALE);

        // Converting to grayscale.
        let gray_linear = 0.299f64.mul_add(red, 0.587f64.mul_add(green, 0.114 * blue));

        // Gamma correction.
        let gray_srgb = linear_to_srgb(gray_linear);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let gray_value = (gray_srgb * SCALE) as u8;
        (gray_value, gray_value, gray_value)
    }
}

mod cube_mapping {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct CubeMappingResult {
        pub cube_ansi256_index: u8,
        pub cube_rgb_color: RgbValue,
    }

    #[must_use]
    pub fn calculate_cube_mapping_for_rgb_color(
        rgb_color: RgbValue,
    ) -> CubeMappingResult {
        let RgbValue { red, green, blue } = rgb_color;

        let red_result = calculate_cube_index_red(red);
        let green_result = calculate_cube_index_green(green);
        let blue_result = calculate_cube_index_blue(blue);

        let cube_ansi256_index = red_result.ansi256_index
            + green_result.ansi256_index
            + blue_result.ansi256_index;

        let cube_rgb_color: RgbValue = {
            let cube_rgb_value_u32_encoded = red_result.red_or_green_or_blue_value
                + green_result.red_or_green_or_blue_value
                + blue_result.red_or_green_or_blue_value;
            cube_rgb_value_u32_encoded.into()
        };

        CubeMappingResult {
            cube_ansi256_index,
            cube_rgb_color,
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct CubeIndexResult {
        ansi256_index: u8,
        red_or_green_or_blue_value: u32,
    }

    #[must_use]
    pub const fn calculate_cube_index_red(red_value: u8) -> CubeIndexResult {
        let CubeIndexResult {
            ansi256_index: index,
            red_or_green_or_blue_value,
        } = find_closest(red_value, 38, 115, 155, 196, 235);

        CubeIndexResult {
            ansi256_index: index * 36 + 16,
            red_or_green_or_blue_value: red_or_green_or_blue_value << 16,
        }
    }

    #[must_use]
    pub const fn calculate_cube_index_green(green_value: u8) -> CubeIndexResult {
        let CubeIndexResult {
            ansi256_index: cube_index,
            red_or_green_or_blue_value,
        } = find_closest(green_value, 36, 116, 154, 195, 235);

        CubeIndexResult {
            ansi256_index: cube_index * 6,
            red_or_green_or_blue_value: red_or_green_or_blue_value << 8,
        }
    }

    #[must_use]
    pub const fn calculate_cube_index_blue(blue_value: u8) -> CubeIndexResult {
        find_closest(blue_value, 35, 115, 155, 195, 235)
    }

    /// - ANSI 256 colors are represented as a `6×6×6` cube.
    /// - On each axis, the six indices map to `[0, 95, 135, 175, 215, 255]` RGB component
    ///   values.
    #[must_use]
    pub const fn find_closest(
        value: u8,
        index_1: u8,
        index_2: u8,
        index_3: u8,
        index_4: u8,
        index_5: u8,
    ) -> CubeIndexResult {
        let (ansi256_index, red_or_green_or_blue_value) = if value < index_1 {
            (0, 0)
        } else if value < index_2 {
            (1, 95)
        } else if value < index_3 {
            (2, 135)
        } else if value < index_4 {
            (3, 175)
        } else if value < index_5 {
            (4, 215)
        } else {
            (5, 255)
        };

        CubeIndexResult {
            ansi256_index,
            red_or_green_or_blue_value,
        }
    }

    /// More info: <https://developer.mozilla.org/en-US/docs/Web/Accessibility/Understanding_Colors_and_Luminance#luminance_and_perception>.
    #[must_use]
    pub fn calculate_luminance(rgb: RgbValue) -> u8 {
        let RgbValue { red, green, blue } = rgb;
        let red_f32 = f32::from(red);
        let green_f32 = f32::from(green);
        let blue_f32 = f32::from(blue);

        let red_squared = red_f32 * red_f32;
        let green_squared = green_f32 * green_f32;
        let blue_squared = blue_f32 * blue_f32;

        let number = 0.212_672_9_f32.mul_add(
            red_squared,
            0.715_152_1_f32.mul_add(green_squared, 0.072_175_f32 * blue_squared),
        );

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let it = number.sqrt() as u8;

        it
    }

    /// Calculates relative "diff" between two colors.
    /// - `d(x, x) = 0` and `d(x, y) < d(x, z)`, implies `x` being closer to `y` than to
    ///   `z`.
    /// - More info: <https://www.compuphase.com/cmetric.htm>.
    #[must_use]
    pub fn calculate_relative_diff_between_colors(
        this: RgbValue,
        other: RgbValue,
    ) -> u32 {
        let RgbValue {
            red: this_red,
            green: this_green,
            blue: this_blue,
        } = this;

        let RgbValue {
            red: other_red,
            green: other_green,
            blue: other_blue,
        } = other;

        let red_sum = i32::from(this_red) + i32::from(other_red);
        let red = i32::from(this_red) - i32::from(other_red);
        let green = i32::from(this_green) - i32::from(other_green);
        let blue = i32::from(this_blue) - i32::from(other_blue);

        let red_factor = 1024 + red_sum;
        let green_factor = 2048;
        let blue_factor = 1534 - red_sum;

        let distance = red_factor * red * red
            + green_factor * green * green
            + blue_factor * blue * blue;

        #[allow(clippy::cast_sign_loss)]
        let it = distance as u32;

        it
    }
}

mod convert_between_rgb_and_u32 {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<RgbValue> for u32 {
        fn from(rgb: RgbValue) -> Self {
            let RgbValue {
                red: r,
                green: g,
                blue: b,
            } = rgb;
            // When combining RGB values into a 32-bit color, each component occupies
            // distinct bit ranges:
            // - **`Red (r)`**: bits `16-23` (shifted left by `16`)
            // - **`Green (g)`**: bits `8-15` (shifted left by `8`)
            // - **`Blue (b)`**: bits `0-7` (no shift)
            // Since these bit ranges **`don't overlap`**, `addition` (`+`) and `bitwise
            // OR` (`|`) produce the same result
            (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
        }
    }
}

pub mod ansi_constants {
    /// Lookup table for approximate shades of gray.
    pub static ANSI256_FROM_GRAY: [u8; 256] = [
        16, 16, 16, 16, 16, 232, 232, 232, 232, 232, 232, 232, 232, 232, 233, 233, 233,
        233, 233, 233, 233, 233, 233, 233, 234, 234, 234, 234, 234, 234, 234, 234, 234,
        234, 235, 235, 235, 235, 235, 235, 235, 235, 235, 235, 236, 236, 236, 236, 236,
        236, 236, 236, 236, 236, 237, 237, 237, 237, 237, 237, 237, 237, 237, 237, 238,
        238, 238, 238, 238, 238, 238, 238, 238, 238, 239, 239, 239, 239, 239, 239, 239,
        239, 239, 239, 240, 240, 240, 240, 240, 240, 240, 240, 59, 59, 59, 59, 59, 241,
        241, 241, 241, 241, 241, 241, 242, 242, 242, 242, 242, 242, 242, 242, 242, 242,
        243, 243, 243, 243, 243, 243, 243, 243, 243, 244, 244, 244, 244, 244, 244, 244,
        244, 244, 102, 102, 102, 102, 102, 245, 245, 245, 245, 245, 245, 246, 246, 246,
        246, 246, 246, 246, 246, 246, 246, 247, 247, 247, 247, 247, 247, 247, 247, 247,
        247, 248, 248, 248, 248, 248, 248, 248, 248, 248, 145, 145, 145, 145, 145, 249,
        249, 249, 249, 249, 249, 250, 250, 250, 250, 250, 250, 250, 250, 250, 250, 251,
        251, 251, 251, 251, 251, 251, 251, 251, 251, 252, 252, 252, 252, 252, 252, 252,
        252, 252, 188, 188, 188, 188, 188, 253, 253, 253, 253, 253, 253, 254, 254, 254,
        254, 254, 254, 254, 254, 254, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 231, 231, 231, 231, 231, 231, 231, 231, 231,
    ];

    /// ANSI Color Palette.
    /// - `u32` value encodes R (u8), G (u8), B(u8).
    /// - [`RgbValue::from`](crate::RgbValue::from) can be used to convert `u32` into
    ///   `RgbValue`.
    /// - Hex literals are in the format `0xRRGGBB` without separators for consistency.
    #[allow(clippy::unreadable_literal)]
    pub static ANSI_COLOR_PALETTE: [u32; 256] = [
        // The 16 system colors as by xterm (the default).
        0x000000, 0xcd0000, 0x00cd00, 0xcdcd00, 0x0000ee, 0xcd00cd, 0x00cdcd, 0xe5e5e5,
        0x7f7f7f, 0xff0000, 0x00ff00, 0xffff00, 0x5c5cff, 0xff00ff, 0x00ffff, 0xffffff,
        // `6×6×6` cube.
        0x000000, 0x00005f, 0x000087, 0x0000af, 0x0000d7, 0x0000ff, 0x005f00, 0x005f5f,
        0x005f87, 0x005faf, 0x005fd7, 0x005fff, 0x008700, 0x00875f, 0x008787, 0x0087af,
        0x0087d7, 0x0087ff, 0x00af00, 0x00af5f, 0x00af87, 0x00afaf, 0x00afd7, 0x00afff,
        0x00d700, 0x00d75f, 0x00d787, 0x00d7af, 0x00d7d7, 0x00d7ff, 0x00ff00, 0x00ff5f,
        0x00ff87, 0x00ffaf, 0x00ffd7, 0x00ffff, 0x5f0000, 0x5f005f, 0x5f0087, 0x5f00af,
        0x5f00d7, 0x5f00ff, 0x5f5f00, 0x5f5f5f, 0x5f5f87, 0x5f5faf, 0x5f5fd7, 0x5f5fff,
        0x5f8700, 0x5f875f, 0x5f8787, 0x5f87af, 0x5f87d7, 0x5f87ff, 0x5faf00, 0x5faf5f,
        0x5faf87, 0x5fafaf, 0x5fafd7, 0x5fafff, 0x5fd700, 0x5fd75f, 0x5fd787, 0x5fd7af,
        0x5fd7d7, 0x5fd7ff, 0x5fff00, 0x5fff5f, 0x5fff87, 0x5fffaf, 0x5fffd7, 0x5fffff,
        0x870000, 0x87005f, 0x870087, 0x8700af, 0x8700d7, 0x8700ff, 0x875f00, 0x875f5f,
        0x875f87, 0x875faf, 0x875fd7, 0x875fff, 0x878700, 0x87875f, 0x878787, 0x8787af,
        0x8787d7, 0x8787ff, 0x87af00, 0x87af5f, 0x87af87, 0x87afaf, 0x87afd7, 0x87afff,
        0x87d700, 0x87d75f, 0x87d787, 0x87d7af, 0x87d7d7, 0x87d7ff, 0x87ff00, 0x87ff5f,
        0x87ff87, 0x87ffaf, 0x87ffd7, 0x87ffff, 0xaf0000, 0xaf005f, 0xaf0087, 0xaf00af,
        0xaf00d7, 0xaf00ff, 0xaf5f00, 0xaf5f5f, 0xaf5f87, 0xaf5faf, 0xaf5fd7, 0xaf5fff,
        0xaf8700, 0xaf875f, 0xaf8787, 0xaf87af, 0xaf87d7, 0xaf87ff, 0xafaf00, 0xafaf5f,
        0xafaf87, 0xafafaf, 0xafafd7, 0xafafff, 0xafd700, 0xafd75f, 0xafd787, 0xafd7af,
        0xafd7d7, 0xafd7ff, 0xafff00, 0xafff5f, 0xafff87, 0xafffaf, 0xafffd7, 0xafffff,
        0xd70000, 0xd7005f, 0xd70087, 0xd700af, 0xd700d7, 0xd700ff, 0xd75f00, 0xd75f5f,
        0xd75f87, 0xd75faf, 0xd75fd7, 0xd75fff, 0xd78700, 0xd7875f, 0xd78787, 0xd787af,
        0xd787d7, 0xd787ff, 0xd7af00, 0xd7af5f, 0xd7af87, 0xd7afaf, 0xd7afd7, 0xd7afff,
        0xd7d700, 0xd7d75f, 0xd7d787, 0xd7d7af, 0xd7d7d7, 0xd7d7ff, 0xd7ff00, 0xd7ff5f,
        0xd7ff87, 0xd7ffaf, 0xd7ffd7, 0xd7ffff, 0xff0000, 0xff005f, 0xff0087, 0xff00af,
        0xff00d7, 0xff00ff, 0xff5f00, 0xff5f5f, 0xff5f87, 0xff5faf, 0xff5fd7, 0xff5fff,
        0xff8700, 0xff875f, 0xff8787, 0xff87af, 0xff87d7, 0xff87ff, 0xffaf00, 0xffaf5f,
        0xffaf87, 0xffafaf, 0xffafd7, 0xffafff, 0xffd700, 0xffd75f, 0xffd787, 0xffd7af,
        0xffd7d7, 0xffd7ff, 0xffff00, 0xffff5f, 0xffff87, 0xffffaf, 0xffffd7, 0xffffff,
        // Grayscale.
        0x080808, 0x121212, 0x1c1c1c, 0x262626, 0x303030, 0x3a3a3a, 0x444444, 0x4e4e4e,
        0x585858, 0x626262, 0x6c6c6c, 0x767676, 0x808080, 0x8a8a8a, 0x949494, 0x9e9e9e,
        0xa8a8a8, 0xb2b2b2, 0xbcbcbc, 0xc6c6c6, 0xd0d0d0, 0xdadada, 0xe4e4e4, 0xeeeeee,
    ];
}

#[cfg(test)]
mod tests {
    use crate::{AnsiValue, RgbValue, TransformColor};
    use pretty_assertions::assert_eq;
    use test_case::test_case;

    #[test_case(0, 0, 0, 0)]
    #[test_case(25, 0, 95, 175)]
    #[test_case(50, 0, 255, 215)]
    #[test_case(100, 135, 135, 0)]
    #[test_case(200, 255, 0, 215)]
    #[test_case(225, 255, 215, 255)]
    #[test_case(255, 238, 238, 238)]
    fn test_ansi_to_rgb(index: u8, red: u8, green: u8, blue: u8) {
        assert_eq!(
            AnsiValue { index }.as_rgb(),
            RgbValue { red, green, blue }
        );
    }

    #[test_case(0, 0, 0, 16)]
    #[test_case(1, 2, 3, 16)]
    #[test_case(25, 25, 25, 234)]
    #[test_case(10, 25, 5, 233)]
    #[test_case(50, 100, 200, 62)]
    #[test_case(255, 255, 255, 231)]
    fn test_rgb_to_ansi(red: u8, green: u8, blue: u8, expected_index: u8) {
        assert_eq!(
            RgbValue { red, green, blue }.as_ansi().index,
            expected_index
        );
    }
}
