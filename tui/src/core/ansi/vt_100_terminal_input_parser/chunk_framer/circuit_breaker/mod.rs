// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-allocation streaming pre-accumulator filter and circuit breaker for runaway
//! [`OSC`] sequences. See [`OscCircuitBreaker`] for more details.
//!
//! [`OSC`]: crate::osc_codes::OscSequence

// Attach source files.
mod circuit_breaker_struct;
mod drain_reason;
mod drain_result;

// Public re-exports (barrel export pattern).
pub use circuit_breaker_struct::*;
pub use drain_reason::*;
pub use drain_result::*;
