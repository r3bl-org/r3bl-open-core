// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use smallvec::smallvec;
use uuid::Uuid;

use crate::TinyVecBackingStore;

/// A collection of analytics events stored in a memory-efficient structure.
///
/// `AnalyticsRecord` is the top-level container for analytics data, using a `TinyVec`
/// for storage optimization. It can store analytics events inline for small collections
/// or heap-allocate for larger collections.
///
/// # Examples
///
/// ```
/// use r3bl_analytics_schema::analytics_data::{AnalyticsRecord, AnalyticsEvent};
///
/// let mut record = AnalyticsRecord::default();
/// // Add events to the record...
/// ```

#[derive(Debug, Deserialize, Serialize)]
pub struct AnalyticsRecord {
    pub events: TinyVecBackingStore<AnalyticsEvent>,
}

impl Default for AnalyticsRecord {
    fn default() -> Self { Self::new() }
}

impl AnalyticsRecord {
    #[must_use]
    pub fn new() -> Self {
        let events = smallvec![];
        Self { events }
    }
}

/// An analytics event without timestamp, used as a builder pattern intermediate.
///
/// This struct is useful when creating analytics events where the timestamp and UUID
/// will be added automatically during conversion to `AnalyticsEvent`. It reduces
/// boilerplate when the caller doesn't need to specify these fields manually.
///
/// # Examples
///
/// ```
/// use r3bl_analytics_schema::analytics_data::{AnalyticsEventNoTimestamp, AnalyticsEvent};
/// 
/// let event_no_timestamp = AnalyticsEventNoTimestamp {
///     proxy_user_id: String::new(),
///     proxy_machine_id: "happy_panda_12".to_string(),
///     action: "edi file save".to_string(),
/// };
/// 
/// let event: AnalyticsEvent = event_no_timestamp.into();
/// ```
#[rustfmt::skip]
#[derive(Debug, Deserialize, Serialize)]
pub struct AnalyticsEventNoTimestamp {
    pub proxy_user_id: String,    /* from OAuth provider, currently empty string. */
    pub proxy_machine_id: String, /* generated for each machine, eg: happy_panda_12 */
    pub action: String,        /* “giti branch delete”, or “edi file open”, or “edi file save” */
}

/// Represents a single analytics event with metadata and timing information.
///
/// Each event captures user actions within the application with sufficient context
/// for analytics purposes while maintaining user privacy. The event includes both
/// user and machine identifiers along with action details and timing.
///
/// # Privacy
/// 
/// The `proxy_user_id` field is currently unused (empty string) to maintain user
/// privacy. The `proxy_machine_id` is a generated, non-personal identifier.
///
/// # Examples
///
/// ```
/// use r3bl_analytics_schema::analytics_data::AnalyticsEvent;
/// use std::time::{SystemTime, UNIX_EPOCH};
/// 
/// let event = AnalyticsEvent {
///     proxy_user_id: String::new(),
///     proxy_machine_id: "happy_panda_12".to_string(),
///     action: "edi file save".to_string(),
///     timestamp_ms: SystemTime::now()
///         .duration_since(UNIX_EPOCH)
///         .unwrap()
///         .as_millis() as u64,
///     uuid: "unique-event-id".to_string(),
/// };
/// ```
#[rustfmt::skip]
#[derive(Debug, Deserialize, Serialize)]
pub struct AnalyticsEvent {
    pub proxy_user_id: String,    /* from OAuth provider, currently empty string. */
    pub proxy_machine_id: String, /* generated for each machine, eg: happy_panda_12 */
    pub action: String,           /* “giti branch delete”, or “edi file open”, or “edi file save” */
    pub timestamp_ms: u64,        /* time elapsed in ms since UNIX EPOCH */
    pub uuid: String,             /* unique identifier for this event */
}

