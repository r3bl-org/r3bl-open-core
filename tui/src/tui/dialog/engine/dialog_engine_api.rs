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

use std::fmt::{Debug, Display};

use r3bl_rs_utils_core::*;

use crate::*;

// ┏━━━━━━━━━━━━━━━━━━┓
// ┃ DialogEngine API ┃
// ┛                  ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
/// Things you can do w/ a dialog engine.
pub struct DialogEngineApi;

impl DialogEngineApi {
  /// Event based interface for the editor. This executes the [InputEvent]. Returns a new
  /// [DialogBuffer] if the operation was applied otherwise returns [None].
  pub async fn apply_event<S, A>(
    args: EditorEngineArgs<'_, S, A>,
    input_event: &InputEvent,
  ) -> CommonResult<ApplyResponse<DialogBuffer>>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    // TODO: impl apply_event
    todo!()
  }

  pub async fn render_engine<S, A>(
    args: EditorEngineArgs<'_, S, A>,
    current_box: &FlexBox,
  ) -> CommonResult<RenderPipeline>
  where
    S: Default + Display + Clone + PartialEq + Debug + Sync + Send,
    A: Default + Display + Clone + Sync + Send,
  {
    // TODO: impl render
    todo!()
  }
}
