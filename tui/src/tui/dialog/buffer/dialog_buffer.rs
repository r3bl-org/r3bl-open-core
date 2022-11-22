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

use std::fmt::{Debug, Formatter, Result};

use get_size::GetSize;
use serde::*;

use crate::*;

// ┏━━━━━━━━━━━━━━━━━━━━━┓
// ┃ DialogBuffer struct ┃
// ┛                     ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Stores the data for a modal dialog. It contains the text content in an [EditorBuffer] and a
/// title that is displayed.
#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct DialogBuffer {
    pub editor_buffer: EditorBuffer,
    pub title: String,
}

impl Default for DialogBuffer {
    fn default() -> Self {
        Self {
            editor_buffer: EditorBuffer::new_empty(),
            title: String::default(),
        }
    }
}

mod debug_format_helpers {

    use super::*;

    impl Debug for DialogBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write! { f,
              "\nDialogBuffer [      \n\
              ├ title: {}            \n\
              └ editor_buffer: {:?}  \n\
              ]",
              self.title,
              self.editor_buffer
            }
        }
    }
}
