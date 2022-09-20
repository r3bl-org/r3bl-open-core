/*
 *   Copyright (c) 2022 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! A grapheme cluster is a user-perceived character. Rust uses `UTF-8` to
//! represent text in `String`. So each character takes up 8 bits or one byte.
//! Grapheme clusters can take up many more bytes, eg 4 bytes or 2 or 3, etc.
//!
//! Docs:
//!
//! - [Grapheme clusters](https://medium.com/flutter-community/working-with-unicode-and-grapheme-clusters-in-dart-b054faab5705)
//! - [UTF-8 String](https://doc.rust-lang.org/book/ch08-02-strings.html)
//!
//! There is a discrepancy between how a `String` that contains grapheme
//! clusters is represented in memory and how it is rendered in a terminal. When
//! writing an TUI editor it is necessary to have a "logical" cursor that the
//! user can move by pressing up, down, left, right, etc. For left, this
//! is assumed to move the caret or cursor one position to the left. Let's
//! unpack that.
//!
//! 1. If we use byte boundaries in the `String` we can move the cursor one byte
//! to the left. 2. This falls apart when we have a grapheme cluster.
//! 3. A grapheme cluster can take up more than one byte, and they don't fall
//! cleanly into byte boundaries.
//!
//! To complicate things further, the size that a grapheme cluster takes up is
//! not the same as its byte size in memory. Let's unpack that.
//!
//! | Character | Byte size | Grapheme cluster size | Compound |
//! | --------- | --------- | --------------------- | -------- |
//! | `H`       | 1         | 1                     | No       |
//! | `😃`      | 4         | 2                     | No       |
//! | `📦`      | 4         | 2                     | No       |
//! | `🙏🏽`      | 4         | 2                     | Yes      |
//!
//! Here are examples of compound grapheme clusters.
//!
//! ```ignore
//! 🏽 + 🙏 = 🙏🏽
//! 🏾‍ + 👨 + 🤝‍ + 👨 +  🏿 = 👨🏾‍🤝‍👨🏿
//! ```
//!
//! Let's say you're browsing this source file in VSCode. The UTF-8 string this
//! Rust source file is rendered by VSCode correctly. But this is not how it
//! looks in a terminal. And the size of the string in memory isn't clear either
//! from looking at the string in VSCode. It isn't apparent that you can't just
//! index into the string at byte boundaries.
//!
//! To further complicate things, the output looks different on different
//! terminals & OSes. The
//! function `test_crossterm_grapheme_cluster_width_calc()` (shown below) uses
//! crossterm commands to try and figure out what the width of a grapheme
//! cluster is. When you run this in an SSH session to a macOS machine from
//! Linux, it will work the same way it would locally on Linux. However, if
//! you run the same program in locally via Terminal.app on macOS it works
//! differently! So there are some serious issues.
//!
//! ```ignore
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
//! The basic problem arises from the fact that it isn't possible to treat the
//! "logical" index into the string (which isn't byte boundary based) as a
//! "physical" index into the rendered output of the string in a terminal.
//!
//! 1. Some parsing is necessary to get "logical" index into the string that is
//! grapheme cluster based (not byte boundary based).
//!    - This is where [`unicode-segmentation`](https://crates.io/crates/unicode-segmentation)
//!      crate comes in and allows us to split our string into a vector of
//!      grapheme clusters.
//! 2. Some translation is necessary to get from the "logical" index to the
//! physical index and back again. This is where we can apply one of the
//! following approaches:
//!    - We can use the [`unicode-width`](https://crates.io/crates/unicode-width)
//!      crate to calculate the width of the grapheme cluster. This works on
//!      Linux, but doesn't work very well on macOS & I haven't tested it on
//!      Windows. This crate will (on Linux) reliably tell us what the displayed
//!      width of a grapheme cluster is.
//!    - We can take the approach from the [`reedline`](https://crates.io/crates/reedline)
//!      crate's [`repaint_buffer()`](https://github.com/nazmulidris/reedline/blob/79e7d8da92cd5ae4f8e459f901189d7419c3adfd/src/painting/painter.rs#L129)
//!      where we split the string based on the "logical" index into the vector
//!      of grapheme clusters. And then we print the 1st part of the string,
//!      then call `SavePosition` to save the cursor at this point, then print
//!      the 2nd part of the string, then call `RestorePosition` to restore the
//!      cursor to where it "should" be.

use std::ops::{Deref, DerefMut};

use get_size::GetSize;
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::*;

use crate::*;

/// Constructor function that creates a [UnicodeString] from a string slice.
pub fn make_unicode_string_from(this: &str) -> UnicodeString {
  let mut total_byte_offset = 0;
  let mut total_grapheme_cluster_count = 0;
  let mut my_unicode_string_segments = vec![];
  let mut my_unicode_width_offset_accumulator: ChUnit = ch!(0);

  for (grapheme_cluster_index, (byte_offset, grapheme_cluster_str)) in
    this.grapheme_indices(true).enumerate()
  {
    let unicode_width = ch!(grapheme_cluster_str.width());
    my_unicode_string_segments.push(GraphemeClusterSegment {
      string: grapheme_cluster_str.into(),
      byte_offset,
      unicode_width,
      logical_index: grapheme_cluster_index,
      byte_size: grapheme_cluster_str.len(),
      display_col_offset: my_unicode_width_offset_accumulator,
    });
    my_unicode_width_offset_accumulator += unicode_width;
    total_byte_offset = byte_offset;
    total_grapheme_cluster_count = grapheme_cluster_index;
  }

  UnicodeString {
    string: this.into(),
    vec_segment: my_unicode_string_segments,
    display_width: my_unicode_width_offset_accumulator,
    byte_size: if total_byte_offset > 0 {
      total_byte_offset + 1 /* size = byte_offset (index) + 1 */
    } else {
      total_byte_offset
    },
    grapheme_cluster_segment_count: if total_grapheme_cluster_count > 0 {
      total_grapheme_cluster_count + 1 /* count = grapheme_cluster_index + 1 */
    } else {
      total_grapheme_cluster_count
    },
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ GraphemeClusterSegment │
// ╯                        ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, GetSize)]
pub struct GraphemeClusterSegment {
  /// The actual grapheme cluster `&str`. Eg: "H", "📦", "🙏🏽".
  pub string: String,
  /// The byte offset (in the original string) of the start of the `grapheme_cluster`.
  pub byte_offset: usize,
  /// Display width of the `string` via [`unicode_width::UnicodeWidthChar`].
  pub unicode_width: ChUnit,
  /// The index of this entry in the `grapheme_cluster_segment_vec`.
  pub logical_index: usize,
  /// The number of bytes the `string` takes up in memory.
  pub byte_size: usize,
  /// Display col at which this grapheme cluster starts.
  pub display_col_offset: ChUnit,
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ UnicodeString │
// ╯               ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, GetSize)]
pub struct UnicodeString {
  pub string: String,
  pub vec_segment: Vec<GraphemeClusterSegment>,
  pub byte_size: usize,
  pub grapheme_cluster_segment_count: usize,
  pub display_width: ChUnit,
}

impl Deref for UnicodeString {
  type Target = Vec<GraphemeClusterSegment>;

  fn deref(&self) -> &Self::Target { &self.vec_segment }
}

impl DerefMut for UnicodeString {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.vec_segment }
}
