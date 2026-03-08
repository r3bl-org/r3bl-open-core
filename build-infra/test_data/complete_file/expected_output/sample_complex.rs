// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Test fixture: Complex example with both tables and inline links
// From r3bl-open-core graphemes module

//! # Grapheme Clusters and [`UTF-8`]
//!
//! This module handles grapheme cluster analysis for terminal rendering.
//!
//! | Character | Byte size | Grapheme cluster size | Compound |
//! | --------- | --------- | --------------------- | -------- |
//! | `H`       | 1         | 1                     | No       |
//! | `😃`      | 4         | 2                     | No       |
//! | `📦`      | 4         | 2                     | No       |
//! | `🙏🏽`      | 4         | 2                     | Yes      |
//!
//! For more details see:
//! - [Grapheme clusters]
//! - [UTF-8 String]
//! - [Unicode in Rust]
//!
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [Grapheme clusters]: https://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
//! [Unicode in Rust]: https://doc.rust-lang.org/stable/std/primitive.str.html
//! [UTF-8 String]: https://doc.rust-lang.org/book/ch08-02-strings.html

fn main() {}
