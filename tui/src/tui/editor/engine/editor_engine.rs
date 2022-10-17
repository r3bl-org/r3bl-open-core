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
// ┃ EditorEngine struct ┃
// ┛                     ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Holds data related to rendering in between render calls. This is not stored in the
/// [EditorBuffer] struct, which lives in the [r3bl_redux::Store]. The store provides the underlying
/// document or buffer struct that holds the actual document.
///
/// In order to change the document, you can use the
/// [apply_event](EditorEngineRenderApi::apply_event) method which takes [InputEvent] and tries to
/// convert it to an [EditorEvent] and then execute them against this buffer.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorEngine {
  /// Set by [render](EditorEngineRenderApi::render_engine).
  pub current_box: FlexBox,
  pub config_options: EditorEngineConfigOptions,
}

impl EditorEngine {
  pub fn new(config_options: EditorEngineConfigOptions) -> Self {
    Self {
      current_box: FlexBox::default(),
      config_options,
    }
  }

  pub fn viewport_width(&self) -> ChUnit { self.current_box.style_adjusted_bounds_size.cols }

  pub fn viewport_height(&self) -> ChUnit { self.current_box.style_adjusted_bounds_size.rows }
}

// ┏━━━━━━━━━━━━━━━━┓
// ┃ Config options ┃
// ┛                ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorEngineConfigOptions {
  pub multiline: bool,
  pub syntax_highlight: bool,
}

impl Default for EditorEngineConfigOptions {
  fn default() -> Self {
    Self {
      multiline: true,
      syntax_highlight: true,
    }
  }
}
