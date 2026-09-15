// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach.
mod channel_monitor;
mod event_conversion;
mod history;
mod lock_manager;
mod readline_struct;
mod types;

// Attach conditionally: public when testing or generating docs, private otherwise.
#[cfg(any(test, doc))]
pub mod line_state;
#[cfg(not(any(test, doc)))]
mod line_state;

// Re-export.
pub use channel_monitor::*;
pub use event_conversion::*;
pub use history::*;
pub use line_state::*;
pub use lock_manager::*;
pub use readline_struct::*;
pub use types::*;

// Integration tests (conditional visibility).
#[cfg(any(test, doc))]
pub mod readline_async_integration_tests;
