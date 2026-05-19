// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Internal engine implementation for the Resilient Reactor Thread (RRT) pattern. See
//! [`run_worker_loop`] for details.

use super::{BroadcastSender, RRTEvent, RRTWorker, RestartPolicy, RetryLoopExhaustion,
            ShutdownReason, StopReason, TerminationGuard, ThreadLifecycleMonitor,
            ThreadState, InterruptHandle};
use crate::core::common::Continuation;
use std::{panic::{AssertUnwindSafe, catch_unwind},
          sync::Arc,
          time::Duration};

/// Runs the poll loop on the dedicated thread with restart policy support.
///
/// Called from the spawned dedicated thread. The loop handles three [`Continuation`]
/// variants:
///
/// - [`Continue`]: Call [`block_until_ready_then_dispatch()`] again.
/// - [`Stop`]: Always respected. Thread exits cleanly.
/// - [`Restart`]: Triggers the self-healing restart sequence (see below).
///
/// # Self-Healing Restart Sequence
///
/// When [`block_until_ready_then_dispatch()`] returns [`Continuation::Restart`], the
/// framework executes the following sequence:
///
/// 1. The framework acquires the [`shared_state`] lock, transitions from [`Running`] to
///    [`Restarting`] (consuming the old [`InterruptHandle`]), **and immediately
///    releases the lock to begin dropping and recreating OS resources**.
/// 2. The current [`RRTWorker`] is dropped and [`RAII`] cleanup releases OS resources.
/// 3. The framework sleeps for the configured delay (see [`RestartPolicy`]).
/// 4. [`RRTWorker::create_and_register_os_sources()`] is called to create a fresh
///    [`RRTWorker`] + [`RRTSoftwareInterrupt`] pair. The new [`RRTWorker`] allocates new
///    OS resources.
/// 5. The framework re-acquired the [`shared_state`] lock, transitions from
///    [`Restarting`] back to [`Running`] (moving the new [`InterruptHandle`] into the
///    state), and calls [`notify_all()`] so the next [`try_subscribe()`] call can cleanly
///    spawn a new thread.
/// 6. The poll loop resumes with the fresh [`RRTWorker`]. The restart budget resets.
///
/// If [`RRTWorker::create_and_register_os_sources()`] itself fails, the framework retries
/// on the pre-existing thread until success or budget exhaustion. If exhausted, the
/// thread exits and transitions to [`Stopped`].
///
/// # Panic Handling
///
/// The loop body is wrapped in [`catch_unwind`] to detect panics from
/// [`block_until_ready_then_dispatch()`]. If a panic is caught, the framework sends
/// [`RRTEvent::Shutdown(Panic)`] to notify subscribers, then exits the thread. No restart
/// is attempted - a panic signals a logic bug, not a transient resource issue.
/// Subscribers can call [`try_subscribe()`] to relaunch a fresh thread if appropriate.
///
/// See [`rrt_restart_pty_tests`] for a [`PTY`] integration test that exercises restart
/// cycles.
///
/// When the loop exits (normally or via panic), [`TerminationGuard::drop()`] runs,
/// transitioning the state to [`Stopped`] and calling [`notify_all()`] so the next
/// [`try_subscribe()`] call can cleanly spawn a new thread.
///
/// # Panics
///
/// Panics if the internal [`Mutex`] is poisoned (another thread panicked while
/// holding the state lock).
///
/// # Poison Safety
///
/// See the [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety] section
/// in the crate root documentation for details.
///
/// [`block_until_ready_then_dispatch()`]:
///     super::RRTWorker::block_until_ready_then_dispatch
/// [`catch_unwind`]: std::panic::catch_unwind
/// [`Continue`]: crate::core::common::Continuation::Continue
/// [`Mutex`]: std::sync::Mutex
/// [`notify_all()`]:
///     crate::core::resilient_reactor_thread::ThreadLifecycleMonitor::notify_all
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`RAII`]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
/// [`Restart`]: crate::core::common::Continuation::Restart
/// [`Restarting`]: super::ThreadState::Restarting
/// [`RestartPolicy`]: super::RestartPolicy
/// [`rrt_restart_pty_tests`]:
///     super::rrt_integration_tests::pty_test_production_factory_restart
/// [`RRTEvent::Shutdown(Panic)`]: super::ShutdownReason::Panic
/// [`RRTSoftwareInterrupt`]: super::RRTSoftwareInterrupt
/// [`RRTWorker::create_and_register_os_sources()`]:
///     super::RRTWorker::create_and_register_os_sources
/// [`Running`]: super::ThreadState::Running
/// [`shared_state`]: field@super::RRT::shared_state
/// [`Stop`]: crate::core::common::Continuation::Stop
/// [`Stopped`]: super::ThreadState::Stopped
/// [`TerminationGuard::drop()`]: super::TerminationGuard#method.drop
/// [`try_subscribe()`]: super::RRT::try_subscribe
/// [Terminal Restoration: Panic, Drop, and Mutex Poison-Safety]:
///     crate#terminal-restoration-panic-drop-and-mutex-poison-safety
/// [Thread Lifecycle]: super::RRT#thread-lifecycle
pub fn run_worker_loop<W: RRTWorker>(
    worker: W,
    sender: BroadcastSender<W::Event>,
    shared_state: Arc<ThreadLifecycleMonitor<W>>,
) {
    let _guard: TerminationGuard<W> = shared_state.clone().into();

    crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
        tracing::info!(message = "RRT: run_worker_loop() started.");
    });

    // Wrap worker in an Option so we can manually drop it at any time using take().
    let mut maybe_worker = Some(worker);

    let policy = W::restart_policy();
    let mut restart_count: u8 = 0;
    let mut current_delay = policy.initial_delay;

    // Clone sender before the closure so it remains available for panic notification.
    let sender_for_panic = sender.clone();

    // Safety: AssertUnwindSafe is sound here. The closure captures &mut maybe_worker,
    // &sender, &shared_state, &policy, &mut restart_count, and &mut current_delay.
    // After catching a panic we don't touch any of the captured loop state - we only
    // send a Shutdown(Panic) notification via the pre-cloned sender_for_panic and
    // then exit. No potentially-corrupted state is observed or reused.
    let result = catch_unwind(AssertUnwindSafe(|| {
        'worker_loop: loop {
            // 1. Framework-initiated Stop: Check if any subscribers are left.
            {
                let state_guard = shared_state.lock();

                let (should_stop, state_guard) =
                    shared_state.read_state(state_guard, |state| {
                        matches!(state, ThreadState::Running(_))
                            && sender.receiver_count() == 0
                    });

                if should_stop {
                    crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                        tracing::info!(message = "RRT: Stopped (all receivers dropped).");
                    });
                    let state_guard = shared_state.set_state(
                        state_guard,
                        ThreadState::Stopping(StopReason::ZeroReceivers),
                    );
                    shared_state.notify_all();
                    drop(state_guard);
                    break 'worker_loop;
                }

                drop(state_guard);
            }

            // 2. Worker-initiated Stop: Handle domain stops (EOF, fatal error).
            let Some(worker) = maybe_worker.as_mut() else {
                unreachable!(
                    "Internal RRT Error: Polling loop started with no worker. \
                     This indicates the restart state machine failed to re-populate \
                     the worker before returning to the top of the loop."
                );
            };

            match worker.block_until_ready_then_dispatch(&sender) {
                Continuation::Continue => {}

                Continuation::Stop => {
                    crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                        tracing::info!(message = "RRT: Stopped (worker requested).");
                    });
                    let state_guard = shared_state.lock();
                    drop(shared_state.set_state(
                        state_guard,
                        ThreadState::Stopping(StopReason::WorkerRequested),
                    ));
                    shared_state.notify_all();
                    break;
                }

                Continuation::Restart => {
                    crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                        tracing::info!(message = "RRT: Restart requested by worker.");
                    });
                    // Transition Running → Restarting (consumes old interrupt handle).
                    let state_guard = shared_state.lock();
                    drop(shared_state.set_state(state_guard, ThreadState::Restarting));
                    shared_state.notify_all();
                    // Lock released - safe to drop/recreate OS resources.

                    // Drop the old worker before applying delays or creating new
                    // sources. This ensures OS handles are released promptly.
                    drop(maybe_worker.take());

                    // Inner retry loop: handles both "restart worker" and
                    // "W::create_and_register_os_sources() itself failed" cases.
                    let outcome = perform_restart_retry_loop(
                        &policy,
                        &mut restart_count,
                        &mut current_delay,
                        &sender,
                        &shared_state,
                        &mut maybe_worker,
                    );

                    // If policy exhausted, exit thread.
                    if outcome == RetryLoopExhaustion::Yes {
                        break 'worker_loop;
                    }
                }
            }
        }
    }));

    // If the worker panicked, notify subscribers so they can take corrective action
    // (e.g., call try_subscribe() to relaunch a fresh thread).
    if let Err(error) = result {
        crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
            tracing::error!(message = "RRT: Worker thread panicked.", ?error);
        });
        drop(sender_for_panic.send(RRTEvent::Shutdown(ShutdownReason::Panic)));
    }

    // Explicitly drop worker before RAII guard drops.
    maybe_worker.take();

    crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
        tracing::info!(message = "RRT: run_worker_loop() exiting.");
    });

    // _guard dropped here, transitioning state to Stopped + notify_all.
}

