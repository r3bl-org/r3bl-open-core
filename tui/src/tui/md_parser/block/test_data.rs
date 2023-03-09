/*
 *   Copyright (c) 2023 R3BL LLC
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

#[rustfmt::skip]
#[cfg(test)]
pub mod raw_strings {

pub const UNORDERED_LIST_ELEMENT: &str =
r#"- this is an element
- this is another element
"#;

pub const ORDERED_LIST_ELEMENT: &str =
r#"1. this is an element
1. here is another
"#;

pub const CODE_BLOCK_0_INPUT: &str =
r#"```bash
```
"#;

pub const CODE_BLOCK_1_EMPTY_INPUT: &str =
r#"```bash

```
"#;

pub const CODE_BLOCK_1_INPUT: &str =
r#"```
pip install foobar
```
"#;

pub const CODE_BLOCK_2_INPUT: &str =
r#"```python
import foobar

foobar.pluralize('word') # returns 'words'
foobar.pluralize('goose') # returns 'geese'
foobar.singularize('phenomena') # returns 'phenomenon'
```
"#;

pub const CODE_BLOCK_3_INPUT: &str =
r#"```bash
pip install foobar
```
"#;

}
