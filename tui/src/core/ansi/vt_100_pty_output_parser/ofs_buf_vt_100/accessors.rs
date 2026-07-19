// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{AlternateBuffer, OfsBufVT100, PrimaryBuffer};
use crate::{ParserGlobalState, TerminalModeState};

/// Public accessor methods for the [`OfsBufVT100`] struct (since all the fields are
/// private).
impl OfsBufVT100 {
    /// The primary screen buffer, backed by a [`GrowableBuffer`] which natively preserves
    /// scrollback history as lines are pushed off the top.
    ///
    /// [`GrowableBuffer`]: crate::tui::GrowableBuffer
    #[must_use]
    pub fn get_primary_buffer(&self) -> &PrimaryBuffer { &self.primary_buffer }

    /// See [`Self::get_primary_buffer()`].
    pub fn primary_buffer_mut(&mut self) -> &mut PrimaryBuffer {
        &mut self.primary_buffer
    }

    /// The alternate screen buffer, backed by a [`Flat2DArray`]. This buffer has no
    /// history and is used when the terminal switches to the alternate screen (`CSI ?
    /// 1049 h`).
    ///
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    #[must_use]
    pub fn get_alternate_buffer(&self) -> &AlternateBuffer { &self.alternate_buffer }

    /// See [`Self::get_alternate_buffer()`].
    pub fn get_alternate_buffer_mut(&mut self) -> &mut AlternateBuffer {
        &mut self.alternate_buffer
    }

    /// High-level runtime state tracking active graphic renditions ([`SGR`], colors,
    /// styling), character set mappings, and protocol requests ([`DSR`], [`OSC`]) that
    /// persist globally.
    ///
    /// [`DSR`]: crate::DsrSequence
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`SGR`]: crate::SgrCode
    #[must_use]
    pub fn get_parser_global_state(&self) -> &ParserGlobalState {
        &self.parser_global_state
    }

    /// See [`Self::get_parser_global_state()`].
    pub fn get_parser_global_state_mut(&mut self) -> &mut ParserGlobalState {
        &mut self.parser_global_state
    }

    /// Tracks active terminal modes and boolean toggles (e.g. [`DECTCEM`] for cursor
    /// visibility, [`DECAWM`] for auto-wrap).
    ///
    /// Crucially, this state includes [`TerminalModeState::active_screen_buffer`] which
    /// dictates whether the primary or alternate screen is currently active. This drives
    /// the internal routing for the following:
    /// 1. **Buffer Mutations:** Methods like [`get_active_screen_buffer_mut()`] route
    ///    mutable [`VT-100`] operations to the correct buffer.
    /// 2. **State Queries:** External components can explicitly check the active screen
    ///    via [`TerminalModeState::active_screen_buffer`].
    ///
    /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
    /// [`DECTCEM`]: https://vt100.net/docs/vt510-rm/DECTCEM.html
    /// [`get_active_screen_buffer_mut()`]: Self::get_active_screen_buffer_mut
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[must_use]
    pub fn get_terminal_mode(&self) -> &TerminalModeState { &self.terminal_mode }

    /// See [`Self::get_terminal_mode()`].
    pub fn get_terminal_mode_mut(&mut self) -> &mut TerminalModeState {
        &mut self.terminal_mode
    }
}
