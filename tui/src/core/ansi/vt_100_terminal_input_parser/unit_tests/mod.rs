// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Component-level tests - round-trip validation of input event generator.
//!
//! Tests validate that the event generator and parser work together correctly
//! through round-trip validation:
//!
//! `InputEvent → generate() → bytes → parse() → InputEvent`
//!
//! This ensures:
//! - Generator produces valid ANSI sequences
//! - Parser correctly interprets those sequences
//! - Generator and parser are compatible/speak the same language
//!
//! See the [parent module] for the overall testing strategy.
//!
//! [parent module]: super#testing-strategy

#[cfg(any(test, doc))]
pub mod generator_round_trip_tests;
