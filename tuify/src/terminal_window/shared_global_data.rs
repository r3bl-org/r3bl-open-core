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

/// These are state values for the overall application:
/// - The `window_size` holds the [Size] of the terminal window.
///
/// - The `maybe_saved_offscreen_buffer` holds the last rendered [OffscreenBuffer].
#[derive(Clone, Default)]
pub struct GlobalData {
    pub window_size: Size,
    pub maybe_saved_offscreen_buffer: Option<OffscreenBuffer>,
    pub inline_row_offset : ChUnit
}
mod global_data_impl {
    use std::fmt::Formatter;

    use super::*;

    impl Debug for GlobalData {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let mut vec_lines = vec![];
            vec_lines.push(format!("{0:?}", self.window_size));
            vec_lines.push(format!("Inline Row Offset : {:?}", self.inline_row_offset));
            vec_lines.push(match &self.maybe_saved_offscreen_buffer {
                None => "no saved offscreen buffer".to_string(),
                Some(ref offscreen_buffer) => match DEBUG_TUIFY_COMPOSITOR {
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

        pub fn try_to_create_inline_instance<T: ViewportHeight>(state : &T) -> CommonResult<GlobalData> {
            let mut global_data = GlobalData::default();
            let window_size = terminal_lib_operations::lookup_size()?;
            let initial_row_offset = terminal_lib_operations::get_inline_row_index();
            global_data.set_size(window_size);
            // Setting up the inline row offset
            let max_row_height = window_size.row_count;
            let offscreen_buffer_height = state.get_viewport_height();
            if (initial_row_offset + offscreen_buffer_height) >= max_row_height {
                global_data.set_inline_row_offset(max_row_height - offscreen_buffer_height -1);
            } else {
                global_data.set_inline_row_offset(initial_row_offset);
            }
            Ok(global_data)
        }

        pub fn set_size(&mut self, new_size: Size) {
            self.window_size = new_size;
            self.dump_to_log("main_event_loop -> Resize");
        }

        pub fn get_size(&self) -> Size { self.window_size }

        pub fn set_inline_row_offset(&mut self, new_offset: ChUnit) {
            self.inline_row_offset = new_offset;
            self.dump_to_log("main_event_loop -> Inline Row Offset");
        }
        pub fn get_inline_row_offset(&self) -> ChUnit {
            self.inline_row_offset
        }

        pub fn dump_to_log(&self, msg: &str) {
            let log_msg = format!("{msg} -> {self:?}");
            call_if_true!(DEBUG_TUIFY_MOD, log_info(log_msg));
        }
    }
}

pub type SharedGlobalData = GlobalData;
