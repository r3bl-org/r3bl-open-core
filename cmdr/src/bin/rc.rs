// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_cmdr::rc::run_app;
use r3bl_tui::{CommonResult, run_with_safe_stack, set_mimalloc_in_main, throws};

fn main() -> CommonResult<()> { run_with_safe_stack!(main_impl()) }

// Note: The `tokio::main` macro internally calls `.expect("Failed building the Runtime")`
// when initializing the Tokio runtime. This is unavoidable and safe, as runtime creation
// failure is a fatal error that should panic. The lint must be suppressed here.
#[tokio::main]
#[allow(clippy::unwrap_in_result)]
async fn main_impl() -> CommonResult<()> {
    set_mimalloc_in_main!();

    throws!({
        run_app()?;
    })
}
