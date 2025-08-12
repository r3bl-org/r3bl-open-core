// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::time::{Duration, Instant};

/// This enum represents the status of the rate limiter:
/// - [`RateLimitStatus::NotStarted`]: The rate limiter has not been run yet.
/// - [`RateLimitStatus::Expired`]: The rate limiter has been run, but the time since the
///   last run exceeds the minimum threshold.
/// - [`RateLimitStatus::Active`]: The rate limiter has been run, and the time since the
///   last run is within the minimum threshold.
#[derive(Debug, PartialEq)]
pub enum RateLimitStatus {
    NotStarted,
    Expired,
    Active,
}

/// If you have an expensive operation that you want to run at most once every `n` time
/// units, you can use this struct to track the last time the operation was run. It tracks
/// the time since the last run and determines if the rate limit is hit or not. Follow
/// these steps to use it:
///
/// 1. Create a [`Self::new`] instance of this struct with the desired minimum time
///    threshold.
/// 2. Before running your expensive operation, call [`Self::get_status`] with the current
///    time. You can use teh [`Self::get_status_and_update_last_run`] method to do the
///    following automatically:
///    - If the status is [`RateLimitStatus::NotStarted`] or [`RateLimitStatus::Expired`],
///      run the operation and update the last run time with [`Self::update_last_run`].
///    - Otherwise, it is [`RateLimitStatus::Active`] and don't run the operation.
#[derive(Debug, PartialEq)]
pub struct RateLimiter {
    /// An `Option<Instant>` that stores the last run time.
    pub last_run: Option<Instant>,
    /// A `Duration` that specifies the minimum time threshold between runs.
    pub min_time_threshold: Duration,
}

impl RateLimiter {
    #[must_use]
    pub fn new(min_time_threshold: Duration) -> Self {
        Self {
            last_run: None,
            min_time_threshold,
        }
    }

    /// This method is useful if you want to run the operation only if the rate limit is
    /// not hit. When called, it will automatically update the last run time if the rate
    /// limit is not hit (so you don't have to call [`Self::update_last_run`] manually.
    pub fn get_status_and_update_last_run(&mut self, now: Instant) -> RateLimitStatus {
        let status = self.get_status(now);
        match status {
            RateLimitStatus::NotStarted | RateLimitStatus::Expired => {
                self.update_last_run(now);
            }
            RateLimitStatus::Active => {}
        }
        status
    }

    /// You probably want to use this instead [`Self::get_status_and_update_last_run`].
    pub fn get_status(&mut self, now: Instant) -> RateLimitStatus {
        match self.last_run {
            None => RateLimitStatus::NotStarted,
            Some(last_run) => {
                if now.duration_since(last_run) > self.min_time_threshold {
                    RateLimitStatus::Expired
                } else {
                    RateLimitStatus::Active
                }
            }
        }
    }

    pub fn update_last_run(&mut self, now: Instant) { self.last_run.replace(now); }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;

    use super::*;

    #[test]
    fn test_not_started() {
        let mut rate_limiter = RateLimiter::new(Duration::from_nanos(1));
        assert_eq!(
            rate_limiter.get_status(Instant::now()),
            RateLimitStatus::NotStarted
        );
    }

    #[test]
    fn test_expired() {
        let mut rate_limiter = RateLimiter::new(Duration::from_nanos(1));
        let now = Instant::now();
        rate_limiter.update_last_run(now.checked_sub(Duration::from_nanos(2)).unwrap());
        assert_eq!(rate_limiter.get_status(now), RateLimitStatus::Expired);
    }

    #[test]
    fn test_active() {
        let mut rate_limiter = RateLimiter::new(Duration::from_nanos(1));
        let now = Instant::now();
        rate_limiter.update_last_run(now);
        assert_eq!(rate_limiter.get_status(now), RateLimitStatus::Active);
    }

    #[test]
    fn test_from_expired_to_active() {
        let mut rate_limiter = RateLimiter::new(Duration::from_nanos(1));
        let now = Instant::now();
        rate_limiter.update_last_run(now.checked_sub(Duration::from_nanos(2)).unwrap());
        assert_eq!(rate_limiter.get_status(now), RateLimitStatus::Expired);
        rate_limiter.update_last_run(now);
        assert_eq!(rate_limiter.get_status(now), RateLimitStatus::Active);
    }

    #[test]
    fn test_active_to_expired() {
        let mut rate_limiter = RateLimiter::new(Duration::from_nanos(1));
        let now = Instant::now();
        rate_limiter.update_last_run(now);
        assert_eq!(rate_limiter.get_status(now), RateLimitStatus::Active);
        sleep(Duration::from_nanos(2));
        assert_eq!(
            rate_limiter.get_status(Instant::now()),
            RateLimitStatus::Expired
        );
    }

    #[test]
    fn test_get_status_and_update_last_run() {
        let mut rate_limiter = RateLimiter::new(Duration::from_nanos(1));
        let now = Instant::now();

        assert_eq!(
            rate_limiter.get_status_and_update_last_run(now),
            RateLimitStatus::NotStarted
        );
        assert_eq!(rate_limiter.last_run, Some(now));

        assert_eq!(
            rate_limiter.get_status_and_update_last_run(now),
            RateLimitStatus::Active
        );
        assert_eq!(rate_limiter.last_run, Some(now));

        assert_eq!(
            rate_limiter.get_status_and_update_last_run(now),
            RateLimitStatus::Active
        );
        assert_eq!(rate_limiter.last_run, Some(now));

        assert_eq!(
            rate_limiter.get_status_and_update_last_run(now + Duration::from_nanos(2)),
            RateLimitStatus::Expired
        );
        assert_eq!(rate_limiter.last_run, Some(now + Duration::from_nanos(2)));

        assert_eq!(
            rate_limiter.get_status_and_update_last_run(now + Duration::from_nanos(2)),
            RateLimitStatus::Active
        );
        assert_eq!(rate_limiter.last_run, Some(now + Duration::from_nanos(2)));
    }
}