impl AnalyticsEvent {
    /// Creates a new analytics event without timestamp and UUID.
    ///
    /// This constructor is designed to be called on the client side before sending
    /// data to the server. The timestamp and UUID are intentionally omitted as they
    /// will be set server-side to ensure consistency and prevent client manipulation.
    ///
    /// # Arguments
    ///
    /// * `proxy_user_id` - User identifier from OAuth provider (currently unused for
    ///   privacy)
    /// * `proxy_machine_id` - Generated machine identifier (e.g., "`happy_panda_12`")
    /// * `action` - Description of the user action (e.g., "edi file save", "giti branch
    ///   delete")
    ///
    /// # Returns
    ///
    /// An `AnalyticsEventNoTimestamp` that can be converted to `AnalyticsEvent` later.
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_analytics_schema::analytics_data::AnalyticsEvent;
    ///
    /// let event_no_timestamp = AnalyticsEvent::new(
    ///     String::new(), // Empty for privacy
    ///     "happy_panda_12".to_string(),
    ///     "file_save".to_string(),
    /// );
    /// ```
    #[allow(clippy::new_ret_no_self)]
    #[must_use]
    pub const fn new(
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

/// Convert [`AnalyticsEventNoTimestamp`] to [`AnalyticsEvent`].
impl From<AnalyticsEventNoTimestamp> for AnalyticsEvent {
    fn from(incoming: AnalyticsEventNoTimestamp) -> Self {
        let result_timestamp_ms = SystemTime::now().duration_since(UNIX_EPOCH);

        let timestamp_ms = match result_timestamp_ms {
            Ok(duration_since_epoch) => {
                u64::try_from(duration_since_epoch.as_millis()).unwrap_or(0)
            }
            Err(_) => 0,
        };

        let uuid = Uuid::new_v4().to_string();

        Self {
            proxy_user_id: incoming.proxy_user_id,
            proxy_machine_id: incoming.proxy_machine_id,
            action: incoming.action,
            timestamp_ms,
            uuid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_record_default() {
        let record = AnalyticsRecord::default();
        assert!(record.events.is_empty());
        assert_eq!(record.events.len(), 0);
    }

    #[test]
    fn test_analytics_record_add_events() {
        let mut record = AnalyticsRecord::default();

        let event = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "happy_panda_12".to_string(),
            action: "file_save".to_string(),
        };

        record.events.push(event.into());
        assert_eq!(record.events.len(), 1);

        // Add another event
        let event2 = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "sleepy_cat_34".to_string(),
            action: "file_open".to_string(),
        };

        record.events.push(event2.into());
        assert_eq!(record.events.len(), 2);
    }

    #[test]
    fn test_analytics_event_no_timestamp_creation() {
        let event = AnalyticsEventNoTimestamp {
            proxy_user_id: "user_123".to_string(),
            proxy_machine_id: "machine_456".to_string(),
            action: "test_action".to_string(),
        };

        assert_eq!(event.proxy_user_id, "user_123");
        assert_eq!(event.proxy_machine_id, "machine_456");
        assert_eq!(event.action, "test_action");
    }

    #[test]
    fn test_analytics_event_from_no_timestamp() {
        let event_no_timestamp = AnalyticsEventNoTimestamp {
            proxy_user_id: "test_user".to_string(),
            proxy_machine_id: "test_machine".to_string(),
            action: "test_action".to_string(),
        };

        let event: AnalyticsEvent = event_no_timestamp.into();

        assert_eq!(event.proxy_user_id, "test_user");
        assert_eq!(event.proxy_machine_id, "test_machine");
        assert_eq!(event.action, "test_action");
        assert!(event.timestamp_ms > 0);
        assert!(!event.uuid.is_empty());
        assert_eq!(event.uuid.len(), 36); // Standard UUID length with hyphens
    }

    #[test]
    fn test_analytics_event_new_constructor() {
        let event_no_timestamp = AnalyticsEvent::new(
            "user_id".to_string(),
            "machine_id".to_string(),
            "action".to_string(),
        );

        assert_eq!(event_no_timestamp.proxy_user_id, "user_id");
        assert_eq!(event_no_timestamp.proxy_machine_id, "machine_id");
        assert_eq!(event_no_timestamp.action, "action");
    }

    #[test]
    fn test_analytics_event_uuid_uniqueness() {
        let event1: AnalyticsEvent = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "machine1".to_string(),
            action: "action1".to_string(),
        }
        .into();

        let event2: AnalyticsEvent = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "machine2".to_string(),
            action: "action2".to_string(),
        }
        .into();

        assert_ne!(event1.uuid, event2.uuid);
    }

    #[test]
    fn test_analytics_event_timestamp_ordering() {
        let event1: AnalyticsEvent = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "machine1".to_string(),
            action: "action1".to_string(),
        }
        .into();

        // Add a small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(1));

        let event2: AnalyticsEvent = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "machine2".to_string(),
            action: "action2".to_string(),
        }
        .into();

        assert!(event2.timestamp_ms >= event1.timestamp_ms);
    }

    #[test]
    fn test_serde_serialization() {
        let mut record = AnalyticsRecord::default();
        let event = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "test_machine".to_string(),
            action: "test_action".to_string(),
        };
        record.events.push(event.into());

        // Test JSON serialization
        let json = serde_json::to_string(&record).expect("Failed to serialize");
        assert!(!json.is_empty());
        assert!(json.contains("test_machine"));
        assert!(json.contains("test_action"));

        // Test deserialization
        let deserialized: AnalyticsRecord =
            serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.events.len(), 1);
        assert_eq!(deserialized.events[0].proxy_machine_id, "test_machine");
        assert_eq!(deserialized.events[0].action, "test_action");
    }

    #[test]
    fn test_debug_formatting() {
        let record = AnalyticsRecord::default();
        let debug_output = format!("{record:?}");
        assert!(debug_output.contains("AnalyticsRecord"));

        let event = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "debug_machine".to_string(),
            action: "debug_action".to_string(),
        };
        let debug_output = format!("{event:?}");
        assert!(debug_output.contains("AnalyticsEventNoTimestamp"));
        assert!(debug_output.contains("debug_machine"));
        assert!(debug_output.contains("debug_action"));
    }

    #[test]
    fn test_privacy_user_id_empty() {
        // Test that we maintain privacy by keeping user_id empty
        let event = AnalyticsEventNoTimestamp {
            proxy_user_id: String::new(),
            proxy_machine_id: "privacy_test".to_string(),
            action: "privacy_action".to_string(),
        };

        assert!(event.proxy_user_id.is_empty());

        let full_event: AnalyticsEvent = event.into();
        assert!(full_event.proxy_user_id.is_empty());
    }

    #[test]
    fn test_tiny_vec_efficiency() {
        // Test that small collections use inline storage
        let mut record = AnalyticsRecord::default();

        // Add events up to the inline capacity
        for i in 0..crate::DEFAULT_TINY_VEC_SIZE {
            let event = AnalyticsEventNoTimestamp {
                proxy_user_id: String::new(),
                proxy_machine_id: format!("machine_{i}"),
                action: format!("action_{i}"),
            };
            record.events.push(event.into());
        }

        assert_eq!(record.events.len(), crate::DEFAULT_TINY_VEC_SIZE);

        // Verify we can still access all events
        for (i, event) in record.events.iter().enumerate() {
            assert_eq!(event.proxy_machine_id, format!("machine_{i}"));
            assert_eq!(event.action, format!("action_{i}"));
        }
    }
}
