// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Parameter extraction utilities for ANSI/VT sequence processing.
//!
//! This module provides common parameter extraction patterns used throughout
//! the ANSI parser implementation, following VT100 specification for parameter
//! handling where missing parameters (None) default to 1 and zero parameters
//! (Some(0)) are treated as 1.

use vte::Params;


/// Extract the nth parameter (0-indexed) with VT100-compliant default handling.
///
/// ## Parameter Handling Rules
/// - **Missing parameters** (None) default to 1
/// - **Zero parameters** (Some(0)) are treated as 1
/// - **Non-zero parameters** (Some(n)) are used as-is
///
/// This ensures compatibility with real VT100 terminals and modern terminal emulators.
///
/// ## Examples
/// - `extract_nth_param_non_zero(params, 0)` extracts the first parameter
/// - `extract_nth_param_non_zero(params, 1)` extracts the second parameter
/// - `ESC[A` (no param) → returns 1 for any n
/// - `ESC[0;5A` → returns 1 for n=0, 5 for n=1
pub fn extract_nth_param_non_zero(params: &Params, n: usize) -> u16 {
    params
        .iter()
        .nth(n)
        .and_then(|p| p.first())
        .copied()
        .map_or(
            /* None -> 1 */ 1,
            /* Some(0) -> 1 */ |v| u16::max(v, 1),
        )
}

/// Extract the nth parameter (0-indexed) without any default transformation.
///
/// This is useful for cases where the parameter's absence has different
/// semantics than a default value.
///
/// ## Returns
/// - `None` if no parameter is present at index n
/// - `Some(value)` if a parameter is present (including 0)
///
/// ## Examples
/// - `extract_nth_optional_param(params, 0)` extracts the first parameter
/// - `extract_nth_optional_param(params, 1)` extracts the second parameter
/// - `ESC[5A` → returns Some(5) for n=0, None for n=1
/// - `ESC[0;7A` → returns Some(0) for n=0, Some(7) for n=1
pub fn extract_nth_optional_param(params: &Params, n: usize) -> Option<u16> {
    params.iter().nth(n).and_then(|p| p.first()).copied()
}
