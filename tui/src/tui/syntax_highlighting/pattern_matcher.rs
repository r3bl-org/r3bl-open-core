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

use r3bl_rs_utils_core::{ch, ChUnit};

#[derive(Debug)]
pub enum CharacterMatchResult {
  Reset,
  ResetAndKeep,
  Keep,
  KeepAndFinish,
  Finished,
  Skip,
}

/// Simple pattern matcher that matches a single character at a time. It is meant to be used to
/// perform text clipping on a single line of text, so that the syntax highlighted version is
/// clipped the same as the plain text version.
pub struct PatternMatcherStateMachine<'a> {
  pattern: &'a str,
  current_index: usize,
  is_finished: bool,
  maybe_scroll_offset_col_index: Option<ChUnit>,
}

impl<'a> PatternMatcherStateMachine<'a> {
  pub fn new(pattern: &'a str, scroll_offset_col_index: Option<ChUnit>) -> Self {
    Self {
      pattern,
      current_index: 0,
      is_finished: false,
      maybe_scroll_offset_col_index: scroll_offset_col_index,
    }
  }

  pub fn get_current_index(&self) -> usize { self.current_index }

  pub fn match_next(&mut self, character_to_test: char) -> CharacterMatchResult {
    // Skip the first N characters.
    if let Some(scroll_offset_col_index) = self.maybe_scroll_offset_col_index {
      if scroll_offset_col_index != ch!(0) {
        self.maybe_scroll_offset_col_index = (scroll_offset_col_index - 1).into();
        return CharacterMatchResult::Skip;
      }
    }

    // Check for early returns.
    if self.is_finished {
      return CharacterMatchResult::Finished;
    }

    let Some(current_pattern_char) = self.pattern.chars().nth(self.current_index) else {
        // Gone past the end of the pattern.
        self.is_finished = true;
        return CharacterMatchResult::Finished;
      };

    match current_pattern_char == character_to_test {
      true => {
        self.current_index += 1;
        if self.current_index == self.pattern.len() {
          self.is_finished = true;
          CharacterMatchResult::KeepAndFinish
        } else {
          CharacterMatchResult::Keep
        }
      }
      false => {
        // Does this match the first character of the pattern?
        if let Some(first_pattern_char) = self.pattern.chars().next() {
          if character_to_test == first_pattern_char {
            self.current_index = 1;
            return CharacterMatchResult::ResetAndKeep;
          }
        }

        // Normal reset.
        self.current_index = 0;
        CharacterMatchResult::Reset
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use r3bl_rs_utils_core::{assert_eq2, ch};

  use super::*;

  #[test]
  fn matches_occurrence_after_scroll_offset() {
    //
    //       ┌→ match this
    //       │   ┌→ don't match this
    //    ▒▒▒████▒▒▒▒
    // R ┌───────────┐
    // 0 │abcabcdabcd│
    //   └───────────┘
    //   C01234567890

    let my_line = "abcabcdabcd";
    let my_pattern = "abcd";

    let mut pattern_matcher = PatternMatcherStateMachine::new(my_pattern, ch!(5).into());

    let mut result = String::new();
    let mut final_index = 0;

    for (index, character) in my_line.chars().enumerate() {
      match pattern_matcher.match_next(character) {
        CharacterMatchResult::Skip => {
          continue;
        }
        CharacterMatchResult::Keep => {
          result.push(character);
          continue;
        }
        CharacterMatchResult::KeepAndFinish => {
          final_index = index;
          result.push(character);
          break;
        }
        CharacterMatchResult::Reset => {
          result.clear();
        }
        CharacterMatchResult::ResetAndKeep => {
          result.clear();
          result.push(character);
          continue;
        }
        CharacterMatchResult::Finished => {
          break;
        }
      }
    }

    assert_eq2!(result, my_pattern);
    assert_eq2!(final_index, 10);
  }

  #[test]
  fn matches_first_occurrence() {
    //
    //       ┌→ match this
    //       │   ┌→ don't match this
    //    ▒▒▒████▒▒▒▒
    // R ┌───────────┐
    // 0 │abcabcdabcd│
    //   └───────────┘
    //   C01234567890

    let my_line = "abcabcdabcd";
    let my_pattern = "abcd";

    let mut pattern_matcher = PatternMatcherStateMachine::new(my_pattern, None);

    let mut result = String::new();
    let mut final_index = 0;

    for (index, character) in my_line.chars().enumerate() {
      match pattern_matcher.match_next(character) {
        CharacterMatchResult::Skip => {
          continue;
        }
        CharacterMatchResult::Keep => {
          result.push(character);
          continue;
        }
        CharacterMatchResult::KeepAndFinish => {
          final_index = index;
          result.push(character);
          break;
        }
        CharacterMatchResult::Reset => {
          result.clear();
        }
        CharacterMatchResult::ResetAndKeep => {
          result.clear();
          result.push(character);
          continue;
        }
        CharacterMatchResult::Finished => {
          break;
        }
      }
    }

    assert_eq2!(result, my_pattern);
    assert_eq2!(final_index, 6);
  }

  #[test]
  fn matches_start() {
    let my_line = "abc_abcdabcd";
    let my_pattern = "abc_";

    let mut pattern_matcher = PatternMatcherStateMachine::new(my_pattern, None);

    let mut result = String::new();
    let mut final_index = 0;

    for (index, character) in my_line.chars().enumerate() {
      match pattern_matcher.match_next(character) {
        CharacterMatchResult::Skip => {
          continue;
        }
        CharacterMatchResult::Reset => {
          result.clear();
        }
        CharacterMatchResult::ResetAndKeep => {
          result.clear();
          result.push(character);
          continue;
        }
        CharacterMatchResult::Keep => {
          result.push(character);
          continue;
        }
        CharacterMatchResult::KeepAndFinish => {
          final_index = index;
          result.push(character);
          break;
        }
        CharacterMatchResult::Finished => {
          break;
        }
      }
    }

    assert_eq2!(result, my_pattern);
    assert_eq2!(final_index, 3);
  }

  #[test]
  fn matches_end() {
    let my_line = "abcabcdabcdx";
    let my_pattern = "cdx";

    let mut pattern_matcher = PatternMatcherStateMachine::new(my_pattern, None);

    let mut result = String::new();
    let mut final_index = 0;

    for (index, character) in my_line.chars().enumerate() {
      match pattern_matcher.match_next(character) {
        CharacterMatchResult::Skip => {
          continue;
        }
        CharacterMatchResult::Reset => {
          result.clear();
        }
        CharacterMatchResult::ResetAndKeep => {
          result.clear();
          result.push(character);
          continue;
        }
        CharacterMatchResult::Keep => {
          result.push(character);
          continue;
        }
        CharacterMatchResult::KeepAndFinish => {
          final_index = index;
          result.push(character);
          break;
        }
        CharacterMatchResult::Finished => {
          break;
        }
      }
    }

    assert_eq2!(result, my_pattern);
    assert_eq2!(final_index, 11);
  }
}
