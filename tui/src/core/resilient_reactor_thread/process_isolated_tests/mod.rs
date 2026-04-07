// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

pub mod fixtures;
pub mod group_b_run_worker_loop;
pub mod group_c_rrt_integration;

use crate::generate_isolated_process_test;
use fixtures::controller_fn;

generate_isolated_process_test!(
    /// Process-isolated test entry point.
    test_rrt_restart_in_isolated_process,
    controller_fn,
    run_all_restart_tests_sequentially,
    std::process::Stdio::null(),
    std::process::Stdio::piped(),
    std::process::Stdio::piped()
);

/// Dispatches all restart integration tests sequentially within a single
/// isolated child process.
fn run_all_restart_tests_sequentially() {
    // Group B Step 5.0: Basic lifecycle.
    group_b_run_worker_loop::test_worker_stop_exits_cleanly();
    group_b_run_worker_loop::test_worker_continue_then_stop();
    group_b_run_worker_loop::test_domain_events_flow_through();

    // Group B Step 5.1: Restart success paths.
    group_b_run_worker_loop::test_single_restart_success();
    group_b_run_worker_loop::test_restart_no_delay_fast();
    group_b_run_worker_loop::test_events_before_and_after_restart();
    group_b_run_worker_loop::test_interrupt_handle_swap_on_restart();
    group_b_run_worker_loop::test_budget_resets_on_successful_create();

    // Group B Step 5.2: Restart exhaustion paths.
    group_b_run_worker_loop::test_restart_exhaustion();
    group_b_run_worker_loop::test_zero_budget_immediate_exhaustion();
    group_b_run_worker_loop::test_shutdown_event_payload();

    // Group B Step 5.3: Worker create() failure paths.
    group_b_run_worker_loop::test_create_failure_then_success();
    group_b_run_worker_loop::test_persistent_create_failure();

    // Group B Step 5.5: Backoff timing.
    group_b_run_worker_loop::test_backoff_delay_applied();
    group_b_run_worker_loop::test_delay_resets_after_successful_create();

    // Group B Step 5.6: Panic handling.
    group_b_run_worker_loop::test_panic_sends_shutdown_panic();
    group_b_run_worker_loop::test_panic_after_events();
    group_b_run_worker_loop::test_no_restart_after_panic();

    // Group C Step 6: RRT<TestWorker> integration tests.
    group_c_rrt_integration::test_subscribe_spawns_thread();
    group_c_rrt_integration::test_subscribe_fast_path_reuse();
    group_c_rrt_integration::test_subscribe_slow_path_after_termination();
    group_c_rrt_integration::test_shutdown_received_by_subscriber();
    group_c_rrt_integration::test_subscribe_after_panic_recovery();
}
