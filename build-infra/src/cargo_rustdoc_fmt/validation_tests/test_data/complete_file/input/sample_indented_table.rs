// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test case for indented markdown tables.
//!
//! This tests tables that appear under numbered list items, which require
//! 4 spaces of indentation to render correctly in rustdoc.
//!
//! 1. First mechanism (tables should be indented under this)
//!
//!     | Trigger | Behavior |
//!     |:---|:---|
//!     | Event A | Does something |
//!     | Event B | Does something else |
//!
//! 2. Second mechanism (another indented table)
//!
//!     | Column One | Column Two |
//!     |:---|:---|
//!     | Value 1 | Result 1 |
//!     | Value 2 | Result 2 |
//!
//! Non-indented table for comparison:
//!
//! | Header A | Header B |
//! |:---|:---|
//! | Cell 1 | Cell 2 |

pub fn example() {}
