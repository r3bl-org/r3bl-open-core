# Plan: Decouple Broadcast Channel and Waker from Thread Lifecycle

## Context

`RRTState<W, E>` currently bundles three fields with **three different lifetimes**:
- `broadcast_tx: Sender<E>` - singleton-lifetime (should never be recreated)
- `waker: W` - needs to be shared across all subscriber generations (old and new)
- `liveness: RRTLiveness` - per-thread-generation (replaced on relaunch)

**Bug 1 (stranded subscribers)**: When a thread crashes while subscribers exist,
`subscribe()` replaces the entire `RRTState` (including the broadcast channel),
stranding old subscribers on an orphaned channel.

**Bug 2 (zombie thread)**: Old subscribers hold a snapshot of the waker from their
generation. After a relaunch, their `drop()` calls the OLD waker (targeting a dead
`mio::Poll`), which fails silently. If all new-waker subscribers drop first, the
thread runs forever with zero subscribers.

**The fix**: Dissolve `RRTState` entirely. Promote each field to `RRT` with the
correct synchronization primitive for its lifetime:
- `broadcast_tx` → `OnceLock` (created once, read forever)
- `waker` → `OnceLock<Arc<Mutex<...>>>` (shared, swapped on relaunch)
- `liveness` → `Mutex<Option<Arc<...>>>` (per-generation, replaced on relaunch)

## New Structure

```
RRT<F> {
    broadcast_tx: OnceLock<Sender<F::Event>>,               // created once
    waker: OnceLock<Arc<Mutex<Option<F::Waker>>>>,           // shared, swapped on relaunch
    liveness: Mutex<Option<Arc<RRTLiveness>>>,               // per-generation
}

SubscriberGuard<W, E> {
    receiver: Option<Receiver<E>>,
    waker: Arc<Mutex<Option<W>>>,     // always reads current waker
}

TerminationGuard<W: RRTWaker> {       // reduced from <W, E> to <W>
    liveness: Arc<RRTLiveness>,
    waker: Arc<Mutex<Option<W>>>,     // cleared to None on thread death
}
```

`OnceLock` is used for `broadcast_tx` and the waker wrapper because
`tokio::sync::broadcast::channel()` is not a [`const expression`] - it allocates
at runtime, so it can't be initialized in the `static` `SINGLETON` declaration.
Link to existing doc: `RRT#const-expression-vs-const-declaration-vs-static-declaration`.

## What Gets Deleted

- **`RRTState<W, E>` struct** — dissolved, fields promoted to `RRT`
- **`rrt_state.rs` file** — deleted (move `CHANNEL_CAPACITY` to `rrt.rs`)
- **`should_self_terminate()` method** — dead code (`handler_receiver_drop.rs`
  checks `tx.receiver_count()` directly)
- **`ShutdownDecision` enum** — only consumer was `should_self_terminate()`

## Phase 1: Core Structural Changes (atomic - must compile together)

### Step 1.1: Delete `rrt_state.rs`, move `CHANNEL_CAPACITY`

- Move `CHANNEL_CAPACITY` constant to `rrt.rs`
- Delete `rrt_state.rs` entirely
- Remove `mod rrt_state` and `pub use rrt_state::*` from `mod.rs`

### Step 1.2: `rrt.rs` - Three top-level fields, rewrite subscribe flow

- Change `RRT` from one field to three:
  - `broadcast_tx: OnceLock<Sender<F::Event>>`
  - `waker: OnceLock<Arc<Mutex<Option<F::Waker>>>>`
  - `liveness: Mutex<Option<Arc<RRTLiveness>>>`
- Update `const fn new()`:
  ```rust
  Self {
      broadcast_tx: OnceLock::new(),
      waker: OnceLock::new(),
      liveness: Mutex::new(None),
  }
  ```
- **Rewrite `subscribe()`**:
  1. `broadcast_tx.get_or_init(...)` — idempotent channel creation
  2. `waker.get_or_init(|| Arc::new(Mutex::new(None)))` — idempotent wrapper creation
  3. Lock `liveness`, check for fast path (running thread)
  4. Slow path: `drop(guard.take())`, `F::create()` → `(worker, new_waker)`,
     swap waker via `*shared_waker.lock() = Some(new_waker)`,
     create `Arc<RRTLiveness>`, spawn thread
  5. Return `SubscriberGuard { receiver, waker: Arc::clone(shared_waker) }`
