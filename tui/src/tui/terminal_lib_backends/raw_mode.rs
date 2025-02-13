/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use r3bl_core::{LockedOutputDevice, Size};

use super::{RenderOp, RenderOps, RenderOpsLocalData};

/// To use this directly, you need to make sure to create an instance using [start](RawMode::start)
/// which enables raw mode and then make sure to call [end](RawMode::end) when you are done.
#[derive(Debug, Clone)]
pub struct RawMode;

impl RawMode {
    pub fn start(
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        let mut skip_flush = false;
        RenderOps::route_paint_render_op_to_backend(
            &mut RenderOpsLocalData::default(),
            &mut skip_flush,
            &RenderOp::EnterRawMode,
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
        RenderOps::route_paint_render_op_to_backend(
            &mut RenderOpsLocalData::default(),
            &mut skip_flush,
            &RenderOp::ExitRawMode,
            window_size,
            locked_output_device,
            is_mock,
        );
    }
}
