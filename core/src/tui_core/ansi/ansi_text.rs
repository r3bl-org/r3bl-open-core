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

use std::borrow::Cow;

use ansi_parser::{AnsiParser, AnsiSequence, Output};

pub trait ANSITextExt {
    fn ansi_text(&self) -> ANSIText;
}

mod ansi_text_ext_trait_impl {
    use super::*;

    impl ANSITextExt for Cow<'_, str> {
        fn ansi_text(&self) -> ANSIText { ANSIText::new(self) }
    }

    impl ANSITextExt for &str {
        fn ansi_text(&self) -> ANSIText { ANSIText::new(self) }
    }

    impl ANSITextExt for String {
        fn ansi_text(&self) -> ANSIText { ANSIText::new(self) }
    }
}

/// Parses a given string into a vector of [ANSIOutput]s. These can be cached if needed since the
/// [constructor](ANSIText::new) is a very expensive function to call.
#[derive(Debug, PartialEq, Default, Clone)]
pub struct ANSIText {
    pub ansi_text: String,
    pub vec_segment: Vec<ANSITextSegment>,
    pub display_width: usize,
}

mod impl_ansi_text {
    use std::ops::Deref;

    use super::*;

    impl From<ANSIText> for String {
        fn from(ansi_text: ANSIText) -> Self { ansi_text.ansi_text }
    }

    impl From<&ANSIText> for String {
        fn from(ansi_text: &ANSIText) -> Self { ansi_text.ansi_text.clone() }
    }

    impl Deref for ANSIText {
        type Target = Vec<ANSITextSegment>;

        fn deref(&self) -> &Self::Target { &self.vec_segment }
    }

    impl ANSIText {
        pub fn len(&self) -> usize { self.vec_segment.len() }

        pub fn is_empty(&self) -> bool { self.vec_segment.is_empty() }

        /// If conversion was successful and ANSI characters were stripped, returns a [String],
        /// otherwise returns [None].
        pub fn try_strip_ansi(text: &str) -> Option<String> {
            if let Ok(vec_u8) = strip_ansi_escapes::strip(text) {
                let result_text_plain = std::str::from_utf8(&vec_u8);
                if let Ok(stripped_text) = result_text_plain {
                    if text != stripped_text {
                        return stripped_text.to_string().into();
                    }
                }
            }
            None
        }

        pub fn new(text: &str) -> Self {
            let vec_output: Vec<Output> = text.ansi_parse().collect();

            let mut total_display_width = 0;
            let mut vec_segment = vec![];

            let mut temp_segment = ANSITextSegment::new();

            for output in &vec_output {
                match output {
                    Output::TextBlock(text) => {
                        // Save the current one.
                        temp_segment.vec_output.push(ANSIOutput::from(output));
                        let text_unicode_width = unicode_width::UnicodeWidthStr::width(*text);
                        temp_segment.display_width = text_unicode_width;
                        vec_segment.push(temp_segment);

                        // Start a new one.
                        temp_segment = ANSITextSegment::new();

                        // Update total display width.
                        total_display_width += text_unicode_width;
                    }
                    Output::Escape(_) => {
                        // Save the current output.
                        temp_segment.vec_output.push(ANSIOutput::from(output));
                    }
                }
            }

            if !vec_segment.contains(&temp_segment) {
                vec_segment.push(temp_segment);
            }

            // Return.
            Self {
                ansi_text: text.to_string(),
                vec_segment,
                display_width: total_display_width,
            }
        }

        /// Filters the [ANSITextSegment]s based on the given `max_display_col_count`. This consumes
        /// `self`.
        /// 1. If `max_display_col_count` is [None], return all the segments that are delimited by
        ///    an [Output::TextBlock].
        /// 2. If `max_display_col_count` is provided, return the maximum number of segments that
        ///    will fit in the given display column width. In other words this works similar to the
        ///    clip function of
        ///    [truncate_end_to_fit_width](crate::UnicodeString::truncate_end_to_fit_width).
        pub fn filter_segments_by_display_width(
            self,
            max_display_col_count: Option<usize>,
        ) -> Self {
            if let Some(max_display_col_count) = max_display_col_count {
                // No need to filter if the display width is less than the max.
                if self.display_width <= max_display_col_count {
                    return self;
                }

                // Filter the `vec_segment` by `max_display_col_count`.
                let mut total_display_width = 0;
                let mut vec_segments_filtered = vec![];

                for segment in self.vec_segment {
                    let lhs = total_display_width + segment.display_width;
                    let rhs = max_display_col_count;
                    if lhs > rhs {
                        break;
                    }
                    if segment.display_width != 0 {
                        total_display_width += segment.display_width;
                        vec_segments_filtered.push(segment);
                    }
                }

                return Self {
                    ansi_text: self.ansi_text,
                    vec_segment: vec_segments_filtered,
                    display_width: total_display_width,
                };
            }

            self
        }
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct ANSITextSegment {
    pub vec_output: Vec<ANSIOutput>,
    /// [crate::UnicodeString] display width.
    pub display_width: usize,
}

mod impl_ansi_text_segment {
    use super::*;

