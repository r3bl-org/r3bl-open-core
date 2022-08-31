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

use r3bl_rs_utils_core::*;
use serde::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EditorBuffer {
  /// A list of lines representing the document being edited.
  pub buffer: Vec<String>,
  /// The current caret position.
  pub cursor: Position,
  /// The col and row offset for scrolling if active.
  pub scroll_offset: Position,
  /// Lolcat struct for generating rainbow colors.
  pub lolcat: Lolcat,
}
