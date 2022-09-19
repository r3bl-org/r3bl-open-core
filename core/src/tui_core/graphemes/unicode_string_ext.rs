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

use std::{borrow::Cow,
          ops::{Deref, DerefMut}};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::*;

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ UnicodeStringExt │
// ╯                  ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// A grapheme cluster is a user-perceived character. Rust uses `UTF-8` to
/// represent text in `String`. So each character takes up 8 bits or one byte.
/// Grapheme clusters can take up many more bytes, eg 4 bytes or 2 or 3, etc.
///
/// Docs:
///
/// - [Grapheme clusters](https://medium.com/flutter-community/working-with-unicode-and-grapheme-clusters-in-dart-b054faab5705)
/// - [UTF-8 String](https://doc.rust-lang.org/book/ch08-02-strings.html)
///
/// There is a discrepancy between how a `String` that contains grapheme
/// clusters is represented in memory and how it is rendered in a terminal. When
/// writing an TUI editor it is necessary to have a "logical" cursor that the
/// user can move by pressing up, down, left, right, etc. For left, this
/// is assumed to move the caret or cursor one position to the left. Let's
/// unpack that.
///
/// 1. If we use byte boundaries in the `String` we can move the cursor one byte
/// to the left. 2. This falls apart when we have a grapheme cluster.
/// 3. A grapheme cluster can take up more than one byte, and they don't fall
/// cleanly into byte boundaries.
///
/// To complicate things further, the size that a grapheme cluster takes up is
/// not the same as its byte size in memory. Let's unpack that.
///
/// | Character | Byte size | Grapheme cluster size | Compound |
/// | --------- | --------- | --------------------- | -------- |
/// | `H`       | 1         | 1                     | No       |
/// | `😃`      | 4         | 2                     | No       |
/// | `📦`      | 4         | 2                     | No       |
/// | `🙏🏽`      | 4         | 2                     | Yes      |
///
/// Here are examples of compound grapheme clusters.
///
/// ```ignore
/// 🏽 + 🙏 = 🙏🏽
/// 🏾‍ + 👨 + 🤝‍ + 👨 +  🏿 = 👨🏾‍🤝‍👨🏿
/// ```
///
/// Let's say you're browsing this source file in VSCode. The UTF-8 string this
/// Rust source file is rendered by VSCode correctly. But this is not how it
/// looks in a terminal. And the size of the string in memory isn't clear either
/// from looking at the string in VSCode. It isn't apparent that you can't just
/// index into the string at byte boundaries.
///
/// To further complicate things, the output looks different on different
/// terminals & OSes. The
/// function `test_crossterm_grapheme_cluster_width_calc()` (shown below) uses
/// crossterm commands to try and figure out what the width of a grapheme
/// cluster is. When you run this in an SSH session to a macOS machine from
/// Linux, it will work the same way it would locally on Linux. However, if
/// you run the same program in locally via Terminal.app on macOS it works
/// differently! So there are some serious issues.
///
/// ```ignore
/// pub fn test_crossterm_grapheme_cluster_width_calc() -> Result<()> {
///   // Enter raw mode, clear screen.
///   enable_raw_mode()?;
///   execute!(stdout(), EnterAlternateScreen)?;
///   execute!(stdout(), Clear(ClearType::All))?;
///   execute!(stdout(), MoveTo(0, 0))?;
///
///   // Perform test of grapheme cluster width.
///   #[derive(Default, Debug, Clone, Copy)]
///   struct Positions {
///     orig_pos: (u16, u16),
///     new_pos: (u16, u16),
///     col_width: u16,
///   }
///
///   let mut map = HashMap::<&str, Positions>::new();
///   map.insert("Hi", Positions::default());
///   map.insert(" ", Positions::default());
///   map.insert("😃", Positions::default());
///   map.insert("📦", Positions::default());
///   map.insert("🙏🏽", Positions::default());
///   map.insert("👨🏾‍🤝‍👨🏿", Positions::default());
///   map.insert(".", Positions::default());
///
///   fn process_map(map: &mut HashMap<&str, Positions>) -> Result<()> {
///     for (index, (key, value)) in map.iter_mut().enumerate() {
///       let orig_pos: (u16, u16) = (/* col: */ 0, /* row: */ index as u16);
///       execute!(stdout(), MoveTo(orig_pos.0, orig_pos.1))?;
///       execute!(stdout(), Print(key))?;
///       let new_pos = cursor::position()?;
///       value.new_pos = new_pos;
///       value.orig_pos = orig_pos;
///       value.col_width = new_pos.0 - orig_pos.0;
///     }
///     Ok(())
///   }
///
///   process_map(&mut map)?;
///
///   // Just blocking on user input.
///   {
///     execute!(stdout(), Print("... Press any key to continue ..."))?;
///     if let Event::Key(_) = read()? {
///       execute!(stdout(), terminal::Clear(ClearType::All))?;
///       execute!(stdout(), cursor::MoveTo(0, 0))?;
///     }
///   }
///
///   // Exit raw mode, clear screen.
///   execute!(stdout(), terminal::Clear(ClearType::All))?;
///   execute!(stdout(), cursor::MoveTo(0, 0))?;
///   execute!(stdout(), LeaveAlternateScreen)?;
///   disable_raw_mode().expect("Unable to disable raw mode");
///   println!("map:{:#?}", map);
///
///   Ok(())
/// }
/// ```
///
/// The basic problem arises from the fact that it isn't possible to treat the
/// "logical" index into the string (which isn't byte boundary based) as a
/// "physical" index into the rendered output of the string in a terminal.
///
/// 1. Some parsing is necessary to get "logical" index into the string that is
/// grapheme cluster based (not byte boundary based).
///    - This is where [`unicode-segmentation`](https://crates.io/crates/unicode-segmentation)
///      crate comes in and allows us to split our string into a vector of
///      grapheme clusters.
/// 2. Some translation is necessary to get from the "logical" index to the
/// physical index and back again. This is where we can apply one of the
/// following approaches:
///    - We can use the [`unicode-width`](https://crates.io/crates/unicode-width)
///      crate to calculate the width of the grapheme cluster. This works on
///      Linux, but doesn't work very well on macOS & I haven't tested it on
///      Windows. This crate will (on Linux) reliably tell us what the displayed
///      width of a grapheme cluster is.
///    - We can take the approach from the [`reedline`](https://crates.io/crates/reedline)
///      crate's [`repaint_buffer()`](https://github.com/nazmulidris/reedline/blob/79e7d8da92cd5ae4f8e459f901189d7419c3adfd/src/painting/painter.rs#L129)
///      where we split the string based on the "logical" index into the vector
///      of grapheme clusters. And then we print the 1st part of the string,
///      then call `SavePosition` to save the cursor at this point, then print
///      the 2nd part of the string, then call `RestorePosition` to restore the
///      cursor to where it "should" be.
pub trait UnicodeStringExt {
  fn unicode_string(&self) -> UnicodeString;
}

