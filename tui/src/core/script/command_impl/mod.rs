// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod command_run;

// Attach.
mod command_run_report;
mod process_execution_outcome;

// Re-export.
pub use command_run::*;
pub use command_run_report::*;
pub use process_execution_outcome::*;
