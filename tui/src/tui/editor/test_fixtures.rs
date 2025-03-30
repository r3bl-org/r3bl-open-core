/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

    use r3bl_core::{col,
                    height,
                    row,
                    telemetry::sizing::TelemetryReportLineStorage,
                    test_fixtures::{output_device_ext::OutputDeviceExt as _,
                                    StdoutMock},
                    width,
                    OutputDevice,
                    Size};
    use tokio::sync::mpsc;

    use crate::{EditorEngine,
                FlexBox,
                GlobalData,
                OffscreenBufferPool,
                PartialFlexBox,
                CHANNEL_WIDTH};

    pub fn make_global_data<S, AS>(
        window_size: Option<Size>,
    ) -> (GlobalData<S, AS>, StdoutMock)
    where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        let (sender, _) = mpsc::channel::<_>(CHANNEL_WIDTH);
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let offscreen_buffer_pool =
            OffscreenBufferPool::new(window_size.unwrap_or_default());

        let global_data = GlobalData {
            window_size: window_size.unwrap_or_default(),
            maybe_saved_offscreen_buffer: Default::default(),
            main_thread_channel_sender: sender,
            state: Default::default(),
            output_device,
            offscreen_buffer_pool,
            hud_report: TelemetryReportLineStorage::new(),
            spinner_helper: Default::default(),
        };

        (global_data, stdout_mock)
    }

    pub fn make_editor_engine_with_bounds(size: Size) -> EditorEngine {
        let flex_box = FlexBox {
            style_adjusted_bounds_size: size,
            style_adjusted_origin_pos: col(0) + row(0),
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
            style_adjusted_bounds_size: width(10) + height(10),
            style_adjusted_origin_pos: col(0) + row(0),
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
    use r3bl_core::{assert_eq2, GCStringExt as _, SegString};

    use crate::{editor_engine::engine_internal_api, EditorBuffer};

    pub fn none_is_at_caret(buffer: &EditorBuffer) {
        assert_eq2!(buffer.string_at_caret(), None);
    }

    pub fn str_is_at_caret(buffer: &EditorBuffer, expected: &str) {
        match buffer.string_at_caret() {
            Some(SegString { string, .. }) => {
                assert_eq2!(&string.string, expected)
            }
            None => panic!("Expected string at caret, but got None."),
        }
    }

    pub fn line_at_caret(editor_buffer: &EditorBuffer, expected: &str) {
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(editor_buffer).unwrap(),
            &expected.grapheme_string()
        );
    }
}
