// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod formatter;

// Attach.
mod calc_str_len;
mod friendly_random_id;
mod string_helper;

// Re-export.
pub use calc_str_len::*;
pub use formatter::*;
pub use friendly_random_id::*;
pub use string_helper::*;