impl UnicodeStringExt for Cow<'_, str> {
  fn unicode_string(&self) -> UnicodeString { make_unicode_string(self.as_ref()) }
}

fn make_unicode_string(this: &'_ str) -> UnicodeString<'_> {
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

impl UnicodeStringExt for String {
  fn unicode_string(&self) -> UnicodeString { make_unicode_string(self) }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ GraphemeClusterSegment │
// ╯                        ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[derive(Debug, Clone)]
pub struct GraphemeClusterSegment<'a> {
  /// The actual grapheme cluster `&str`. Eg: "H", "📦", "🙏🏽".
  pub string: Cow<'a, str>,
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

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ UnicodeStringSegmentSliceResult │
// ╯                                 ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// We need a [String] (since we're returning a slice of a temporary [UnicodeString] that is
/// dropped by the function that creates it, not as a result of mutation).
#[derive(Debug, PartialEq, Eq)]
pub struct UnicodeStringSegmentSliceResult {
  pub str_seg: String,
  pub unicode_width: ChUnit,
  pub display_col_at_which_seg_starts: ChUnit,
}

impl UnicodeStringSegmentSliceResult {
  pub fn new(
    string: &str, unicode_width: ChUnit, display_col_at_which_this_segment_starts: ChUnit,
  ) -> Self {
    Self {
      str_seg: string.to_string(),
      unicode_width,
      display_col_at_which_seg_starts: display_col_at_which_this_segment_starts,
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ NewUnicodeStringResult │
// ╯                        ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
/// We need a [String] (since we're returning a new [String] as a result of this [UnicodeString]
/// mutation).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct NewUnicodeStringResult {
  pub new_string: String,
  pub unicode_width: ChUnit,
}

impl NewUnicodeStringResult {
  pub fn new(new_string: String, unicode_width: ChUnit) -> Self {
    Self {
      new_string,
      unicode_width,
    }
  }
}

// ╭┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄╮
// │ UnicodeString │
// ╯               ╰┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄
#[derive(Debug, Clone)]
pub struct UnicodeString<'a> {
  pub string: Cow<'a, str>,
  pub vec_segment: Vec<GraphemeClusterSegment<'a>>,
  pub byte_size: usize,
  pub grapheme_cluster_segment_count: usize,
  pub display_width: ChUnit,
}

impl<'a> Deref for UnicodeString<'a> {
  type Target = Vec<GraphemeClusterSegment<'a>>;

  fn deref(&self) -> &Self::Target { &self.vec_segment }
}

impl<'a> DerefMut for UnicodeString<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { &mut self.vec_segment }
}

impl<'a> UnicodeString<'a> {
  /// If any segment in `self.vec_segment` has a `display_col_offset` greater than 1 then this is
  /// true. The semantic is that the string is displayed using more than 1 column of the terminal.
  pub fn contains_wide_segments(&self) -> bool {
    let mut contains_wide_segments = false;

    for grapheme_cluster_segment in self.iter() {
      if grapheme_cluster_segment.unicode_width > ch!(1) {
        contains_wide_segments = true;
        break;
      }
    }

    contains_wide_segments
  }

  pub fn char_display_width(character: char) -> usize {
    let display_width: usize = UnicodeWidthChar::width(character).unwrap_or(0);
    display_width
  }

  pub fn str_display_width(string: &str) -> usize {
    let display_width: usize = UnicodeWidthStr::width(string);
    display_width
  }

  pub fn truncate_to_fit_size(&self, size: Size) -> &str {
    let display_cols: ChUnit = size.col;
    self.truncate_to_fit_display_cols(display_cols)
  }

  pub fn truncate_to_fit_display_cols(&self, display_cols: ChUnit) -> &str {
    let mut avail_cols = display_cols;
    let mut string_end_byte_index = 0;

    for segment in self.iter() {
      if avail_cols < segment.unicode_width {
        break;
      }
      string_end_byte_index += segment.byte_size;
      avail_cols -= segment.unicode_width;
    }

    &self.string[..string_end_byte_index]
  }

  /// `local_index` is the index of the grapheme cluster in the `vec_segment`.
  pub fn at_logical_index(&self, logical_index: usize) -> Option<&GraphemeClusterSegment> {
    self.get(logical_index)
  }

  /// `display_col` is the col index in the terminal where this grapheme cluster can be displayed.
  pub fn at_display_col(&self, display_col: ChUnit) -> Option<&GraphemeClusterSegment> {
    self.iter().find(|&grapheme_cluster_segment| {
      let segment_display_col_start: ChUnit = grapheme_cluster_segment.display_col_offset;
      let segment_display_col_end: ChUnit =
        segment_display_col_start + grapheme_cluster_segment.unicode_width;
      display_col >= segment_display_col_start && display_col < segment_display_col_end
    })
  }

  /// Convert a `display_col` to a `logical_index`.
  /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
  /// - `display_col` is the col index in the terminal where this grapheme cluster can be displayed.
  pub fn logical_index_at_display_col(&self, display_col: ChUnit) -> Option<usize> {
    self
      .at_display_col(display_col)
      .map(|segment| segment.logical_index)
  }

  /// Convert a `logical_index` to a `display_col`.
  /// - `local_index` is the index of the grapheme cluster in the `vec_segment`.
  /// - `display_col` is the col index in the terminal where this grapheme cluster can be displayed.
  pub fn display_col_at_logical_index(&self, logical_index: usize) -> Option<ChUnit> {
    self
      .at_logical_index(logical_index)
      .map(|segment| segment.display_col_offset)
  }

  /// Return the string and unicode width of the grapheme cluster segment at the given `display_col`.
  /// If this `display_col` falls in the middle of a grapheme cluster, then return [None].
  pub fn get_string_at_display_col(
    &self, display_col: ChUnit,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    let segment = self.at_display_col(display_col)?;
    // What if the display_col is in the middle of a grapheme cluster?
    if display_col != segment.display_col_offset {
      None
    } else {
      Some(UnicodeStringSegmentSliceResult::new(
        &segment.string,
        segment.unicode_width,
        segment.display_col_offset,
      ))
    }
  }

  /// If the given `display_col` falls in the middle of a grapheme cluster, then return the
  /// [GraphemeClusterSegment] at that `display_col`. Otherwise return [None].
  pub fn is_display_col_in_middle_of_grapheme_cluster(
    &self, display_col: ChUnit,
  ) -> Option<GraphemeClusterSegment> {
    let segment = self.at_display_col(display_col);
    if let Some(segment) = segment {
      if display_col != segment.display_col_offset {
        return Some(segment.clone());
      }
    }
    None
  }

  pub fn get_string_at_left_of_display_col(
    &self, display_col: ChUnit,
  ) -> Option<UnicodeStringSegmentSliceResult> {
    let segment_at_col = self.at_display_col(display_col)?;
    if segment_at_col.logical_index > 0 {
      let segment_left_of_col = self.at_logical_index(segment_at_col.logical_index - 1)?;
      Some(UnicodeStringSegmentSliceResult::new(
        &segment_left_of_col.string,
        segment_left_of_col.unicode_width,
        segment_left_of_col.display_col_offset,
      ))
    } else {
      None
    }
  }

  pub fn get_string_at_end(&self) -> Option<UnicodeStringSegmentSliceResult> {
    let segment = self.last()?;
    Some(UnicodeStringSegmentSliceResult::new(
      &segment.string,
      segment.unicode_width,
      segment.display_col_offset,
    ))
  }
}

pub mod mutate {
  use super::*;

  /// Convert [&str] to [GraphemeClusterSegment]. This is used to create a new [String] after the
  /// [UnicodeString] is modified.
  pub fn make_new_grapheme_cluster_segment_from(chunk: &str) -> GraphemeClusterSegment {
    let my_string: String = chunk.to_string();
    let result = my_string.unicode_string()[0].clone();

    GraphemeClusterSegment {
      string: Cow::Owned(result.string.into()),
      ..result
    }
  }

  /// Convert [Vec<GraphemeClusterSegment>] to [String]. This is used to create a new [String] after
  /// the [UnicodeString] is modified.
  pub fn make_new_string_from(vec_grapheme_cluster_segment: Vec<GraphemeClusterSegment>) -> String {
    let mut my_string = String::new();
    for grapheme_cluster_segment in vec_grapheme_cluster_segment {
      my_string.push_str(&grapheme_cluster_segment.string);
    }
    my_string
  }

  impl<'a> UnicodeString<'a> {
    /// Returns a new [String]. Does not modify [self.string](UnicodeString::string).
    pub fn merge_with(&self, other: UnicodeString) -> Option<String> {
      let mut new_string: String = self.string.as_ref().into();
      new_string.push_str(&other.string);
      Some(new_string)
    }

    /// Returns a new ([NewUnicodeStringResult]) tuple. Does not modify
    /// [self.string](UnicodeString::string).
    pub fn insert_char_at_display_col(
      &self, display_col: ChUnit, chunk: &str,
    ) -> Option<NewUnicodeStringResult> {
      let maybe_logical_index = self.logical_index_at_display_col(display_col);
      match maybe_logical_index {
        // Insert somewhere inside bounds of self.string.
        Some(logical_index) => {
          // Convert the character into a grapheme cluster.
          let character_g_c_s: GraphemeClusterSegment =
            make_new_grapheme_cluster_segment_from(chunk);
          let character_display_width: ChUnit = character_g_c_s.unicode_width;

          // Insert self grapheme cluster to self.vec_segment.
          let mut vec_segment_clone = self.vec_segment.clone();
          vec_segment_clone.insert(logical_index, character_g_c_s);

          // Generate a new string from self.vec_segment and return it and the unicode width of the
          // character.
          let new_string = make_new_string_from(vec_segment_clone);

          // In the caller - update the caret position based on the unicode width of the character.
          Some(NewUnicodeStringResult::new(
            new_string,
            character_display_width,
          ))
        }
        // Add to end of self.string.
        None => {
          // Push character to the end of the cloned string.
          let mut new_string: String = self.string.as_ref().into();
          new_string.push_str(chunk);

          // Get the unicode width of the character.
          let character_display_width = UnicodeString::str_display_width(chunk);

          // In the caller - update the caret position based on the unicode width of the character.
          Some(NewUnicodeStringResult::new(
            new_string,
            ch!(character_display_width),
          ))
        }
      }
    }

    /// Returns two new tuples: *left* ([NewUnicodeStringResult]), *right*
    /// ([NewUnicodeStringResult]). Does not modify [self.string](UnicodeString::string).
    pub fn split_at_display_col(
      &self, display_col: ChUnit,
    ) -> Option<(NewUnicodeStringResult, NewUnicodeStringResult)> {
      let split_logical_index = self.logical_index_at_display_col(display_col)?;
      let max_logical_index = self.len();

      let mut str_left = String::new();
      let mut str_left_unicode_width = ch!(0);
      {
        for logical_idx in 0..split_logical_index {
          let segment = self.at_logical_index(logical_idx)?;
          str_left.push_str(&segment.string);
          str_left_unicode_width += segment.unicode_width;
        }
      }

      let mut str_right = String::new();
      let mut str_right_unicode_width = ch!(0);
      {
        for logical_idx in split_logical_index..max_logical_index {
          let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
          str_right.push_str(&seg_at_logical_idx.string);
          str_right_unicode_width += seg_at_logical_idx.unicode_width;
        }
      }

      if *str_right_unicode_width > 0 || *str_left_unicode_width > 0 {
        Some((
          NewUnicodeStringResult::new(str_left, str_left_unicode_width),
          NewUnicodeStringResult::new(str_right, str_right_unicode_width),
        ))
      } else {
        None
      }
    }

    /// Returns a new ([NewUnicodeStringResult]) tuple. Does not modify
    /// [self.string](UnicodeString::string).
    pub fn delete_char_at_display_col(
      &self, display_col: ChUnit,
    ) -> Option<NewUnicodeStringResult> {
      // There is only one segment present.
      if self.len() == 1 {
        return Some(NewUnicodeStringResult::default());
      }

      // There are more than 1 segments present.i
      let split_logical_index = self.logical_index_at_display_col(display_col)?;
      let max_logical_index = self.len();

      let mut str_left = String::new();
      let mut str_left_unicode_width = ch!(0);
      {
        for logical_idx in 0..split_logical_index {
          let segment = self.at_logical_index(logical_idx)?;
          str_left.push_str(&segment.string);
          str_left_unicode_width += segment.unicode_width;
        }
      }

      let skip_split_logical_index = split_logical_index + 1; // Drop one segment.
      let mut str_right = String::new();
      let mut str_right_unicode_width = ch!(0);
      {
        for logical_idx in skip_split_logical_index..max_logical_index {
          let seg_at_logical_idx = self.at_logical_index(logical_idx)?;
          str_right.push_str(&seg_at_logical_idx.string);
          str_right_unicode_width += seg_at_logical_idx.unicode_width;
        }
      }

      str_left.push_str(&str_right);
      str_left_unicode_width += str_right_unicode_width;

      if *str_left_unicode_width > 0 {
        Some(NewUnicodeStringResult::new(
          str_left,
          str_left_unicode_width,
        ))
      } else {
        None
      }
    }
  }
}
