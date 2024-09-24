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

pub mod mock_real_objects_for_dialog {
    use std::{collections::HashMap, fmt::Debug};

    use r3bl_rs_utils_core::Size;
    use tokio::sync::mpsc;

    use crate::{test_fixtures::mock_real_objects_for_editor,
                DialogBuffer,
                DialogEngine,
                FlexBoxId,
                GlobalData,
                HasDialogBuffers,
                CHANNEL_WIDTH};

    pub fn make_global_data(window_size: Option<Size>) -> GlobalData<State, ()> {
        let (main_thread_channel_sender, _) = mpsc::channel::<_>(CHANNEL_WIDTH);
        let state = create_state();
        let window_size = window_size.unwrap_or_default();
        let maybe_saved_offscreen_buffer = Default::default();
        GlobalData {
            state,
            window_size,
            maybe_saved_offscreen_buffer,
            main_thread_channel_sender,
        }
    }

    #[derive(Clone, PartialEq, Default, Debug)]
    pub struct State {
        pub dialog_buffers: HashMap<FlexBoxId, DialogBuffer>,
    }

    impl HasDialogBuffers for State {
        fn get_mut_dialog_buffer(&mut self, id: FlexBoxId) -> Option<&mut DialogBuffer> {
            self.dialog_buffers.get_mut(&id)
        }
    }

    pub fn create_state() -> State {
        let dialog_buffers = {
            let mut it = HashMap::new();
            it.insert(FlexBoxId::from(0), DialogBuffer::new_empty());
            it
        };
        State { dialog_buffers }
    }

    pub fn make_dialog_engine() -> DialogEngine {
        DialogEngine {
            editor_engine: mock_real_objects_for_editor::make_editor_engine(),
            ..Default::default()
        }
    }
}
