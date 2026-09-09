// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{OscEvent, PtyControlledChildExitStatus};

// Output event definitions.

/// Events received from a [`PTY`] process.
///
/// This is a unified event type used by both read-only and read-write sessions.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone)]
pub enum PtyOutputEvent {
    /// Raw output from the child process.
    Output(Vec<u8>),

    /// [`OSC`] (Operating System Command) sequences.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    Osc(OscEvent),

    /// Child process exited normally.
    Exit(PtyControlledChildExitStatus),

    /// Child process crashed or terminated unexpectedly.
    UnexpectedExit(String),

    /// Write operation failed: session will terminate.
    ///
    /// This gives users a chance to understand why the session ended.
    WriteError(String),
}
