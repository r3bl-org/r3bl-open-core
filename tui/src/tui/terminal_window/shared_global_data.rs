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

use std::fmt::{Debug, Formatter};

use r3bl_rs_utils_core::*;
use tokio::sync::mpsc::Sender;

use crate::*;

/// This is a global data structure that holds state for the entire application [App] and
/// the terminal window [TerminalWindow] itself. These are global state values for the
/// entire application:
/// - The `window_size` holds the [Size] of the terminal window.
/// - The `maybe_saved_offscreen_buffer` holds the last rendered [OffscreenBuffer].
/// - The `main_thread_channel_sender` is used to send [TerminalWindowMainThreadSignal]s
/// - The `state` holds the application's state.
pub struct GlobalData<S, A>
where
    S: Debug + Default + Clone + Sync + Send,
    A: Debug + Default + Clone + Sync + Send,
{
    pub window_size: Size,
    pub maybe_saved_offscreen_buffer: Option<OffscreenBuffer>,
    pub main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<A>>,
    pub state: S,
}

mod global_data_impl {
    use super::*;

    impl<S, A> Debug for GlobalData<S, A>
    where
        S: Debug + Default + Clone + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let vec_lines = {
                let mut it = vec![];
                it.push(format!("window_size: {0:?}", self.window_size));
                it.push(match &self.maybe_saved_offscreen_buffer {
                    None => "no saved offscreen buffer".to_string(),
                    Some(ref offscreen_buffer) => match DEBUG_TUI_COMPOSITOR {
                        false => {
                            "offscreen buffer saved from previous render".to_string()
                        }
                        true => offscreen_buffer.pretty_print(),
                    },
                });
                it
            };
            write!(f, "\nGlobalData\n  - {}", vec_lines.join("\n  - "))
        }
    }

    impl<S, A> GlobalData<S, A>
    where
        S: Debug + Default + Clone + Sync + Send,
        A: Debug + Default + Clone + Sync + Send,
    {
        pub fn try_to_create_instance(
            main_thread_channel_sender: Sender<TerminalWindowMainThreadSignal<A>>,
        ) -> CommonResult<GlobalData<S, A>>
        where
            A: Debug + Default + Clone + Sync + Send,
        {
            let mut it = GlobalData {
                window_size: Default::default(),
                maybe_saved_offscreen_buffer: Default::default(),
                state: Default::default(),
                main_thread_channel_sender,
            };

            it.set_size(terminal_lib_operations::lookup_size()?);

            Ok(it)
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
