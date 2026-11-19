// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod command_runner;

// Attach.
mod command_output_result;
mod command_run_result;

// Re-export.
pub use command_output_result::*;
pub use command_run_result::*;
pub use command_runner::*;
