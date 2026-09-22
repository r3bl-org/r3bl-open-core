// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Synthetic fixture for testing code fence language tag normalization.
//!
//! Standalone `rust` fence:
//!
//! ```rust
//! let x = 42;
//! assert_eq!(x, 42);
//! ```
//!
//! Comma-separated `rust,ignore` fence:
//!
//! ```rust,ignore
//! let skipped = compute_expensive_value();
//! ```
//!
//! Comma-separated `rust,no_run` fence:
//!
//! ```rust,no_run
//! start_background_service();
//! ```
//!
//! Non-Rust `text` fence (must remain unchanged):
//!
//! ```text
//! +---------+
//! | Diagram |
//! +---------+
//! ```
//!
//! Bare fence (must remain unchanged):
//!
//! ```
//! let already_clean = true;
//! ```

/// Function with outer doc comments containing a redundant `rust` fence.
///
/// ```rust
/// let sum = 1 + 2;
/// assert_eq!(sum, 3);
/// ```
pub fn sample_function() {}
