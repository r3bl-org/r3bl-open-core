/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use r3bl_cmdr::rc::run_app;
use r3bl_tui::{CommonResult, run_with_safe_stack, set_mimalloc_in_main, throws};

fn main() -> CommonResult<()> {
    run_with_safe_stack!(main_impl())
}

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main_impl() -> CommonResult<()> {
    set_mimalloc_in_main!();

    throws!({
        run_app()?;
    })
}
