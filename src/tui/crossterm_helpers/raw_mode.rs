/*
 *   Copyright (c) 2022 R3BL LLC
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

use crate::*;

/// This will automatically disable [raw
/// mode](https://docs.rs/crossterm/0.23.2/crossterm/terminal/index.html#raw-mode) when the enclosed
/// block ends. Note that this macro must be called from a function that returns a `Result`.
///
/// Example 1:
/// ```ignore
/// pub async fn emit_crossterm_commands() -> CommonResult<()> {
///   raw_mode! { repl().await? }
/// }
/// ```
///
/// Example 2:
/// ```ignore
/// pub async fn emit_crossterm_commands() -> CommonResult<()> {
///   raw_mode!({
///     repl().await?;
///     Ok(())
///   })
/// }
/// ```
///
/// Example 3:
/// ```ignore
/// pub async fn emit_crossterm_commands() -> CommonResult<()> {
///   raw_mode!({
///     println!("crossterm: Entering raw mode...");
///     repl().await?;
///     println!("crossterm: Exiting raw mode...");
///     return Ok(());
///   });
/// }
/// ```
#[macro_export]
macro_rules! raw_mode {
  ($code_block: stmt) => {{
    use $crate::*;
    let raw_mode = RawMode::start().await;
    $code_block
    raw_mode.end().await;
    Ok(())
  }};
  ($code_block: block) => {{
    use $crate::*;
    let raw_mode = RawMode::start();
    $code_block
    raw_mode.end().await;
    Ok(())
  }};
}

/// Instead of using this directly, please consider using [raw_mode!] instead.
///
/// To use this directly, you need to make sure to create an instance using `start()` (which enables
/// raw mode) and then when this instance is dropped (when the enclosing code block falls out of
/// scope) raw mode will be disabled.
pub struct RawMode;

impl RawMode {
  pub async fn start() -> Self {
    tw_command_queue!(TWCommand::EnterRawMode)
      .flush(FlushKind::JustFlushQueue)
      .await;
    RawMode
  }

  pub async fn end(&self) {
    tw_command_queue!(TWCommand::ExitRawMode)
      .flush(FlushKind::JustFlushQueue)
      .await;
  }
}
