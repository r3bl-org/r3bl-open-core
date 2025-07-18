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
use clap::Parser;
use r3bl_cmdr::{AnalyticsAction,
                edi::{clap_config::CLIArg, launcher, ui_templates},
                report_analytics, upgrade_check};
use r3bl_tui::{CommonResult, log::try_initialize_logging_global, set_mimalloc_in_main,
               throws};

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() -> CommonResult<()> {
    set_mimalloc_in_main!();

    throws!({
        // Parse CLI args.
        let cli_arg: CLIArg = CLIArg::parse();

        // Start logging.
        let enable_logging = cli_arg.global_options.enable_logging;

        enable_logging.then(|| {
            try_initialize_logging_global(tracing_core::LevelFilter::DEBUG).ok();
            // % is Display, ? is Debug.
            tracing::debug!(
                message = "Start logging...",
                cli_arg = ?cli_arg
            );
        });

        // Check analytics reporting.
        if cli_arg.global_options.no_analytics {
            report_analytics::disable();
        }

        upgrade_check::start_task_to_check_if_upgrade_is_needed();
        report_analytics::start_task_to_generate_event(
            String::new(),
            AnalyticsAction::EdiAppStart,
        );

        // Open the editor.
        match cli_arg.file_paths.len() {
            0 => {
                report_analytics::start_task_to_generate_event(
                    String::new(),
                    AnalyticsAction::EdiFileNew,
                );
                launcher::run_app(None).await?;
            }
            1 => {
                report_analytics::start_task_to_generate_event(
                    String::new(),
                    AnalyticsAction::EdiFileOpenSingle,
                );
                let maybe_file_path = Some(cli_arg.file_paths[0].as_str());
                launcher::run_app(maybe_file_path).await?;
            }
            _ => {
                if let Some(ref file_path) =
                    ui_templates::handle_multiple_files_not_supported_yet(cli_arg).await
                {
                    report_analytics::start_task_to_generate_event(
                        String::new(),
                        AnalyticsAction::EdiFileOpenMultiple,
                    );
                    launcher::run_app(Some(file_path)).await?;
                }
            }
        }

        // Stop logging.
        enable_logging.then(|| {
            tracing::debug!(message = "Stop logging...");
        });

        // Exit message.
        upgrade_check::show_exit_message().await;
    })
}