/// Inner retry loop for RRT restarts. Handles both "restart worker" and
/// "`W::create_and_register_os_sources()` itself failed" cases.
///
/// Returns [`RetryLoopExhaustion::Yes`] if the budget is exhausted, otherwise
/// [`RetryLoopExhaustion::No`].
pub fn perform_restart_retry_loop<W: RRTWorker>(
    policy: &RestartPolicy,
    restart_count: &mut u8,
    current_delay: &mut Option<Duration>,
    sender: &BroadcastSender<W::Event>,
    shared_state: &Arc<ThreadLifecycleMonitor<W>>,
    maybe_worker: &mut Option<W>,
) -> RetryLoopExhaustion {
    loop {
        *restart_count += 1;
        if *restart_count > policy.max_restarts {
            crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                tracing::error!(
                    message = "RRT: Restart budget exhausted, thread exiting.",
                    attempts = *restart_count
                );
            });
            drop(sender.send(RRTEvent::Shutdown(
                ShutdownReason::RestartPolicyExhausted {
                    attempts: *restart_count,
                },
            )));
            return RetryLoopExhaustion::Yes;
        }

        // Apply delay before attempting restart.
        if let Some(delay) = *current_delay {
            crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                tracing::info!(
                    message = "RRT: Restarting (retry with delay).",
                    attempt = *restart_count,
                    ?delay
                );
            });
            std::thread::sleep(delay);
            *current_delay = advance_backoff_delay(delay, policy);
        } else {
            crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                tracing::info!(
                    message = "RRT: Restarting (retry without delay).",
                    attempt = *restart_count
                );
            });
        }

        match W::create_and_register_os_sources() {
            Ok((new_worker, new_interrupt)) => {
                crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                    tracing::info!(message = "RRT: Restart successful.");
                });
                // Re-populate the worker and restore the loop invariant.
                *maybe_worker = Some(new_worker);
                // Transition Restarting → Running (installs new interrupt handle).
                let state_guard = shared_state.lock();
                drop(shared_state.set_state(
                    state_guard,
                    ThreadState::Running(InterruptHandle::new(new_interrupt)),
                ));
                shared_state.notify_all();
                // Reset budget so the fresh worker gets a full allowance
                // for future incidents.
                *restart_count = 0;
                *current_delay = policy.initial_delay;
                return RetryLoopExhaustion::No; // Success - back to outer poll loop.
            }
            Err(error) => {
                crate::DEBUG_TUI_SHOW_RESILIENT_REACTOR_THREAD.then(|| {
                    tracing::error!(
                        message = "RRT: Restart failed (could not recreate sources).",
                        ?error,
                        attempt = *restart_count
                    );
                });
            }
        }
        // Err: retry create with next delay.
    }
}

/// Advances the backoff delay for the next restart attempt.
#[must_use]
pub fn advance_backoff_delay(
    current: Duration,
    policy: &RestartPolicy,
) -> Option<Duration> {
    match policy.backoff_multiplier {
        Some(multiplier) => {
            let next = current.mul_f64(multiplier);
            Some(match policy.max_delay {
                Some(max) => next.min(max),
                None => next,
            })
        }
        None => Some(current), // No backoff - constant delay.
    }
}
