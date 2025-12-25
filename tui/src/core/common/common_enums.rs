// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Control flow signal for loops and threads.
///
/// A unified type for indicating whether a loop or thread should continue
/// processing or stop. Used across:
/// - Main event loop (terminal window)
/// - Mio poller thread (input handling)
/// - PTY input processing loop
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Continuation {
    /// Continue to the next iteration.
    #[default]
    Continue,
    /// Stop processing and exit the loop/thread.
    Stop,
}

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub enum ContainsResult {
    #[default]
    DoesNotContain,
    DoesContain,
}
