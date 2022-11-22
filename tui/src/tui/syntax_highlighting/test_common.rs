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

#[cfg(test)]
pub mod shared {
    use std::borrow::Cow;

    use crate::FRONTMATTER_DELIMITER_PATTERN;

    pub fn get_md_file_invalid_frontmatter<'caller>() -> Cow<'caller, str> {
        let markdown_input = r#"
  ---
  # My Heading
  "#;
        Cow::Borrowed(markdown_input)
    }

    pub fn get_md_file_no_frontmatter<'caller>() -> Cow<'caller, str> {
        let markdown_input = include_str!("test_assets/valid-content.md");
        Cow::Borrowed(markdown_input)
    }

    pub fn get_md_file_with_json_frontmatter<'caller>() -> Cow<'caller, str> {
        let frontmatter_json = include_str!("test_assets/valid-frontmatter.json");

        let markdown = get_md_file_no_frontmatter();

        let final_str =
            format!("{FRONTMATTER_DELIMITER_PATTERN}\n{frontmatter_json}\n{FRONTMATTER_DELIMITER_PATTERN}\n{markdown}");
        Cow::Owned(final_str)
    }
}
