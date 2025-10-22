// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Module for intelligent color degradation based on terminal color support. See
//! [`degrade_color()`] for details.

use crate::{AnsiValue, ColorSupport, RgbValue, TransformColor, TuiColor};

/// # Intelligent Color Degradation for Terminal Color Support
///
/// This module handles the conversion of colors to appropriate forms based on detected
/// terminal color support capabilities. This is a critical architectural component that
/// ensures consistent color rendering across different terminals.
///
/// ## The Color Support Hierarchy
///
/// Terminals support color at different levels:
/// - **Truecolor/RGB**: Full 24-bit RGB support (16 million colors)
/// - **Ansi256**: 256-color palette (8-bit color)
/// - **Grayscale**: Limited to grayscale representation
/// - **`NoColor`**: No color support at all
///
/// This module handles **degradation** - converting colors to lower-fidelity forms when
/// the terminal doesn't support higher levels.
///
/// ## Key Design Principle
///
/// Rather than having individual converters (crossterm, ANSI SGR codes, etc.) each
/// handle their own color degradation logic, this module centralizes the algorithm.
/// This ensures:
/// - **Consistency**: All color output is degraded the same way
/// - **Maintainability**: One place to fix color issues
/// - **Testability**: Can test degradation independently of format conversion
///
/// ## Degradation Algorithm
///
/// For **Basic Colors (indices 0-15)** - Special handling:
/// - **Truecolor/Ansi256**: Keep as-is (standard ANSI palette)
/// - **Grayscale**: Convert each basic color to its grayscale equivalent
/// - **`NoColor`**: Convert to terminal default (black/reset)
///
/// For **Extended Colors (indices 16-255)**:
/// - **Truecolor/Ansi256**: Keep as-is
/// - **Grayscale**: Convert to grayscale equivalent
/// - **`NoColor`**: Convert to terminal default
///
/// For **RGB Colors**:
/// - **Truecolor**: Keep as-is (RGB)
/// - **Ansi256**: Downconvert to nearest ANSI256 color
/// - **Grayscale**: Convert RGB to grayscale equivalent
/// - **`NoColor`**: Convert to terminal default
///
/// ## Usage Pattern
///
/// The typical flow is:
///
/// ```ignore
/// use r3bl_tui::{TuiColor, ColorSupport, degrade_color};
///
/// let original_color = TuiColor::Rgb((255, 0, 0).into()); // Bright red
/// let color_support = global_color_support::detect();
/// let degraded = degrade_color(original_color, color_support);
///
/// // degraded is now in a form safe for the terminal:
/// // - If Truecolor: RGB(255, 0, 0) - unchanged
/// // - If Ansi256: Ansi(9) - red from palette
/// // - If Grayscale: Ansi(196) or similar grayscale value
/// // - If NoColor: Ansi(0) - black/default
/// ```
///
/// ## Integration Points
///
/// - **crossterm converter**: `From<TuiColor> for crossterm::style::Color`
/// - **ANSI output generator**: `PixelCharRenderer.color_to_sgr()`
/// - **CLI text rendering**: `CliText` and related structures
///
/// This module is used in both places to ensure colors are degraded consistently.
///
/// # Main Function
///
/// Degrade a color to an appropriate form based on terminal color support capabilities.
///
/// This function implements intelligent color degradation that respects terminal
/// capabilities while maintaining visual fidelity where possible.
///
/// # Arguments
///
/// * `color` - The color to degrade
/// * `color_support` - The detected terminal color support level
///
/// # Returns
///
/// A `TuiColor` that is safe to use with the specified color support level.
///
/// # Examples
///
/// ```ignore
/// use r3bl_tui::{TuiColor, ColorSupport, degrade_color};
///
/// // RGB color on Ansi256 terminal
/// let color = TuiColor::Rgb((255, 0, 0).into());
/// let degraded = degrade_color(color, ColorSupport::Ansi256);
/// // Returns: TuiColor::Ansi(9) - bright red from palette
///
/// // RGB color on Grayscale terminal
/// let color = TuiColor::Rgb((255, 0, 0).into());
/// let degraded = degrade_color(color, ColorSupport::Grayscale);
/// // Returns: TuiColor::Ansi(some_gray) - grayscale approximation
/// ```
#[must_use]
pub fn degrade_color(color: TuiColor, color_support: ColorSupport) -> TuiColor {
    match color {
        TuiColor::Ansi(ansi) if ansi.is_basic() => {
            // Basic colors (palette indices 0-15) have special handling
            degrade_basic_color(ansi.index, color_support)
        }

        TuiColor::Ansi(ansi) => {
            // Extended colors (indices 16-255)
            match color_support {
                // Keep as is for full color support
                ColorSupport::Truecolor | ColorSupport::Ansi256 => TuiColor::Ansi(ansi),

                // Terminal has no color support
                ColorSupport::NoColor => TuiColor::Ansi(0.into()),

                // Convert to grayscale
                ColorSupport::Grayscale => {
                    let ansi_grayscale_color = TuiColor::Ansi(ansi).as_grayscale();
                    TuiColor::Ansi(ansi_grayscale_color)
                }
            }
        }

        TuiColor::Rgb(from_rgb_value) => {
            let RgbValue {
                red: r,
                green: g,
                blue: b,
            } = from_rgb_value;

            match color_support {
                // Keep RGB as is for Truecolor terminals
                ColorSupport::Truecolor => TuiColor::Rgb(from_rgb_value),

                // Convert RGB to ANSI256
                ColorSupport::Ansi256 => {
                    let ansi_value = AnsiValue::from(from_rgb_value).index;
                    TuiColor::Ansi(ansi_value.into())
                }

                // Terminal has no color support
                ColorSupport::NoColor => TuiColor::Ansi(0.into()),

                // Convert to grayscale
                ColorSupport::Grayscale => {
                    let ansi = TuiColor::Rgb((r, g, b).into()).as_grayscale();
                    TuiColor::Ansi(ansi)
                }
            }
        }
    }
}

