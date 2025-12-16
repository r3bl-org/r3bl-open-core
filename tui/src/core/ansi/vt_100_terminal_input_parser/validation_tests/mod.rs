// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Protocol validation - ground truth discovery and conformance testing.
//!
//! This module contains two types of validation that ensure our parser conforms to
//! the VT-100 ANSI protocol:
//!
//! ## 1. Ground Truth Discovery
//!
//! [`observe_real_interactive_terminal_input_events`] - Interactive test that captures
//! raw bytes from real terminal interactions to establish what terminals actually emit.
//! This serves as the authoritative reference for the ANSI protocol.
//!
//! Run with: `cargo test observe_terminal -- --ignored --nocapture`
//!
//! ## 2. Protocol Conformance Testing
//!
//! [`input_parser_validation_test`] - Automated unit tests using hardcoded ANSI
//! sequences captured from real terminals. These tests validate that our parser
//! correctly interprets the protocol.
//!
//! # Design Philosophy
//!
//! Both use **hardcoded/observed sequences** (not generated) to ensure:
//! - **Independence**: Tests validate against the VT-100 spec, not our generator
//! - **Ground truth**: Sequences represent actual terminal behavior
//! - **Error detection**: Catches bugs in both parser AND generator implementations
//!
//! See the [parent module] for the overall testing strategy.
//!
//! ## ⚠️ WARNING: DO NOT Refactor These Tests to Use Generators!
//!
//! If you're considering replacing hardcoded sequences in this module with generator
//! function calls, **STOP**! This would break the validation chain:
//!
//! ```text
//! ❌ BROKEN: Circular validation (no ground truth)
//!    Generator → Bytes → Parser → Generator validates itself ✗
//!
//! ✅ CORRECT: Independent validation against reality
//!    Terminal observation → Bytes → Parser validates against reality ✓
//!    Generator → Bytes validates against reality ✓
//! ```
//!
//! **These hardcoded sequences ARE the ground truth.** The generators in
//! [`generator`] are validated by producing sequences that match these literals.
//!
//! If you want to test generator correctness, see the round-trip tests in
//! [`unit_tests::generator_round_trip_tests`] instead.
//!
//! [`generator`]: crate::core::ansi::generator
//! [`unit_tests::generator_round_trip_tests`]: super::unit_tests::generator_round_trip_tests
//! [parent module]: super#testing-strategy

#[cfg(any(test, doc))]
pub mod input_parser_validation_test;
#[cfg(any(test, doc))]
pub mod observe_real_interactive_terminal_input_events;
