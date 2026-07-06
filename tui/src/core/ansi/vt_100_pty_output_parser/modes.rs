// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Auto-wrap mode ([`DECAWM`]) state.
///
/// Controls line wrapping behavior when text reaches the right margin.
///
/// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoWrapMode {
    /// Characters automatically wrap to the next line (DECAWM `?7h`)
    #[default]
    Enabled,

    /// Characters overwrite at the right margin (DECAWM `?7l`)
    Disabled,
}

/// Terminal cursor visibility state.
///
/// Controls whether the terminal cursor is displayed or hidden. Corresponds to the
/// [`DECTCEM`] (`?25`) private mode.
///
/// # Usage in [`PTY`] Mux
///
/// When used inside [`ParserGlobalState::cursor_visibility`], it stores the *requested*
/// visibility state of the child process. The [`PTY Mux`] compositor
/// ([`OutputRenderer::composite_virtual_cursor_into_buffer`]) reads this to determine if
/// it needs to paint a simulated, virtual block cursor into the [`OfsBuf`].
///
/// > Note: The host terminal emulator's actual cursor is permanently suppressed via
/// > [`hide_cursor`] when the multiplexer is active. We rely exclusively
/// > on the virtual block cursor rendering (which allows us to have multiple cursors).
///
/// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
/// [`hide_cursor`]: crate::TerminalModeController::hide_cursor
/// [`OfsBuf`]: crate::OfsBuf
/// [`OutputRenderer::composite_virtual_cursor_into_buffer`]:
///     crate::core::pty::OutputRenderer::composite_virtual_cursor_into_buffer
/// [`ParserGlobalState::cursor_visibility`]: crate::ParserGlobalState::cursor_visibility
/// [`PTY Mux`]: crate::PTYMux
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorVisibilityMode {
    /// Cursor is visible ([`DECTCEM`] `ESC [ ? 25 h`)
    ///
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    #[default]
    Visible,

    /// Cursor is hidden ([`DECTCEM`] `ESC [ ? 25 l`)
    ///
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    Hidden,
}

/// Mouse event tracking state.
///
/// Controls whether the terminal captures and reports mouse events.
///
/// # Implementation Note
///
/// [`PTYMux`] routes mouse events using the following logic:
///
/// - Rather than strictly adhering to the legacy [`1000`]/[`1002`]/[`1003`] protocols for
///   event filtering (e.g. clicks only vs cell motion), we treat any request for mouse
///   tracking identically and route all events (clicks, drags, motion).
/// - If a child process requests *any* mouse tracking, we route those events back using
///   precise byte sequence format requested by the app.
///   - When an app requests basic tracking ([`1000`], [`1002`], or [`1003`]), it defaults
///     to the [X10] format.
///   - It will only use the modern [Sgr] format if the app explicitly requests
///     [`PrivateModeType::SgrMouseMode`].
///
/// [`1000`]: crate::PrivateModeType::X11MouseTracking
/// [`1002`]: crate::PrivateModeType::CellMotionMouseTracking
/// [`1003`]: crate::PrivateModeType::ApplicationMouseTracking
/// [`PrivateModeType::SgrMouseMode`]: crate::PrivateModeType::SgrMouseMode
/// [`PTYMux`]: crate::PTYMux
/// [`SGR`]: crate::SgrCode
/// [Sgr]: MouseTrackingFormat::Sgr
/// [X10]: MouseTrackingFormat::X10
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseTrackingFormat {
    /// [`X10`] format (e.g. `\x1b[M...`)
    ///
    /// [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
    #[default]
    X10,
    /// Modern [`SGR`] format (e.g. `\x1b[<...`)
    ///
    /// [`SGR`]: crate::SgrCode
    Sgr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseTrackingMode {
    /// Mouse tracking enabled.
    Enabled,

    /// Mouse tracking disabled.
    #[default]
    Disabled,
}

/// The [`VT-100`] requested screen buffer mode.
///
/// This represents the *mode* requested by the external terminal application via escape
/// sequences (e.g., `ESC [ ? 1049 h` for Alternate, `l` for Primary).
///
/// The engine processes this requested mode and updates its internal
/// [`ActiveScreenBuffer`] accordingly (which performs the actual buffer swapping).
///
/// [`ActiveScreenBuffer`]: crate::ActiveScreenBuffer
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedScreenMode {
    /// Request to use the primary (default) screen buffer.
    Primary,
    /// Request to use the alternate screen buffer (preserving primary content).
    Alternate,
}

pub mod terminal_mode_state_todo {
    /// Bracketed paste mode state.
    ///
    /// Controls whether text pasted from clipboard is wrapped with special escape
    /// sequences (`OSC 52`), allowing applications to distinguish pasted text from
    /// keyboard input.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    #[allow(dead_code)]
    pub enum BracketedPasteMode {
        /// Bracketed paste mode enabled
        Enabled,
        /// Bracketed paste mode disabled
        #[default]
        Disabled,
    }
}
