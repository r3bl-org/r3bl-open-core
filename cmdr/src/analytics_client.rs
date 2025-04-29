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
use std::{fmt::{Display, Formatter, Result},
          fs::{self, File},
          io::{BufReader, Read, Write, stderr},
          path::PathBuf,
          process::{Command, ExitStatus, Stdio},
          sync::{Arc, atomic::AtomicBool},
          time::Duration};

use dirs::config_dir;
use miette::IntoDiagnostic as _;
use r3bl_analytics_schema::AnalyticsEvent;
use r3bl_tui::{ColorWheel,
               CommonError,
               CommonErrorType,
               CommonResult,
               DefaultIoDevices,
               GradientGenerationPolicy,
               HowToChoose,
               InlineString,
               ReadlineAsync,
               SpinnerStyle,
               StdMutex,
               StyleSheet,
               TextColorizationPolicy,
               choose,
               friendly_random_id,
               glyphs,
               height,
               inline_string,
               spinner::Spinner,
               try_get_latest_release_version_from_crates_io,
               width};
use reqwest::{Client, Response};
use tokio::task;

use crate::DEBUG_ANALYTICS_CLIENT_MOD;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalyticsAction {
    GitiBranchDelete,
    GitiFailedToRun,
    GitiAppStart,
    EdiAppStart,
    EdiFileNew,
    EdiFileOpenSingle,
    EdiFileOpenMultiple,
    EdiFileSave,
    MachineIdProxyCreate,
}

impl std::fmt::Display for AnalyticsAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #[rustfmt::skip]
        let action = match self {
            AnalyticsAction::GitiAppStart =>          "giti app start",
            AnalyticsAction::GitiBranchDelete =>      "giti branch delete",
            AnalyticsAction::GitiFailedToRun =>       "giti failed to run",
            AnalyticsAction::EdiAppStart =>           "edi app start",
            AnalyticsAction::EdiFileNew =>            "edi file new",
            AnalyticsAction::EdiFileOpenSingle =>     "edi file open one file",
            AnalyticsAction::EdiFileOpenMultiple =>   "edi file open many files",
            AnalyticsAction::EdiFileSave =>           "edi file save",
            AnalyticsAction::MachineIdProxyCreate =>  "proxy machine id create",
        };
        write!(f, "{action}")
    }
}

pub mod config_folder {
    use super::*;

    pub enum ConfigPaths {
        R3BLTopLevelFolderName,
        ProxyMachineIdFile,
    }

    impl Display for ConfigPaths {
        /// This generates a `to_string()` method used by [config_folder::get_id_file_path].
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            let path = match self {
                ConfigPaths::R3BLTopLevelFolderName => "r3bl-cmdr",
                ConfigPaths::ProxyMachineIdFile => "id",
            };
            write!(f, "{path}")
        }
    }

    /// This is where the config file is stored.
    pub fn get_id_file_path(path: PathBuf) -> PathBuf {
        path.join(format!("{}", ConfigPaths::ProxyMachineIdFile))
    }

    /// This is where the config folder is.
    pub fn try_get_config_folder_path() -> Option<PathBuf> {
        let home_config_folder_path = config_dir()?;
        let config_file_path =
            home_config_folder_path.join(ConfigPaths::R3BLTopLevelFolderName.to_string());
        Some(config_file_path)
    }

    pub fn exists() -> bool {
        match try_get_config_folder_path() {
            Some(config_file_path) => config_file_path.exists(),
            None => false,
        }
    }

    pub fn create() -> CommonResult<PathBuf> {
        match try_get_config_folder_path() {
            Some(config_folder_path) => {
                let result_create_dir_all = fs::create_dir_all(&config_folder_path);
                match result_create_dir_all {
                    Ok(_) => {
                        DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug!(
                                message = "Successfully created config folder.",
                                config_folder = ?config_folder_path
                            );
                        });
                        Ok(config_folder_path)
                    }
                    Err(error) => {
                        // % is Display, ? is Debug.
                        tracing::error!(
                            message = "Could not create config folder.",
                            error = ?error
                        );
                        CommonError::new_error_result_with_only_type(
                            CommonErrorType::ConfigFolderCountNotBeCreated,
                        )
                    }
                }
            }
            None => {
                // % is Display, ? is Debug.
                tracing::error!(
                    message = "Could not access config folder.",
                    error = "None"
                );
                CommonError::new_error_result_with_only_type(
                    CommonErrorType::ConfigFolderPathCouldNotBeAccessed,
                )
            }
        }
    }
}

