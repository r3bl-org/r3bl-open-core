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
mod md_to_ast {
    use pulldown_cmark::*;

    use crate::{syntax_highlighting::test_common::shared::*, FrontmatterExtractionResponse};

    #[test]
    fn parse_md_content_with_json_frontmatter() {
        let md_content = get_md_file_with_json_frontmatter();

        // Strip all the frontmatter out of the markdown content.
        let result: FrontmatterExtractionResponse = md_content.as_ref().into();
        let FrontmatterExtractionResponse::ValidFrontmatter(_frontmatter, content) = result else {
      panic!();
    };

        // No frontmatter in content. Otherwise it will be parsed as well as the markdown content.
        let parser = pulldown_cmark::Parser::new(&content);

        let mut h1_count = 0;
        let mut unordered_list_count = 0;
        let mut ordered_list_count = 0;
        for event in parser {
            println!("{event:?}");
            match event {
                Event::Start(Tag::Heading(HeadingLevel::H1, _, _)) => h1_count += 1,
                Event::Start(Tag::List(Some(_))) => ordered_list_count += 1,
                Event::Start(Tag::List(None)) => unordered_list_count += 1,
                _ => {}
            }
        }
        assert_eq!(h1_count, 1);
        assert_eq!(unordered_list_count, 1);
        assert_eq!(ordered_list_count, 1);
    }
}
