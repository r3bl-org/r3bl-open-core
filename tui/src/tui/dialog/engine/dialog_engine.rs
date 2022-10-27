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

use std::fmt::Debug;

use r3bl_rs_utils_core::*;
use serde::*;

use crate::*;

// ┏━━━━━━━━━━━━━━━━━━━━━┓
// ┃ DialogEngine struct ┃
// ┛                     ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Holds data related to rendering in between render calls. This is not stored in the
/// [DialogBuffer] struct, which lives in the [r3bl_redux::Store]. The store provides the underlying
/// document or buffer struct that holds the actual document.
///
/// In order to change the document, you can use the
/// [apply_event](DialogEngineApi::apply_event) method which takes [InputEvent] and tries to
/// execute it against this buffer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogEngine {
  pub editor_engine: EditorEngine,
  pub maybe_style_border: Option<Style>,
  pub maybe_style_title: Option<Style>,
  pub maybe_style_editor: Option<Style>,
}

pub mod constructor {
  use super::*;

  impl DialogEngine {
    pub fn new() -> Self {
      let editor_engine = {
        let config_options = EditorEngineConfigOptions {
          multiline: false,
          syntax_highlight: false,
        };
        EditorEngine::new(config_options)
      };
      Self {
        editor_engine,
        ..Default::default()
      }
    }
  }
}
pub use constructor::*;
