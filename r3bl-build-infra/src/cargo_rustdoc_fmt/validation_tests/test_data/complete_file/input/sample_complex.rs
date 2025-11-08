// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Test fixture: Complex example with both tables and inline links
// From r3bl-open-core graphemes module

//! # Grapheme Clusters and UTF-8
//!
//! This module handles grapheme cluster analysis for terminal rendering.
//!
//! | Character | Byte size | Grapheme cluster size | Compound |
//! | --- | --- | --------------------- | -------- |
//! | `H`  | 1  | 1  | No |
//! | `😃`    | 4  | 2    | No |
//! | `📦` | 4  | 2      | No |
//! | `🙏🏽`         | 4  | 2         | Yes |
//!
//! For more details see:
//! - [Grapheme clusters](https://medium.com/flutter-community/working-with-unicode-and-grapheme-clusters-in-dart-b054faab5705)
//! - [UTF-8 String](https://doc.rust-lang.org/book/ch08-02-strings.html)
//! - [Unicode in Rust](https://doc.rust-lang.org/stable/std/primitive.str.html)

fn main() {}
