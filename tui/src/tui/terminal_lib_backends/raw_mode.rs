// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{RenderOpCommon, RenderOpIR, RenderOpsIR, RenderOpsLocalData};
use crate::{LockedOutputDevice, Size};

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
        let mut skip_flush = false;
        RenderOpsIR::route_paint_render_op_ir_to_backend(
            &mut RenderOpsLocalData::default(),
            &mut skip_flush,
            &RenderOpIR::Common(RenderOpCommon::EnterRawMode),
            window_size,
            locked_output_device,
            is_mock,
        );
    }

    pub fn end(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut skip_flush = false;
        RenderOpsIR::route_paint_render_op_ir_to_backend(
            &mut RenderOpsLocalData::default(),
            &mut skip_flush,
            &RenderOpIR::Common(RenderOpCommon::ExitRawMode),
            window_size,
            locked_output_device,
            is_mock,
        );
    }
}
