// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Crate-wide (Intra-crate).
pub(crate) mod rolling_file_appender_impl; // Cross-folder rustdoc link to ROLLING_LOG_FILE_WRITER_GUARD.

// Attach. Folder-local (Intra-module).
mod tracing_init;
mod custom_event_formatter;
mod log_public_api;
mod tracing_config;

// Re-export.
pub use custom_event_formatter::*;
pub use log_public_api::*;
pub use rolling_file_appender_impl::*;
pub use tracing_config::*;
pub use tracing_init::*;

// Tests.
#[cfg(test)]
mod log_integration_tests;
