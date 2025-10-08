// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(test)]
pub mod mock_real_objects_for_editor {
    use crate::{DefaultSize, EditorEngine, FlexBox, GlobalData, OffscreenBufferPool,
                OutputDevice, OutputDeviceExt, PartialFlexBox, Size, SpinnerHelper, col,
                core::test_fixtures::StdoutMock, height, row,
                telemetry::telemetry_sizing::TelemetryReportLineStorage, width};
    use std::fmt::Debug;
    use tokio::sync::mpsc;

    #[must_use]
    pub fn make_global_data<S, AS>(
        window_size: Option<Size>,
    ) -> (GlobalData<S, AS>, StdoutMock)
    where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        let (sender, _) =
            mpsc::channel::<_>(DefaultSize::MainThreadSignalChannelBufferSize.into());
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let offscreen_buffer_pool =
            OffscreenBufferPool::new(window_size.unwrap_or_default());

        let global_data = GlobalData {
            window_size: window_size.unwrap_or_default(),
            maybe_saved_ofs_buf: Option::default(),
            main_thread_channel_sender: sender,
            state: Default::default(),
            output_device,
            offscreen_buffer_pool,
            hud_report: TelemetryReportLineStorage::new(),
            spinner_helper: SpinnerHelper::default(),
        };

        (global_data, stdout_mock)
    }

    #[must_use]
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

    #[must_use]
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
    use crate::{EditorBuffer, SegStringOwned, assert_eq2,
                editor_engine::engine_internal_api};

    pub fn none_is_at_caret(buffer: &EditorBuffer) {
        assert_eq2!(buffer.string_at_caret(), None);
    }

    /// # Panics
    ///
    /// This test fixture function will panic if the string at the caret
    /// does not match the expected string.
    pub fn str_is_at_caret(buffer: &EditorBuffer, expected: &str) {
        match buffer.string_at_caret() {
            Some(SegStringOwned { string, .. }) => {
                assert_eq2!(&string.string, expected);
            }
            None => panic!("Expected string at caret, but got None."),
        }
    }

    /// # Panics
    ///
    /// This test fixture function will panic if the line at the caret
    /// does not match the expected string.
    pub fn line_at_caret(editor_buffer: &EditorBuffer, expected: &str) {
        assert_eq2!(
            engine_internal_api::line_at_caret_to_string(editor_buffer)
                .unwrap()
                .content(),
            expected
        );
    }
}
