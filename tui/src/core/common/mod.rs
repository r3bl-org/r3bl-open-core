// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # String Allocation Performance and Strategy for the entire codebase
//!
//! This codebase is heavily optimized for zero-allocation rendering and formatting.
//! Depending on the specific use case, we use one of three core approaches for building
// Attach sources.
pub mod common_atomic;
pub mod common_enums;
pub mod common_math;
pub mod common_result_and_error;
pub mod fast_strings;
pub mod flat_2d_array;
pub mod get_mem_size;
pub mod lru_cache;
pub mod memoized_value;
pub mod miette_setup_global_report_handler;
pub mod monitor;
pub mod ordered_map;
pub mod rate_limiter;
pub mod ring_buffer;
pub mod ring_buffer_heap;
pub mod ring_buffer_stack;
pub mod scoped_mutex;
pub mod string_repeat_cache;
pub mod telemetry;
pub mod time_duration;

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
pub use scoped_mutex::*;
pub use string_repeat_cache::*;
pub use telemetry::*;
pub use time_duration::*;
