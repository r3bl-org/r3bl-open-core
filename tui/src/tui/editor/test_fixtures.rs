/*
 *   Copyright (c) 2024 R3BL LLC
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

#[cfg(test)]
pub mod mock_real_objects_for_editor {
    use std::fmt::Debug;

    use r3bl_core::{position, size, OutputDevice, Size};
    use r3bl_test_fixtures::{output_device_ext::OutputDeviceExt as _, StdoutMock};
    use tokio::sync::mpsc;

    use crate::{EditorEngine, FlexBox, GlobalData, PartialFlexBox, CHANNEL_WIDTH};

    pub fn make_global_data<S, AS>(
        window_size: Option<Size>,
    ) -> (GlobalData<S, AS>, StdoutMock)
    where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        let (sender, _) = mpsc::channel::<_>(CHANNEL_WIDTH);
        let (output_device, stdout_mock) = OutputDevice::new_mock();

        let global_data = GlobalData {
            window_size: window_size.unwrap_or_default(),
            maybe_saved_offscreen_buffer: Default::default(),
            main_thread_channel_sender: sender,
            state: Default::default(),
            output_device,
        };

        (global_data, stdout_mock)
    }

    pub fn make_editor_engine_with_bounds(size: Size) -> EditorEngine {
        let flex_box = FlexBox {
            style_adjusted_bounds_size: size,
            style_adjusted_origin_pos: position!( col_index: 0, row_index: 0 ),
            ..Default::default()
        };
        let current_box: PartialFlexBox = (&flex_box).into();
        EditorEngine {
            current_box,
            ..Default::default()
        }
    }

    pub fn make_editor_engine() -> EditorEngine {
        let flex_box = FlexBox {
            style_adjusted_bounds_size: size!( col_count: 10, row_count: 10 ),
            style_adjusted_origin_pos: position!( col_index: 0, row_index: 0 ),
            ..Default::default()
        };
        let current_box: PartialFlexBox = (&flex_box).into();
        EditorEngine {
            current_box,
            ..Default::default()
        }
    }
}

#[cfg(test)]
pub mod assert {
    use r3bl_core::{assert_eq2, UnicodeStringExt as _, UnicodeStringSegmentSliceResult};

    use crate::{EditorBuffer, EditorEngine, EditorEngineInternalApi};

    pub fn none_is_at_caret(buffer: &EditorBuffer, engine: &EditorEngine) {
        assert_eq2!(
            EditorEngineInternalApi::string_at_caret(buffer, engine),
            None
        );
    }

    pub fn str_is_at_caret(
        editor_buffer: &EditorBuffer,
        engine: &EditorEngine,
        expected: &str,
    ) {
        match EditorEngineInternalApi::string_at_caret(editor_buffer, engine) {
            Some(UnicodeStringSegmentSliceResult {
                unicode_string: string,
                ..
            }) => {
                assert_eq2!(&string.string, expected)
            }
            None => panic!("Expected string at caret, but got None."),
        }
    }

    pub fn line_at_caret(
        editor_buffer: &EditorBuffer,
        engine: &EditorEngine,
        expected: &str,
    ) {
        assert_eq2!(
            EditorEngineInternalApi::line_at_caret_to_string(editor_buffer, engine)
                .unwrap(),
            &expected.unicode_string()
        );
    }
}
