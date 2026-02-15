<!-- cspell:words EBADF EINTR mult antipattern -->
<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Why simple test doubles, not real mio?](#why-simple-test-doubles-not-real-mio)
  - [Test architecture](#test-architecture)
  - [Files to create/modify](#files-to-createmodify)
- [Implementation plan](#implementation-plan)
  - [Step 0: Make `advance_backoff_delay` testable [COMPLETE]](#step-0-make-advance_backoff_delay-testable-complete)
  - [Step 1: Register test module in `mod.rs` [COMPLETE]](#step-1-register-test-module-in-modrs-complete)
  - [Step 2: Create test types in `rrt_restart_tests.rs` [COMPLETE]](#step-2-create-test-types-in-rrt_restart_testsrs-complete)
    - [Step 2.0: TestEvent](#step-20-testevent)
    - [Step 2.1: TestWaker](#step-21-testwaker)
    - [Step 2.2: TestWorker](#step-22-testworker)
    - [Step 2.3: TestFactory with static state](#step-23-testfactory-with-static-state)
    - [Step 2.4: Helper functions](#step-24-helper-functions)
  - [Step 3: Process-isolated test coordinator [COMPLETE]](#step-3-process-isolated-test-coordinator-complete)
  - [Step 4: Implement `advance_backoff_delay` pure function tests (Group A) [COMPLETE]](#step-4-implement-advance_backoff_delay-pure-function-tests-group-a-complete)
  - [Step 5: Implement `run_worker_loop` tests with mpsc channels (Group B) [COMPLETE]](#step-5-implement-run_worker_loop-tests-with-mpsc-channels-group-b-complete)
    - [Step 5.0: Basic lifecycle tests](#step-50-basic-lifecycle-tests)
    - [Step 5.1: Restart success paths](#step-51-restart-success-paths)
    - [Step 5.2: Restart exhaustion paths](#step-52-restart-exhaustion-paths)
    - [Step 5.3: Factory create() failure paths](#step-53-factory-create-failure-paths)
    - [Step 5.4: TerminationGuard cleanup](#step-54-terminationguard-cleanup)
    - [Step 5.5: Backoff timing](#step-55-backoff-timing)
    - [Step 5.6: Panic handling](#step-56-panic-handling)
    - [Step 5.7: Production poll error path [COMPLETE]](#step-57-production-poll-error-path-complete)
  - [Step 6: Implement `RRT<TestFactory>` integration tests (Group C) [COMPLETE]](#step-6-implement-rrttestfactory-integration-tests-group-c-complete)
  - [Step 7: PTY test for production factory `create()` ordering (Group D) [COMPLETE]](#step-7-pty-test-for-production-factory-create-ordering-group-d-complete)
    - [Step 7.0: RestartTestFactory wrapper](#step-70-restarttestfactory-wrapper)
    - [Step 7.1: RestartTestWorker wrapper](#step-71-restarttestworker-wrapper)
    - [Step 7.2: PTY test using `generate_pty_test!`](#step-72-pty-test-using-generate_pty_test)
  - [Step 8: Run tests and verify [COMPLETE]](#step-8-run-tests-and-verify-complete)
  - [Synchronization strategy](#synchronization-strategy)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

The RRT (Resilient Reactor Thread) framework recently gained self-healing restart capabilities
(Steps 0-5 in `task/fix-rrt-restart.md`). However, there are **zero tests** for the restart logic
itself. The existing RRT tests are PTY integration tests that exercise basic lifecycle
(spawn/reuse/shutdown) but never trigger `Continuation::Restart` or `RestartPolicy`.

This task adds comprehensive test coverage for all restart codepaths. Groups A/B/C use lightweight
`mpsc`-channel-based test doubles, while Groups D (Step 5.7) and E (Step 7) exercise the real
production `mio::Poll`/`MioPollWorkerFactory` stack.

## Why simple test doubles, not real mio?

The original plan called for real `mio::Poll` + `mio::Waker` + Unix socket pairs in the test worker.
This was **over-testing**: edge-triggered epoll semantics introduced deadlocks that had nothing to
do with the restart logic under test.

The restart framework (budget tracking, backoff delays, `TerminationGuard` cleanup, panic recovery)
only cares about what `poll_once()` **returns** (`Continue`, `Stop`, `Restart`) - not how it blocks
internally. Therefore:

- **Groups A/B/C**: `TestWorker` blocks on `mpsc::Receiver::recv()` and `TestWaker` is a no-op with
  a unique ID. This makes tests deterministic, fast, and free of OS-level edge cases.
- **Step 5.7**: Uses the real `MioPollWorkerFactory::create()` with a corrupted epoll fd to test the
  production error path. This is the right place to exercise real OS resources.
- **Step 7 (Group D)**: Full PTY test with the real production stack end-to-end. This catches bugs
  in the production factory's `create()` ordering, fd leaks, and stdin/signal registration.

**Lesson learned**: Test doubles should be as simple as possible. Reserve real OS resources for
tests that specifically exercise OS-level behavior. Using production-level complexity in a mock is a
classic over-testing antipattern.

## Test architecture

**TestFactory** - a test-specific `RRTFactory` implementation that:

- Uses `std::sync::mpsc` channels for control (not mio or Unix sockets)
- `TestWorker` blocks on `mpsc::Receiver::recv()`, reads a command byte, returns the corresponding
  `Continuation`
- `TestWaker` is a no-op with a unique ID (for identity comparison on waker swap)
- Test thread holds the `mpsc::Sender` ends and sends commands: `b'c'` (Continue), `b'r'` (Restart),
  `b's'` (Stop), `b'e'` (send event + Continue), `b'p'` (panic inside `catch_unwind`)
- Factory state stored in `static Mutex<...>` - safe because each test runs in an isolated process
  (same pattern as `test_all_rendered_output_in_isolated_process` in
  `text_operations_rendered.rs:344`)

**Production error test** (Step 5.7) - uses real `MioPollWorkerFactory::create()` with a corrupted
epoll fd to exercise the non-EINTR error path in `MioPollWorker::poll_once()`.

**PTY integration test** (Step 7) - uses `generate_pty_test!` to test that the production
`MioPollWorkerFactory::create()` works correctly when called repeatedly (simulating restart). This
covers the one gap the channel-based tests miss: a bug in the production factory's specific
`create()` ordering (e.g., registering sources before the waker, or leaking fds between restarts).

## Files to create/modify

| File                                                                   | Change                                           |
| :--------------------------------------------------------------------- | :----------------------------------------------- |
| `tui/src/core/resilient_reactor_thread/tests/mod.rs`                   | **NEW** - re-exports test submodules             |
| `tui/src/core/resilient_reactor_thread/tests/rrt_restart_tests.rs`     | **NEW** - test types + Groups A/B/C tests        |
| `tui/src/core/resilient_reactor_thread/tests/rrt_restart_pty_tests.rs` | **NEW** - PTY test for production factory create |
| `tui/src/core/resilient_reactor_thread/mod.rs`                         | Add `#[cfg(test)] mod tests;`                    |
| `tui/src/core/resilient_reactor_thread/rrt.rs`                         | `advance_backoff_delay` -> `pub(crate)`          |

# Implementation plan

## Step 0: Make `advance_backoff_delay` testable [COMPLETE]

Changed visibility from private to `pub` (not `pub(crate)`) in `rrt.rs:648`. With barrel exports,
`pub` is safe since the function isn't re-exported in `mod.rs`.

## Step 1: Register test module in `mod.rs` [COMPLETE]

Add at the bottom of `mod.rs`:

```rust
#[cfg(test)]
mod tests;
```

And create `tests/mod.rs`:

```rust
mod rrt_restart_tests;
mod rrt_restart_pty_tests;
```

## Step 2: Create test types in `rrt_restart_tests.rs` [COMPLETE]

All test types use real OS resources. Unix-only (`#[cfg(unix)]` is implicit since mio's `SourceFd`
is unix-only).

### Step 2.0: TestEvent

```rust
#[derive(Debug, Clone, PartialEq)]
struct TestEvent(u32);
```

### Step 2.1: TestWaker

No-op waker with a unique ID for identity comparison (verifying waker swap on restart).

```rust
static NEXT_WAKER_ID: AtomicU32 = AtomicU32::new(0);

struct TestWaker { id: u32 }

impl RRTWaker for TestWaker {
    fn wake(&self) -> std::io::Result<()> { Ok(()) }
}
```

### Step 2.2: TestWorker

Blocks on `mpsc::Receiver::recv()` - each call to `poll_once()` reads one command byte. No mio, no
Unix sockets, no OS resources. This keeps tests deterministic and avoids edge-triggered epoll
semantics that caused deadlocks in the original Unix socket approach.

```rust
struct TestWorker {
    cmd_rx: mpsc::Receiver<u8>,
    event_counter: u32,
}

impl RRTWorker for TestWorker {
    type Event = TestEvent;

    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation {
        match self.cmd_rx.recv() {
            Ok(b'c') => Continuation::Continue,
            Ok(b'r') => Continuation::Restart,
            Ok(b's') => Continuation::Stop,
            Ok(b'e') => {
                let id = self.event_counter;
                self.event_counter += 1;
                drop(tx.send(RRTEvent::Worker(TestEvent(id))));
                Continuation::Continue
            }
            Ok(b'p') => panic!("TestWorker: deliberate panic for testing"),
            _ => Continuation::Stop,
        }
    }
}
```

### Step 2.3: TestFactory with static state

Uses `static Mutex<Option<TestFactoryState>>` because `run_worker_loop` spawns on a different
thread. Safe without `#[serial]` because all tests run inside a process-isolated coordinator (see
`text_operations_rendered.rs:344` for the pattern). Wrapped in `Option` for explicit
initialization/teardown.

```rust
static TEST_FACTORY_STATE: Mutex<Option<TestFactoryState>> = Mutex::new(None);

struct TestFactoryState {
    create_results: VecDeque<Result<(TestWorker, TestWaker), Report>>,
    create_count: u32,
    restart_policy: RestartPolicy,
    create_notify: Option<mpsc::Sender<()>>, // signals test when create() called
}

struct TestFactory;

impl RRTFactory for TestFactory {
    type Event = TestEvent;
    type Worker = TestWorker;
    type Waker = TestWaker;

    fn create() -> Result<(Self::Worker, Self::Waker), Report> {
        let mut guard = TEST_FACTORY_STATE.lock().unwrap();
        let state = guard.as_mut().expect("TEST_FACTORY_STATE not initialized");
        state.create_count += 1;
        if let Some(ref notify_tx) = state.create_notify {
            notify_tx.send(()).ok();
        }
        state.create_results.pop_front()
            .unwrap_or_else(|| Err(miette::miette!("TestFactory: no create results")))
    }

    fn restart_policy() -> RestartPolicy {
        TEST_FACTORY_STATE.lock().unwrap()
            .as_ref().expect("TEST_FACTORY_STATE not initialized")
            .restart_policy.clone()
    }
}
```

### Step 2.4: Helper functions

```rust
/// Creates (TestWorker, TestWaker, cmd_tx). The test thread uses cmd_tx to send commands.
fn create_test_resources() -> (TestWorker, TestWaker, mpsc::Sender<u8>) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let worker = TestWorker { cmd_rx, event_counter: 0 };
    let waker = TestWaker { id: NEXT_WAKER_ID.fetch_add(1, Ordering::Relaxed) };
    (worker, waker, cmd_tx)
}

/// Convenience: creates resources wrapped in Ok() for factory pre-loading.
fn create_ok_result() -> (Result<(TestWorker, TestWaker), Report>, mpsc::Sender<u8>) { ... }

/// Sets up TEST_FACTORY_STATE with create results and policy.
/// Returns (create_notify_rx, Vec<cmd_senders>).
fn setup_factory(
    results_and_senders: Vec<(Result<(TestWorker, TestWaker), Report>, Option<mpsc::Sender<u8>>)>,
    policy: RestartPolicy,
) -> (mpsc::Receiver<()>, Vec<mpsc::Sender<u8>>) { ... }

/// Clears TEST_FACTORY_STATE. Called after every test.
fn teardown_factory() { ... }

/// Reads create_count from the factory state.
fn get_create_count() -> u32 { ... }

/// Send a command byte to the worker via its cmd_tx.
fn send_cmd(cmd_tx: &mpsc::Sender<u8>, cmd: u8) { ... }

/// RestartPolicy with no delay (fast tests).
fn no_delay_policy(max_restarts: u8) -> RestartPolicy { ... }

/// Spawn run_worker_loop on a named thread, return JoinHandle.
fn spawn_worker_loop(
    worker: TestWorker,
    tx: Sender<RRTEvent<TestEvent>>,
    liveness: Arc<RRTLiveness>,
    shared_waker: Arc<Mutex<Option<TestWaker>>>,
) -> std::thread::JoinHandle<()> { ... }
```

## Step 3: Process-isolated test coordinator [COMPLETE]

All Groups B and C tests run inside a single process-isolated coordinator (same pattern as
`text_operations_rendered.rs:344`). This eliminates `#[serial_test::serial]` - each subprocess gets
a fresh address space with clean static state.

```rust
#[test]
fn test_rrt_restart_in_isolated_process() {
    crate::suppress_wer_dialogs();
    if std::env::var("ISOLATED_RRT_RESTART_TEST").is_ok() {
        run_all_restart_tests_sequentially();
        std::process::exit(0);
    }

    let mut cmd = crate::new_isolated_test_command();
    cmd.env("ISOLATED_RRT_RESTART_TEST", "1")
        .env("RUST_BACKTRACE", "1")
        .args(["--test-threads", "1", "test_rrt_restart_in_isolated_process"]);

    let output = cmd.output().expect("Failed to run isolated test");
    // Check exit status, stderr for panics...
}

fn run_all_restart_tests_sequentially() {
    // Group B: run_worker_loop tests (each resets TEST_FACTORY_STATE)
    test_worker_stop_exits_cleanly();
    test_single_restart_success();
    test_budget_resets_on_successful_create();
    test_delay_resets_after_successful_create();
    // ... all remaining Group B Steps 5.0-5.5 tests ...

    // Group B Step 5.6: Panic handling tests
    test_panic_sends_shutdown_panic();
    test_panic_after_events();
    test_guard_clears_waker_on_panic();
    test_guard_marks_terminated_on_panic();
    test_no_restart_after_panic();

    // Group C: RRT<TestFactory> integration tests
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        test_subscribe_spawns_thread().await;
        // ... all Group C tests ...
        test_subscribe_after_panic_recovery().await;
    });
}
```

Group A tests (`advance_backoff_delay`) are plain `#[test]` functions outside the coordinator since
they're pure functions with no shared state.

## Step 4: Implement `advance_backoff_delay` pure function tests (Group A) [COMPLETE]

These are plain `#[test]` functions - no mio, no threads, no process isolation needed.

| Test                                | Input                      | Assert                     |
| :---------------------------------- | :------------------------- | :------------------------- |
| `test_backoff_exponential_doubling` | 100ms, mult=2.0, no cap    | Output = 200ms             |
| `test_backoff_max_delay_capping`    | 200ms, mult=2.0, cap=300ms | Output = 300ms (not 400ms) |
| `test_backoff_constant_delay`       | 100ms, mult=None           | Output = 100ms (unchanged) |
| `test_backoff_unbounded_growth`     | 1s, mult=3.0, no cap       | Output = 3s                |

## Step 5: Implement `run_worker_loop` tests with mpsc channels (Group B) [COMPLETE]

Each test is a plain `fn` called by the process-isolated coordinator. Each test spawns
`run_worker_loop` on a thread and communicates via `mpsc` channels (`cmd_tx`/`cmd_rx`).

### Step 5.0: Basic lifecycle tests

| Test                              | Commands sent  | Verifies                                          |
| :-------------------------------- | :------------- | :------------------------------------------------ |
| `test_worker_stop_exits_cleanly`  | `[s]`          | Liveness=Terminated, waker=None                   |
| `test_worker_continue_then_stop`  | `[e, e, e, s]` | 3 events received, then clean exit                |
| `test_domain_events_flow_through` | `[e, e, s]`    | Receiver gets TestEvent(0), TestEvent(1) in order |

### Step 5.1: Restart success paths

| Test                                      | Scenario                               | Verifies                                                          |
| :---------------------------------------- | :------------------------------------- | :---------------------------------------------------------------- |
| `test_single_restart_success`             | Worker1: `r`, Worker2: `s`             | create_count=1, no shutdown, clean exit                           |
| `test_restart_no_delay_fast`              | delay=None, Worker1: `r`, Worker2: `s` | Elapsed < 100ms                                                   |
| `test_events_before_and_after_restart`    | W1: `e, r`, W2: `e, s`                 | Both events received in order                                     |
| `test_waker_swap_on_restart`              | W1: `r`, W2: `s`                       | Shared waker points to new waker after                            |
| `test_budget_resets_on_successful_create` | max=1, W1:`r`, W2:`r`, W3:`r`, W4:`s`  | create_count=3, no shutdown, clean exit (budget resets each time) |

`test_budget_resets_on_successful_create` is critical: with `max_restarts=1`, without the
`restart_count = 0` reset at `rrt.rs:599`, W2's Restart would exhaust the policy (`restart_count`
would reach 2, exceeding `max_restarts=1`). With the reset, all 3 restarts succeed because each
fresh worker gets a full budget. Pre-load factory with `[Ok(W2), Ok(W3), Ok(W4)]`.

### Step 5.2: Restart exhaustion paths

| Test                                    | Scenario                      | Verifies                                                       |
| :-------------------------------------- | :---------------------------- | :------------------------------------------------------------- |
| `test_restart_exhaustion`               | max=2, W1:`r`, W2:`r`, W3:`r` | Shutdown event with attempts=3                                 |
| `test_zero_budget_immediate_exhaustion` | max=0, W1:`r`                 | Shutdown(attempts=1), create_count=0                           |
| `test_shutdown_event_payload`           | max=0, W1:`r`                 | Exact `ShutdownReason::RestartPolicyExhausted { attempts: 1 }` |

**How `test_restart_exhaustion` works**: Pre-load factory with `[Ok(W2), Ok(W3)]` only (2 restart
results). W1 and W2 restart successfully (budget resets to 0 each time). When W3 returns Restart,
the factory is empty and returns `Err` for every `create()` call. These repeated `create()` failures
consume the restart budget: `restart_count` climbs 1 → 2 → 3, exceeding `max_restarts=2`. The
exhaustion is caused by `create()` failures, not by workers directly hitting the limit - this is the
only way exhaustion can happen since every successful `create()` resets the budget.

### Step 5.3: Factory create() failure paths

| Test                               | Scenario                        | Verifies                             |
| :--------------------------------- | :------------------------------ | :----------------------------------- |
| `test_create_failure_then_success` | max=3, create:[Err, Ok(W2:`s`)] | create_count=2, no shutdown          |
| `test_persistent_create_failure`   | max=3, create:[Err, Err, Err]   | Shutdown(attempts=4), create_count=3 |

### Step 5.4: TerminationGuard cleanup

| Test                                    | Scenario      | Verifies                            |
| :-------------------------------------- | :------------ | :---------------------------------- |
| `test_guard_clears_waker_on_stop`       | W1:`s`        | shared_waker is None after exit     |
| `test_guard_marks_terminated_on_stop`   | W1:`s`        | liveness.is_running() == Terminated |
| `test_guard_clears_waker_on_exhaustion` | max=0, W1:`r` | Waker is None after exhaustion exit |

### Step 5.5: Backoff timing

| Test                                        | Policy                           | Verifies                                |
| :------------------------------------------ | :------------------------------- | :-------------------------------------- |
| `test_backoff_delay_applied`                | max=1, delay=50ms                | Elapsed >= 50ms                         |
| `test_backoff_delay_capping_in_loop`        | max=2, delay=20ms, 10x, cap=25ms | Elapsed >= 45ms and < 250ms             |
| `test_delay_resets_after_successful_create` | max=3, delay=50ms, mult=2.0      | Second restart delay is 50ms, not 200ms |

**How `test_delay_resets_after_successful_create` works**: Pre-load factory with
`[Err, Ok(W2), Ok(W3)]`. W1 returns Restart:

```text
restart_count=1 → sleep(50ms) → create() Err
restart_count=2 → sleep(100ms) → create() Ok(W2) → restart_count=0, delay resets to 50ms
```

W2 returns Restart:

```text
restart_count=1 → sleep(50ms) → create() Ok(W3) → restart_count=0
```

W3 returns Stop. Total elapsed should be ~250ms (50+100+50), not ~350ms (50+100+200). The test
verifies the delay is between 200ms and 300ms (generous bounds for timing), proving that
`current_delay = policy.initial_delay` at `rrt.rs:600` fires after each successful `create()`.

### Step 5.6: Panic handling

These tests verify the `catch_unwind(AssertUnwindSafe(...))` wrapper around the poll loop in
`run_worker_loop()`. When a worker panics inside `poll_once()`, the framework:

1. Catches the panic (no crash)
2. Sends `RRTEvent::Shutdown(ShutdownReason::Panic)` via the pre-cloned `tx_for_panic`
3. Does **not** attempt any restart (unlike `Restart`, which retries up to `max_restarts`)
4. `TerminationGuard` still runs (it lives outside the `catch_unwind` boundary), clearing the waker
   and marking liveness as terminated

The `b'p'` command triggers `panic!()` inside `TestWorker::poll_once()`, which is inside the
`catch_unwind` closure.

| Test                                   | Commands sent | Verifies                                                               |
| :------------------------------------- | :------------ | :--------------------------------------------------------------------- |
| `test_panic_sends_shutdown_panic`      | W1: `p`       | Subscriber receives `Shutdown(ShutdownReason::Panic)`                  |
| `test_panic_after_events`              | W1: `e, p`    | Domain event received first, then `Shutdown(Panic)`                    |
| `test_guard_clears_waker_on_panic`     | W1: `p`       | `shared_waker` is `None` after panic exit                              |
| `test_guard_marks_terminated_on_panic` | W1: `p`       | `liveness.is_running()` == `Terminated`                                |
| `test_no_restart_after_panic`          | W1: `p`       | `create_count` == 0 (factory never called for restart), max_restarts=3 |

**Key difference from `Restart`**: When `Restart` is returned, the framework enters the inner retry
loop and may call `F::create()` up to `max_restarts` times. When a panic occurs, the `catch_unwind`
boundary is _outside_ the retry loop - control jumps past it entirely. The panic notification uses
`tx_for_panic` (cloned before the closure) because the original `tx` may be in a corrupted state.

### Step 5.7: Production poll error path [COMPLETE]

This test exercises the **production** `MioPollWorker::poll_once()` error handling at
`mio_poll_worker.rs:106-115` - the non-EINTR codepath that sends `StdinEvent::Error` and returns
`Continuation::Restart`. Unlike all other Group B tests (which use `TestWorker`), this test creates
a real `MioPollWorker` via `MioPollWorkerFactory::create()` and corrupts its underlying epoll fd to
trigger the error path.

**Approach**: Uses `generate_pty_test!` to run the test in a PTY subprocess. The PTY provides a real
terminal stdin (fd 0 on the controlled end), which allows `MioPollWorkerFactory::create()` to
register stdin with epoll successfully. After construction, `mio::Poll`'s epoll fd is corrupted via
`OwnedFd::from_raw_fd()` + drop, making the next `poll.poll()` call fail with `EBADF` (not `EINTR`),
which hits the exact codepath under test.

This avoids adding `libc` as a dependency - `std::os::unix::io::OwnedFd` handles the close.

The controlled process creates a real `MioPollWorker` via factory, corrupts the fd, calls
`poll_once()`, asserts `Continuation::Restart` and `StdinEvent::Error`, then signals success. The
controller waits for signals and drains the PTY. Uses `drain_pty_and_wait()` to prevent macOS PTY
buffer deadlocks.

| Test                                                  | Approach                                        | Verifies                                                |
| :---------------------------------------------------- | :---------------------------------------------- | :------------------------------------------------------ |
| `test_production_poll_error_sends_error_and_restarts` | PTY + real factory, close epoll fd, `poll_once` | Returns `Restart`, channel receives `StdinEvent::Error` |

## Step 6: Implement `RRT<TestFactory>` integration tests (Group C) [COMPLETE]

Each test is an `async fn` called from the coordinator's tokio runtime. Use
`RRT::<TestFactory>::new()` (not the production SINGLETON). Tests exercise `subscribe()` with real
thread spawning.

| Test                                         | Verifies                                                              |
| :------------------------------------------- | :-------------------------------------------------------------------- |
| `test_subscribe_spawns_thread`               | subscribe() -> Running, send `s` -> Terminated                        |
| `test_subscribe_fast_path_reuse`             | Two subscribes -> same generation, receiver_count=2                   |
| `test_subscribe_slow_path_after_termination` | Subscribe -> terminate -> subscribe -> new generation                 |
| `test_shutdown_received_by_subscriber`       | max=0, worker returns Restart -> subscriber gets Shutdown event       |
| `test_subscribe_after_panic_recovery`        | Subscribe -> panic -> terminated -> subscribe again -> new generation |

## Step 7: PTY test for production factory `create()` ordering (Group D) [COMPLETE]

A single PTY test using `generate_pty_test!` that exercises the real
`MioPollWorkerFactory::create()` multiple times in sequence - simulating what happens during
restart. This catches bugs that pipe-based tests miss: incorrect resource registration ordering, fd
leaks between create calls, or stdin/signal handler issues specific to the production factory.

File: `tests/rrt_restart_pty_tests.rs`

### Step 7.0: `create_count` module and `RestartTestFactory`

The `create_count` inner module encapsulates an `AtomicU32` counter behind a clean API
(`reset()`, `increment()`, `get()`, `spin_wait_until()`), hiding the raw atomic and `Ordering`
details.

`RestartTestFactory` wraps `MioPollWorkerFactory` and increments the counter on each `create()`.

```rust
mod create_count {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNT: AtomicU32 = AtomicU32::new(0);
    pub fn reset() { COUNT.store(0, Ordering::SeqCst); }
    pub fn increment() { COUNT.fetch_add(1, Ordering::SeqCst); }
    pub fn get() -> u32 { COUNT.load(Ordering::SeqCst) }
    pub fn spin_wait_until(target: u32) { /* spin-wait with 5s timeout */ }
}

struct RestartTestFactory;

impl RRTFactory for RestartTestFactory {
    type Event = PollerEvent;
    type Worker = RestartTestWorker;
    type Waker = MioPollWaker;

    fn create() -> Result<(Self::Worker, Self::Waker), Report> {
        create_count::increment();
        let (inner_worker, waker) = MioPollWorkerFactory::create()?;
        Ok((RestartTestWorker { inner: inner_worker, poll_count: 0 }, waker))
    }

    fn restart_policy() -> RestartPolicy {
        RestartPolicy { max_restarts: 3, initial_delay: None,
                        backoff_multiplier: None, max_delay: None }
    }
}
```

### Step 7.1: RestartTestWorker wrapper

Delegates the first `poll_once()` to the real `MioPollWorker` (blocks on `epoll` until a
keystroke arrives), then on the second call returns `Restart` or `Stop` based on total create
count:

```rust
struct RestartTestWorker {
    inner: MioPollWorker,
    poll_count: u32,
}

impl RRTWorker for RestartTestWorker {
    type Event = PollerEvent;

    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation {
        self.poll_count += 1;
        if self.poll_count == 1 {
            self.inner.poll_once(tx) // blocks until keystroke
        } else {
            if create_count::get() < 3 { Continuation::Restart }
            else { Continuation::Stop }
        }
    }
}
```

### Step 7.2: PTY test using `generate_pty_test!`

Uses `mode: PtyTestMode::Raw` - this was the bug fix. The original test hung because cooked mode
buffers keystrokes until Enter, so `poll.poll()` never saw single-character keystrokes. The
`PtyTestMode` macro parameter was added to `generate_pty_test!` as part of this work.

```rust
generate_pty_test! {
    test_fn: test_production_factory_restart_cycle,
    controller: factory_restart_controller,
    controlled: factory_restart_controlled,
    mode: PtyTestMode::Raw,
}
```

**Controlled process** (uses `RRT::subscribe()` instead of manual thread management):

1. `create_count::reset()`
2. Create `RRT::<RestartTestFactory>::new()` and `subscribe()` (spawns worker thread)
3. Print `SEND_KEY` - controller sends keystroke to unblock `poll.poll()`
4. `create_count::spin_wait_until(2)` - wait for restart, then signal for next keystroke
5. `create_count::spin_wait_until(3)` - wait for second restart, signal again
6. Spin-wait on `rrt.is_thread_running() != LivenessState::Terminated`
7. Assert `create_count::get() == 3` (initial + 2 restarts)
8. Print `FACTORY_RESTART_PASSED`, then `exit(0)`

**Controller process**:

1. Wait for `FACTORY_RESTART_READY`
2. For each of 3 `SEND_KEY` signals, write one keystroke (`b"x"`) to PTY
3. Wait for `FACTORY_RESTART_PASSED`
4. `drain_pty_and_wait()`

This test verifies:

- `MioPollWorkerFactory::create()` works correctly 3 times in sequence
- Each restarted worker can actually poll stdin and process events
- No fd leaks or stale epoll state between create/drop cycles
- Production waker correctly couples to new Poll registry each time

## Step 8: Run tests and verify [COMPLETE]

```bash
./check.fish --full  # All checks pass: check, build, clippy, tests (2682), doctests, docs, windows
```

## Synchronization strategy

| Category                  | Threading                           | Isolation                      | Sync mechanism                          |
| :------------------------ | :---------------------------------- | :----------------------------- | :-------------------------------------- |
| Group A (pure fn)         | Single thread                       | Stateless - plain `#[test]`    | None                                    |
| Group B (run_worker_loop) | Test thread + worker thread         | Process isolation (subprocess) | `create_notify` channel + mpsc commands |
| Group C (RRT integration) | Test thread + spawned worker thread | Process isolation (subprocess) | `create_notify` + `is_thread_running()` |
| Group D (PTY)             | Controller + controlled process     | PTY process isolation          | `create_count::spin_wait_until()` + PTY stdout markers |

**Process isolation pattern** (from `text_operations_rendered.rs:344`): A single `#[test]`
coordinator spawns itself as a subprocess with an env var. The subprocess runs all Groups B+C tests
sequentially with exclusive access to static state. No `#[serial]` needed.

**Key sync pattern**: After sending `b'r'` (restart) through `cmd_tx`, the test waits on
`create_notify_rx.recv()` before sending commands to the next worker's `cmd_tx`. This eliminates
sleep-based timing.

**Panic sync pattern**: After sending `b'p'` (panic) through `cmd_tx`, the worker thread panics
inside `catch_unwind`. The test waits for the thread `JoinHandle` to finish (the thread exits after
sending `Shutdown(Panic)` and dropping `TerminationGuard`). No `create_notify` wait is needed
because no restart is attempted - the thread exits unconditionally.