pub mod file_io {
    use super::*;

    pub fn try_read_file_contents(path: &PathBuf) -> CommonResult<String> {
        let file = File::open(path).into_diagnostic()?;
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        let _ = reader.read_to_string(&mut contents).into_diagnostic()?;
        Ok(contents)
    }

    pub fn try_write_file_contents(path: &PathBuf, contents: &str) -> CommonResult<()> {
        let mut file = File::create(path).into_diagnostic()?;
        file.write_all(contents.as_bytes()).into_diagnostic()?;
        Ok(())
    }
}

pub mod proxy_machine_id {
    use super::*;

    /// Read the file contents from [config_folder::get_id_file_path] and return it as a
    /// string if it exists and can be read.
    pub fn load_id_from_file_or_generate_and_save_it() -> String {
        match config_folder::create() {
            Ok(config_folder_path) => {
                let id_file_path =
                    config_folder::get_id_file_path(config_folder_path.clone());
                let result = file_io::try_read_file_contents(&id_file_path);
                match result {
                    Ok(contents) => {
                        DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                            // % is Display, ? is Debug.
                            tracing::debug!(
                                message = "Successfully read proxy machine ID from file.",
                                contents = %contents
                            );
                        });

                        contents
                    }
                    Err(_) => {
                        let new_id = friendly_random_id::generate_friendly_random_id();
                        let result_write_file_contents =
                            file_io::try_write_file_contents(&id_file_path, &new_id);
                        match result_write_file_contents {
                            Ok(_) => {
                                report_analytics::start_task_to_generate_event(
                                    "".to_string(),
                                    AnalyticsAction::MachineIdProxyCreate,
                                );
                                DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                                    // % is Display, ? is Debug.
                                    tracing::debug!(
                                        message = "Successfully wrote proxy machine ID to file.",
                                        new_id = %new_id
                                    );
                                });
                            }
                            Err(error) => {
                                // % is Display, ? is Debug.
                                tracing::error!(
                                    message = "Could not write proxy machine ID to file.",
                                    error = ?error
                                );
                            }
                        }

                        new_id.to_string()
                    }
                }
            }
            Err(_) => friendly_random_id::generate_friendly_random_id().to_string(),
        }
    }
}

pub mod report_analytics {
    use super::*;

    static mut ANALYTICS_REPORTING_ENABLED: bool = true;

    const ANALYTICS_REPORTING_ENDPOINT: &str =
        "https://r3bl-base.shuttleapp.rs/add_analytics_event"; // "http://localhost:8000/add_analytics_event"

    pub fn disable() {
        unsafe {
            ANALYTICS_REPORTING_ENABLED = false;
        }
    }

    pub fn start_task_to_generate_event(proxy_user_id: String, action: AnalyticsAction) {
        unsafe {
            if !ANALYTICS_REPORTING_ENABLED {
                return;
            };
        }

        tokio::spawn(async move {
            let proxy_machine_id =
                proxy_machine_id::load_id_from_file_or_generate_and_save_it().to_string();

            let event =
                AnalyticsEvent::new(proxy_user_id, proxy_machine_id, action.to_string());
            let result_event_json = serde_json::to_value(&event);
            match result_event_json {
                Ok(json) => {
                    let result = http_client::make_post_request(
                        ANALYTICS_REPORTING_ENDPOINT,
                        &json,
                    )
                    .await;
                    match result {
                        Ok(_) => {
                            DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                                // % is Display, ? is Debug.
                                tracing::debug!(
                                    message = "Successfully reported analytics event to r3bl-base.",
                                    json = %inline_string!("{json:#?}")
                                );
                            });
                        }
                        Err(error) => {
                            // % is Display, ? is Debug.
                            tracing::error!(
                                message = "Could not report analytics event to r3bl-base.",
                                error = ?error
                            );
                        }
                    }
                }
                Err(error) => {
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = "Could not serialize analytics event to JSON.",
                        error = ?error
                    );
                }
            }
        });
    }
}

