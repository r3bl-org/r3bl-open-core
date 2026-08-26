// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach (private).
mod backpressure_stdout_struct;

// Attach platform-specific implementation.

#[cfg(unix)]
mod impl_unix;

#[cfg(not(unix))]
mod impl_win;

// Re-export.
pub use backpressure_stdout_struct::*;
