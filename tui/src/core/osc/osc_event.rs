// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC event types and definitions.

/// Represents the different types of OSC events that can be handled.
#[derive(Debug, Clone, PartialEq)]
pub enum OscEvent {
    /// Set specific progress value 0-100% (OSC 9;4 state 1).
    ProgressUpdate(u8),
    /// Clear/remove progress indicator (OSC 9;4 state 0).
    ProgressCleared,
    /// Build error occurred (OSC 9;4 state 2).
    BuildError,
    /// Indeterminate progress - build is running but no
    /// specific progress (OSC 9;4 state 3).
    IndeterminateProgress,
    /// Hyperlink (OSC 8) with URI and display text.
    Hyperlink { uri: String, text: String },
}
