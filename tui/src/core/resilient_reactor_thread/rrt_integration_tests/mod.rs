// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach.
mod double_panic_prevention_test;
#[cfg(target_os = "linux")]
pub(crate) mod pty_test_production_factory_restart; // Cross-folder rustdoc link (handler_stdin).
#[cfg(target_os = "linux")]
mod pty_test_production_poll_error;
mod smart_sender_test;
