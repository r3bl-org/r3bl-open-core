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

use std::{collections::HashMap,
          fmt::{Debug, Display, Formatter, Result}};

use r3bl_tui::*;

#[derive(Clone, PartialEq, Default)]
pub struct State {
  pub editor_buffers: HashMap<FlexBoxIdType, EditorBuffer>,
  pub dialog_buffer: DialogBuffer,
}

impl HasEditorBuffers for State {
  fn get_editor_buffer(&self, id: FlexBoxIdType) -> Option<&EditorBuffer> {
    if let Some(buffer) = self.editor_buffers.get(&id) {
      Some(buffer)
    } else {
      None
    }
  }
}

impl HasDialogBuffer for State {
  fn get_dialog_buffer(&self) -> &DialogBuffer { &self.dialog_buffer }
}

mod debug_format_helpers {
  use super::*;

  fn fmt(this: &State, f: &mut Formatter<'_>) -> Result {
    write! { f,
      "\nState [                               \n\
      - dialog_buffer: title: {:?}             \n\
      - dialog_buffer: editor_buffer: {:?}     \n\
      - buffers: {:?}                          \n\
      ]",
      this.dialog_buffer.title,
      // this.dialog_buffer.buffer,
      this.dialog_buffer.editor_buffer.get_as_string(),
      this.editor_buffers,
    }
  }

  impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt(self, f) }
  }

  impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { fmt(self, f) }
  }
}
