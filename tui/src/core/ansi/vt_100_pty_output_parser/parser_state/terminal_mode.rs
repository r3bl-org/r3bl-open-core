// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::super::modes::{MouseTrackingFormat, MouseTrackingMode,
                          terminal_mode_state_todo};
use crate::{ActiveScreenBuffer, CursorKeyMode};

/// State tracking for terminal operational modes.
///
/// Used by the [`VT-100`] [`ANSI`] parser performer ([`AnsiToOfsBufPerformer`])
/// to maintain state information about the operational modes requested by the
/// underlying [`PTY`] process.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
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
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
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
    /// [`AnsiToOfsBufPerformer`]: crate::AnsiToOfsBufPerformer
    pub active_screen_buffer: ActiveScreenBuffer,

    /// Mouse tracking enabled/disabled state.
    pub mouse_tracking_mode: MouseTrackingMode,

    /// Mouse tracking encoding format requested by the app - [X10] or [Sgr].
    ///
    /// See the [implementation note] in [`MouseTrackingFormat`] for exact details on how
    /// events are routed and formatted based on the app's requested protocols.
    ///
    /// [`MouseTrackingFormat`]: crate::MouseTrackingFormat
    /// [implementation note]: crate::MouseTrackingFormat#implementation-note
    /// [Sgr]: MouseTrackingFormat::Sgr
    /// [X10]: MouseTrackingFormat::X10
    pub mouse_tracking_format: MouseTrackingFormat,

    /// Bracketed paste mode status.
    ///
    /// **TODO**: The parser currently ignores this [`VT-100`] sequence
    /// (`vt_100_shim_mode_ops.rs`) because the [`PTY`] multiplexer does not yet route
    /// complex input events. When supported, this field should be wired up to the
    /// [`ANSI`] parser and the `dead_code` allowance removed.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[allow(dead_code)]
    pub bracketed_paste: terminal_mode_state_todo::BracketedPasteMode,
}
