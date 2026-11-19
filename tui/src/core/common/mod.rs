// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # String Allocation Performance and Strategy for the entire codebase
//!
//! This codebase is heavily optimized for zero-allocation rendering and formatting.
//! Depending on the specific use case, we use one of three core approaches for building

#![rustfmt::skip]

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their macros).
#[macro_use]
pub(crate) mod fast_strings; // Cross-folder rustdoc module links (lib.rs, into_existing).
#[macro_use]
mod scoped_mutex;
#[macro_use]
mod telemetry;

// Attach. Crate-wide (Intra-crate).
pub(crate) mod string_repeat_cache; // Cross-folder rustdoc link to DYNAMIC_CACHE (scoped_mutex).

// Attach. Folder-local (Intra-module).
mod common_atomic;
mod common_enums;
mod common_math;
mod common_result_and_error;
mod flat_2d_array;
mod get_mem_size;
mod lru_cache;
mod memoized_value;
mod miette_setup_global_report_handler;
mod monitor;
mod ordered_map;
mod rate_limiter;
mod ring_buffer;
mod ring_buffer_heap;
mod ring_buffer_stack;
mod std_thread_tokio_runner;
mod time_duration;
mod primitive_casting;
mod os_string_ext;
mod type_aliases;

// Re-export.
pub use common_atomic::*;
pub use common_enums::*;
pub use common_math::*;
pub use common_result_and_error::*;
pub use fast_strings::*;
pub use flat_2d_array::*;
pub use get_mem_size::*;
pub use lru_cache::*;
pub use memoized_value::*;
pub use miette_setup_global_report_handler::*;
pub use monitor::*;
pub use ordered_map::*;
pub use rate_limiter::*;
pub use ring_buffer::*;
pub use ring_buffer_heap::*;
pub use ring_buffer_stack::*;
pub use std_thread_tokio_runner::*;
pub use scoped_mutex::*;
pub use string_repeat_cache::*;
pub use telemetry::*;
pub use time_duration::*;
pub use primitive_casting::*;
pub use os_string_ext::*;
pub use type_aliases::*;