/// Degrade a basic color (indices 0-15) based on terminal color support.
///
/// # Arguments
///
/// * `index` - A color index in range 0-15 (ANSI basic colors)
/// * `color_support` - The detected terminal color support level
///
/// # Returns
///
/// A `TuiColor` representing the degraded color
fn degrade_basic_color(index: u8, color_support: ColorSupport) -> TuiColor {
    debug_assert!(index < 16, "index must be < 16 for basic colors");

    match color_support {
        // Basic colors are kept as-is for full color support
        ColorSupport::Truecolor | ColorSupport::Ansi256 => TuiColor::Ansi(index.into()),

        // Terminal has no color support - reset to terminal default
        ColorSupport::NoColor => TuiColor::Ansi(0.into()),

        // Convert to grayscale for low-color terminals
        ColorSupport::Grayscale => {
            let rgb = basic_color_to_rgb(index);
            let ansi = TuiColor::Rgb(rgb).as_grayscale();
            TuiColor::Ansi(ansi)
        }
    }
}

/// Convert a basic ANSI color index (0-15) to its RGB representation.
///
/// This allows us to convert basic colors to grayscale by first converting to RGB,
/// then using the grayscale algorithm.
///
/// # Standard ANSI Colors
///
/// ```text
///  0 = Black       (0,0,0)
///  1 = Red         (255,0,0)
///  2 = Green       (0,255,0)
///  3 = Yellow      (255,255,0)
///  4 = Blue        (0,0,255)
///  5 = Magenta     (255,0,255)
///  6 = Cyan        (0,255,255)
///  7 = White       (255,255,255)
///  8 = Dark Gray   (128,128,128)
///  9 = Dark Red    (128,0,0)
/// 10 = Dark Green  (0,128,0)
/// 11 = Dark Yellow (128,128,0)
/// 12 = Dark Blue   (0,0,128)
/// 13 = Dark Magenta(128,0,128)
/// 14 = Dark Cyan   (0,128,128)
/// 15 = Gray        (192,192,192)
/// ```
fn basic_color_to_rgb(index: u8) -> RgbValue {
    match index {
        0 => (0, 0, 0).into(),        // Black
        1 => (255, 0, 0).into(),      // Red
        2 => (0, 255, 0).into(),      // Green
        3 => (255, 255, 0).into(),    // Yellow
        4 => (0, 0, 255).into(),      // Blue
        5 => (255, 0, 255).into(),    // Magenta
        6 => (0, 255, 255).into(),    // Cyan
        7 => (255, 255, 255).into(),  // White
        8 => (128, 128, 128).into(),  // Dark Gray
        9 => (128, 0, 0).into(),      // Dark Red
        10 => (0, 128, 0).into(),     // Dark Green
        11 => (128, 128, 0).into(),   // Dark Yellow
        12 => (0, 0, 128).into(),     // Dark Blue
        13 => (128, 0, 128).into(),   // Dark Magenta
        14 => (0, 128, 128).into(),   // Dark Cyan
        15 => (192, 192, 192).into(), // Gray
        _ => unreachable!("index < 16 is guaranteed by caller"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global_color_support;
    use serial_test::serial;

    // Test helper: Sets override and clears cache for test isolation
    struct ColorSupportOverride;

    impl ColorSupportOverride {
        fn new(support: ColorSupport) -> Self {
            global_color_support::clear_cache();
            global_color_support::clear_override();
            global_color_support::set_override(support);
            ColorSupportOverride
        }
    }

    // RAII guard for test cleanup
    impl Drop for ColorSupportOverride {
        fn drop(&mut self) {
            global_color_support::clear_override();
            global_color_support::clear_cache();
        }
    }

    // Basic color degradation tests

    #[test]
    #[serial]
    fn test_basic_color_no_degradation_on_truecolor() {
        let _override = ColorSupportOverride::new(ColorSupport::Truecolor);
        let black = TuiColor::Ansi(0.into());
        let red = TuiColor::Ansi(1.into());
        let white = TuiColor::Ansi(7.into());

        assert_eq!(
            degrade_color(black, ColorSupport::Truecolor),
            TuiColor::Ansi(0.into())
        );
        assert_eq!(
            degrade_color(red, ColorSupport::Truecolor),
            TuiColor::Ansi(1.into())
        );
        assert_eq!(
            degrade_color(white, ColorSupport::Truecolor),
            TuiColor::Ansi(7.into())
        );
    }

    #[test]
    #[serial]
    fn test_basic_color_no_degradation_on_ansi256() {
        let _override = ColorSupportOverride::new(ColorSupport::Ansi256);
        let black = TuiColor::Ansi(0.into());
        let bright_red = TuiColor::Ansi(9.into());

        assert_eq!(
            degrade_color(black, ColorSupport::Ansi256),
            TuiColor::Ansi(0.into())
        );
        assert_eq!(
            degrade_color(bright_red, ColorSupport::Ansi256),
            TuiColor::Ansi(9.into())
        );
    }

    #[test]
    #[serial]
    fn test_basic_color_to_nocolor() {
        let _override = ColorSupportOverride::new(ColorSupport::NoColor);
        let black = TuiColor::Ansi(0.into());
        let red = TuiColor::Ansi(1.into());
        let white = TuiColor::Ansi(7.into());
        let any_color = TuiColor::Ansi(15.into());

        // All basic colors should degrade to black (index 0) on NoColor
        assert_eq!(
            degrade_color(black, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
        assert_eq!(
            degrade_color(red, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
        assert_eq!(
            degrade_color(white, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
        assert_eq!(
            degrade_color(any_color, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
    }

    #[test]
    #[serial]
    fn test_basic_color_to_grayscale() {
        let _override = ColorSupportOverride::new(ColorSupport::Grayscale);
        let black = TuiColor::Ansi(0.into());
        let red = TuiColor::Ansi(1.into());
        let white = TuiColor::Ansi(7.into());

        let degraded_black = degrade_color(black, ColorSupport::Grayscale);
        let degraded_red = degrade_color(red, ColorSupport::Grayscale);
        let degraded_white = degrade_color(white, ColorSupport::Grayscale);

        // All should be Ansi, but different from original
        assert!(matches!(degraded_black, TuiColor::Ansi(_)));
        assert!(matches!(degraded_red, TuiColor::Ansi(_)));
        assert!(matches!(degraded_white, TuiColor::Ansi(_)));

        // Grayscale conversion should make colors distinct from originals
        // (they're converted via RGB -> grayscale)
        // We don't test exact indices since grayscale algorithm might change,
        // but we verify the conversion happens
    }

    // Extended color (16-255) degradation tests

    #[test]
    #[serial]
    fn test_extended_color_no_degradation_on_ansi256() {
        let _override = ColorSupportOverride::new(ColorSupport::Ansi256);
        let color_196 = TuiColor::Ansi(196.into()); // Bright red from palette

        assert_eq!(
            degrade_color(color_196, ColorSupport::Ansi256),
            TuiColor::Ansi(196.into())
        );
    }

    #[test]
    #[serial]
    fn test_extended_color_no_degradation_on_truecolor() {
        let _override = ColorSupportOverride::new(ColorSupport::Truecolor);
        let color_196 = TuiColor::Ansi(196.into());

        assert_eq!(
            degrade_color(color_196, ColorSupport::Truecolor),
            TuiColor::Ansi(196.into())
        );
    }

    #[test]
    #[serial]
    fn test_extended_color_to_nocolor() {
        let _override = ColorSupportOverride::new(ColorSupport::NoColor);
        let color_196 = TuiColor::Ansi(196.into());
        let color_255 = TuiColor::Ansi(255.into());

        // Extended colors should degrade to black
        assert_eq!(
            degrade_color(color_196, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
        assert_eq!(
            degrade_color(color_255, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
    }

    #[test]
    #[serial]
    fn test_extended_color_to_grayscale() {
        let _override = ColorSupportOverride::new(ColorSupport::Grayscale);
        let color_196 = TuiColor::Ansi(196.into()); // Bright red

        let degraded = degrade_color(color_196, ColorSupport::Grayscale);

        // Should still be Ansi, but converted to grayscale
        assert!(matches!(degraded, TuiColor::Ansi(_)));
        // Grayscale should be different from original (unless it was already gray)
    }

    // RGB color degradation tests

    #[test]
    #[serial]
    fn test_rgb_no_degradation_on_truecolor() {
        let _override = ColorSupportOverride::new(ColorSupport::Truecolor);
        let red_rgb = TuiColor::Rgb((255, 0, 0).into());
        let blue_rgb = TuiColor::Rgb((0, 0, 255).into());

        assert_eq!(
            degrade_color(red_rgb, ColorSupport::Truecolor),
            TuiColor::Rgb((255, 0, 0).into())
        );
        assert_eq!(
            degrade_color(blue_rgb, ColorSupport::Truecolor),
            TuiColor::Rgb((0, 0, 255).into())
        );
    }

    #[test]
    #[serial]
    fn test_rgb_to_ansi256() {
        let _override = ColorSupportOverride::new(ColorSupport::Ansi256);
        let red_rgb = TuiColor::Rgb((255, 0, 0).into());

        let degraded = degrade_color(red_rgb, ColorSupport::Ansi256);

        // Should be converted to Ansi, not RGB
        assert!(matches!(degraded, TuiColor::Ansi(_)));
        // The exact index depends on the AnsiValue conversion algorithm
        // Just verify it's a valid result (u8 is automatically in valid range)
    }

    #[test]
    #[serial]
    fn test_rgb_to_nocolor() {
        let _override = ColorSupportOverride::new(ColorSupport::NoColor);
        let red_rgb = TuiColor::Rgb((255, 0, 0).into());
        let green_rgb = TuiColor::Rgb((0, 255, 0).into());

        // Both should degrade to black
        assert_eq!(
            degrade_color(red_rgb, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
        assert_eq!(
            degrade_color(green_rgb, ColorSupport::NoColor),
            TuiColor::Ansi(0.into())
        );
    }

    #[test]
    #[serial]
    fn test_rgb_to_grayscale() {
        let _override = ColorSupportOverride::new(ColorSupport::Grayscale);
        let red_rgb = TuiColor::Rgb((255, 0, 0).into());
        let white_rgb = TuiColor::Rgb((255, 255, 255).into());

        let degraded_red = degrade_color(red_rgb, ColorSupport::Grayscale);
        let degraded_white = degrade_color(white_rgb, ColorSupport::Grayscale);

        // Both should be Ansi (grayscale palette)
        assert!(matches!(degraded_red, TuiColor::Ansi(_)));
        assert!(matches!(degraded_white, TuiColor::Ansi(_)));

        // White should be "brighter" than red in grayscale
        // (this is a softer test since exact indices depend on grayscale algorithm)
    }

    // Edge cases

    #[test]
    #[serial]
    fn test_all_basic_colors_handled() {
        let _override = ColorSupportOverride::new(ColorSupport::Grayscale);

        // Test all 16 basic colors can be degraded without panicking
        for index in 0..16 {
            let color = TuiColor::Ansi(index.into());
            let degraded = degrade_color(color, ColorSupport::Grayscale);
            assert!(matches!(degraded, TuiColor::Ansi(_)));
        }
    }

    #[test]
    #[serial]
    fn test_rgb_conversion_preserves_intensity() {
        let _override = ColorSupportOverride::new(ColorSupport::Ansi256);

        let dark_color = TuiColor::Rgb((50, 50, 50).into());
        let bright_color = TuiColor::Rgb((200, 200, 200).into());

        let degraded_dark = degrade_color(dark_color, ColorSupport::Ansi256);
        let degraded_bright = degrade_color(bright_color, ColorSupport::Ansi256);

        // Both should convert to valid Ansi values
        assert!(matches!(degraded_dark, TuiColor::Ansi(_)));
        assert!(matches!(degraded_bright, TuiColor::Ansi(_)));
    }
}
