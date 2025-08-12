// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::{Add, AddAssign};

use portable_pty::PtySize;

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
