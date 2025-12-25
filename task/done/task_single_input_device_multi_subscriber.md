# Single Input Device with Multiple Subscribers

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Problem](#problem)
  - [Solution](#solution)
  - [Architecture After Change](#architecture-after-change)
- [Implementation Plan](#implementation-plan)
  - [Step 0: Add Global Flag and Gate `new()` [COMPLETE]](#step-0-add-global-flag-and-gate-new-complete)
    - [Step 0.0: Add `DEVICE_EXISTS` Static](#step-00-add-device_exists-static)
    - [Step 0.1: Update `new()` to Panic on Second Call](#step-01-update-new-to-panic-on-second-call)
    - [Step 0.2: Update `Drop` to Reset Flag](#step-02-update-drop-to-reset-flag)
  - [Step 1: Subscriber Handle for Additional Receivers [COMPLETE - USED EXISTING TYPE]](#step-1-subscriber-handle-for-additional-receivers-complete---used-existing-type)
    - [Step 1.0: Actual Type Used - `InputDeviceResHandle`](#step-10-actual-type-used---inputdevicereshandle)
    - [Step 1.1: `Drop` Implementation [COMPLETE]](#step-11-drop-implementation-complete)
    - [Step 1.2: `recv()` Method [NOT IMPLEMENTED]](#step-12-recv-method-not-implemented)
  - [Step 2: Add `subscribe()` Method to Device [COMPLETE]](#step-2-add-subscribe-method-to-device-complete)
    - [Step 2.0: Add Helper Function in `global_input_res.rs`](#step-20-add-helper-function-in-global_input_resrs)
    - [Step 2.1: Add `subscribe()` to `DirectToAnsiInputDevice`](#step-21-add-subscribe-to-directtoansiinputdevice)
  - [Step 3: Update Documentation [COMPLETE]](#step-3-update-documentation-complete)
    - [Step 3.0: Update Module Docs](#step-30-update-module-docs)
    - [Step 3.1: Update `DirectToAnsiInputDevice` Docs](#step-31-update-directtoansiinputdevice-docs)
  - [Step 4: Update Tests [COMPLETE]](#step-4-update-tests-complete)
    - [Step 4.0: Add Test for Panic on Double `new()`](#step-40-add-test-for-panic-on-double-new)
    - [Step 4.1: Add Test for `subscribe()`](#step-41-add-test-for-subscribe)
    - [Step 4.2: Add Test for Flag Reset on Drop](#step-42-add-test-for-flag-reset-on-drop)
  - [Step 5: Consider Renaming `InputDeviceResHandle` [N/A - KEPT EXISTING TYPE]](#step-5-consider-renaming-inputdevicereshandle-na---kept-existing-type)
- [Related Files](#related-files)
- [Related Tasks](#related-tasks)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Problem

The current `DirectToAnsiInputDevice` API allows multiple calls to `new()`, which is semantically
incorrect. There is only ONE stdin, ONE mio-poller thread, but the API suggests you can create
multiple "devices."

```rust
// Currently allowed but semantically wrong:
let device_a = DirectToAnsiInputDevice::new();  // Creates thread, receiver #1
let device_b = DirectToAnsiInputDevice::new();  // Subscribes, receiver #2
// Two "devices" for one stdin - conceptual nonsense!
```

The broadcast channel architecture already supports multiple subscribers, but the API doesn't expose
this cleanly. Users must create multiple "devices" to get multiple subscribers.

## Solution

1. **Gate `new()`**: Panic on second call with helpful error message
2. **Add `subscribe()`**: Method to get additional receivers from existing device
3. **Reset on Drop**: Allow `new()` again after device is dropped

This is clean, simple, and obvious - runtime enforcement with clear guidance.

## Architecture After Change

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ IMPLEMENTED API                                                         │
│                                                                         │
│   DirectToAnsiInputDevice::new()  ←── PANICS if called twice            │
│          │                                                              │
│          ▼                                                              │
│   ┌──────────────────────────┐                                          │
│   │ DirectToAnsiInputDevice  │                                          │
│   │   res_handle (primary)   │  ← InputDeviceResHandle                  │
│   └────────────┬─────────────┘                                          │
│                │                                                        │
│                │ .subscribe()  ←── Can be called MANY times             │
│                │                                                        │
│                ▼                                                        │
│   ┌─────────────────────────────┐  ┌─────────────────────────────┐      │
│   │ InputDeviceResHandle        │  │ InputDeviceResHandle        │ ...  │
│   │   maybe_stdin_rx: Option<>  │  │   maybe_stdin_rx: Option<>  │      │
│   │   mio_poller_thread_waker   │  │   mio_poller_thread_waker   │      │
│   └─────────────────────────────┘  └─────────────────────────────┘      │
│                                                                         │
│   Drop behavior:                                                        │
│   • Handle drops: decrements receiver_count, wakes thread               │
│   • Device drops: resets DEVICE_EXISTS flag, allows future new()        │
│   • Thread exits when: all receivers dropped (receiver_count = 0)       │
└─────────────────────────────────────────────────────────────────────────┘
```

# Implementation Plan

## Step 0: Add Global Flag and Gate `new()` [COMPLETE]

Add atomic flag to prevent multiple `new()` calls.

### Step 0.0: Add `DEVICE_EXISTS` Static

In `input_device.rs`:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

static DEVICE_EXISTS: AtomicBool = AtomicBool::new(false);
```

### Step 0.1: Update `new()` to Panic on Second Call

```rust
impl DirectToAnsiInputDevice {
    pub fn new() -> Self {
        if DEVICE_EXISTS.swap(true, Ordering::SeqCst) {
            panic!(
                "DirectToAnsiInputDevice::new() called twice. \
                 Use device.subscribe() for additional receivers."
            );
        }
        Self {
            res_handle: allocate_or_get_existing_thread(),
        }
    }
}
```

### Step 0.2: Update `Drop` to Reset Flag

```rust
impl Drop for DirectToAnsiInputDevice {
    fn drop(&mut self) {
        DEVICE_EXISTS.store(false, Ordering::Release);
        // Note: res_handle drop happens automatically after this
    }
}
```

## Step 1: Subscriber Handle for Additional Receivers [COMPLETE - USED EXISTING TYPE]

Instead of creating a new `InputEventSubscriber` type, the implementation reused the existing
`InputDeviceResHandle` for both the primary device and additional subscribers. This reduces type
proliferation while maintaining the same functionality.

### Step 1.0: Actual Type Used - `InputDeviceResHandle`

The existing `InputDeviceResHandle` in `input_device.rs` was reused:

```rust
/// Receiver wrapper that wakes the `mio_poller` thread on drop.
///
/// When this receiver is dropped, it calls [`mio::Waker::wake()`] to interrupt the
/// poll loop, allowing the thread to check if it should exit (when [`receiver_count()`]
/// reaches 0).
pub struct InputDeviceResHandle {
    /// The actual broadcast receiver.
    pub maybe_stdin_rx: Option<InputEventReceiver>,
    /// Waker to signal the `mio_poller` thread.
    pub mio_poller_thread_waker: Arc<mio::Waker>,
}
```

### Step 1.1: `Drop` Implementation [COMPLETE]

The `InputDeviceResHandle::drop()` implementation wakes the mio-poller thread:

```rust
impl Drop for InputDeviceResHandle {
    fn drop(&mut self) {
        // Drop the inner receiver first so `receiver_count()` decrements.
        self.maybe_stdin_rx.take();

        // Now wake the thread so it can check if it should exit.
        if let Err(err) = self.mio_poller_thread_waker.wake() {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "InputDeviceResHandle::drop: failed to wake mio_poller thread",
                    error = ?err
                );
            });
        }
    }
}
```

### Step 1.2: `recv()` Method [NOT IMPLEMENTED]

The `recv()` method was **not added** to `InputDeviceResHandle`. Instead:

- The primary device uses `DirectToAnsiInputDevice::try_read_event()` for receiving events.
- Additional subscribers receive a raw `InputDeviceResHandle` with the `maybe_stdin_rx` field
  exposed, allowing callers to interact directly with the broadcast receiver.

This design choice exposes the internal receiver rather than providing a convenience method.

## Step 2: Add `subscribe()` Method to Device [COMPLETE]

### Step 2.0: Add Helper Function in `global_input_res.rs`

```rust
/// Subscribe to input events from an existing thread.
///
/// # Panics
/// Panics if no device exists (thread not spawned).
pub fn subscribe_to_existing() -> crate::InputDeviceResHandle {
    let guard = INPUT_RESOURCE.lock().expect(
        "INPUT_RESOURCE mutex poisoned: another thread panicked while holding this lock.",
    );

    let state = guard.as_ref().expect(
        "subscribe_to_existing() called before DirectToAnsiInputDevice::new(). \
         Create a device first, then call device.subscribe().",
    );

    crate::InputDeviceResHandle {
        maybe_stdin_rx: Some(state.tx.subscribe()),
        mio_poller_thread_waker: Arc::clone(&state.waker),
    }
}
```

### Step 2.1: Add `subscribe()` to `DirectToAnsiInputDevice`

```rust
impl DirectToAnsiInputDevice {
    /// Get an additional subscriber to input events.
    ///
    /// Use this for logging, debugging, or multiple concurrent consumers. Each
    /// subscriber independently receives all input events. When dropped, notifies
    /// the [`mio_poller`] thread to check if it should exit.
    #[must_use]
    pub fn subscribe(&self) -> InputDeviceResHandle {
        subscribe_to_existing()
    }
}
```

## Step 3: Update Documentation [COMPLETE]

### Step 3.0: Update Module Docs

Update `global_input_res.rs` module docs to reflect new API.

### Step 3.1: Update `DirectToAnsiInputDevice` Docs

Add examples showing correct usage:

````rust
/// # Example
///
/// ```no_run
/// let device = DirectToAnsiInputDevice::new();
///
/// // Primary consumer
/// let event = device.try_read_event().await;
///
/// // Additional subscriber for logging - access receiver directly
/// let mut logger_sub = device.subscribe();
/// tokio::spawn(async move {
///     if let Some(ref mut rx) = logger_sub.maybe_stdin_rx {
///         while let Ok(msg) = rx.recv().await {
///             log::debug!("Input: {:?}", msg);
///         }
///     }
/// });
/// ```
````

## Step 4: Update Tests [COMPLETE]

### Step 4.0: Add Test for Panic on Double `new()`

```rust
#[test]
#[should_panic(expected = "called twice")]
fn test_new_panics_on_second_call() {
    let _device1 = DirectToAnsiInputDevice::new();
    let _device2 = DirectToAnsiInputDevice::new();  // Should panic
}
```

### Step 4.1: Add Test for `subscribe()`

```rust
#[tokio::test]
async fn test_subscribe_creates_additional_receiver() {
    let device = DirectToAnsiInputDevice::new();
    let sub1 = device.subscribe();
    let sub2 = device.subscribe();

    // Verify receiver_count is 3 (primary + 2 subscribers)
    assert_eq!(get_receiver_count(), 3);

    drop(sub1);
    assert_eq!(get_receiver_count(), 2);

    drop(sub2);
    assert_eq!(get_receiver_count(), 1);
}
```

### Step 4.2: Add Test for Flag Reset on Drop

```rust
#[test]
fn test_new_allowed_after_drop() {
    {
        let _device = DirectToAnsiInputDevice::new();
    }  // device dropped here

    // Should not panic - flag was reset
    let _device2 = DirectToAnsiInputDevice::new();
}
```

## Step 5: Consider Renaming `InputDeviceResHandle` [N/A - KEPT EXISTING TYPE]

The implementation reused `InputDeviceResHandle` for both the primary device and additional
subscribers, making this renaming consideration moot. The type serves both purposes:

- Primary receiver (held by `DirectToAnsiInputDevice`)
- Additional subscribers (returned by `device.subscribe()`)

This approach reduces type proliferation at the cost of a less specialized API.

# Related Files

- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/input_device.rs`
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/global_input_res.rs`
- `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/channel_types.rs`

# Related Tasks

- `simplify-thread-lifecycle-orchestration.md` - Thread lifecycle design context
