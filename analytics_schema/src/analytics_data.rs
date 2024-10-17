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

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct AnalyticsRecord {
    pub events: Vec<AnalyticsEvent>,
}

impl Default for AnalyticsRecord {
    fn default() -> Self { Self::new() }
}

impl AnalyticsRecord {
    pub fn new() -> AnalyticsRecord {
        let events = Vec::new();
        AnalyticsRecord { events }
    }
}

#[rustfmt::skip]
#[derive(Deserialize, Serialize)]
pub struct AnalyticsEventNoTimestamp {
    pub proxy_user_id: String,    /* from OAuth provider, currently empty string. */
    pub proxy_machine_id: String, /* generated for each machine, eg: happy_panda_12 */
    pub action: String,        /* “giti branch delete”, or “edi file open”, or “edi file save” */
}

#[rustfmt::skip]
#[derive(Deserialize, Serialize)]
pub struct AnalyticsEvent {
    pub proxy_user_id: String,    /* from OAuth provider, currently empty string. */
    pub proxy_machine_id: String, /* generated for each machine, eg: happy_panda_12 */
    pub action: String,           /* “giti branch delete”, or “edi file open”, or “edi file save” */
    pub timestamp_ms: u64,        /* time elapsed in ms since UNIX EPOCH */
    pub uuid: String,             /* unique identifier for this event */
}

impl AnalyticsEvent {
    /// This is meant to be called on the client, before the data is sent to the server.
    /// The time is not set here since it will be set on the server-side.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        proxy_user_id: String,
        proxy_machine_id: String,
        action: String,
    ) -> AnalyticsEventNoTimestamp {
        AnalyticsEventNoTimestamp {
            proxy_user_id,
            proxy_machine_id,
            action,
        }
    }
}

/// Convert [AnalyticsEventNoTimestamp] to [AnalyticsEvent].
impl From<AnalyticsEventNoTimestamp> for AnalyticsEvent {
    fn from(incoming: AnalyticsEventNoTimestamp) -> AnalyticsEvent {
        let result_timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH);

        let timestamp_ms = match result_timestamp_ms {
            Ok(duration_since_epoch) => duration_since_epoch.as_millis() as u64,
            Err(_) => 0,
        };

        let uuid = Uuid::new_v4().to_string();

        AnalyticsEvent {
            proxy_user_id: incoming.proxy_user_id,
            proxy_machine_id: incoming.proxy_machine_id,
            action: incoming.action,
            timestamp_ms,
            uuid,
        }
    }
}
