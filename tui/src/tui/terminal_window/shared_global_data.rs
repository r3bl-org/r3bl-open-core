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

use crate::*;

/// This is a global data structure that holds state for the entire application. This is wrapped in
/// an [Arc](std::sync::Arc) and [Mutex](std::sync::Mutex) so that it can be accessed from anywhere
/// as [SharedGlobalData].
///
/// These are global state values for the entire application:
/// - The `window_size` holds the [Size] of the terminal window.
/// - The `maybe_saved_offscreen_buffer` holds the last rendered [OffscreenBuffer].
#[derive(Clone, Default)]
pub struct GlobalData {
    pub window_size: Size,
    pub maybe_saved_offscreen_buffer: Option<OffscreenBuffer>,
}

mod global_data_impl {
    use std::fmt::Formatter;

    use super::*;

    impl Debug for GlobalData {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let mut vec_lines = vec![];
            vec_lines.push(format!("{0:?}", self.window_size));
            vec_lines.push(match &self.maybe_saved_offscreen_buffer {
                None => "no saved offscreen buffer".to_string(),
                Some(ref offscreen_buffer) => match DEBUG_TUI_COMPOSITOR {
                    false => "offscreen buffer saved from previous render".to_string(),
                    true => offscreen_buffer.pretty_print(),
                },
            });
            write!(f, "\nGlobalData\n  - {}", vec_lines.join("\n  - "))
        }
    }

    impl GlobalData {
        pub fn try_to_create_instance() -> CommonResult<GlobalData> {
            let mut global_data = GlobalData::default();
            global_data.set_size(terminal_lib_operations::lookup_size()?);
            Ok(global_data)
        }

        pub fn set_size(&mut self, new_size: Size) {
            self.window_size = new_size;
            self.dump_to_log("main_event_loop -> Resize");
        }

        pub fn get_size(&self) -> Size { self.window_size }

        pub fn dump_to_log(&self, msg: &str) {
            let log_msg = format!("{msg} -> {self:?}");
            call_if_true!(DEBUG_TUI_MOD, log_info(log_msg));
        }
    }
}
