// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DeadlockPreventionPolicy::PanicOnSpecificLockNesting, ScopedMutex};
use crossterm::event::Event;
use futures_core::Stream;
use std::{io::Error, pin::Pin, sync::Arc};

/// Type alias for [`ScopedMutex`] with the [`PanicOnSpecificLockNesting`] policy.
///
/// # Architectural Rationale for [`PanicOnSpecificLockNesting`] ([`SPECIFIC`])
///
/// We use the [`SPECIFIC`] policy as the default for these core terminal type aliases
/// because:
/// 1. **Granular Protection**: It protects the internal state of individual components
///    (like [`Readline`] or [`OutputDevice`]) from concurrent interference. This enforces
///    a loud and fast failure if a thread attempts to recursively lock the **same
///    instance**, protecting against the most common source of deadlocks.
/// 2. **Flexible UI Coordination**: Unlike the [`ANY`] policy, [`SPECIFIC`] allows a
///    thread to acquire multiple **different** lock instances simultaneously. This is
///    essential for complex UI coordination where one component must hold its own lock
///    while interacting with another protected component. [`ANY`] is typically reserved
///    for use in locks that are process wide global statics.
///
/// # Types using this Policy
///
/// This policy is used by all type aliases that wrap internal component state:
/// - [`SafeRawTerminal`] (for terminal output)
/// - [`SafeLineState`] (for async readline state)
/// - [`SafeHistory`] (for input history)
/// - [`SafePauseBuffer`] (for output buffering)
/// - [`SafeBool`] and [`SafeInlineString`] (for various thread-safe flags and labels)
///
/// [`ANY`]: crate::DeadlockPreventionPolicy::PanicOnAnyLockNesting
/// [`OutputDevice`]: crate::OutputDevice
/// [`PanicOnSpecificLockNesting`]:
///     crate::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
/// [`Readline`]: crate::Readline
/// [`SafeBool`]: crate::SafeBool
/// [`SafeHistory`]: crate::SafeHistory
/// [`SafeInlineString`]: crate::SafeInlineString
/// [`SafeLineState`]: crate::SafeLineState
/// [`SafePauseBuffer`]: crate::SafePauseBuffer
/// [`SPECIFIC`]: crate::DeadlockPreventionPolicy::PanicOnSpecificLockNesting
pub type StdMutex<T> = ScopedMutex<T, { PanicOnSpecificLockNesting }>;

/// Type alias for a `Send`-able output device (raw terminal, `SharedWriter`, etc).
pub type SendRawTerminal = dyn std::io::Write + Send;
/// Type alias for a `Send`-able raw terminal wrapped in an `Arc<StdMutex>`.
pub type SafeRawTerminal = Arc<StdMutex<SendRawTerminal>>;

/// Type alias for crossterm streaming (input) event result.
pub type CrosstermEventResult = Result<Event, Error>;
/// Type alias for a pinned stream that is async safe. `T` is usually
/// [`CrosstermEventResult`].
pub type PinnedInputStream<T> = Pin<Box<dyn Stream<Item = T>>>;
