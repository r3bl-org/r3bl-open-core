// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[cfg(test)]
pub mod mock_real_objects_for_dialog {
    use std::{collections::HashMap, fmt::Debug};

    use tokio::sync::mpsc;

    use crate::{DefaultSize, DialogBuffer, DialogEngine, FlexBoxId, GlobalData,
                HasDialogBuffers, OffscreenBufferPool, OutputDevice, OutputDeviceExt,
                Size, SpinnerHelper, core::test_fixtures::StdoutMock,
                editor::test_fixtures_editor::mock_real_objects_for_editor,
                telemetry::telemetry_sizing::TelemetryReportLineStorage};

    #[must_use]
    pub fn make_global_data(
        window_size: Option<Size>,
    ) -> (GlobalData<State, ()>, StdoutMock) {
        let (main_thread_channel_sender, _) =
            mpsc::channel::<_>(DefaultSize::MainThreadSignalChannelBufferSize.into());
        let state = create_state();
        let window_size = window_size.unwrap_or_default();
        let maybe_saved_ofs_buf = Option::default();
        let (output_device, stdout_mock) = OutputDevice::new_mock();
        let offscreen_buffer_pool = OffscreenBufferPool::new(window_size);
        let spinner_helper = SpinnerHelper::default();

        let global_data = GlobalData {
            state,
            window_size,
            maybe_saved_ofs_buf,
            main_thread_channel_sender,
            output_device,
            offscreen_buffer_pool,
            hud_report: TelemetryReportLineStorage::new(),
            spinner_helper,
        };

        (global_data, stdout_mock)
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

    #[must_use]
    pub fn create_state() -> State {
        let dialog_buffers = {
            let mut it = HashMap::new();
            it.insert(FlexBoxId::from(0), DialogBuffer::new_empty());
            it
        };
        State { dialog_buffers }
    }

    #[must_use]
    pub fn make_dialog_engine() -> DialogEngine {
        DialogEngine {
            editor_engine: mock_real_objects_for_editor::make_editor_engine(),
            ..Default::default()
        }
    }
}
