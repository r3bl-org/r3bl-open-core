// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::Duration;

use strum_macros::AsRefStr;

/// Minimum terminal dimensions required for the TUI to function properly.
/// These values ensure that dialog boxes and other UI elements have sufficient
/// space to render correctly without being truncated or overlapping.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinSize {
    /// Minimum number of columns (width) required for the terminal.
    Col = 65,
    /// Minimum number of rows (height) required for the terminal.
    Row = 11,
}

/// Default buffer sizes used throughout the TUI system.
/// These values provide reasonable defaults for various internal buffers
/// to ensure good performance without excessive memory usage.
#[repr(usize)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultSize {
    /// Buffer size for MPSC channels used for inter-thread communication
    /// in the main event loop. This size allows for sufficient message
    /// queuing without blocking senders under normal load.
    MainThreadSignalChannelBufferSize = 1_000,
}

/// Converts `DefaultSize` enum variants to their corresponding usize values.
impl From<DefaultSize> for usize {
    #[allow(clippy::cast_possible_truncation)]
    fn from(default_size: DefaultSize) -> Self { default_size as usize }
}

/// Default timing constants used for telemetry and performance monitoring.
/// These values control how telemetry data is filtered and rate-limited
/// to balance between useful monitoring and system performance.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultTiming {
    /// Minimum response time threshold (in microseconds) for telemetry filtering.
    /// Operations faster than this are filtered out to reduce noise in telemetry data.
    TelemetryFilterLowestResponseTimeMinMicros = 100,
    /// Rate limiting threshold (in microseconds) for telemetry reporting.
    /// This prevents telemetry from overwhelming the system during high-frequency
    /// operations. Equivalent to 16 milliseconds, targeting ~60 FPS refresh rate.
    TelemetryRateLimitTimeThresholdMicros = 16_000,
}

/// Converts `DefaultTiming` enum variants to Duration objects.
impl From<DefaultTiming> for Duration {
    #[allow(clippy::cast_possible_truncation)]
    fn from(default_timing: DefaultTiming) -> Self {
        Duration::from_micros(default_timing as u64)
    }
}

/// Unicode box-drawing characters used for rendering dialog borders and UI elements.
/// These characters provide a consistent visual style for drawing rectangular
/// borders, frames, and separators in the terminal user interface.
#[derive(Debug, Eq, PartialEq, AsRefStr)]
pub enum BorderGlyphCharacter {
    /// Top-right corner character: ╮
    #[strum(to_string = "╮")]
    TopRight,

    /// Top-left corner character: ╭
    #[strum(to_string = "╭")]
    TopLeft,

    /// Bottom-right corner character: ╯
    #[strum(to_string = "╯")]
    BottomRight,

    /// Bottom-left corner character: ╰
    #[strum(to_string = "╰")]
    BottomLeft,

    /// Horizontal line character: ─
    #[strum(to_string = "─")]
    Horizontal,

    /// Vertical line character: │
    #[strum(to_string = "│")]
    Vertical,

    /// T-junction pointing left (├ rotated): ┤
    #[strum(to_string = "┤")]
    LineUpDownLeft,

    /// T-junction pointing right: ├
    #[strum(to_string = "├")]
    LineUpDownRight,
}

/// Default character used to represent the cursor in text editing contexts.
/// The medium shade block (▒) provides good visibility while being distinguishable
/// from regular text content.
pub const DEFAULT_CURSOR_CHAR: &str = "▒";

/// Default file extension used for syntax highlighting when the actual file
/// extension cannot be determined. Markdown (.md) is chosen as a reasonable
/// default that provides basic syntax highlighting without being too intrusive.
pub const DEFAULT_SYN_HI_FILE_EXT: &str = "md";

/// Newline byte used to terminate lines.
pub const LINE_FEED_BYTE: u8 = b'\n';

/// Null byte used for padding unused capacity.
pub const NULL_BYTE: u8 = b'\0';