/// To trigger an update message to be displayed to the user here are the details:
///
/// - The value of `GET_LATEST_VERSION_ENDPOINT` needs to be different,
/// - from the value of `UPDATE_IF_NOT_THIS_VERSION` in the `r3bl_base` repo.
pub mod upgrade_check {
    use std::os::unix::process::ExitStatusExt as _;

    use super::*;

    static UPDATE_REQUIRED: AtomicBool = AtomicBool::new(false);

    /// Gets version number from `Cargo.toml` (at compile time).
    pub fn get_self_version() -> &'static str { env!("CARGO_PKG_VERSION") }

    /// Returns the crate name from `Cargo.toml` (at compile time).
    pub fn get_self_crate_name() -> &'static str { env!("CARGO_PKG_NAME") }

    /// Get the filename of currently running executable (at run time).
    pub fn get_self_bin_name() -> String {
        std::env::current_exe()
            .ok()
            .and_then(|path| {
                path.file_name()
                    .and_then(|os_str| os_str.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn get_self_bin_emoji() -> String {
        let bin_name = get_self_bin_name();
        match bin_name.as_str() {
            "giti" => "🐱".to_string(),
            "edi" => "🦜".to_string(),
            _ => "👾".to_string(),
        }
    }

    pub fn is_update_required() -> bool {
        UPDATE_REQUIRED.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn start_task_to_check_for_updates() {
        tokio::spawn(async move {
            let version_self = get_self_version();
            if let Ok(version_crates) =
                try_get_latest_release_version_from_crates_io(get_self_crate_name()).await
            {
                DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                    // % is Display, ? is Debug.
                    tracing::info!(
                        message = "📦📦📦 Latest version of cmdr",
                        version = %version_crates
                    );
                });

                if version_self != version_crates {
                    UPDATE_REQUIRED.store(true, std::sync::atomic::Ordering::Relaxed);
                    DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                        // % is Display, ? is Debug.
                        tracing::info!(
                            message = "💿💿💿 There is a new version of cmdr available",
                            version = %version_crates
                        );
                    });
                }
            }
        });
    }

    pub fn upgrade_required_message() -> InlineString {
        let bin_name = get_self_bin_name();
        let crate_name = get_self_crate_name();

        let plain_text_exit_msg = inline_string!(
            "\n{}\n{}",
            inline_string!(" 🎁 A new version of {} is available.", bin_name),
            inline_string!(
                " {} You can run `cargo install {}` to upgrade.",
                glyphs::PROMPT,
                crate_name
            )
        );

        ColorWheel::default().colorize_into_string(
            &plain_text_exit_msg,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
            None,
        )
    }

    pub fn goodbye_thanks_for_using_message() -> InlineString {
        let bin_name = get_self_bin_name();
        let goodbye = match std::env::var("USER") {
            Ok(username) => {
                inline_string!(
                    "\n Goodbye, 👋 {username}. Thanks for using 😺 {bin_name}!"
                )
            }
            Err(_) => inline_string!("\n Goodbye 👋.\n\n 😺 {bin_name}!"),
        };

        let please_star_us = inline_string!(
            " Please report issues & star us on GitHub: 🌟 🐞 \
            \n https://github.com/r3bl-org/r3bl-open-core/issues/new/choose"
        );

        let combined = inline_string!("{goodbye}\n{please_star_us}");

        ColorWheel::lolcat_into_string(&combined, None)
    }

    /// Prints any pending upgrade message, then asks the user if they'd like to install
    /// the new version now.
    pub async fn show_exit_message() {
        if upgrade_check::is_update_required() {
            let crate_name = get_self_crate_name();
            let upgrade_available_text = upgrade_check::upgrade_required_message();

            // 1. Show the “upgrade available” text
            println!("{upgrade_available_text}\n");

            // 2. Ask the user
            let options = &["Yes, upgrade now", "No, thanks"];
            let mut io = DefaultIoDevices::default();
            let picked = choose(
                inline_string!("Would you like to upgrade {} now?", crate_name),
                options,
                Some(height(2)),
                Some(width(0)),
                HowToChoose::Single,
                StyleSheet::default(),
                io.as_mut_tuple(),
            )
            .await
            .ok()
            .and_then(|v| v.into_iter().next());

            // 3. If they chose “Yes, upgrade now”, run `cargo install …`
            if let Some(choice) = picked {
                // Without spinner.
                // if choice == options[0] {
                //     let res = Command::new("cargo")
                //         .args(&["install", "r3bl-cmdr"])
                //         .status();
                //     match res {
                //         Ok(status) => {
                //             if status.success() {
                //                 println!(" Upgrade successful! 🎉");
                //             } else {
                //                 println!(" Upgrade failed. Please try again.");
                //             }
                //         }
                //         Err(e) => {
                //             println!(" Error running cargo install: {e}");
                //         }
                //     }
                // }

                // With spinner.
                if choice == options[0] {
                    upgrade_check::install_with_spinner().await;
                }
            }
        }

        // Print goodbye message.
        let exit_msg = upgrade_check::goodbye_thanks_for_using_message();
        println!("{exit_msg}");
    }

    async fn install_with_spinner() {
        let crate_name = upgrade_check::get_self_crate_name();

        // 1) Create readline async.
        let res_readline_async = ReadlineAsync::try_new(Some(format!(
            " {} cargo install {}... ",
            glyphs::PROMPT,
            crate_name
        )))
        .await;

        let mut maybe_spinner: Option<Spinner> = None;

        // 2) Spawn spinner task.
        if let Ok(Some(readline_async)) = &res_readline_async {
            // Configure the spinner.
            let res = Spinner::try_start(
                format!("Installing {crate_name}..."),
                Duration::from_millis(100),
                SpinnerStyle::default(),
                Arc::new(StdMutex::new(stderr())),
                readline_async.clone_shared_writer(),
            )
            .await;

            if let Ok(Some(spinner)) = res {
                maybe_spinner = Some(spinner);
            }
        }

        // 3) Run the install in a blocking thread.
        let res_install_status = task::spawn_blocking(|| {
            Command::new("cargo")
                .args(["install", crate_name])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .status()
        })
        .await
        .unwrap_or_else(|err| {
            eprintln!(" Failed to spawn cargo install {crate_name}: {err}");
            let it = ExitStatus::from_raw(1);
            Ok(it)
        });

        // 4) Stop the spinner & wait.
        if let Ok(Some(readline_async)) = res_readline_async {
            if let Some(mut spinner) = maybe_spinner
                && !spinner.is_shutdown()
            {
                let _ = spinner.stop(" Finished installation!").await;
            };
            let msg = format!("{crate_name} is installed 🎉.");
            _ = readline_async.exit(Some(&msg)).await;
        }

        // 5) Report result.
        match res_install_status {
            Ok(status) => {
                if status.success() {
                    println!("\n ✅ Update installed successfully.");
                } else {
                    eprintln!("\n ❌ Update failed (exit code {:?}).", status.code());
                }
            }
            Err(err) => {
                eprintln!(" Failed to run install: {err}");
            }
        }
    }
}

pub mod http_client {
    use super::*;

    pub async fn make_get_request(
        url: &str,
    ) -> core::result::Result<Response, reqwest::Error> {
        let client = Client::new();
        let response = client.get(url).send().await?;
        if response.status().is_success() {
            // Handle successful response.
            DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                tracing::debug!(
                    message = "GET request succeeded.",
                    response = %inline_string!("{response:#?}")
                );
            });
            Ok(response)
        } else {
            // Handle error response.
            // % is Display, ? is Debug.
            tracing::error!(
                message = "GET request failed.",
                response = %inline_string!("{response:#?}")
            );
            response.error_for_status()
        }
    }

    pub async fn make_post_request(
        url: &str,
        data: &serde_json::Value,
    ) -> core::result::Result<Response, reqwest::Error> {
        let client = Client::new();
        let response = client.post(url).json(data).send().await?;
        if response.status().is_success() {
            // Handle successful response.
            DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "POST request succeeded.",
                    response = %inline_string!("{response:#?}")
                );
            });
            Ok(response)
        } else {
            // Handle error response.
            tracing::error!(
                message = "POST request failed.",
                response = %inline_string!("{response:#?}")
            );
            response.error_for_status()
        }
    }
}
