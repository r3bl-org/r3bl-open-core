// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CrosstermEventResult, CrosstermInputDevice, DirectToAnsiInputDevice,
            InlineVec, InputDeviceExt, InputEvent, MockInputDevice,
            TERMINAL_LIB_BACKEND, TerminalLibBackend};
use std::time::Duration;

/// Generic input device wrapper that abstracts over different backend implementations.
///
/// Provides a unified interface for reading terminal input events, similar to how
/// [`crate::OutputDevice`] abstracts over different output backends.
///
/// ## Architecture
///
/// Uses an enum to dispatch to the appropriate backend implementation at runtime:
/// - **Crossterm**: Cross-platform terminal input (default on non-Linux)
/// - **DirectToAnsi**: Pure Rust async input with tokio (default on Linux)
/// - **Mock**: Synthetic event generator for testing
///
/// Backend selection is automatic based on [`TERMINAL_LIB_BACKEND`], or can be
/// explicitly chosen via `new_crossterm()` / `new_direct_to_ansi()`.
///
/// ## Examples
///
/// ### Auto-select backend based on platform
/// ```no_run
/// use r3bl_tui::InputDevice;
///
/// let mut device = InputDevice::new();
/// while let Some(event) = device.next_input_event().await {
///     // Process event
/// }
/// ```
///
/// ### Explicitly choose backend
/// ```no_run
/// use r3bl_tui::InputDevice;
///
/// let mut device = InputDevice::new_crossterm();
/// ```
///
/// ### Mock for testing
/// ```no_run
/// use r3bl_tui::InputDevice;
/// use smallvec::smallvec;
/// use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
///
/// let events = smallvec![
///     Ok(Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))),
/// ];
/// let mut device = InputDevice::new_mock(events);
/// ```
#[derive(Debug)]
pub enum InputDevice {
    /// Crossterm backend - cross-platform terminal input
    Crossterm(CrosstermInputDevice),
    /// DirectToAnsi backend - pure Rust async I/O
    DirectToAnsi(DirectToAnsiInputDevice),
    /// Mock backend - synthetic events for testing
    Mock(MockInputDevice),
}

impl InputDevice {
    /// Create a new [`InputDevice`] using the platform-default backend.
    ///
    /// - Linux: DirectToAnsi (pure Rust async I/O)
    /// - Others: Crossterm (cross-platform compatibility)
    ///
    /// Backend is selected via [`TERMINAL_LIB_BACKEND`] constant.
    #[must_use]
    pub fn new() -> Self {
        match TERMINAL_LIB_BACKEND {
            TerminalLibBackend::Crossterm => Self::new_crossterm(),
            TerminalLibBackend::DirectToAnsi => Self::new_direct_to_ansi(),
        }
    }

    /// Create a new InputDevice using the Crossterm backend explicitly.
    #[must_use]
    pub fn new_crossterm() -> Self {
        Self::Crossterm(CrosstermInputDevice::new_event_stream())
    }

    /// Create a new InputDevice using the DirectToAnsi backend explicitly.
    #[must_use]
    pub fn new_direct_to_ansi() -> Self {
        Self::DirectToAnsi(DirectToAnsiInputDevice::new())
    }

    /// Create a new mock InputDevice for testing.
    ///
    /// Events are yielded from the provided vector in order.
    #[must_use]
    pub fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> Self {
        Self::Mock(MockInputDevice::new(generator_vec))
    }

    /// Create a new mock InputDevice with a delay between events.
    ///
    /// Useful for testing timing-sensitive behavior.
    #[must_use]
    pub fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> Self {
        Self::Mock(MockInputDevice::new_with_delay(generator_vec, delay))
    }

    /// Read the next input event asynchronously.
    ///
    /// Returns `None` if the input stream is closed or encounters an error.
    ///
    /// ## Implementation
    ///
    /// Dispatches to the appropriate backend's `next_input_event()` implementation
    /// via the [`InputDeviceExt`] trait.
    pub async fn next_input_event(&mut self) -> Option<InputEvent> {
        match self {
            Self::Crossterm(device) => device.next_input_event().await,
            Self::DirectToAnsi(device) => device.next_input_event().await,
            Self::Mock(device) => device.next_input_event().await,
        }
    }

    /// Check if this is a mock device (for testing).
    ///
    /// This method exists for API symmetry with [`crate::OutputDevice`].
    #[must_use]
    pub fn is_mock(&self) -> bool { matches!(self, Self::Mock(_)) }
}

impl Default for InputDevice {
    fn default() -> Self { Self::new() }
}
