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

use r3bl_core::{col, ColIndex, UnicodeString};

#[derive(Debug)]
pub enum CharacterMatchResult {
    Reset,
    ResetAndKeep,
    Keep,
    Finished,
    Skip,
}

/// Simple pattern matcher that matches a single character at a time.
///
/// It is meant to be used to perform text clipping on a single line of text, so that the
/// syntax highlighted version is clipped the same as the plain text version.
pub struct PatternMatcherStateMachine<'a> {
    pattern: &'a str,
    current_index: usize,
    is_finished: bool,
    maybe_scr_ofs_col_index: Option<ColIndex>,
}

impl<'a> PatternMatcherStateMachine<'a> {
    pub fn new(pattern: &'a str, scroll_offset_col_index: Option<ColIndex>) -> Self {
        Self {
            pattern,
            current_index: 0,
            is_finished: false,
            maybe_scr_ofs_col_index: scroll_offset_col_index,
        }
    }

    pub fn get_current_index(&self) -> usize { self.current_index }

    pub fn match_next(&mut self, character_to_test: char) -> CharacterMatchResult {
        let character_to_test_width =
            UnicodeString::char_display_width(character_to_test);

        // Skip the first "N" characters (these are display cols, so use the unicode width).
        if let Some(scroll_offset_col_index) = self.maybe_scr_ofs_col_index {
            if scroll_offset_col_index != col(0) {
                self.maybe_scr_ofs_col_index =
                    (scroll_offset_col_index - character_to_test_width).into();
                return CharacterMatchResult::Skip;
            }
        }

        // Check for early returns.
        if self.is_finished {
            return CharacterMatchResult::Finished;
        }

        let Some(current_pattern_char) = self.pattern.chars().nth(self.current_index)
        else {
            // Gone past the end of the pattern.
            self.is_finished = true;
            return CharacterMatchResult::Finished;
        };

        match current_pattern_char == character_to_test {
            true => {
                self.current_index += 1;
                CharacterMatchResult::Keep
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
    use r3bl_core::{assert_eq2, ch};

    use super::*;

    #[test]
    fn test_with_emoji() {
        let my_pattern = "🙏🏽foo";

        let my_line = "😃monkey🙏🏽foo👍bar";
        // index[0]: '😃' -> width: 2
        // index[1]: 'm' -> width: 1
        // index[2]: 'o' -> width: 1
        // index[3]: 'n' -> width: 1
        // index[4]: 'k' -> width: 1
        // index[5]: 'e' -> width: 1
        // index[6]: 'y' -> width: 1
        // index[7]: '🙏' -> width: 2 -> folded hands - Person With Folded Hands (U+1F64F)
        // index[8]: '🏽' -> width: 2  -> brown color - Emoji Modifier Fitzpatrick Type-4 (U+1F3FD)
        // index[9]: 'f' -> width: 1
        // index[10]: 'o' -> width: 1
        // index[11]: 'o' -> width: 1
        // index[12]: '👍' -> width: 2
        // index[13]: 'b' -> width: 1
        // index[14]: 'a' -> width: 1
        // index[15]: 'r' -> width: 1

        let mut final_index = 0;

        for (index, character) in my_line.chars().enumerate() {
            println!(
                "index[{a}]: '{b}' -> width: {c:?}",
                a = index,
                b = character,
                c = UnicodeString::char_display_width(character),
            );
        }

        let scroll_offset_col_index = *(UnicodeString::char_display_width('😃')
            + UnicodeString::char_display_width('m')
            + UnicodeString::char_display_width('o')
            + UnicodeString::char_display_width('n')
            + UnicodeString::char_display_width('k')
            + UnicodeString::char_display_width('e')
            + UnicodeString::char_display_width('y'));
        assert_eq2!(scroll_offset_col_index, ch(8));

        let mut pattern_matcher = PatternMatcherStateMachine::new(
            my_pattern,
            col(scroll_offset_col_index).into(),
        );
        let mut result = String::new();

        for (index, character) in my_line.chars().enumerate() {
            match pattern_matcher.match_next(character) {
                CharacterMatchResult::Skip => {
                    continue;
                }
                CharacterMatchResult::Keep => {
                    result.push(character);
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
                CharacterMatchResult::Finished => {
                    final_index = index;
                    break;
                }
            }
        }

        assert_eq2!(result, my_pattern);
        assert_eq2!(final_index, 12);
    }

    /// ```text
    ///       ⎛match this
    ///       │   ⎛don't match this
    ///    ▒▒▒████▒▒▒▒
    /// R ┌───────────┐
    /// 0 │abcabcdabcd│
    ///   └───────────┘
    ///   C01234567890
    /// ```
    #[test]
    fn matches_occurrence_after_scroll_offset() {
        let my_line = "abcabcdabcd";
        let my_pattern = "abcd";

        let mut pattern_matcher =
            PatternMatcherStateMachine::new(my_pattern, Some(col(4)));

        let mut result = String::new();
        let mut final_index = 0;

        for (index, character) in my_line.chars().enumerate() {
            final_index = index;
            match pattern_matcher.match_next(character) {
                CharacterMatchResult::Skip => {
                    continue;
                }
                CharacterMatchResult::Keep => {
                    result.push(character);
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
                CharacterMatchResult::Finished => {
                    break;
                }
            }
        }

        assert_eq2!(result, my_pattern);
        assert_eq2!(final_index, 10);
    }

    /// ```text
    ///       ⎛match this
    ///       │   ⎛don't match this
    ///    ▒▒▒████▒▒▒▒
    /// R ┌───────────┐
    /// 0 │abcabcdabcd│
    ///   └───────────┘
    ///   C01234567890
    /// ```
    #[test]
    fn matches_first_occurrence() {
        let my_line = "abcabcdabcd";
        let my_pattern = "abcd";

        let mut pattern_matcher = PatternMatcherStateMachine::new(my_pattern, None);

        let mut result = String::new();
        let mut final_index = 0;

        for (index, character) in my_line.chars().enumerate() {
            final_index = index;
            match pattern_matcher.match_next(character) {
                CharacterMatchResult::Skip => {
                    continue;
                }
                CharacterMatchResult::Keep => {
                    result.push(character);
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
                CharacterMatchResult::Finished => {
                    break;
                }
            }
        }

        assert_eq2!(result, my_pattern);
        assert_eq2!(final_index, 7);
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
                CharacterMatchResult::Finished => {
                    final_index = index;
                    break;
                }
            }
        }

        assert_eq2!(result, my_pattern);
        assert_eq2!(final_index, 4);
    }

    #[test]
    fn matches_end() {
        let my_line = "abcabcdabcdx";
        let my_pattern = "cdx";

        let mut pattern_matcher = PatternMatcherStateMachine::new(my_pattern, None);

        let mut result = String::new();
        let mut final_index = 0;

        for (index, character) in my_line.chars().enumerate() {
            final_index = index;
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
                CharacterMatchResult::Finished => {
                    break;
                }
            }
        }

        assert_eq2!(result, my_pattern);
        assert_eq2!(final_index, 11);
    }
}
