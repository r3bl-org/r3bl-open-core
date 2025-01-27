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

use std::time::Duration;

use r3bl_core::{CrosstermEventResult, InputDevice, VecArray};

use super::{gen_input_stream, gen_input_stream_with_delay};

pub trait InputDeviceExt {
    fn new_mock(generator_vec: VecArray<CrosstermEventResult>) -> InputDevice;

    fn new_mock_with_delay(
        generator_vec: VecArray<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice;
}

impl InputDeviceExt for InputDevice {
    fn new_mock(generator_vec: VecArray<CrosstermEventResult>) -> InputDevice {
        InputDevice {
            resource: gen_input_stream(generator_vec),
        }
    }

    fn new_mock_with_delay(
        generator_vec: VecArray<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice {
        InputDevice {
            resource: gen_input_stream_with_delay(generator_vec, delay),
        }
    }
}
