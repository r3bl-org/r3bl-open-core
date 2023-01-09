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

/// Please do not construct this struct directly and use [new_empty](DialogBuffer::new_empty)
/// instead.
///
/// Stores the data for a modal dialog. It contains the text content in an [EditorBuffer] and a
/// title that is displayed.
#[derive(Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct DialogBuffer {
    pub editor_buffer: EditorBuffer,
    pub title: String,
    pub maybe_results: Option<Vec<String>>,
}

mod dialog_buffer_impl {
    use r3bl_rs_utils_core::{ch, ChUnit};

    use super::*;

    impl DialogBuffer {
        pub fn get_results_count(&self) -> ChUnit {
            if let Some(ref it) = self.maybe_results {
                ch!(it.len())
            } else {
                ch!(0)
            }
        }
    }

    impl DialogBuffer {
        pub fn new_empty() -> Self {
            DialogBuffer {
                editor_buffer: EditorBuffer::new_empty(DEFAULT_SYN_HI_FILE_EXT.to_string()),
                title: Default::default(),
                maybe_results: None,
            }
        }
    }

    impl Debug for DialogBuffer {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            let maybe_results: &dyn Debug = format_option!(&self.maybe_results);
            write! { f,
              "\nDialogBuffer [      \n\
              ├ title: {}            \n\
              ├ maybe_results: {:?}  \n\
              └ editor_buffer: {}  \n\
              ]",
              self.title,
              maybe_results,
              self.editor_buffer.get_as_string()
            }
        }
    }
}
