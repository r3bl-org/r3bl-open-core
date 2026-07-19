// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(any(test, doc))]
pub mod mock_real_objects_for_editor {
    use crate::{DefaultSize, EditorEngine, FlexBox, GlobalData, HudData, OfsBufPool,
                OutputDevice, OutputDeviceExt, PartialFlexBox, RenderPipeline,
                VPBoundingBox, VPSize, core::test_fixtures::StdoutMock, vp_col,
                vp_height, vp_row, vp_width};
    use rustc_hash::FxHashMap;
    use std::fmt::Debug;
    use tokio::sync::mpsc;

    #[must_use]
    pub fn make_global_data<S, AS>(
        window_size: Option<VPSize>,
    ) -> (GlobalData<S, AS>, StdoutMock)
    where
        S: Debug + Default + Clone + Sync + Send,
        AS: Debug + Default + Clone + Sync + Send,
    {
        let (sender, _) =
            mpsc::channel::<_>(DefaultSize::MainThreadSignalChannelBufferSize.into());
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let ofs_buf_pool = OfsBufPool::new(window_size.unwrap_or_default());

        let global_data = GlobalData {
            pipeline: RenderPipeline::default(),
            window_size: window_size.unwrap_or_default(),
            maybe_saved_ofs_buf: Option::default(),
            main_thread_channel_sender: sender,
            state: Default::default(),
            output_device,
            ofs_buf_pool,
            hud_data: HudData::default(),
            memoized_text_widths: FxHashMap::default(),
        };

        (global_data, stdout_mock)
    }

    #[must_use]
    pub fn make_editor_engine_with_bounds(size: VPSize) -> EditorEngine {
        let flex_box = FlexBox {
            style_adjusted_bounds: VPBoundingBox {
                origin_pos: vp_col(0) + vp_row(0),
                bounds_size: size,
            },
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
            style_adjusted_bounds: VPBoundingBox {
                origin_pos: vp_col(0) + vp_row(0),
                bounds_size: vp_width(10) + vp_height(10),
            },
            ..Default::default()
        };
        let current_box: PartialFlexBox = (&flex_box).into();
        EditorEngine {
            current_box,
            ..Default::default()
        }
    }
}

#[cfg(any(test, doc))]
pub mod assert {
    use crate::{EditorBuffer, assert_eq2, editor_engine::engine_internal_api};

    pub fn none_is_at_caret(buffer: &EditorBuffer) {
        assert_eq2!(buffer.get_str_at_caret(), None);
    }

    /// # Panics
    ///
    /// This test fixture function will panic if the string at the caret
    /// does not match the expected string.
    pub fn str_is_at_caret(buffer: &EditorBuffer, expected: &str) {
        match buffer.get_str_at_caret() {
            Some(str_slice) => {
                assert_eq2!(str_slice, expected);
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
                .expect("conversion error")
                .content(),
            expected
        );
    }
}
