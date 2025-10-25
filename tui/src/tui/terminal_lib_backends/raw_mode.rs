// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::RenderOpCommon;
use crate::{LockedOutputDevice, RenderOpOutput, RenderOpOutputVec, RenderOpsExec, Size};

/// To use this directly, you need to make sure to create an instance using
/// [start](RawMode::start) which enables raw mode and then make sure to call
/// [end](RawMode::end) when you are done.
#[derive(Debug, Clone)]
pub struct RawMode;

impl RawMode {
    pub fn start(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        // Create Output operations for entering raw mode.
        // Raw mode is a terminal state change, so it goes through the Output pipeline.
        let mut ops = RenderOpOutputVec::new();
        ops += RenderOpOutput::Common(RenderOpCommon::EnterRawMode);

        // Execute the operations using the ExecutableRenderOps trait.
        let mut skip_flush = false;
        ops.execute_all(&mut skip_flush, window_size, locked_output_device, is_mock);
    }

    pub fn end(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        // Create Output operations for exiting raw mode.
        let mut ops = RenderOpOutputVec::new();
        ops += RenderOpOutput::Common(RenderOpCommon::ExitRawMode);

        // Execute the operations using the ExecutableRenderOps trait.
        let mut skip_flush = false;
        ops.execute_all(&mut skip_flush, window_size, locked_output_device, is_mock);
    }
}
