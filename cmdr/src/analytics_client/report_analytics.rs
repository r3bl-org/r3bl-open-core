// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use r3bl_analytics_schema::AnalyticsEvent;
use r3bl_tui::inline_string;

use super::{AnalyticsAction, http_client, proxy_machine_id};
use crate::DEBUG_ANALYTICS_CLIENT_MOD;

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
        }
    }

    tokio::spawn(async move {
        let proxy_machine_id =
            proxy_machine_id::load_id_from_file_or_generate_and_save_it().to_string();

        let event =
            AnalyticsEvent::new(proxy_user_id, proxy_machine_id, action.to_string());
        let result_event_json = serde_json::to_value(&event);
        match result_event_json {
            Ok(json) => {
                let result =
                    http_client::make_post_request(ANALYTICS_REPORTING_ENDPOINT, &json)
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
