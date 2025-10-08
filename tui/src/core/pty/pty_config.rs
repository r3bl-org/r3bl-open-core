// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use portable_pty::PtySize;
use std::ops::{Add, AddAssign};

/// Configuration options that can be combined to build a PTY configuration.
///
/// These options are the building blocks that combine using the `+` operator
/// to create a [`PtyConfig`]. The combination follows a "last write wins per field"
/// strategy - each option modifies only the fields it cares about:
///
/// - `Osc` only sets `capture_osc` to true
/// - `Output` only sets `capture_output` to true
/// - `Size` only modifies `pty_size`
/// - `NoCaptureOutput` sets both capture flags to false
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{PtyConfigOption, PtyConfig};
/// use portable_pty::PtySize;
/// use PtyConfigOption::*;
///
/// // Single option (automatically converts to PtyConfig)
/// let config: PtyConfig = Osc.into();
///
/// // Combine multiple options
/// let config = Osc + Output;
///
/// // With custom size (last size wins)
/// let custom_size = PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 };
/// let config = Osc + Output + Size(custom_size);
///
/// // NoCaptureOutput overrides previous capture settings
/// let config = Osc + Output + NoCaptureOutput; // Both captures disabled
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PtyConfigOption {
    /// Capture and parse OSC sequences.
    Osc,
    /// Capture raw output data.
    Output,
    /// Set custom PTY dimensions.
    Size(PtySize),
    /// Disable all capture (sets both capture flags to false).
    NoCaptureOutput,
}

/// Final configuration for PTY command execution.
///
/// This struct is built by combining [`PtyConfigOption`] values using the `+` operator.
/// It represents the complete configuration state after all options have been applied.
///
/// # Examples
///
/// ```rust
/// use r3bl_tui::{PtyConfigOption, PtyConfig};
/// use portable_pty::PtySize;
/// use PtyConfigOption::*;
///
/// // Build from options
/// let config = Osc + Output; // Creates a PtyConfig
///
/// // Can continue adding to an existing config
/// let custom_size = PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 };
/// let config = config + Size(custom_size);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PtyConfig {
    pub capture_osc: bool,
    pub capture_output: bool,
    pub pty_size: PtySize,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            capture_osc: false,
            capture_output: true,
            pty_size: PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            },
        }
    }
}

impl PtyConfig {
    /// Check if OSC capture is enabled.
    #[must_use]
    pub fn is_osc_capture_enabled(&self) -> bool { self.capture_osc }

    /// Check if output capture is enabled.
    #[must_use]
    pub fn is_output_capture_enabled(&self) -> bool { self.capture_output }

    /// Get the PTY size configuration.
    #[must_use]
    pub fn get_pty_size(&self) -> PtySize { self.pty_size }

    /// Apply a configuration option to this config. Uses "last write wins per field"
    /// strategy.
    fn apply(&mut self, option: PtyConfigOption) {
        match option {
            PtyConfigOption::Osc => self.capture_osc = true,
            PtyConfigOption::Output => self.capture_output = true,
            PtyConfigOption::Size(size) => self.pty_size = size,
            PtyConfigOption::NoCaptureOutput => {
                self.capture_osc = false;
                self.capture_output = false;
            }
        }
    }
}

/// Convert a single option into a complete [`PtyConfig`].
impl From<PtyConfigOption> for PtyConfig {
    fn from(option: PtyConfigOption) -> Self {
        let mut config = PtyConfig::default();
        config.apply(option);
        config
    }
}

/// Combine two options to create a [`PtyConfig`].
impl Add for PtyConfigOption {
    type Output = PtyConfig;

    fn add(self, rhs: Self) -> PtyConfig {
        let mut config = PtyConfig::from(self);
        config.apply(rhs);
        config
    }
}

/// Add an option to an existing config.
impl Add<PtyConfigOption> for PtyConfig {
    type Output = PtyConfig;

    fn add(mut self, rhs: PtyConfigOption) -> PtyConfig {
        self.apply(rhs);
        self
    }
}

/// Add a config to an option (for symmetry).
impl Add<PtyConfig> for PtyConfigOption {
    type Output = PtyConfig;

    fn add(self, rhs: PtyConfig) -> PtyConfig { rhs + self }
}

/// Implement [`AddAssign`] for `+=` operator on [`PtyConfig`].
impl AddAssign<PtyConfigOption> for PtyConfig {
    fn add_assign(&mut self, rhs: PtyConfigOption) { self.apply(rhs); }
}

/// Allow creating [`PtyConfig`] from [`PtySize`] via [`PtyConfigOption`].
impl From<PtySize> for PtyConfigOption {
    fn from(size: PtySize) -> Self { PtyConfigOption::Size(size) }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_config_default() {
        let config = PtyConfig::default();
        assert!(!config.capture_osc);
        assert!(config.capture_output);
        assert_eq!(config.pty_size.rows, 24);
        assert_eq!(config.pty_size.cols, 80);
    }

