// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::modes::{MouseTrackingFormat, MouseTrackingMode,
                          terminal_mode_state_todo};
use crate::CursorKeyMode;

/// Internal offscreen buffer state for the alternate screen.
///
/// This represents the *internal state* of the engine, tracking whether the grid
/// buffers are currently swapped.
///
/// For the external VT100 mode requested by the child process, see
/// [`RequestedScreenMode`].
///
/// [`RequestedScreenMode`]: crate::RequestedScreenMode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveScreenBuffer {
    /// Alternate screen buffer is actively displayed.
    Alternate,

    /// Alternate screen buffer is inactive, primary screen is displayed.
    #[default]
    Primary,
}

/// State tracking for terminal operational modes.
///
/// Used by the [`VT-100`] [`ANSI`] parser performer ([`AnsiToOfsBufPerformer`])
/// to maintain state information about the operational modes requested by the
/// underlying [`PTY`] process.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]: crate::core::ansi::AnsiToOfsBufPerformer
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalModeState {
    /// Cursor key mode status ([`DECCKM`]).
    ///
    /// Controls whether cursor keys (arrows, home, end) send normal or application
    /// escape sequences.
    ///
    /// Toggled by the [`AnsiToOfsBufPerformer`] when processing the `ESC [ ? 1 h` and
    /// `ESC [ ? 1 l` sequences.
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::core::ansi::AnsiToOfsBufPerformer
    /// [`DECCKM`]: https://vt100.net/docs/vt100-ug/chapter3.html#DECCKM
    pub cursor_key_mode: CursorKeyMode,

    /// Alternate screen buffer status.
    ///
    /// When active, terminal output is redirected to an alternate screen buffer,
    /// preserving the original screen content.
    ///
    /// Toggled by the [`AnsiToOfsBufPerformer`] when processing the `ESC [ ? 1049 h`
    /// and `ESC [ ? 1049 l` sequences.
    ///
    /// [`AnsiToOfsBufPerformer`]: crate::core::ansi::AnsiToOfsBufPerformer
    pub active_screen_buffer: ActiveScreenBuffer,

    /// Mouse tracking enabled/disabled state.
    pub mouse_tracking_mode: MouseTrackingMode,

    /// Mouse tracking encoding format requested by the app - [`X10`] or [Sgr].
    ///
    /// See the [implementation note] in [`MouseTrackingFormat`] for exact details on how
    /// events are routed and formatted based on the app's requested protocols.
    ///
    /// [`MouseTrackingFormat`]: crate::MouseTrackingFormat
    /// [`X10`]: MouseTrackingFormat::X10
    /// [implementation note]: crate::MouseTrackingFormat#implementation-note
    /// [Sgr]: MouseTrackingFormat::Sgr
    pub mouse_tracking_format: MouseTrackingFormat,

    /// [`Bracketed paste`] mode status (`CSI ? 2004 h` / `l`).
    ///
    /// When enabled, the app wraps pasted text in `CSI 200 ~` and `CSI 201 ~` so the
    /// [`PTY`] child can tell pasted input from typed input. See
    /// [`Self::is_bracketed_paste_enabled()`].
    ///
    /// [`Bracketed paste`]: https://en.wikipedia.org/wiki/Bracketed-paste
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub bracketed_paste: terminal_mode_state_todo::BracketedPasteMode,

    /// Focus event reporting mode.
    ///
    /// When enabled (by `CSI ? 1004 h`), the PTY child wants to receive
    /// `CSI I` / `CSI O` focus events from the terminal multiplexer.
    pub focus_events: bool,
}

impl TerminalModeState {
    /// Whether the [`PTY`] child has enabled bracketed paste mode (`CSI ? 2004 h`).
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    #[must_use]
    pub fn is_bracketed_paste_enabled(&self) -> bool {
        self.bracketed_paste == terminal_mode_state_todo::BracketedPasteMode::Enabled
    }
}
