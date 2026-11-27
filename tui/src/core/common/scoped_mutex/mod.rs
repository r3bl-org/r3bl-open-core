// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod mutex_ext;

// Attach.
mod deadlock_prevention;
mod scoped_mutex_public_api;

// Re-export.
pub use deadlock_prevention::*;
pub use mutex_ext::*;
pub use scoped_mutex_public_api::*;