    #[test]
    fn test_pty_config_accessors() {
        let config = PtyConfig::default();
        assert!(!config.is_osc_capture_enabled());
        assert!(config.is_output_capture_enabled());

        let size = config.get_pty_size();
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
    }

    #[test]
    fn test_pty_config_option_osc() {
        let config: PtyConfig = PtyConfigOption::Osc.into();
        assert!(config.capture_osc);
        assert!(config.capture_output);
    }

    #[test]
    fn test_pty_config_option_output() {
        let config: PtyConfig = PtyConfigOption::Output.into();
        assert!(!config.capture_osc);
        assert!(config.capture_output);
    }

    #[test]
    fn test_pty_config_option_size() {
        let custom_size = PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };
        let config: PtyConfig = PtyConfigOption::Size(custom_size).into();
        assert_eq!(config.pty_size, custom_size);
        assert!(!config.capture_osc);
        assert!(config.capture_output);
    }

    #[test]
    fn test_pty_config_option_no_capture_output() {
        let config: PtyConfig = PtyConfigOption::NoCaptureOutput.into();
        assert!(!config.capture_osc);
        assert!(!config.capture_output);
    }

    #[test]
    fn test_pty_config_option_add() {
        let config = PtyConfigOption::Osc + PtyConfigOption::Output;
        assert!(config.capture_osc);
        assert!(config.capture_output);
    }

    #[test]
    fn test_pty_config_option_add_with_config() {
        let config = PtyConfig::default() + PtyConfigOption::Osc;
        assert!(config.capture_osc);
        assert!(config.capture_output);
    }

    #[test]
    fn test_config_plus_option() {
        let config = PtyConfigOption::Osc + PtyConfig::default();
        assert!(config.capture_osc);
        assert!(config.capture_output);
    }

    #[test]
    fn test_config_add_assign() {
        let mut config = PtyConfig::default();
        config += PtyConfigOption::Osc;
        assert!(config.capture_osc);
        assert!(config.capture_output);
    }

    #[test]
    fn test_last_write_wins_capture_flags() {
        // Enable both, then disable.
        let config = PtyConfigOption::Osc
            + PtyConfigOption::Output
            + PtyConfigOption::NoCaptureOutput;
        assert!(!config.capture_osc);
        assert!(!config.capture_output);
    }

    #[test]
    fn test_last_write_wins_size() {
        let size1 = PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };
        let size2 = PtySize {
            rows: 40,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        };

        let config = PtyConfigOption::Size(size1) + PtyConfigOption::Size(size2);
        assert_eq!(config.pty_size, size2);
    }

    #[test]
    fn test_complex_combination() {
        let custom_size = PtySize {
            rows: 50,
            cols: 150,
            pixel_width: 0,
            pixel_height: 0,
        };

        let config = PtyConfigOption::Osc
            + PtyConfigOption::Output
            + PtyConfigOption::Size(custom_size)
            + PtyConfigOption::NoCaptureOutput
            + PtyConfigOption::Osc; // Re-enable osc after no capture

        assert!(config.capture_osc); // Re-enabled last
        assert!(!config.capture_output); // Disabled by NoCaptureOutput
        assert_eq!(config.pty_size, custom_size);
    }

    #[test]
    fn test_pty_size_conversion() {
        let size = PtySize {
            rows: 25,
            cols: 90,
            pixel_width: 0,
            pixel_height: 0,
        };

        let option: PtyConfigOption = size.into();
        match option {
            PtyConfigOption::Size(converted_size) => assert_eq!(converted_size, size),
            _ => panic!("Expected Size option"),
        }
    }

    #[test]
    fn test_pty_config_equality() {
        let config1 = PtyConfigOption::Osc + PtyConfigOption::Output;
        let config2 = PtyConfigOption::Output + PtyConfigOption::Osc;

        assert_eq!(config1, config2);
    }

    #[test]
    fn test_pty_config_option_equality() {
        assert_eq!(PtyConfigOption::Osc, PtyConfigOption::Osc);
        assert_eq!(PtyConfigOption::Output, PtyConfigOption::Output);
        assert_eq!(
            PtyConfigOption::NoCaptureOutput,
            PtyConfigOption::NoCaptureOutput
        );

        let size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };
        assert_eq!(PtyConfigOption::Size(size), PtyConfigOption::Size(size));
    }

    #[test]
    fn test_pty_config_debug() {
        let config = PtyConfigOption::Osc + PtyConfigOption::Output;
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("PtyConfig"));
        assert!(debug_str.contains("capture_osc"));
        assert!(debug_str.contains("capture_output"));
    }
}
