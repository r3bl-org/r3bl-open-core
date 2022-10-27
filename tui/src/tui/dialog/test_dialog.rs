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
mod test_dialog_api {
  use r3bl_rs_utils_core::*;
  use r3bl_rs_utils_macro::style;

  use super::*;
  use crate::*;

  #[test]
  fn flex_box_from() {
    // TODO: impl this
  }

  #[test]
  fn apply_event() {
    let mut buffer = DialogBuffer::default();
    let mut engine = mock_real_objects::make_dialog_engine();

    // TODO: impl this
  }

  #[test]
  fn render_engine() {
    let mut buffer = DialogBuffer::default();
    let mut engine = mock_real_objects::make_dialog_engine();

    // TODO: impl this
  }
}

pub mod mock_real_objects {
  use crate::{test_editor::mock_real_objects, *};

  pub fn make_dialog_engine() -> DialogEngine {
    DialogEngine {
      editor_engine: mock_real_objects::make_editor_engine(),
      ..Default::default()
    }
  }
}
