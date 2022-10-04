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

use std::fmt::{Display, Formatter};

use r3bl_tui::*;

#[derive(Clone, Debug)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum Action {
  UpdateEditorBuffer(EditorBuffer),
  Noop,
}

impl Default for Action {
  fn default() -> Self { Action::Noop }
}

impl Display for Action {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{:?}", self) }
}
