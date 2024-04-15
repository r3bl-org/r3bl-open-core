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

use miette::IntoDiagnostic;
use r3bl_terminal_async::SharedWriter;
use tracing::Level;

pub fn init(stdout_override: SharedWriter) -> miette::Result<()> {
    let builder = tracing_subscriber::fmt()
        .compact() /* one line output */
        .with_max_level(Level::DEBUG)
        .without_time()
        .with_thread_ids(true)
        .with_thread_names(false)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_ansi(true);

    let writer_stdout = move || -> Box<dyn std::io::Write> { Box::new(stdout_override.clone()) };
    let subscriber = builder.with_writer(writer_stdout).finish();

    tracing::subscriber::set_global_default(subscriber).into_diagnostic()?;

    Ok(())
}
