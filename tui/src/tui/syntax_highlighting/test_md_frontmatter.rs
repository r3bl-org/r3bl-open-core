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
mod extract_frontmatter {
    use serde_json::Value;

    use crate::{syntax_highlighting::test_common::shared::*, FrontmatterExtractionResponse};

    #[test]
    fn no_frontmatter() {
        let md_content = get_md_file_no_frontmatter();
        let result: FrontmatterExtractionResponse = md_content.as_ref().into();

        let FrontmatterExtractionResponse::NoFrontmatter = result else {
      panic!();
    };
    }

    #[test]
    fn invalid_frontmatter() {
        let md_content = get_md_file_invalid_frontmatter();
        let result: FrontmatterExtractionResponse = md_content.as_ref().into();

        let FrontmatterExtractionResponse::NoFrontmatter = result else {
      panic!();
    };
    }

    #[test]
    fn valid_json_frontmatter() {
        let md_content = get_md_file_with_json_frontmatter();
        let result: FrontmatterExtractionResponse = md_content.as_ref().into();

        let FrontmatterExtractionResponse::ValidFrontmatter(frontmatter, content) = result else {
      panic!();
    };

        let json: Value = serde_json::from_str(&frontmatter).unwrap();

        let object = json.as_object().unwrap();

        assert_eq!(object.get("date").unwrap(), "2021-06-30");
        assert_eq!(object.get("description").unwrap(), "My Description");
        assert_eq!(object.get("title").unwrap(), "My Title");
        matches!(
          object.get("tags").unwrap(),
          Value::Array(array) if array.len() == 2 && array[0] == "tag1" && array[1] == "tag2"
        );
        assert!(!content.is_empty());
    }
}