    impl From<&ANSITextSegment> for String {
        fn from(ansi_text_segment: &ANSITextSegment) -> Self {
            let mut result = String::new();
            for output2 in &ansi_text_segment.vec_output {
                match output2 {
                    ANSIOutput::TextBlock(text) => result.push_str(&text.to_string()),
                    ANSIOutput::Escape(ansi_sequence) => {
                        result.push_str(&ansi_sequence.to_string())
                    }
                }
            }
            result
        }
    }

    impl From<ANSITextSegment> for String {
        fn from(ansi_text_segment: ANSITextSegment) -> Self { String::from(&ansi_text_segment) }
    }

    impl ANSITextSegment {
        pub fn new() -> Self {
            Self {
                vec_output: vec![],
                display_width: 0,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ANSIOutput {
    TextBlock(String),
    Escape(AnsiSequence),
}

mod impl_ansi_output {
    use core::fmt::{Display, Formatter, Result as DisplayResult};

    use super::*;

    impl Display for ANSIOutput {
        /// This automatically generates the `to_string()` method which is used above.
        fn fmt(&self, formatter: &mut Formatter) -> DisplayResult {
            match self {
                ANSIOutput::TextBlock(txt) => write!(formatter, "{txt}"),
                ANSIOutput::Escape(seq) => write!(formatter, "{seq}"),
            }
        }
    }

    impl From<&Output<'_>> for ANSIOutput {
        fn from(value: &Output) -> Self {
            match value {
                Output::TextBlock(text) => Self::TextBlock(text.to_string()),
                Output::Escape(ansi_sequence) => Self::Escape(ansi_sequence.clone()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_lolcat_no_max_display_cols() {
        let test_data = "\u{1b}[38;2;51;254;77mS\u{1b}[39m\u{1b}[38;2;52;254;77mt\u{1b}[39m\u{1b}[38;2;52;254;77ma\u{1b}[39m\u{1b}[38;2;52;254;76mt\u{1b}[39m\u{1b}[38;2;53;254;76me\u{1b}[39m\u{1b}[38;2;53;254;76m \u{1b}[39m\u{1b}[38;2;53;254;75m{\u{1b}[39m\u{1b}[38;2;54;254;75m \u{1b}[39m\u{1b}[38;2;54;254;74ms\u{1b}[39m\u{1b}[38;2;54;254;74mt\u{1b}[39m\u{1b}[38;2;55;254;74ma\u{1b}[39m\u{1b}[38;2;55;254;73mc\u{1b}[39m\u{1b}[38;2;56;254;73mk\u{1b}[39m\u{1b}[38;2;56;254;72m:\u{1b}[39m\u{1b}[38;2;56;254;72m \u{1b}[39m\u{1b}[38;2;57;254;72m[\u{1b}[39m\u{1b}[38;2;57;254;71m0\u{1b}[39m\u{1b}[38;2;57;254;71m]\u{1b}[39m\u{1b}[38;2;58;254;71m \u{1b}[39m\u{1b}[38;2;58;254;70m}\u{1b}[39m";
        let unparsed_ansi_text = ANSIText::new(test_data);
        let it = unparsed_ansi_text.filter_segments_by_display_width(None);
        assert_eq2!(it.len(), 21);
    }

    #[test]
    fn test_lolcat_with_max_display_cols() {
        let test_data = "\u{1b}[38;2;51;254;77mS\u{1b}[39m\u{1b}[38;2;52;254;77mt\u{1b}[39m\u{1b}[38;2;52;254;77ma\u{1b}[39m\u{1b}[38;2;52;254;76mt\u{1b}[39m\u{1b}[38;2;53;254;76me\u{1b}[39m\u{1b}[38;2;53;254;76m \u{1b}[39m\u{1b}[38;2;53;254;75m{\u{1b}[39m\u{1b}[38;2;54;254;75m \u{1b}[39m\u{1b}[38;2;54;254;74ms\u{1b}[39m\u{1b}[38;2;54;254;74mt\u{1b}[39m\u{1b}[38;2;55;254;74ma\u{1b}[39m\u{1b}[38;2;55;254;73mc\u{1b}[39m\u{1b}[38;2;56;254;73mk\u{1b}[39m\u{1b}[38;2;56;254;72m:\u{1b}[39m\u{1b}[38;2;56;254;72m \u{1b}[39m\u{1b}[38;2;57;254;72m[\u{1b}[39m\u{1b}[38;2;57;254;71m0\u{1b}[39m\u{1b}[38;2;57;254;71m]\u{1b}[39m\u{1b}[38;2;58;254;71m \u{1b}[39m\u{1b}[38;2;58;254;70m}\u{1b}[39m";
        let unparsed_ansi_text = ANSIText::new(test_data);
        let it = unparsed_ansi_text.filter_segments_by_display_width(Some(4));
        assert_eq2!(it.len(), 4);
    }
}
