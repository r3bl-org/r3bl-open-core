// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{gen_input_stream, gen_input_stream_with_delay};
use crate::{CrosstermEventResult, InlineVec, InputDevice};
use std::time::Duration;

pub trait InputDeviceExtMock {
    fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> InputDevice;

    fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice;
}

impl InputDeviceExtMock for InputDevice {
    fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> InputDevice {
        InputDevice {
            resource: gen_input_stream(generator_vec),
        }
    }

    fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice {
        InputDevice {
            resource: gen_input_stream_with_delay(generator_vec, delay),
        }
    }
}
