// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{OutputDevice, PaintMode, StdoutMock, StdMutex};
use std::sync::Arc;

pub trait OutputDeviceExt {
    fn new_mock() -> (OutputDevice, StdoutMock);
}

impl OutputDeviceExt for OutputDevice {
    fn new_mock() -> (OutputDevice, StdoutMock) {
        let stdout_mock = StdoutMock::default();
        let this = OutputDevice {
            resource: Arc::new(StdMutex::new(stdout_mock.clone())),
            paint_mode: PaintMode::Mock,
        };
        (this, stdout_mock)
    }
}

#[cfg(test)]
mod tests {
    use super::OutputDeviceExt;
    use crate::{OutputDevice, PaintMode};

    #[test]
    fn test_mock_output_device() {
        let (device, mock) = OutputDevice::new_mock();
        device.write(|writer| {
            drop(writer.write_all(b"Hello, world!\n"));
        });
        assert_eq!(
            mock.get_copy_of_buffer_as_string_strip_ansi(),
            "Hello, world!\n"
        );
    }

    #[test]
    fn test_mock_output_device_is_mock() {
        let (device, _) = OutputDevice::new_mock();
        assert_eq!(device.paint_mode, PaintMode::Mock);
    }
}
