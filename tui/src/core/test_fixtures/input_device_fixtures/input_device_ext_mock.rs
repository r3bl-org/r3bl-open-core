// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{CrosstermEventResult, InlineVec, InputDevice};
use std::time::Duration;

/// Extension trait for creating mock `InputDevice` instances for testing.
///
/// This trait provides a backward-compatible API for existing tests.
/// Internally, it delegates to the `InputDevice` enum's mock constructors.
pub trait InputDeviceExtMock {
    fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> InputDevice;

    fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice;
}

impl InputDeviceExtMock for InputDevice {
    fn new_mock(generator_vec: InlineVec<CrosstermEventResult>) -> InputDevice {
        InputDevice::new_mock(generator_vec)
    }

    fn new_mock_with_delay(
        generator_vec: InlineVec<CrosstermEventResult>,
        delay: Duration,
    ) -> InputDevice {
        InputDevice::new_mock_with_delay(generator_vec, delay)
    }
}
