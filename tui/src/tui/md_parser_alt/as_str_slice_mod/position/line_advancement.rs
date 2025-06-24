/*
 *   Copyright (c) 2025 R3BL LLC
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

use std::convert::AsRef;

use nom::{IResult, Parser};

use crate::{as_str_slice_mod::AsStrSlice,
            CharacterIndex,
            CharacterLength,
            NErr,
            NError,
            NErrorKind};

/// Represents the overall input state for parsing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputState {
    /// Input has been exhausted - no more content to parse
    AtEndOfInput,
    /// Input still has content available for parsing
    HasMoreContent,
}

/// Represents the advancement state after a parser operation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdvancementState {
    /// Parser advanced to a new line (ideal case)
    AdvancedToNewLine,
    /// Parser made progress within the current line
    MadeCharProgress,
    /// Parser successfully handled an empty line
    HandledEmptyLine,
    /// Parser made no progress at all
    NoProgress,
}

/// Captures the initial position state before parsing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InitialParsePosition {
    pub line_index: CharacterIndex,
    pub char_index: CharacterIndex,
    pub current_taken: CharacterLength,
}

impl<'a> AsStrSlice<'a> {
    /// Ensures parser advancement with fail-safe line progression for `AsStrSlice` input.
    ///
    /// This method guarantees that parsing always makes progress by advancing to the next
    /// line when a parser succeeds but doesn't naturally advance lines. It prevents
    /// infinite loops in parsing by implementing a fail-safe advancement mechanism.
    ///
    /// # How it works
    ///
    /// 1. **Input validation**: Checks if input is exhausted before attempting to parse
    /// 2. **Parser application**: Applies the provided parser to a clone of the current
    ///    input
    /// 3. **Advancement analysis**: Determines what type of advancement occurred:
    ///    - `AdvancedToNewLine`: Parser naturally advanced to next line (ideal case)
    ///    - `MadeCharProgress`: Parser advanced within current line
    ///    - `HandledEmptyLine`: Parser handled an empty/whitespace-only line
    ///    - `NoProgress`: Parser made no advancement at all
    /// 4. **Fail-safe handling**: For cases where parser didn't advance lines, manually
    ///    advances to the beginning of the next line to ensure progress
    ///
    /// # State Management
    ///
    /// Uses clean enum-based state tracking:
    /// - `InputState`: Distinguishes between exhausted input and available content
    /// - `AdvancementState`: Categorizes different types of parser advancement
    /// - `InitialParsePosition`: Captures position before parsing for comparison
    ///
    /// # Error Handling
    ///
    /// - Returns `Eof` error when input is exhausted
    /// - Returns `Verify` error when parser makes no progress (prevents infinite loops)
    /// - Propagates parser-specific errors unchanged
    ///
    /// # Usage Pattern
    ///
    /// This method is designed to be called within closure-based parser alternatives,
    /// typically used with [`nom::branch::alt()`]:
    ///
    /// ```
    /// # use r3bl_tui::*;
    /// # use nom::{branch::alt, combinator::map, IResult};
    /// # use nom::Parser as _;
    /// #
    /// # fn some_parser_function<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    /// #     nom::bytes::complete::tag("test")(input)
    /// # }
    /// # fn another_parser_function<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    /// #     nom::bytes::complete::tag("other")(input)
    /// # }
    /// # fn transform_output(s: AsStrSlice<'_>) -> String { s.extract_to_line_end().to_string() }
    /// # fn another_transform(s: AsStrSlice<'_>) -> String { format!("transformed: {}", s.extract_to_line_end()) }
    ///
    /// // Example usage with a single parser
    /// fn example_parser(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     input.ensure_advance_with_parser(&mut map(
    ///         some_parser_function,
    ///         transform_output,
    ///     ))
    /// }
    ///
    /// // Helper functions for alt() usage (avoids closure lifetime issues)
    /// fn parser_branch_1(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     input.ensure_advance_with_parser(&mut map(
    ///         some_parser_function,
    ///         transform_output,
    ///     ))
    /// }
    ///
    /// fn parser_branch_2(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     input.ensure_advance_with_parser(&mut map(
    ///         another_parser_function,
    ///         another_transform,
    ///     ))
    /// }
    ///
    /// // Example usage in alt() chain
    /// fn parse_alternatives(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, String> {
    ///     let mut parser = alt([parser_branch_1, parser_branch_2]);
    ///     parser.parse(input)
    /// }
    /// ```
    ///
    /// # Parameters
    ///
    /// * `parser` - A mutable reference to a nom parser that operates on `AsStrSlice`
    ///   input. The mutable reference is required by nom's `Parser` trait implementation.
    ///
    /// # Returns
    ///
    /// * `Ok((remainder, output))` - Parser succeeded with guaranteed line advancement
    /// * `Err(nom::Err)` - Parser failed or input was exhausted
    ///
    /// # See Also
    ///
    /// * `determine_input_state` - Input exhaustion detection
    /// * `handle_parser_advancement` - Core advancement logic
    /// * [`crate::ensure_advance_fail_safe_alt`] - Legacy wrapper function for backward
    ///   compatibility (deprecated in favor of this method)
    pub fn ensure_advance_with_parser<F, O>(
        &self,
        parser: &mut F,
    ) -> IResult<AsStrSlice<'a>, O>
    where
        F: Parser<AsStrSlice<'a>, Output = O, Error = nom::error::Error<AsStrSlice<'a>>>,
    {
        // Check input state before attempting to parse.
        let input_state = self.determine_input_state();
        if let InputState::AtEndOfInput = input_state {
            return Err(NErr::Error(NError::new(self.clone(), NErrorKind::Eof)));
        }

        // Capture initial state and apply parser.
        let initial_position = self.capture_initial_position();
        let result = parser.parse(self.clone());

        match result {
            Ok((remainder, output)) => {
                let advancement_result =
                    self.handle_parser_advancement(initial_position, remainder)?;
                Ok((advancement_result, output))
            }
            Err(e) => Err(e),
        }
    }

    /// Determines if input has been exhausted.
    fn determine_input_state(&self) -> InputState {
        if self.line_index >= self.lines.len().into()
            || self.current_taken >= self.total_size
        {
            InputState::AtEndOfInput
        } else {
            InputState::HasMoreContent
        }
    }

    /// Captures the current position state before parsing.
    fn capture_initial_position(&self) -> InitialParsePosition {
        InitialParsePosition {
            line_index: self.line_index,
            char_index: self.char_index,
            current_taken: self.current_taken,
        }
    }

    /// Determines what type of advancement occurred after parsing.
    fn determine_advancement_state(
        &self,
        initial_position: InitialParsePosition,
        remainder: &AsStrSlice<'a>,
    ) -> AdvancementState {
        // Check if parser advanced to a new line (ideal case).
        if remainder.line_index > initial_position.line_index {
            return AdvancementState::AdvancedToNewLine;
        }

        // Check if parser made progress within the current line.
        let made_char_progress = remainder.current_taken > initial_position.current_taken
            || remainder.char_index > initial_position.char_index;

        if made_char_progress {
            return AdvancementState::MadeCharProgress;
        }

        // Check if we're dealing with an empty line.
        let current_line = remainder
            .lines
            .get(remainder.line_index.as_usize())
            .map(|line| line.as_ref())
            .unwrap_or("");

        if current_line.trim().is_empty() {
            return AdvancementState::HandledEmptyLine;
        }

        AdvancementState::NoProgress
    }

    /// Handles the advancement logic based on parser results.
    fn handle_parser_advancement(
        &self,
        initial_position: InitialParsePosition,
        remainder: AsStrSlice<'a>,
    ) -> Result<AsStrSlice<'a>, NErr<NError<AsStrSlice<'a>>>> {
        let advancement_state =
            self.determine_advancement_state(initial_position, &remainder);

        match advancement_state {
            AdvancementState::AdvancedToNewLine => {
                // Parser already made proper line advancement.
                Ok(remainder)
            }
            AdvancementState::MadeCharProgress | AdvancementState::HandledEmptyLine => {
                // Need to manually advance to next line.
                self.advance_to_next_line(remainder)
            }
            AdvancementState::NoProgress => {
                // Check if we're at end of input.
                if remainder.determine_input_state() == InputState::AtEndOfInput {
                    Err(NErr::Error(NError::new(self.clone(), NErrorKind::Eof)))
                } else {
                    // No progress made - return error to break parsing loop.
                    Err(NErr::Error(NError::new(self.clone(), NErrorKind::Verify)))
                }
            }
        }
    }

    /// Advances the slice to the beginning of the next line.
    fn advance_to_next_line(
        &self,
        mut remainder: AsStrSlice<'a>,
    ) -> Result<AsStrSlice<'a>, NErr<NError<AsStrSlice<'a>>>> {
        // Ensure we're within valid line bounds.
        if remainder.line_index >= remainder.lines.len().into() {
            return Err(NErr::Error(NError::new(self.clone(), NErrorKind::Eof)));
        }

        // Get current line length.
        let current_line_len = remainder
            .lines
            .get(remainder.line_index.as_usize())
            .map(|line| line.as_ref().chars().count())
            .unwrap_or(0);

        // Advance to end of current line if not already there.
        if remainder.char_index.as_usize() < current_line_len {
            let chars_to_advance = current_line_len - remainder.char_index.as_usize();
            for _ in 0..chars_to_advance {
                remainder.advance();
            }
        }

        // Check if we can advance to the next line.
        if remainder.line_index.as_usize() < remainder.lines.len() - 1 {
            // Create a fresh AsStrSlice at the next line with no max_len constraint.
            let next_line_index = remainder.line_index + crate::idx(1);
            remainder = AsStrSlice::with_limit(
                remainder.lines,
                next_line_index,
                crate::idx(0), // Start at beginning of next line.
                None,          // Remove max_len constraint
            );
        }

        Ok(remainder)
    }

    /// Helper method to check if the current line is empty or whitespace-only.
    pub fn is_current_line_empty_or_whitespace(&self) -> bool {
        self.lines
            .get(self.line_index.as_usize())
            .map(|line| line.as_ref().trim().is_empty())
            .unwrap_or(true)
    }

    /// Helper method to get the current line as a string reference.
    pub fn get_current_line(&self) -> Option<&str> {
        self.lines
            .get(self.line_index.as_usize())
            .map(|line| line.as_ref())
    }
}

#[cfg(test)]
mod tests_ensure_advance_with_parser {
    use nom::{bytes::complete::tag, IResult};

    use super::*;
    use crate::{assert_eq2, GCString};

    fn simple_parser(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, AsStrSlice<'_>> {
        tag("test")(input)
    }

    fn empty_line_parser(input: AsStrSlice<'_>) -> IResult<AsStrSlice<'_>, ()> {
        if input.is_current_line_empty_or_whitespace() {
            Ok((input, ()))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    }

    #[test]
    fn test_parser_advances_to_new_line() {
        let lines = [GCString::new("test"), GCString::new("next")];
        let input = AsStrSlice::from(&lines);

        let result = input.ensure_advance_with_parser(&mut simple_parser);
        assert!(result.is_ok());

        let (remainder, _) = result.unwrap();
        assert_eq2!(remainder.line_index, crate::idx(1));
        assert_eq2!(remainder.char_index, crate::idx(0));
    }

    #[test]
    fn test_parser_handles_empty_line() {
        let lines = [GCString::new(""), GCString::new("next")];
        let input = AsStrSlice::from(&lines);

        let result = input.ensure_advance_with_parser(&mut empty_line_parser);
        assert!(result.is_ok());

        let (remainder, _) = result.unwrap();
        assert_eq2!(remainder.line_index, crate::idx(1));
    }

    #[test]
    fn test_parser_at_end_of_input() {
        let lines = [GCString::new("test")];
        let mut input = AsStrSlice::from(&lines);
        input.line_index = crate::idx(1); // Beyond available lines

        let result = input.ensure_advance_with_parser(&mut simple_parser);
        assert!(result.is_err());

        if let Err(nom::Err::Error(error)) = result {
            assert_eq2!(error.code, nom::error::ErrorKind::Eof);
        }
    }

    #[test]
    fn test_determine_input_state() {
        let lines = [GCString::new("test")];
        let input = AsStrSlice::from(&lines);

        assert_eq2!(input.determine_input_state(), InputState::HasMoreContent);

        let mut exhausted_input = input;
        exhausted_input.line_index = crate::idx(1);
        assert_eq2!(
            exhausted_input.determine_input_state(),
            InputState::AtEndOfInput
        );
    }

    #[test]
    fn test_capture_initial_position() {
        let lines = [GCString::new("test")];
        let input = AsStrSlice::from(&lines);

        let position = input.capture_initial_position();
        assert_eq2!(position.line_index, input.line_index);
        assert_eq2!(position.char_index, input.char_index);
        assert_eq2!(position.current_taken, input.current_taken);
    }
}
