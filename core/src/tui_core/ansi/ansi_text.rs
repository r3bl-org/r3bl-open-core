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

use std::{borrow::Cow, fmt::Write, ops::Deref};

use ansi_parser::{AnsiParser, Output};

// ┏━━━━━━━━━━━━━━━━━┓
// ┃ ANSITextSegment ┃
// ┛                 ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[derive(Debug, PartialEq, Default, Clone)]
pub struct ANSITextSegment<'a> {
    pub vec_parts: Vec<&'a Output<'a>>,
    /// [crate::UnicodeString] display width.
    pub display_width: usize,
}

impl ANSITextSegment<'_> {
    pub fn new() -> Self {
        Self {
            vec_parts: vec![],
            ..Default::default()
        }
    }
}

// ┏━━━━━━━━━━━━━━━━━━┓
// ┃ ANSITextSegments ┃
// ┛                  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[derive(Debug, PartialEq, Default, Clone)]
pub struct ANSITextSegments<'a> {
    pub vec_segments: Vec<ANSITextSegment<'a>>,
    /// [crate::UnicodeString] display width.
    pub display_width: usize,
}

impl<'a> ANSITextSegments<'a> {
    pub fn new(vec_segments: Vec<ANSITextSegment<'a>>, unicode_width: usize) -> Self {
        Self {
            vec_segments,
            display_width: unicode_width,
        }
    }

    pub fn len(&self) -> usize { self.vec_segments.len() }

    #[must_use]
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

impl<'a> Deref for ANSITextSegments<'a> {
    type Target = Vec<ANSITextSegment<'a>>;

    fn deref(&self) -> &Self::Target { &self.vec_segments }
}

mod ansi_text_segments {
    use super::*;

    impl<'a> From<&ANSITextSegments<'a>> for String {
        fn from(ansi_text_segments: &ANSITextSegments<'a>) -> Self {
            let mut buff = String::new();

            for segment in ansi_text_segments.iter() {
                for part in &segment.vec_parts {
                    write!(&mut buff, "{part}").expect("failed to write");
                }
            }

            buff
        }
    }

    impl<'a> From<ANSITextSegments<'a>> for String {
        fn from(ansi_text_segments: ANSITextSegments<'a>) -> Self { (&ansi_text_segments).into() }
    }
}

mod ansi_text_segment {
    use super::*;

    impl<'a> From<&ANSITextSegment<'a>> for String {
        fn from(ansi_text_segment: &ANSITextSegment<'a>) -> Self {
            let mut buff = String::new();

            for part in &ansi_text_segment.vec_parts {
                write!(&mut buff, "{part}").expect("failed to write");
            }

            buff
        }
    }
}

// ┏━━━━━━━━━━━━━━━┓
// ┃ ANSIStringExt ┃
// ┛               ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
pub trait ANSIStringExt {
    fn ansi_text(&self) -> ANSIText;
}

impl ANSIStringExt for Cow<'_, str> {
    fn ansi_text(&self) -> ANSIText { ANSIText::new(self) }
}

impl ANSIStringExt for &str {
    fn ansi_text(&self) -> ANSIText { ANSIText::new(self) }
}

impl ANSIStringExt for String {
    fn ansi_text(&self) -> ANSIText { ANSIText::new(self) }
}

// ┏━━━━━━━━━━┓
// ┃ ANSIText ┃
// ┛          ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[derive(Debug, PartialEq, Default)]
pub struct ANSIText<'a> {
    pub ansi_text: &'a str,
    pub parts: Vec<Output<'a>>,
}

impl<'a> ANSIText<'a> {
    /// Given an unparsed ANSI text &[str], parse it and return an [ANSIText].
    pub fn new(ansi_text: &'a str) -> Self {
        let parts: Vec<Output> = ansi_text.ansi_parse().collect();
        Self { ansi_text, parts }
    }

    /// 1. If `max_display_col_count` is [None], return all the segments that are delimited by an
    ///    [Output::TextBlock].
    /// 2. If `max_display_col_count` is provided, return the maximum number of segments that will
    ///    fit in the given display column width. In other words this works similar to the clip
    ///    function of [truncate_end_to_fit_width](crate::UnicodeString::truncate_end_to_fit_width).
    pub fn segments(&'a self, max_display_col_count: Option<usize>) -> ANSITextSegments<'a> {
        let mut vec_segment_copy = self.make_copy();

        // Calculate the unicode_width of each segment.
        let mut unicode_width_total = 0;
        for (_index, segment) in vec_segment_copy.iter_mut().enumerate() {
            for part in &segment.vec_parts {
                if let Output::TextBlock(text) = part {
                    let text_unicode_width = unicode_width::UnicodeWidthStr::width(*text);
                    segment.display_width += text_unicode_width;
                    unicode_width_total += segment.display_width;
                }
            }
        }

        // If `max_display_col_count` is provided then filter the vec_segments.
        if let Some(max_display_col_count) = max_display_col_count {
            let mut vec_segments_filtered = Vec::new();
            unicode_width_total = 0;

            for segment in vec_segment_copy {
                let lhs = unicode_width_total + segment.display_width;
                let rhs = max_display_col_count;
                if lhs > rhs {
                    break;
                }
                if segment.display_width != 0 {
                    unicode_width_total += segment.display_width;
                    vec_segments_filtered.push(segment);
                }
            }

            vec_segment_copy = vec_segments_filtered;
        }

        ANSITextSegments::new(vec_segment_copy, unicode_width_total)
    }

    fn make_copy(&'a self) -> Vec<ANSITextSegment<'a>> {
        let mut ret_val = vec![];

        let mut cur_item = ANSITextSegment::new();

        for part in &self.parts {
            match part {
                Output::TextBlock(_) => {
                    // Save the current segment.
                    cur_item.vec_parts.push(part);

                    // Save the current one & start a new segment.
                    ret_val.push(cur_item);
                    cur_item = ANSITextSegment::new();
                }
                Output::Escape(_) => {
                    // Save the current segment.
                    cur_item.vec_parts.push(part);
                }
            }
        }

        if !ret_val.contains(&cur_item) {
            ret_val.push(cur_item);
        }

        ret_val
    }
}

impl ANSIText<'_> {
    /// If conversion was successful and ANSI characters were stripped, returns a [String], otherwise
    /// returns [None].
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
}
