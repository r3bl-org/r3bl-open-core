// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
mod tui_styled_text_impl;
#[macro_use]
mod tui_styled_texts_impl;

// Attach.
#[cfg(test)]
mod vec_vs_smallvec_bench_tests;

// Re-export.
pub use tui_styled_text_impl::*;
pub use tui_styled_texts_impl::*;
