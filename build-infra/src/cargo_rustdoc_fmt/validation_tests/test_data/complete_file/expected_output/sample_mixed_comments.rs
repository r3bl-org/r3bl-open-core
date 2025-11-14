// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Module-level documentation.
//!
//! | Feature | Status |
//! |---|---|
//! | Tables | Implemented |
//! | Links | [Working](https://example.com) |

/// Function documentation with a table.
///
/// | Parameter | Type | Description |
/// |---|---|---|
/// | x | i32 | The first number |
/// | y | i32 | The second number |
///
/// See [Rust docs](https://rust-lang.org) for more.
pub fn add(x: i32, y: i32) -> i32 {
    x + y
}

/// Another function with links.
///
/// Check out:
/// - [The Rust book](https://doc.rust-lang.org/book/)
/// - [Rust by example](https://doc.rust-lang.org/rust-by-example/)
pub fn subtract(x: i32, y: i32) -> i32 {
    x - y
}
