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
use r3bl_tui::*;

use super::*;

pub fn debug_log_action(src: String, action: Action) {
  call_if_true!(
    DEBUG_TUI_MOD,
    log_no_err!(INFO, "🚀 {} -> dispatch action: {}", src, action,)
  );
}

pub fn debug_log_has_focus(src: String, has_focus: &HasFocus) {
  call_if_true!(
    DEBUG_TUI_MOD,
    log_no_err!(
      INFO,
      "👀 {} -> focus change & rerender: {:?}",
      src,
      has_focus
    )
  );
}
