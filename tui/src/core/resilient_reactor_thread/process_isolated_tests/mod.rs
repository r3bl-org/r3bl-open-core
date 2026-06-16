// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

pub mod fixtures;
pub mod group_b_run_worker_loop;
pub mod group_c_rrt_integration;

use crate::generate_isolated_process_test;
use fixtures::controller_fn;

#[test]
fn test_run_all_directly() { run_all_restart_tests_sequentially(); }

generate_isolated_process_test!(
    /// Process-isolated test entry point.
    test_rrt_restart_in_isolated_process,
    controller_fn,
    run_all_restart_tests_sequentially,
    std::process::Stdio::null(),
    std::process::Stdio::piped(),
    std::process::Stdio::piped()
);

// Helper macro to reduce boilerplate of running tests and printing status.
macro_rules! run_tests_and_print_status {
    ($($test:ident),+ $(,)?) => {
        $(
            println!("Running {}...", stringify!($test));
            $test();
            println!("Done {}", stringify!($test));
        )+
    };
}

/// Dispatches all restart integration tests sequentially within a single
/// isolated child process.
fn run_all_restart_tests_sequentially() {
    use group_b_run_worker_loop::*;
    use group_c_rrt_integration::*;

    run_tests_and_print_status!(
        // Group B Step 5.0: Basic lifecycle.
        test_worker_stop_exits_cleanly,
        test_worker_continue_then_stop,
        test_domain_events_flow_through,
        // Group B Step 5.1: Restart success paths.
        test_single_restart_success,
        test_restart_no_delay_fast,
        test_events_before_and_after_restart,
        test_interrupt_handle_swap_on_restart,
        test_budget_resets_on_successful_create,
        // Group B Step 5.2: Restart exhaustion paths.
        test_restart_exhaustion,
        test_zero_budget_immediate_exhaustion,
        test_shutdown_event_payload,
        // Group B Step 5.3: Worker create() failure paths.
        test_create_failure_then_success,
        test_persistent_create_failure,
        // Group B Step 5.5: Backoff timing.
        test_backoff_delay_applied,
        test_delay_resets_after_successful_create,
        // Group B Step 5.6: Panic handling.
        test_panic_sends_shutdown_panic,
        test_panic_after_events,
        test_no_restart_after_panic,
        // Group C Step 6: RRT<TestWorker> integration tests.
        test_subscribe_spawns_thread,           
        test_subscribe_fast_path_reuse,
        test_subscribe_slow_path_after_termination,
        test_shutdown_received_by_subscriber,
        test_subscribe_after_panic_recovery,
    );
}
