// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`OSC`] event types and definitions.
//!
//! [`OSC`]: crate::osc_codes::OscSequence

/// Represents parsed events from INCOMING [`OSC`] (Operating System Command) sequences.
///
/// These events are extracted when parsing [`OSC`] sequences received FROM applications
/// and represent the semantic meaning of those sequences. This is distinct from
/// `OscSequence` which is used to build OUTGOING [`OSC`] sequences to send TO the
/// terminal.
///
/// ## Architecture Overview
///
/// The [`OSC`] processing pipeline has two distinct directions:
///
/// ### INCOMING (Application → Terminal Emulator)
/// 1. Application sends [`OSC`] sequences (e.g., `ESC]9;4;1;50ESC\\` for 50% progress)
/// 2. [`ANSI`] parser extracts these into `OscEvent` variants
/// 3. Terminal emulator acts on these events (updates progress bar, sets title, etc.)
///
/// ### OUTGOING (Terminal Emulator → Application)  
/// 1. Terminal emulator needs to send [`OSC`] sequences
/// 2. Creates `OscSequence` instances
/// 3. Formats them using `FastStringify`/`Display` traits
/// 4. Sends formatted sequences to the application
///
/// ## Common [`OSC`] Events
///
/// - **Progress Tracking**: Build tools like cargo send progress updates
/// - **Window Management**: Applications can set terminal title/tab names
/// - **Hyperlinks**: Modern terminals support clickable links in output
///
/// ## Usage Example
///
/// ```rust
/// use r3bl_tui::OscEvent;
///
/// // Example of matching OSC events:
/// let event = OscEvent::ProgressUpdate(75);
/// match event {
///     OscEvent::ProgressUpdate(pct) => {
///         println!("Progress: {}%", pct);
///     },
///     OscEvent::SetTitleAndTab(title) => {
///         println!("Title set to: {}", title);
///     },
///     _ => {
///         println!("Other OSC event");
///     }
/// }
/// ```
///
/// ## Relationship to Other Types
///
/// - **`OscSequence`**: Builds OUTGOING [`OSC`] sequences for terminal output
/// - **`DsrRequestFromPty`**: Represents [`DSR`] requests FROM [`PTY`] requiring
///   responses
/// - **`CsiSequence`**: Builds OUTGOING [`CSI`] sequences for cursor/formatting control
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`CSI`]: crate::CsiSequence
/// [`DSR`]: crate::DsrSequence
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, PartialEq)]
pub enum OscEvent {
    /// Set specific progress value 0-100% ([`OSC`] 9;4 state 1).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    ProgressUpdate(u8),
    /// Clear/remove progress indicator ([`OSC`] 9;4 state 0).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    ProgressCleared,
    /// Build error occurred ([`OSC`] 9;4 state 2).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    BuildError,
    /// Indeterminate progress - build is running but no
    /// specific progress ([`OSC`] 9;4 state 3).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    IndeterminateProgress,
    /// Hyperlink ([`OSC`] 8) with URI and display text.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Hyperlink { uri: String, text: String },
    /// Set terminal window title and tab name ([`OSC`] 0).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    SetTitleAndTab(String),
}
