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

use std::{collections::HashMap, fmt::Debug};

use r3bl_rs_utils_core::*;

use crate::*;

/// These are global state values for the entire application:
/// - The `window_size` holds the [Size] of the terminal window.
/// - The `maybe_saved_offscreen_buffer` holds the last rendered [OffscreenBuffer].
/// - The `cache_ansi_text` holds a cache of [ANSIText] objects. You can use
///   [get_from_ansi_text_cache](GlobalData::get_from_cache_ansi_text) method to access this.
/// - The `cache_try_strip_ansi_text` holds a cache of [ANSIText::try_strip_ansi] results. You can
///   use [get_from_cache_try_strip_ansi_text](GlobalData::get_from_cache_try_strip_ansi_text)
///   method to access this.
#[derive(Clone, Default)]
pub struct GlobalData {
    pub window_size: Size,

    pub maybe_saved_offscreen_buffer: Option<OffscreenBuffer>,

    pub cache_ansi_text: HashMap<String, ANSIText>,

    pub cache_try_strip_ansi_text: HashMap<String, Option<String>>,

    // FUTURE: 🐵 use global_user_data (contains key: String, value: HashMap<String, String>).
    pub global_user_data: HashMap<String, HashMap<String, String>>,
}

mod manage_cache {
    use int_enum::IntEnum;

    use super::*;

    impl GlobalData {
        pub fn get_from_cache_try_strip_ansi_text(&mut self, key: &str) -> Option<String> {
            match self.cache_try_strip_ansi_text.get(key) {
                Some(existing_value) => existing_value.clone(),
                None => {
                    let new_value = ANSIText::try_strip_ansi(key);
                    self.cache_try_strip_ansi_text
                        .insert(key.into(), new_value.clone());

                    // Clean up the cache if it gets too big.
                    if self.cache_try_strip_ansi_text.len()
                        > DefaultSize::GlobalDataCacheSize.int_value()
                    {
                        self.cache_try_strip_ansi_text.clear();
                    }

                    new_value
                }
            }
        }

        pub fn get_from_cache_ansi_text(&mut self, key: &str) -> ANSIText {
            match self.cache_ansi_text.get(key) {
                Some(existing_value) => existing_value.clone(),
                None => {
                    let new_value = ANSIText::new(key);
                    self.cache_ansi_text.insert(key.into(), new_value.clone());

                    // Clean up the cache if it gets too big.
                    if self.cache_ansi_text.len() > DefaultSize::GlobalDataCacheSize.int_value() {
                        self.cache_ansi_text.clear();
                    }

                    new_value
                }
            }
        }
    }
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
            vec_lines.push(format!("{0:?}", self.global_user_data));
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
