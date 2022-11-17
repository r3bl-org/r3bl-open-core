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

use extract_frontmatter::{config::{Modifier, Splitter},
                          Extractor};

/// This string pattern is used to split the [Markdown
/// frontmatter](https://markdoc.dev/docs/frontmatter#parse-the-frontmatter) from the Markdown
/// content. It is expected that this will show up twice to delimit the entire frontmatter content,
/// which can be JSON or YAML, etc.
pub const FRONTMATTER_DELIMITER_PATTERN: &str = "---";

/// Creates a new [Extractor] instance with the default configuration:
/// 1. Delimit frontmatter content using [FRONTMATTER_DELIMITER_PATTERN].
/// 2. Trim whitespace for each line of frontmatter content.
pub fn create_frontmatter_extractor<'caller>() -> Extractor<'caller> {
  let splitter = Splitter::EnclosingLines(FRONTMATTER_DELIMITER_PATTERN);
  let modifier = Modifier::TrimWhitespace;
  let mut extractor = Extractor::new(splitter);
  extractor.with_modifier(modifier);
  extractor
}

/// Extracts the frontmatter content from the given Markdown input. If the input does not contain
/// the correct frontmatter delimiters, then [FrontmatterExtractionResponse::NoFrontmatter] is
/// returned. Otherwise [FrontmatterExtractionResponse::ValidFrontmatter] is returned.
pub fn try_extract_front_matter(markdown_input: &str) -> FrontmatterExtractionResponse {
  let extractor = create_frontmatter_extractor();
  let (frontmatter, content) = extractor.extract(markdown_input);
  if content.is_empty() {
    FrontmatterExtractionResponse::NoFrontmatter
  } else {
    FrontmatterExtractionResponse::ValidFrontmatter(frontmatter, Cow::Borrowed(content))
  }
}

#[derive(Debug, Clone)]
pub enum FrontmatterExtractionResponse<'caller> {
  /// No frontmatter was found in the Markdown input. Or invalid frontmatter was found (not
  /// delimited correctly).
  NoFrontmatter,
  /// Valid frontmatter was found in the Markdown input.
  ValidFrontmatter(
    /* frontmatter: */ Cow<'caller, str>,
    /* content: */ Cow<'caller, str>,
  ),
}

/// Convenience trait implementation to convert &[str] to [FrontmatterExtractionResponse].
impl<'caller> From<&'caller str> for FrontmatterExtractionResponse<'caller> {
  fn from(markdown_input: &'caller str) -> Self { try_extract_front_matter(markdown_input) }
}
