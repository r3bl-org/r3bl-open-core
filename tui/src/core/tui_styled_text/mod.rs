// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Attach sources.
pub mod tui_styled_text_impl;
pub mod tui_styled_texts_impl;

#[cfg(test)]
mod vec_vs_smallvec_bench_tests;

// Re-export.
pub use tui_styled_text_impl::*;
pub use tui_styled_texts_impl::*;