- Update `subscribe_to_existing()` similarly
- Update query methods:
  - `is_thread_running()` — lock `liveness` only
  - `get_receiver_count()` — read from `broadcast_tx` OnceLock (no mutex!)
  - `get_thread_generation()` — lock `liveness` only
- Simplify `TerminationGuard<W>` — reduced from `<W, E>` to `<W>`, holds
  `Arc<RRTLiveness>` + `Arc<Mutex<Option<W>>>`. Drop impl clears waker to `None`
  first, then marks terminated:
  ```rust
  fn drop(&mut self) {
      // Clear waker FIRST so no subscriber can call stale wake().
      // Order matters: if we mark_terminated() first, subscribe() could
      // race in, install a new waker, and our cleanup would clear it.
      if let Ok(mut guard) = self.waker.lock() {
          *guard = None;
      }
      self.liveness.mark_terminated();
  }
  ```
- Update `run_worker_loop()` — receives `Arc<RRTLiveness>` and
  `Arc<Mutex<Option<W>>>` (for `TerminationGuard`) instead of `Arc<RRTState<W, E>>`

### Step 1.3: `rrt_subscriber_guard.rs` - Replace state with shared waker

- Replace `state: Arc<RRTState<W, E>>` with `waker: Arc<Mutex<Option<W>>>`
- Update `Drop` impl:
  ```rust
  fn drop(&mut self) {
      drop(self.receiver.take());
      if let Ok(guard) = self.waker.lock() {
          if let Some(w) = guard.as_ref() {
              drop(w.wake());
          }
      }
  }
  ```
- This always calls the CURRENT waker, solving the zombie thread bug

## Phase 2: Documentation and Cleanup (parallelizable)

### Step 2.1: `rrt_liveness.rs` - Remove `ShutdownDecision`

- Delete `ShutdownDecision` enum (only consumer was `should_self_terminate()`)
- Update module doc

### Step 2.2: `rrt_di_traits.rs` - Doc updates

- Update `RRTState` references throughout trait docs

### Step 2.3: `mod.rs` - Extensive doc updates

- Update type hierarchy diagram to show three-field `RRT` structure
- Update ~20+ `RRTState` references throughout the doc
- Update "How It Works" section to reflect channel-outlives-thread
- Remove `should_self_terminate()` and `ShutdownDecision` references
- Remove `rrt_state` module declaration and re-export

### Step 2.4: `input_device_impl.rs` - Doc updates

- Update `RRTState` references in doc comments
- Type alias `InputSubscriberGuard` unchanged

### Step 2.5: `input_device_public_api.rs` - Doc updates

- Update `RRTState` doc link references

## Phase 3: Verification

- `./check.fish --full` (typecheck, build, clippy, tests, doctests, docs)
- Cross-platform: `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`
- Existing PTY integration tests validate behavior (they use public API, not internal types)

## Files Changed

| File | Change Type |
|:-----|:------------|
| `rrt_state.rs` | **Deleted** |
| `rrt.rs` | Three top-level fields, rewrite subscribe flow, `TerminationGuard<W>` clears waker on death |
| `rrt_subscriber_guard.rs` | Shared waker via `Arc<Mutex>` |
| `rrt_liveness.rs` | Remove `ShutdownDecision` enum |
| `rrt_di_traits.rs` | Doc updates |
| `mod.rs` | Extensive doc updates, remove `rrt_state` module |
| `input_device_impl.rs` | Doc updates |
| `input_device_public_api.rs` | Doc updates |

## Type Simplifications

| Before | After |
|:-------|:------|
| `RRTState<W, E>` | Deleted — fields live on `RRT` |
| `TerminationGuard<W, E>` | `TerminationGuard<W>` (clears waker to `None` on death) |
| `SubscriberGuard.state: Arc<RRTState<W, E>>` | `SubscriberGuard.waker: Arc<Mutex<Option<W>>>` |
| `run_worker_loop(state: Arc<RRTState<W, E>>)` | `run_worker_loop(liveness: Arc<RRTLiveness>, waker: Arc<Mutex<Option<W>>>)` |
