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
/// it needs to paint a simulated, virtual block cursor into the [`OffscreenBuffer`].
///
/// > Note: The host terminal emulator's actual cursor is permanently suppressed via
/// > [`hide_cursor`] when the multiplexer is active. We rely exclusively
/// > on the virtual block cursor rendering (which allows us to have multiple cursors).
///
/// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
/// [`hide_cursor`]: crate::TerminalModeController::hide_cursor
/// [`OffscreenBuffer`]: crate::OffscreenBuffer
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
/// [`PTYMux`] uses a simplified "firehose" approach for mouse events:
///
/// - Rather than strictly adhering to the legacy [`1000`]/[`1002`]/[`1003`] protocols
///   (which differ in event filtering and byte-encoded coordinates), we treat any request
///   for mouse tracking identically.
/// - If a sub-process requests *any* mouse tracking, we route all mouse events (clicks,
///   drags, motion) using the modern `1006` [`SGR`] [`PrivateModeType::SgrMouseMode`]
///   protocol format unconditionally. This satisfies 99% of modern TUI apps.
///
/// [`1000`]: crate::PrivateModeType::X11MouseTracking
/// [`1002`]: crate::PrivateModeType::CellMotionMouseTracking
/// [`1003`]: crate::PrivateModeType::ApplicationMouseTracking
/// [`PrivateModeType::SgrMouseMode`]: crate::PrivateModeType::SgrMouseMode
/// [`PTYMux`]: crate::PTYMux
/// [`SGR`]: crate::SgrCode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseTrackingMode {
    /// Mouse tracking enabled.
    ///
    /// The multiplexer engine unconditionally formats all events as [`SGR`] extended
    /// mouse tracking (`1006` protocol), which uses string coordinates. See
    /// [`PrivateModeType::SgrMouseMode`].
    ///
    /// [`PrivateModeType::SgrMouseMode`]: crate::PrivateModeType::SgrMouseMode
    /// [`SGR`]: crate::SgrCode
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
    pub enum BracketedPasteMode {
        /// Bracketed paste mode enabled
        Enabled,
        /// Bracketed paste mode disabled
        #[default]
        Disabled,
    }
}
