// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Rust uses [`UTF-8`] to represent text in [String]. [`UTF-8`] is a variable width
//! encoding, so each character can take up a different number of bytes, between 1 and 4,
//! and 1 byte is 8 bits; this is why we use [Vec] of [u8] to represent a [String].
//!
//! For example, the character `H` takes up 1 byte. [`UTF-8`] is also backward compatible
//! with [`ASCII`], meaning that the first 128 characters (the [`ASCII`] characters) are
//! represented using the same single byte as in [`ASCII`]. So the character `H` is
//! represented by the same byte value in [`UTF-8`] as it is in [`ASCII`]. This is why
//! [`UTF-8`] is so popular, as it allows for the representation of all the characters in
//! the Unicode standard, while still being able to represent [`ASCII`] characters in the
//! same way.
//!
//! A grapheme cluster is a user-perceived character. Grapheme clusters can take up many
//! more bytes, eg 4 bytes or 2 or 3, etc. Here are some examples:
//! - `😃` takes up 4 bytes.
//! - `📦` also takes up 4 bytes.
//! - `🙏🏽` takes up 4 bytes, but it is a compound grapheme cluster.
//! - `H` takes up only 1 byte.
//!
//! Videos:
//!
//! - [Live coding video on Rust String]
//! - [UTF-8 encoding video]
//!
//! Docs:
//!
//! - [Grapheme clusters]
//! - [UTF-8 String]
//!
//! There is a discrepancy between how a [String] that contains grapheme clusters is
//! represented in memory and how it is rendered in a terminal. When writing an TUI editor
//! it is necessary to have a caret (cursor position) that the user can move by pressing
//! up, down, left, right, etc. For left, this is assumed to move the caret or cursor one
//! position to the left, regardless of how wide that character may be. Let's unpack that.
//!
//! - If we use byte boundaries in the [String] we can move the cursor one byte to the
//!   left.
//! - This falls apart when we have a grapheme cluster.
//! - A grapheme cluster can take up more than one byte, and they don't fall cleanly into
//!   byte boundaries.
//!
//! To complicate things further, the size that a grapheme cluster takes up is not the
//! same as its byte size in memory. Let's unpack that.
//!
//! | Character   | Byte size   | Grapheme cluster size   | Compound   |
//! | :---------- | :---------- | :---------------------- | :--------- |
//! | `H`         | 1           | 1                       | No         |
//! | `😃`        | 4           | 2                       | No         |
//! | `📦`        | 4           | 2                       | No         |
//! | `🙏🏽`        | 4           | 2                       | Yes        |
//!
//! > **Note**: For input parsing of [`UTF-8`] byte sequences from terminal input, see
//! > [`mod@crate::vt_100_terminal_input_parser::utf8`]. That module
//! > handles byte-level decoding (converting raw bytes to characters), while this
//! > module handles display width calculation (determining how many terminal columns
//! > a character occupies for rendering).
//!
//! Here are examples of compound grapheme clusters.
//!
//! ```text
//! 🏽 + 🙏 = 🙏🏽
//! 🏾‍ + 👨 + 🤝‍ + 👨 +  🏿 = 👨🏾‍🤝‍👨🏿
//! ```
//!
//! Let's say you're browsing this source file in `VSCode`. The [`UTF-8`] string this Rust
//! source file is rendered by `VSCode` correctly. But this is not how it looks in a
//! terminal. And the size of the string in memory isn't clear either from looking at the
//! string in `VSCode`. It isn't apparent that you can't just index into the string at
//! byte boundaries.
//!
//! To further complicate things, the output looks different on different terminals &
//! OSes. The function `test_crossterm_grapheme_cluster_width_calc()` (shown below) uses
//! crossterm commands to try and figure out what the width of a grapheme cluster is. When
//! you run this in an SSH session to a macOS machine from Linux, it will work the same
//! way it would locally on Linux. However, if you run the same program in locally via
//! Terminal.app on macOS it works differently! So there are some serious issues.
//!
//! ```no_run
//! use crossterm::{self, *, terminal::*, style::*, cursor::*, event::*};
//! use std::io::*;
//! use std::collections::*;
//!
//! pub fn test_crossterm_grapheme_cluster_width_calc() -> Result<()> {
//!   // Enter raw mode, clear screen.
//!   enable_raw_mode()?;
//!   execute!(stdout(), EnterAlternateScreen)?;
//!   execute!(stdout(), Clear(ClearType::All))?;
//!   execute!(stdout(), MoveTo(0, 0))?;
//!
//!   // Perform test of grapheme cluster width.
//!   #[derive(Default, Debug, Clone, Copy)]
//!   struct Positions {
//!     orig_pos: (u16, u16),
//!     new_pos: (u16, u16),
//!     col_width: u16,
//!   }
//!
//!   let mut map = HashMap::<&str, Positions>::new();
//!   map.insert("Hi", Positions::default());
//!   map.insert(" ", Positions::default());
//!   map.insert("😃", Positions::default());
//!   map.insert("📦", Positions::default());
//!   map.insert("🙏🏽", Positions::default());
//!   map.insert("👨🏾‍🤝‍👨🏿", Positions::default());
//!   map.insert(".", Positions::default());
//!
//!   fn process_map(map: &mut HashMap<&str, Positions>) -> Result<()> {
//!     for (index, (key, value)) in map.iter_mut().enumerate() {
//!       let orig_pos: (u16, u16) = (/* col: */ 0, /* row: */ index as u16);
//!       execute!(stdout(), MoveTo(orig_pos.0, orig_pos.1))?;
//!       execute!(stdout(), Print(key))?;
//!       let new_pos = cursor::position()?;
//!       value.new_pos = new_pos;
//!       value.orig_pos = orig_pos;
//!       value.col_width = new_pos.0 - orig_pos.0;
//!     }
//!     Ok(())
//!   }
//!
//!   process_map(&mut map)?;
//!
//!   // Just blocking on user input.
//!   {
//!     execute!(stdout(), Print("... Press any key to continue ..."))?;
//!     if let Event::Key(_) = read()? {
//!       execute!(stdout(), terminal::Clear(ClearType::All))?;
//!       execute!(stdout(), cursor::MoveTo(0, 0))?;
//!     }
//!   }
//!
//!   // Exit raw mode, clear screen.
//!   execute!(stdout(), terminal::Clear(ClearType::All))?;
//!   execute!(stdout(), cursor::MoveTo(0, 0))?;
//!   execute!(stdout(), LeaveAlternateScreen)?;
//!   disable_raw_mode().expect("Unable to disable raw mode");
//!   println!("map:{:#?}", map);
//!
//!   Ok(())
//! }
//! ```
//!
//! The basic problem arises from the fact that it isn't possible to treat the "logical"
//! index into the string (which isn't byte boundary based) as a "display" (or "physical")
//! index into the rendered output of the string in a terminal.
//!
//! - Some parsing is necessary to get "logical" index into the string that is grapheme
//!   cluster based (not byte boundary based).
//!   - This is where [`unicode-segmentation`] crate comes in and allows us to split our
//!     string into a vector of grapheme clusters.
//! - Some translation is necessary to get from the "logical" index to the physical index
//!   and back again. This is where we can apply one of the following approaches:
//!   - We can use the [`unicode-width`] crate to calculate the width of the grapheme
//!     cluster. This works on Linux, but doesn't work very well on macOS & I haven't
//!     tested it on Windows. This crate will (on Linux) reliably tell us what the
//!     displayed width of a grapheme cluster is.
//!   - We can take the approach from the [`reedline`] crate's [`repaint_buffer()`] where
//!     we split the string based on the "logical" index into the vector of grapheme
//!     clusters. And then we print the 1st part of the string, then call `SavePosition`
//!     to save the cursor at this point, then print the 2nd part of the string, then call
//!     `RestorePosition` to restore the cursor to where it "should" be.
//!
//! Please take a look at [`crate::graphemes::GCStringOwned`] for the following
//! items:
//! - Methods in [`mod@crate::graphemes::gc_string`] for more details on how the
//!   conversion between "display" (or `display_col_index`), ie, [`crate::ColIndex`] and
//!   "logical" or "segment", ie, [`SegIndex`] is done.
//! - The choices that were made in the design of the [`GCStringOwned`] struct for
//!   performance to minimize memory latency (for access and allocation). The results
//!   might surprise you, as intuition around performance is often not reliable.
//!
//! # The Three Types of Indices
//!
//! When working with Unicode text in a terminal-based editor, we need three distinct
//! types of indices to handle text correctly. This is because there's a fundamental
//! mismatch between how text is stored in memory, how it's logically organized, and
//! how it's displayed on screen.
//!
//! ## 1. `ByteIndex` - Memory Position
//!
//! [`ByteIndex`] represents the raw byte offset in a [`UTF-8`] encoded
//! string. This is crucial for:
//! - String slicing operations (Rust strings must be sliced at valid [`UTF-8`]
//!   boundaries)
//! - Memory access and manipulation
//! - Efficient storage and retrieval
//!
//! Example: In the string "H😀!", 'H' starts at byte 0, '😀' starts at byte 1,
//! and '!' starts at byte 5 (since '😀' takes 4 bytes).
//!
//! ## 2. `SegIndex` - Logical Position (Grapheme Clusters)
//!
//! [`SegIndex`] represents the index of a grapheme cluster (user-perceived character).
//! This is essential for:
//! - Cursor movement (users expect to move by visible characters)
//! - Text editing operations (insert/delete should work on whole characters)
//! - Logical text manipulation
//!
//! Example: In "H😀!", there are 3 segments: seg\[0\]='H', seg\[1\]='😀', seg\[2\]='!'
//!
//! ## 3. `ColIndex` - Display Position
//!
//! [`ColIndex`] represents the column position on the terminal screen.
//! This is necessary because:
//! - Some characters are wider than others (emojis typically take 2 columns)
//! - Terminal rendering requires knowing exact column positions
//! - Cursor positioning and selection highlighting need visual coordinates
//!
//! Example: In "H😀!", 'H' is at col 0, '😀' spans cols 1-2, '!' is at col 3
//!
//! ## Visual Example
//!
//! ```text
//! String: "H😀!"
//!
//! ByteIndex: 0 1 2 3 4 5
//! Content:  [H][😀----][!]
//!
//! SegIndex:  0    1     2
//! Segments: [H] [😀]  [!]
//!
//! ColIndex:  0  1  2   3
//! Display:  [H][😀--] [!]
//! ```
//!
//! ## Conversion Between Index Types
//!
//! The [`GCStringOwned`] struct provides conversion operators to translate between these
//! index types:
//!
//! - `&GCStringOwned + ByteIndex → Option<SegIndex>`: Find which segment contains a byte
//! - `&GCStringOwned + ColIndex → Option<SegIndex>`: Find which segment is at a display
//!   column
//! - `&GCStringOwned + SegIndex → Option<ColIndex>`: Find the display column of a segment
//!
//! These conversions can return `None` when indices are out of bounds or fall between
//! characters. For example, a `ByteIndex` in the middle of a multi-byte character would
//! return `None`.
//!
//! [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
//! [`ByteIndex`]: crate::ByteIndex
//! [`ColIndex`]: crate::ColIndex
//! [`reedline`]: https://crates.io/crates/reedline
//! [`repaint_buffer()`]: https://github.com/nushell/reedline/blob/79e7d8da92cd5ae4f8e459f901189d7419c3adfd/src/painting/painter.rs#L129
//! [`unicode-segmentation`]: https://crates.io/crates/unicode-segmentation
//! [`unicode-width`]: https://crates.io/crates/unicode-width
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [Grapheme clusters]: https://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries
//! [Live coding video on Rust String]: https://youtu.be/7I11degAElQ?si=xPDIhITDro7Pa_gq
//! [UTF-8 encoding video]: https://youtu.be/wIVmDPc16wA?si=D9sTt_G7_mBJFLmc
//! [UTF-8 String]: https://doc.rust-lang.org/book/ch08-02-strings.html

// Attach sources.
pub mod gc_string;
pub mod traits;
pub mod unicode_segment;
pub mod word_boundaries;

// Re-export.
#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use gc_string::owned;
pub use gc_string::*;

#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use traits::{grapheme_doc, grapheme_string, grapheme_string_owned_ext, seg_content};
pub use traits::*;

#[doc(inline)] // Create doc pages at re-export path so rustdoc search links resolve.
pub use unicode_segment::{seg, seg_index, seg_length, segment_builder};
pub use unicode_segment::*;

pub use word_boundaries::*;
