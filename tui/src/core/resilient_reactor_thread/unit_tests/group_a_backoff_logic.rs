// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::resilient_reactor_thread::{RestartPolicy, advance_backoff_delay};
use std::time::Duration;

#[test]
fn test_backoff_exponential_doubling() {
    let policy = RestartPolicy {
        max_restarts: 5,
        initial_delay: Some(Duration::from_millis(100)),
        backoff_multiplier: Some(2.0),
        max_delay: Some(Duration::from_secs(10)),
    };

    let d1 = advance_backoff_delay(Duration::from_millis(100), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(200));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(400));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(800));
}

#[test]
fn test_backoff_max_delay_capping() {
    let policy = RestartPolicy {
        max_restarts: 5,
        initial_delay: Some(Duration::from_millis(100)),
        backoff_multiplier: Some(2.0),
        max_delay: Some(Duration::from_millis(300)),
    };

    let d1 = advance_backoff_delay(Duration::from_millis(100), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(200));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(300));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(300));
}

#[test]
fn test_backoff_constant_delay() {
    let policy = RestartPolicy {
        max_restarts: 3,
        initial_delay: Some(Duration::from_millis(50)),
        backoff_multiplier: None,
        max_delay: None,
    };

    let d1 = advance_backoff_delay(Duration::from_millis(50), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(50));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(50));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(50));
}

#[test]
fn test_backoff_unbounded_growth() {
    let policy = RestartPolicy {
        max_restarts: 10,
        initial_delay: Some(Duration::from_millis(100)),
        backoff_multiplier: Some(3.0),
        max_delay: None,
    };

    let d1 = advance_backoff_delay(Duration::from_millis(100), &policy).unwrap();
    assert_eq!(d1, Duration::from_millis(300));

    let d2 = advance_backoff_delay(d1, &policy).unwrap();
    assert_eq!(d2, Duration::from_millis(900));

    let d3 = advance_backoff_delay(d2, &policy).unwrap();
    assert_eq!(d3, Duration::from_millis(2700));
}
