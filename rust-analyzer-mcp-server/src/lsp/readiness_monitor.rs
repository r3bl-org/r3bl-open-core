// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Thread-safe monitor for tracking `rust-analyzer` indexing readiness.
//!
//! # Architecture: The Monitor Pattern
//!
//! A **Monitor** is a synchronization construct that encapsulates:
//! 1. **Shared State**: The mutable server status payload ([`ServerReadiness`]).
//! 2. **Mutual Exclusion**: An internal [`Mutex`] guarding state transitions.
//! 3. **Condition Variable**: A [`Condvar`] signaling waiting threads when the server
//!    finishes background indexing.
//! 4. **Synchronized Operations**: High-level atomic procedures ([`update`], [`reset`],
//!    [`wait_until_indexed`]) preventing ad-hoc lock/condvar unpacking.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                 ServerReadinessMonitor                      │
//! │                                                             │
//! │  ┌────────────────────────┐       ┌──────────────────────┐  │
//! │  │ Mutex<ServerReadiness> │       │       Condvar        │  │
//! │  │  - status:             │       │                      │  │
//! │  │    IndexingStatus      │       │                      │  │
//! │  │  - health: String      │       │                      │  │
//! │  │  - message: Option     │       │                      │  │
//! │  └───────────┬────────────┘       └──────────┬───────────┘  │
//! │              │                               │              │
//! │              ▼                               ▼              │
//! │   update(new_state) ─────────────► notify_all()             │
//! │   wait_until_indexed(timeout) ◄─── wait_timeout()           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! See the [Server Readiness Flow] documentation in crate root for the complete
//! 3-step initialization and AST tool gating lifecycle.
//!
//! [`Condvar`]: std::sync::Condvar
//! [`Mutex`]: std::sync::Mutex
//! [`reset`]: ServerReadinessMonitor::reset
//! [`ServerReadiness`]: crate::lsp::ServerReadiness
//! [`update`]: ServerReadinessMonitor::update
//! [`wait_until_indexed`]: ServerReadinessMonitor::wait_until_indexed
//! [Server Readiness Flow]: mod@crate#server-readiness--background-indexing-synchronization

use super::protocol::readiness_types::{IndexingStatus, ServerReadiness};
use std::{sync::{Condvar, Mutex},
          time::{Duration, Instant}};

/// Thread-safe monitor encapsulating server readiness state, mutex, and condition
/// variable.
#[derive(Debug, Default)]
pub struct ServerReadinessMonitor {
    /// Mutual-exclusion lock protecting the latest [`ServerReadiness`] snapshot.
    state: Mutex<ServerReadiness>,

    /// Condition variable signaled whenever the server readiness state transitions or
    /// finishes indexing.
    cvar: Condvar,
}

impl ServerReadinessMonitor {
    /// Atomically updates the server readiness state and wakes all threads waiting on
    /// indexing completion.
    pub fn update(&self, new_state: ServerReadiness) {
        if let Ok(mut guard) = self.state.lock() {
            *guard = new_state;
            self.cvar.notify_all();
        }
    }

    /// Resets the server readiness state to default uninitialized.
    pub fn reset(&self) { self.update(ServerReadiness::default()); }

    /// Returns a point-in-time clone of the current [`ServerReadiness`] state.
    #[must_use]
    pub fn get_snapshot(&self) -> ServerReadiness {
        self.state.lock().map_or_else(
            |poisoned| poisoned.into_inner().clone(),
            |guard| guard.clone(),
        )
    }

    /// Blocks the calling thread until `rust-analyzer` completes indexing
    /// ([`IndexingStatus::Complete`]) or until the specified timeout expires.
    ///
    /// Returns the latest [`ServerReadiness`] snapshot.
    #[must_use]
    pub fn wait_until_indexed(&self, timeout: Duration) -> ServerReadiness {
        let mut guard = match self.state.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        if guard.status == IndexingStatus::Complete {
            return guard.clone();
        }

        let start = Instant::now();
        while guard.status != IndexingStatus::Complete {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return guard.clone();
            }
            let remaining = timeout.checked_sub(elapsed).unwrap_or(Duration::ZERO);
            let result = self
                .cvar
                .wait_timeout(guard, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard = result.0;
            if result.1.timed_out() {
                break;
            }
        }

        guard.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, thread};

    #[test]
    fn test_monitor_initial_state() {
        let monitor = ServerReadinessMonitor::default();
        let snapshot = monitor.get_snapshot();
        assert_eq!(snapshot.status, IndexingStatus::InProgress);
        assert_eq!(snapshot.health, "ok");
        assert_eq!(snapshot.message, None);
    }

    #[test]
    fn test_monitor_update_and_reset() {
        let monitor = ServerReadinessMonitor::default();
        monitor.update(ServerReadiness {
            status: IndexingStatus::Complete,
            health: "warning".to_string(),
            message: Some("Indexing complete".to_string()),
        });

        let snapshot = monitor.get_snapshot();
        assert_eq!(snapshot.status, IndexingStatus::Complete);
        assert_eq!(snapshot.health, "warning");
        assert_eq!(snapshot.message.as_deref(), Some("Indexing complete"));

        monitor.reset();
        let reset_snapshot = monitor.get_snapshot();
        assert_eq!(reset_snapshot.status, IndexingStatus::InProgress);
        assert_eq!(reset_snapshot.health, "ok");
        assert_eq!(reset_snapshot.message, None);
    }

    #[test]
    fn test_monitor_wait_until_indexed_instant_when_ready() {
        let monitor = ServerReadinessMonitor::default();
        monitor.update(ServerReadiness {
            status: IndexingStatus::Complete,
            health: "ok".to_string(),
            message: None,
        });

        let status = monitor.wait_until_indexed(Duration::from_millis(50));
        assert_eq!(status.status, IndexingStatus::Complete);
        assert_eq!(status.health, "ok");
    }

    #[test]
    fn test_monitor_wait_until_indexed_times_out() {
        let monitor = ServerReadinessMonitor::default();
        let status = monitor.wait_until_indexed(Duration::from_millis(15));
        assert_eq!(status.status, IndexingStatus::InProgress);
        assert_eq!(status.health, "ok");
    }

    #[test]
    fn test_monitor_wait_until_indexed_signaled_concurrently() {
        let monitor = Arc::new(ServerReadinessMonitor::default());
        let monitor_clone = Arc::clone(&monitor);

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(15));
            monitor_clone.update(ServerReadiness {
                status: IndexingStatus::Complete,
                health: "ok".to_string(),
                message: Some("Ready".to_string()),
            });
        });

        let status = monitor.wait_until_indexed(Duration::from_millis(500));
        assert_eq!(status.status, IndexingStatus::Complete);
        assert_eq!(status.message.as_deref(), Some("Ready"));
    }
}
