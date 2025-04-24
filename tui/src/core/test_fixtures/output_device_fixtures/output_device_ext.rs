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

use std::sync::Arc;

use crate::{OutputDevice, StdMutex, StdoutMock};

pub trait OutputDeviceExt {
    fn new_mock() -> (OutputDevice, StdoutMock);
}

impl OutputDeviceExt for OutputDevice {
    fn new_mock() -> (OutputDevice, StdoutMock) {
        let stdout_mock = StdoutMock::default();
        let this = OutputDevice {
            resource: Arc::new(StdMutex::new(stdout_mock.clone())),
            is_mock: true,
        };
        (this, stdout_mock)
    }
}

#[cfg(test)]
mod tests {
    use super::OutputDeviceExt;
    use crate::{lock_output_device_as_mut, LockedOutputDevice, OutputDevice};

    #[test]
    fn test_mock_output_device() {
        let (device, mock) = OutputDevice::new_mock();
        let mut_ref: LockedOutputDevice<'_> = lock_output_device_as_mut!(device);
        let _ = mut_ref.write_all(b"Hello, world!\n");
        assert_eq!(
            mock.get_copy_of_buffer_as_string_strip_ansi(),
            "Hello, world!\n"
        );
    }

    #[test]
    fn test_mock_output_device_is_mock() {
        let (device, _) = OutputDevice::new_mock();
        assert!(device.is_mock);
    }
}
