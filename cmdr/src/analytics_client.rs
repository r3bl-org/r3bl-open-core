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
          io::{BufReader, Read, Write},
          path::PathBuf,
          sync::atomic::AtomicBool};

use dirs::config_dir;
use miette::IntoDiagnostic as _;
use r3bl_analytics_schema::AnalyticsEvent;
use r3bl_ansi_color::{green, magenta, red};
use r3bl_core::{CommonError,
                CommonErrorType,
                CommonResult,
                friendly_random_id,
                string_storage};
use reqwest::{Client, Response};

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
        write!(f, "{}", action)
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
            write!(f, "{}", path)
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
                            let message = "Successfully created config folder.";
                            let config_folder = string_storage!("{config_folder_path:?}");
                            let config_folder_fmt = green(&config_folder);
                            // % is Display, ? is Debug.
                            tracing::debug!(
                                message = %message,
                                config_folder_path = %config_folder_fmt
                            );
                        });
                        Ok(config_folder_path)
                    }
                    Err(error) => {
                        let message = "Could not create config folder.";
                        let error = string_storage!("{error:?}");
                        let error_fmt = red(&error);
                        // % is Display, ? is Debug.
                        tracing::error!(
                            message = %message,
                            error_fmt = %error_fmt,
                        );
                        CommonError::new_error_result_with_only_type(
                            CommonErrorType::ConfigFolderCountNotBeCreated,
                        )
                    }
                }
            }
            None => {
                let message = "Could not get config folder.";
                let config_folder_path =
                    string_storage!("{:?}", try_get_config_folder_path());
                let config_folder_fmt = red(&config_folder_path);
                // % is Display, ? is Debug.
                tracing::error!(
                    message = %message,
                    config_folder_path = %config_folder_fmt
                );
                CommonError::new_error_result_with_only_type(
                    CommonErrorType::ConfigFolderPathCouldNotBeGenerated,
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
                            let message = "Successfully read proxy machine ID from file.";
                            let contents = string_storage!("{contents:?}");
                            let contents_fmt = green(&contents);
                            // % is Display, ? is Debug.
                            tracing::debug!(
                                message = %message,
                                contents = %contents_fmt
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
                                    let message =
                                        "Successfully wrote proxy machine ID to file.";
                                    let new_id = string_storage!("{new_id:?}");
                                    let new_id_fmt = green(&new_id);
                                    // % is Display, ? is Debug.
                                    tracing::debug!(
                                        message = %message,
                                        new_id = %new_id_fmt
                                    );
                                });
                            }
                            Err(error) => {
                                let message = "Could not write proxy machine ID to file.";
                                let error = string_storage!("{error:?}");
                                let error_fmt = red(&error);
                                // % is Display, ? is Debug.
                                tracing::error!(
                                    message = %message,
                                    error = %error_fmt
                                );
                            }
                        }
                        new_id
                    }
                }
            }
            Err(_) => friendly_random_id::generate_friendly_random_id(),
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
                proxy_machine_id::load_id_from_file_or_generate_and_save_it();

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
                                let message =
                                    "Successfully reported analytics event to r3bl-base.";
                                let json = string_storage!("{json:#?}");
                                let json_fmt = green(&json);
                                // % is Display, ? is Debug.
                                tracing::debug!(
                                    message = %message,
                                    json = %json_fmt
                                );
                            });
                        }
                        Err(error) => {
                            let message =
                                "Could not report analytics event to r3bl-base.";
                            let error = string_storage!("{error:#?}");
                            let error_fmt = red(&error);
                            // % is Display, ? is Debug.
                            tracing::error!(
                                message = %message,
                                error = %error_fmt
                            );
                        }
                    }
                }
                Err(error) => {
                    let message = "Could not serialize analytics event to JSON.";
                    let error = string_storage!("{error:#?}");
                    let error_fmt = red(&error);
                    // % is Display, ? is Debug.
                    tracing::error!(
                        message = %message,
                        error = %error_fmt
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
    use super::*;

    static UPDATE_REQUIRED: AtomicBool = AtomicBool::new(false);

    const UPDATE_IF_NOT_THIS_VERSION: &str = "0.0.15";

    const GET_LATEST_VERSION_ENDPOINT: &str =
        "https://r3bl-base.shuttleapp.rs/get_latest_version"; // "http://localhost:8000/get_latest_version"

    pub fn is_update_required() -> bool {
        UPDATE_REQUIRED.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn start_task_to_check_for_updates() {
        tokio::spawn(async move {
            let result = http_client::make_get_request(GET_LATEST_VERSION_ENDPOINT).await;
            if let Ok(response) = result {
                if let Ok(body_text) = response.text().await {
                    let latest_version = body_text.trim().to_string();
                    DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                        let message = "\n📦📦📦\nLatest version of cmdr";
                        let version_fmt = magenta(&latest_version);
                        // % is Display, ? is Debug.
                        tracing::info!(
                            message = %message,
                            version = %version_fmt
                        );
                    });
                    let current_version = UPDATE_IF_NOT_THIS_VERSION.to_string();
                    if latest_version != current_version {
                        UPDATE_REQUIRED.store(true, std::sync::atomic::Ordering::Relaxed);
                        DEBUG_ANALYTICS_CLIENT_MOD.then(|| {
                            let message =
                                "\n💿💿💿\nThere is a new version of cmdr available";
                            let version_fmt = magenta(&latest_version);
                            // % is Display, ? is Debug.
                            tracing::info!(
                                message = %message,
                                version = %version_fmt
                            );
                        });
                    }
                }
            }
        });
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
                let message = "GET request succeeded.";
                let response = string_storage!("{response:#?}");
                let response_fmt = green(&response);
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = %message,
                    response = %response_fmt
                );
            });
            Ok(response)
        } else {
            // Handle error response.
            let message = "GET request failed.";
            let response_msg = string_storage!("{response:#?}");
            let response_msg_fmt = red(&response_msg);
            // % is Display, ? is Debug.
            tracing::error!(
                message = %message,
                response = %response_msg_fmt
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
                let message = "POST request succeeded.";
                let response_msg = string_storage!("{response:#?}");
                let response_msg_fmt = green(&response_msg);
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = %message,
                    response = %response_msg_fmt
                );
            });
            Ok(response)
        } else {
            // Handle error response.
            let message = "POST request failed.";
            let response_msg = string_storage!("{response:#?}");
            let response_msg_fmt = red(&response_msg);
            // % is Display, ? is Debug.
            tracing::error!(
                message = %message,
                response = %response_msg_fmt
            );
            response.error_for_status()
        }
    }
}
