// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Conformance test suite for the `env-source` cross-platform environment loader.
//!
//! # Architecture
//!
//! Following the patterns established in `vt_100_pty_output_conformance_tests` and
//! `md_parser`, testing is divided into two tiers:
//!
//! 1. **Tier 1 (Fast Unit Tests)**: Embedded directly in module files (`parser.rs`,
//!    `diff.rs`, `filter.rs`, `formatters/*.rs`) to validate algorithms, escaping rules,
//!    and filtering logic in memory without subprocess spawns.
//! 2. **Tier 2 (Hermetic Subshell Conformance & Golden Tests)**: Located in this module.
//!    Executes real-world shell scripts (`/bin/sh` on Unix, `cmd.exe` on Windows) and
//!    asserts exact golden outputs against deterministically mocked baseline
//!    environments.
//!
//! # Module Organization
//!
//! - [`test_data`]: Input test scripts (`.sh`, `.bat`) and golden output files
//!   (`.fish`, `.ps1`, `.json`, `.env`).
//! - [`test_fixtures`]: Test harness scaffolding, temp script runners, and hermetic mock
//!   environment builders.
//! - `tests`: Integration test runners validating golden formatters and subshell
//!   evaluation.

#![rustfmt::skip]

// Attach.
mod test_data;
mod test_fixtures;
mod tests;
