/*
 *   Copyright (c) 2023 R3BL LLC
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

use crossterm::style::Stylize;
use r3bl_rs_utils_core::{call_if_true, log_debug, log_error};
use reqwest::Client;

use crate::{AnalyticsEvent, DEBUG_ANALYTICS_MOD};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnalyticsAction {
    GitiBranchDelete,
    EdiFileOpen,
    EdiFileSave,
}

impl AnalyticsAction {
    pub fn to_string(&self) -> String {
        let action = match self {
            AnalyticsAction::GitiBranchDelete => "giti branch delete",
            AnalyticsAction::EdiFileOpen => "edi file open",
            AnalyticsAction::EdiFileSave => "edi file save",
        };
        action.to_string()
    }
}

// 00: [_] implement this using the config.rs crate
pub fn get_proxy_for_id() -> String {
    // Check to see whether the ID file exists in the config folder.
    // If it does, then read the ID from the file.
    // If it does not, then generate a new ID using generate_random_friendly_id() and
    // write it to the file. And return this ID.
    "happy_panda_12".to_string()
}

// 00: [x] test this
pub fn report_analytics_event(proxy_user_id: String, action: AnalyticsAction) {
    tokio::spawn(async move {
        let proxy_machine_id = get_proxy_for_id();
        let event =
            AnalyticsEvent::new(proxy_user_id, proxy_machine_id, action.to_string());
        let result_event_json = serde_json::to_value(&event);

        match result_event_json {
            Ok(json) => {
                let result =
                    make_post_request("http://localhost:8000/add_analytics_event", &json)
                        .await;
                match result {
                    Ok(_) => {
                        println!("Successfully reported analytics event to r3bl-base.");
                    }
                    Err(error) => {
                        log_error(
                            format!(
                                "Could not report analytics event to r3bl-base.\n{:#?}",
                                error
                            )
                            .red()
                            .to_string(),
                        );
                    }
                }
            }
            Err(error) => {
                log_error(
                    format!(
                        "Could not report analytics event to r3bl-base.\n{:#?}",
                        error
                    )
                    .red()
                    .to_string(),
                );
            }
        }
    });
}

// 00: [x] test this
async fn make_post_request(
    url: &str,
    data: &serde_json::Value,
) -> Result<(), reqwest::Error> {
    let client = Client::new();
    let response = client.post(url).json(data).send().await?;
    if response.status().is_success() {
        // Handle successful response.
        call_if_true!(DEBUG_ANALYTICS_MOD, {
            log_debug(
                format!("Analytics event reported successfully: {response:?}",)
                    .green()
                    .to_string(),
            );
        });
    } else {
        // Handle error response
        log_error(
            format!("Analytics event could not be reported: {response:?}",)
                .red()
                .to_string(),
        );
    }

    Ok(())
}

const PET_NAMES: [&str; 20] = [
    "Buddy", "Max", "Bella", "Charlie", "Lucy", "Daisy", "Molly", "Lola", "Sadie",
    "Maggie", "Bailey", "Sophie", "Chloe", "Duke", "Lily", "Rocky", "Jack", "Cooper",
    "Riley", "Zoey",
];

const FRUIT_NAMES: [&str; 20] = [
    "Apple",
    "Banana",
    "Orange",
    "Pear",
    "Peach",
    "Strawberry",
    "Grape",
    "Kiwi",
    "Mango",
    "Pineapple",
    "Watermelon",
    "Cherry",
    "Blueberry",
    "Raspberry",
    "Lemon",
    "Lime",
    "Grapefruit",
    "Plum",
    "Apricot",
    "Pomegranate",
];

// 00: [x] add more variety here
pub fn generate_random_friendly_id() -> String {
    use rand::Rng;

    // Generate friendly pet and fruit name combination.
    let pet = {
        let mut rng = rand::thread_rng();
        let pet = PET_NAMES[rng.gen_range(0..PET_NAMES.len())];
        pet.to_lowercase()
    };

    let fruit = {
        let mut rng = rand::thread_rng();
        let fruit = FRUIT_NAMES[rng.gen_range(0..FRUIT_NAMES.len())];
        fruit.to_lowercase()
    };

    let random_number = {
        let mut rng = rand::thread_rng();
        rng.gen_range(0..1000)
    };

    format!("{pet}-{fruit}-{random_number}")
}
