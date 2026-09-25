// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`OSC`] event types emitted by child processes running in a [`PTY`].
//!
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

use crate::Pc;

/// Represents parsed events from incoming [`OSC`] (Operating System Command) sequences
/// emitted by child processes running inside a [`PTY`].
///
/// These events are extracted when parsing [`OSC`] sequences received from child
/// processes (e.g. by [`PtyOscProgressScanner`]) and represent the semantic meaning of
/// those sequences. This is distinct from [`OscSequence`] which is used to build outgoing
/// [`OSC`] sequences to send to the host terminal.
///
/// ## Architecture Overview
///
/// The [`OSC`] processing pipeline has two distinct directions:
///
/// ### INCOMING (Child Process in [`PTY`] -> Terminal Emulator / Host)
/// 1. Child process sends [`OSC`] sequences (e.g., `ESC ] 9 ; 4 ; 1 ; 50 ST` for 50%
///    progress).
/// 2. Stream scanner or [`ANSI`] parser extracts these into [`OscPtyEvent`] variants.
/// 3. Terminal emulator or [`pty`] acts on these events (updates progress bar, sets
///    title, etc.).
///
/// ### OUTGOING (Host / Terminal Emulator -> Terminal Display)
/// 1. Terminal emulator or application needs to send [`OSC`] sequences.
/// 2. Creates [`OscSequence`] instances via [`OscSender`].
/// 3. Formats them using `FastStringify`/`Display` traits.
/// 4. Sends formatted sequences over `stdout`.
///
/// ## Common [`OSC`] Events
///
/// - **Progress Tracking**: Build tools like cargo send progress updates.
/// - **Window Management**: Applications can set terminal title/tab names.
/// - **Hyperlinks**: Modern terminals support clickable links in output.
///
/// ## Usage Example
///
/// ```rust
/// use r3bl_tui::{OscPtyEvent, pc};
///
/// // Example of matching OSC events from a PTY:
/// let event = OscPtyEvent::ProgressUpdate(pc!(75).unwrap());
/// match event {
///     OscPtyEvent::ProgressUpdate(pct) => {
///         println!("Progress: {}%", *pct);
///     }
///     OscPtyEvent::SetTitleAndTab(title) => {
///         println!("Title set to: {title}");
///     }
///     _ => {
///         println!("Other OSC event");
///     }
/// }
/// ```
///
/// ## Relationship to Other Types
///
/// - **[`OscSequence`]**: Builds outgoing [`OSC`] sequences for terminal output.
/// - **[`PtyResponseEvent`]**: Represents [`DSR`] responses sent back to the [`PTY`].
/// - **[`CsiSequence`]**: Builds outgoing [`CSI`] sequences for cursor/formatting
///   control.
/// - **[`PtyOscProgressScanner`]**: Scans child stream and extracts [`OscPtyEvent`]
///   instances.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`CSI`]: crate::CsiSequence
/// [`CsiSequence`]: crate::CsiSequence
/// [`DSR`]: crate::DsrSequence
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`OscSender`]: crate::OscSender
/// [`OscSequence`]: crate::core::ansi::osc::OscSequence
/// [`pty`]: crate::core::pty
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtyOscProgressScanner`]: crate::PtyOscProgressScanner
/// [`PtyResponseEvent`]: crate::PtyResponseEvent
#[derive(Debug, Clone, PartialEq)]
pub enum OscPtyEvent {
    /// Set specific progress value 0-100% ([`OSC`] 9;4 state 1).
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    ProgressUpdate(Pc),
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
