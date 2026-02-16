<!-- cspell:words Proactor greppable RPITIT rrtwaker -->
<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [io_uring Compatibility](#io_uring-compatibility)
  - [Before / After](#before--after)
    - [Before (3 traits, 3+ types per implementation)](#before-3-traits-3-types-per-implementation)
    - [After Phase 1 (1 trait, 1 type alias)](#after-phase-1-1-trait-1-type-alias)
    - [After Phase 2 (2 traits, associated types)](#after-phase-2-2-traits-associated-types)
- [Implementation plan](#implementation-plan)
  - [Step 1: Define `WakeFn` and rewrite `RRTWorker` trait (`rrt_di_traits.rs`) [COMPLETE]](#step-1-define-wakefn-and-rewrite-rrtworker-trait-rrt_di_traitsrs-complete)
  - [Step 2: Update `SubscriberGuard` (`rrt_subscriber_guard.rs`) [COMPLETE]](#step-2-update-subscriberguard-rrt_subscriber_guardrs-complete)
  - [Step 3: Update `RRT` struct and `TerminationGuard` (`rrt.rs`) [COMPLETE]](#step-3-update-rrt-struct-and-terminationguard-rrtrs-complete)
  - [Step 4: Update production implementation (`mio_poller/`) [COMPLETE]](#step-4-update-production-implementation-mio_poller-complete)
  - [Step 5: Update singleton and type alias (`input_device_impl.rs`) [COMPLETE]](#step-5-update-singleton-and-type-alias-input_device_implrs-complete)
  - [Step 6: Update unit tests (`rrt_restart_tests.rs`) [COMPLETE]](#step-6-update-unit-tests-rrt_restart_testsrs-complete)
  - [Step 7: Update PTY integration tests (`rrt_restart_pty_tests.rs`) [COMPLETE]](#step-7-update-pty-integration-tests-rrt_restart_pty_testsrs-complete)
  - [Step 8: Update module-level documentation (`mod.rs`) [COMPLETE]](#step-8-update-module-level-documentation-modrs-complete)
  - [Step 9: Update crate-level documentation (`lib.rs`) [COMPLETE]](#step-9-update-crate-level-documentation-librs-complete)
  - [Step 10: Update other doc references (`mio_poller/` module docs) [COMPLETE]](#step-10-update-other-doc-references-mio_poller-module-docs-complete)
  - [Step 11: Verify [COMPLETE]](#step-11-verify-complete)
  - [Step 12: Reintroduce `RRTWaker` trait (`rrt_di_traits.rs`) [COMPLETE]](#step-12-reintroduce-rrtwaker-trait-rrt_di_traitsrs-complete)
  - [Step 13: Update `SubscriberGuard` to use `RRTWaker` generic (`rrt_subscriber_guard.rs`) [COMPLETE]](#step-13-update-subscriberguard-to-use-rrtwaker-generic-rrt_subscriber_guardrs-complete)
  - [Step 14: Update `RRT`, `TerminationGuard`, and `run_worker_loop` (`rrt.rs`) [COMPLETE]](#step-14-update-rrt-terminationguard-and-run_worker_loop-rrtrs-complete)
  - [Step 15: Recreate `mio_poll_waker.rs` and update `mio_poll_worker.rs` [COMPLETE]](#step-15-recreate-mio_poll_wakerrs-and-update-mio_poll_workerrs-complete)
  - [Step 16: Update `input_device_impl.rs` and test files [COMPLETE]](#step-16-update-input_device_implrs-and-test-files-complete)
  - [Step 17: Update documentation references across all files [COMPLETE]](#step-17-update-documentation-references-across-all-files-complete)
  - [Step 18: Verify Phase 2 [COMPLETE]](#step-18-verify-phase-2-complete)
  - [Files Changed (Summary)](#files-changed-summary)
  - [Risk Assessment](#risk-assessment)
  - [What Does NOT Change](#what-does-not-change)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

**Goal (Phase 1)**: Replace `RRTFactory` + `RRTWorker` + `RRTWaker` (3 traits, 3+ types) with a
single `RRTWorker` trait + `WakeFn` type alias (1 trait, 1 type alias).

**Goal (Phase 2)**: Reintroduce `RRTWaker` as a trait (replacing the `WakeFn` type alias) with an
associated `type Waker: RRTWaker` on `RRTWorker`, and `MioPollWaker` comes back as a newtype.

**Goal (Phase 3)**: Remove the associated `type Waker: RRTWaker` from `RRTWorker` and use trait
object erasure instead. `create()` returns `(Self, impl RRTWaker)` via RPITIT (Rust 1.75+), and the
framework stores the waker as `Box<dyn RRTWaker>`. This eliminates the waker generic from all
framework types (`SubscriberGuard<W, E>` -> `SubscriberGuard<E>`, `TerminationGuard<W>` ->
`TerminationGuard`) while keeping the `RRTWaker` trait for type safety at the implementor boundary.

**Motivation (Phase 1)**: The three traits are always coupled - no one mixes factories with workers
or swaps wakers independently. The `RRTWaker` impl is always trivial (1 line), and the factory is
always a zero-sized type tag. Collapsing removes concepts without losing expressiveness.

**Motivation (Phase 2)**: The `WakeFn` type alias (`Box<dyn Fn() + Send + Sync>`) loses type
safety - any closure matches, with no compile-time guarantee that the waker matches the worker. An
associated `type Waker: RRTWaker` on `RRTWorker` restores this: `create()` returns
`(Self, Self::Waker)`, tying the waker to the worker at the type level. The cost (one extra generic
on `SubscriberGuard<W, E>`) is minimal since `E` was already generic.

## io_uring Compatibility

The `WakeFn` closure approach preserves full io_uring compatibility. Every wake strategy from the
"Why Is RRTWaker User-Provided?" table (mod.rs:558-563) maps cleanly to a closure:

| Blocking on... | Wake closure captures...                   |
| :------------- | :----------------------------------------- |
| `mio::Poll`    | `mio::Waker` (triggers epoll/kqueue)       |
| TCP `accept()` | Socket address for connect-to-self pattern |
| Pipe `read(2)` | Write-end fd for self-pipe trick           |
| `io_uring`     | `EventFd` or `Arc<IoUring>` for MSG_RING   |

The type-theoretic equivalence: any `T: Send + Sync + 'static` with
`fn wake(&self) -> io::Result<()>` can be captured by-move into
`Box<dyn Fn() -> io::Result<()> + Send + Sync>`. The closure type-erases `T` at the cost of one heap
allocation + vtable indirection - both negligible since `wake()` is called rarely (only on
subscriber drop).

## Before / After

### Before (3 traits, 3+ types per implementation)

```rust
struct MioPollWorkerFactory;   // zero-sized type tag
struct MioPollWorker { ... }
struct MioPollWaker(Waker);

impl RRTFactory for MioPollWorkerFactory { ... }  // 60 lines
impl RRTWorker for MioPollWorker { ... }          // 50 lines
impl RRTWaker for MioPollWaker { ... }            //  3 lines

static SINGLETON: RRT<MioPollWorkerFactory> = RRT::new();
pub type InputSubscriberGuard = SubscriberGuard<MioPollWaker, PollerEvent>;
```

### After Phase 1 (1 trait, 1 type alias)

```rust
pub type WakeFn = Box<dyn Fn() -> std::io::Result<()> + Send + Sync>;

struct MioPollWorker { ... }

impl RRTWorker for MioPollWorker {
    type Event = PollerEvent;
    fn create() -> miette::Result<(Self, WakeFn)> { ... }
    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation { ... }
    fn restart_policy() -> RestartPolicy { RestartPolicy::default() }
}

static SINGLETON: RRT<MioPollWorker> = RRT::new();
pub type InputSubscriberGuard = SubscriberGuard<PollerEvent>;
```

### After Phase 2 (2 traits, associated types)

```rust
pub trait RRTWaker: Send + Sync + 'static { fn wake(&self); }

struct MioPollWaker(pub mio::Waker);
impl RRTWaker for MioPollWaker { fn wake(&self) { let _ = self.0.wake(); } }

struct MioPollWorker { ... }

impl RRTWorker for MioPollWorker {
    type Event = PollerEvent;
    type Waker = MioPollWaker;
    fn create() -> miette::Result<(Self, Self::Waker)> { ... }
    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation { ... }
    fn restart_policy() -> RestartPolicy { RestartPolicy::default() }
}

static SINGLETON: RRT<MioPollWorker> = RRT::new();
pub type InputSubscriberGuard = SubscriberGuard<MioPollWaker, PollerEvent>;
```

### After Phase 3 (2 traits, trait object erasure - CURRENT)

```rust
pub trait RRTWaker: Send + Sync + 'static { fn wake(&self); }

struct MioPollWaker(pub mio::Waker);
impl RRTWaker for MioPollWaker { fn wake(&self) { let _ = self.0.wake(); } }

struct MioPollWorker { ... }

impl RRTWorker for MioPollWorker {
    type Event = PollerEvent;
    // No type Waker - erased via Box<dyn RRTWaker> inside framework
    fn create() -> miette::Result<(Self, impl RRTWaker)> { ... }
    fn poll_once(&mut self, tx: &Sender<RRTEvent<Self::Event>>) -> Continuation { ... }
    fn restart_policy() -> RestartPolicy { RestartPolicy::default() }
}

static SINGLETON: RRT<MioPollWorker> = RRT::new();
pub type InputSubscriberGuard = SubscriberGuard<PollerEvent>;  // one fewer generic!
```

**Key difference from Phase 2**: The `RRTWaker` trait is kept (type safety at the implementor
boundary), but the associated `type Waker` is removed. Implementors return `impl RRTWaker` (RPITIT)
from `create()`, and the framework boxes it into `Box<dyn RRTWaker>` for storage. This eliminates
the waker generic from `SubscriberGuard`, `TerminationGuard`, and `RRT`'s waker field.

# Implementation plan

## Step 1: Define `WakeFn` and rewrite `RRTWorker` trait (`rrt_di_traits.rs`) [COMPLETE]

- Add `pub type WakeFn = Box<dyn Fn() -> std::io::Result<()> + Send + Sync>;`
- Move `create()` and `restart_policy()` from `RRTFactory` into `RRTWorker`
- Change `create()` return type to `miette::Result<(Self, WakeFn)>`
  - Add `where Self: Sized` bound on `create()`
- Remove `type Worker` and `type Waker` associated types (were on `RRTFactory`)
- Keep `type Event: Clone + Send + 'static` on `RRTWorker`
- Delete the `RRTFactory` trait entirely
- Delete the `RRTWaker` trait entirely
- **Doc migration** - no docs are deleted, they migrate to new homes:

  | Deleted source                        | Content                                                                             | New home                                                         |
  | :------------------------------------ | :---------------------------------------------------------------------------------- | :--------------------------------------------------------------- |
  | `RRTFactory` trait docs (50 lines)    | DI overview, two-phase setup ref, `create()` semantics, `restart_policy()`          | `RRTWorker` trait-level docs + `RRTWorker::create()` method docs |
  | `RRTWaker` trait docs (80 lines)      | `Send + Sync + 'static` rationale, shared-access ASCII diagram, "why user-provided" | `WakeFn` type alias docs + `SubscriberGuard` docs                |
  | `MioPollWaker` struct docs (30 lines) | Poll coupling, "How It Works", `wake()` triggers epoll                              | `RRTWorker::create()` method docs (coupling rationale)           |

## Step 2: Update `SubscriberGuard` (`rrt_subscriber_guard.rs`) [COMPLETE]

**Before**: `SubscriberGuard<W, E>` where `W: RRTWaker, E: Clone + Send + 'static` **After**:
`SubscriberGuard<E>` where `E: Clone + Send + 'static`

- Remove generic `W` parameter
- Change `waker: Arc<Mutex<Option<W>>>` to `waker: Arc<Mutex<Option<WakeFn>>>`
- Update `Drop` impl: replace `w.wake()` with `w()` (call the closure directly)
- Update all rustdoc references to `RRTWaker`

## Step 3: Update `RRT` struct and `TerminationGuard` (`rrt.rs`) [COMPLETE]

**`RRT<F>` becomes `RRT<W>`**:

- Change generic from `F: RRTFactory` to `W: RRTWorker`
- Remove `F::Waker: RRTWaker` bound (no longer needed)
- Change `F::Event` to `W::Event` everywhere
- Change `waker: OnceLock<Arc<Mutex<Option<F::Waker>>>>` to
  `waker: OnceLock<Arc<Mutex<Option<WakeFn>>>>`
- Update `subscribe()`:
  - Change `F::create()` to `W::create()`
  - Return `SubscriberGuard<W::Event>` instead of `SubscriberGuard<F::Waker, F::Event>`
- Update `subscribe_to_existing()` similarly

**Add `SubscribeError` typed error enum** (`rrt.rs`):

- Define `SubscribeError` with three variants covering the failure modes of `subscribe()`:
  - `MutexPoisoned { which: &'static str }` - internal mutex poisoned by prior thread panic
  - `WorkerCreation(#[source] miette::Report)` - `RRTWorker::create()` failed (wraps the source
    error chain from the worker implementation, e.g. `PollCreationError`)
  - `ThreadSpawn(#[source] std::io::Error)` - `std::thread::Builder::spawn()` failed
- Derive `thiserror::Error` + `miette::Diagnostic` with:
  - `code(r3bl_tui::rrt::*)` for greppable error namespace
  - `help(...)` with actionable remediation advice
  - `#[source]` on `WorkerCreation` and `ThreadSpawn` for error chain propagation
- Change `subscribe()` return type from `Result<..., Report>` to `Result<..., SubscribeError>`
- Replace ad-hoc error construction:
  - `miette::miette!("RRT liveness mutex poisoned")` ->
    `SubscribeError::MutexPoisoned { which: "liveness" }`
  - `.context("Failed to create worker thread resources")` ->
    `.map_err(SubscribeError::WorkerCreation)`
  - `miette::miette!("RRT waker mutex poisoned")` ->
    `SubscribeError::MutexPoisoned { which: "waker" }`
  - `.into_diagnostic().context("Failed to spawn worker thread")` ->
    `.map_err(SubscribeError::ThreadSpawn)`
- Callers using `.unwrap()` / `.expect()` are unaffected
- Callers propagating with `?` into `Result<_, miette::Report>` are unaffected (miette auto-converts
  any `Diagnostic + Send + Sync + 'static` into `Report`)

**`TerminationGuard<W>` becomes `TerminationGuard`** (no generics):

- Remove generic `W: RRTWaker` parameter
- Change `waker: Arc<Mutex<Option<W>>>` to `waker: Arc<Mutex<Option<WakeFn>>>`
- Update `Drop` impl: `(*guard)()` instead of `guard.wake()` - or even cleaner,
  `if let Some(wake) = guard.as_ref() { drop(wake()); }`

**`run_worker_loop<F>` becomes `run_worker_loop<W>`**:

- Change generic from `F: RRTFactory` to `W: RRTWorker`
- Change parameter `worker: F::Worker` to `worker: W` (the worker IS the type now)
- Change `tx: Sender<RRTEvent<F::Event>>` to `tx: Sender<RRTEvent<W::Event>>`
- Change `waker: Arc<Mutex<Option<F::Waker>>>` to `waker: Arc<Mutex<Option<WakeFn>>>`
- Remove `F::Waker: RRTWaker` and other factory-specific bounds
- Change `F::restart_policy()` to `W::restart_policy()`
- Change `F::create()` to `W::create()`

## Step 4: Update production implementation (`mio_poller/`) [COMPLETE]

**`mio_poll_worker.rs`**:

- Delete `MioPollWorkerFactory` struct and its `impl RRTFactory` block (~70 lines)
- Add `create()` and `restart_policy()` methods to `impl RRTWorker for MioPollWorker`
  - `create()` body moves from `MioPollWorkerFactory::create()`, but returns
    `(MioPollWorker, WakeFn)` - the waker becomes `Box::new(move || waker.wake()) as WakeFn`
  - `restart_policy()` can use `RestartPolicy::default()` (inherited from trait default)
- Update imports: remove `RRTFactory`, add `WakeFn`
- Update module docs

**`mio_poll_waker.rs`**:

- Delete the entire file - `MioPollWaker` newtype and `impl RRTWaker` are no longer needed. The
  `mio::Waker` is captured directly in the `WakeFn` closure.

**`mio_poller/mod.rs`**:

- Remove `mod mio_poll_waker;` and its `pub use`
- Remove `MioPollWaker` from any re-exports
- Update module-level docs that reference `MioPollWaker` and `RRTWaker`

## Step 5: Update singleton and type alias (`input_device_impl.rs`) [COMPLETE]

- Change `use ... MioPollWorkerFactory` to `use ... MioPollWorker`
- Remove `use ... MioPollWaker`
- Change `pub type InputSubscriberGuard = SubscriberGuard<MioPollWaker, PollerEvent>` to
  `pub type InputSubscriberGuard = SubscriberGuard<PollerEvent>`
- Change `pub static SINGLETON: RRT<MioPollWorkerFactory>` to
  `pub static SINGLETON: RRT<MioPollWorker>`
- Update rustdoc

## Step 6: Update unit tests (`rrt_restart_tests.rs`) [COMPLETE]

**Delete `TestWaker` and `TestFactory`**:

- Delete `TestWaker` struct and `impl RRTWaker for TestWaker` (~6 lines)
- Delete `NEXT_WAKER_ID` static
- Delete `TestFactory` struct and `impl RRTFactory for TestFactory` (~30 lines)
- Move `create()` and `restart_policy()` logic into `impl RRTWorker for TestWorker`

**Update `TestFactoryState`**:

- Change `create_results: VecDeque<Result<(TestWorker, TestWaker), Report>>` to
  `create_results: VecDeque<miette::Result<(TestWorker, WakeFn)>>`

**Update helper functions**:

- `create_test_resources()`: Return `(TestWorker, WakeFn, Sender<u8>)` - wake fn is
  `Box::new(|| Ok(()))`
- `create_ok_result()`: Return `(miette::Result<(TestWorker, WakeFn)>, Sender<u8>)`
- `setup_factory()`: Update result types to use `WakeFn`
- `spawn_worker_loop()`: Change `shared_waker: Arc<Mutex<Option<TestWaker>>>` to
  `Arc<Mutex<Option<WakeFn>>>`, and `run_worker_loop::<TestFactory>` to
  `run_worker_loop::<TestWorker>`

**Update all ~23 test functions**:

- Change all `Arc<Mutex<Option<TestWaker>>>` to `Arc<Mutex<Option<WakeFn>>>`
- Tests that verify waker identity (`test_waker_swap_on_restart`) need adaptation:
  - Old: compare `TestWaker.id` before/after restart
  - New: use `Arc<AtomicBool>` captured in wake closure; check that the old closure is replaced (old
    Arc's strong count drops, or just verify `is_some()` after restart and `create_count`
    incremented)
- Tests that check waker cleared (`test_guard_clears_waker_on_stop`): unchanged - still check
  `shared_waker.lock().unwrap().is_none()`

## Step 7: Update PTY integration tests (`rrt_restart_pty_tests.rs`) [COMPLETE]

- Delete `RestartTestFactory` struct and its `impl RRTFactory` block
- Move `create()` and `restart_policy()` into `impl RRTWorker for RestartTestWorker`
  - `create()` calls `MioPollWorker::create()` (was `MioPollWorkerFactory::create()`) and wraps the
    worker in `RestartTestWorker`
  - Return type becomes `miette::Result<(Self, WakeFn)>`
- Change `RRT::<RestartTestFactory>::new()` to `RRT::<RestartTestWorker>::new()`
- Remove `MioPollWorkerFactory` from imports, add `WakeFn`
- Update module docs

## Step 8: Update module-level documentation (`mod.rs`) [COMPLETE]

This is the largest documentation update. All existing content is preserved - sections are
consolidated to reflect the simpler 1-trait model, not deleted.

Section-by-section plan (line numbers are approximate, will shift as edits accumulate):

| Section                                   | Lines    | What changes                                                                                                                                                                                                                                        | Content preserved?                                          |
| :---------------------------------------- | :------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------------------------------------------------------- |
| "How It Works"                            | ~74-136  | `RRTFactory` -> `RRTWorker` in thread creation/relaunch prose. `RRTWaker` -> `WakeFn` in cooperative shutdown prose (line ~106).                                                                                                                    | Yes - all explanations remain, only trait/type names change |
| "Self-Healing Restart Details"            | ~137-188 | `F::create()` -> `W::create()` in restart sequence. Minor reference updates.                                                                                                                                                                        | Yes - full restart lifecycle preserved                      |
| "Separation of Concerns and DI"           | ~368-407 | Consolidate DI table from 5 rows (Factory/Worker/Waker/Event/Policy) to 4 rows (Worker/WakeFn/Event/Policy). `RRTWorker` row gains `create()`. `RRTWaker` row becomes `WakeFn` row. `RRTFactory` row deleted (content merged into `RRTWorker` row). | Yes - table simplified, all content redistributed           |
| "Design Principles"                       | ~390-407 | Update DI description: "you provide one trait implementation (`RRTWorker`) and a concrete `Event` type" instead of listing 3 traits.                                                                                                                | Yes - principles unchanged, just fewer items listed         |
| "Type Hierarchy Diagram"                  | ~408-445 | Major ASCII diagram update: `F: RRTFactory` -> `W: RRTWorker`, remove `F::Waker`/`F::Worker` lines, add `WakeFn` to waker field, `SubscriberGuard<E>` instead of `<W, E>`                                                                           | Yes - same structure, updated generics                      |
| "The RRT Contract and Benefits"           | ~448-493 | Update example: `RRT<MioPollWorkerFactory>` -> `RRT<MioPollWorker>`. Update prose: `RRTFactory::create()` -> `RRTWorker::create()`. `RRTWaker` -> `WakeFn`.                                                                                         | Yes - all contract details preserved                        |
| "Two-Phase Setup"                         | ~494-551 | Still conceptually valid. Update ASCII diagrams: `RRTFactory::create()` -> `RRTWorker::create()`. "Worker + Waker" -> "Worker + WakeFn". Phase 2 "split and distribute" unchanged.                                                                  | Yes - core concept and diagrams preserved, names updated    |
| "Why Is RRTWaker User-Provided?"          | ~552-574 | Rename to "Why Is `WakeFn` User-Provided?". Same wake strategy table (mio, TCP, pipe, io_uring). Update prose to explain closure captures instead of trait implementations.                                                                         | Yes - full rationale preserved, expressed in closure terms  |
| "Example"                                 | ~603-659 | **Major simplification**: Rewrite from 3 trait impls (~40 lines) to 1 trait impl (~20 lines). Shows `WakeFn` closure instead of `MyWaker` struct. Same `GLOBAL.subscribe()` usage.                                                                  | Yes - same concepts demonstrated with less code             |
| "Module Contents"                         | ~666-674 | Update: "`rrt_di_traits`: Core trait (`RRTWorker`) and `WakeFn` type alias" instead of listing 3 traits.                                                                                                                                            | Yes - simplified listing                                    |
| "Waker Mechanism Adaptation" (io_uring)   | ~766-779 | Update: "The RRT's `WakeFn` type already abstracts this" instead of "`RRTWaker` trait". Reference `RRTWorker::create()` instead of `RRTFactory`.                                                                                                    | Yes - all io_uring guidance preserved                       |
| "Why RRT and Not Actor/Reactor/Proactor?" | ~781-808 | No changes needed - doesn't reference specific trait names.                                                                                                                                                                                         | Yes - unchanged                                             |
| Intra-doc link references                 | ~830-960 | Remove: `RRTFactory`, `RRTWaker`, `F::Event`, `Factory::create()` links. Add: `WakeFn` link. Update: `create()` -> `RRTWorker::create`, `restart_policy()` -> `RRTWorker::restart_policy`, `wake()` -> link to `WakeFn` docs.                       | Yes - all targets updated, no dangling links                |

## Step 9: Update crate-level documentation (`lib.rs`) [COMPLETE]

- Update the RRT table (line ~1702): Remove `RRTWaker` row or replace with `WakeFn`
- Update link definitions (line ~2542): Remove `RRTWaker` link, update `RRTWorker` link

## Step 10: Update other doc references (`mio_poller/` module docs) [COMPLETE]

- `mio_poller/mod.rs`: Update references to `RRTWaker`, `MioPollWaker`
- `mio_poller/dispatcher.rs`, `handler_*.rs`: Check for `RRTFactory`/`RRTWaker` references in docs
  and update
- `input_device_public_api.rs`: Update any architecture docs referencing the 3-trait model

## Step 11: Verify [COMPLETE]

- `./check.fish --full` - must pass all checks (typecheck, build, clippy, tests, docs, Windows
  cross-compilation)
- Verify rustdoc renders correctly: `./check.fish --doc`

## Step 12: Reintroduce `RRTWaker` trait (`rrt_di_traits.rs`) [COMPLETE]

- Delete `pub type WakeFn = Box<dyn Fn() + Send + Sync>;`
- Add `pub trait RRTWaker: Send + Sync + 'static { fn wake(&self); }`
- Add `type Waker: RRTWaker` associated type to `RRTWorker`
- Change `create()` return type from `miette::Result<(Self, WakeFn)>` to
  `miette::Result<(Self, Self::Waker)>`
- Update all rustdoc (trait-level docs, method docs, link references)

## Step 13: Update `SubscriberGuard` to use `RRTWaker` generic (`rrt_subscriber_guard.rs`) [COMPLETE]

- Add generic `W: RRTWaker` parameter: `SubscriberGuard<W, E>`
- Change `waker: Arc<Mutex<Option<WakeFn>>>` to `waker: Arc<Mutex<Option<W>>>`
- Update `Drop` impl: replace closure call `w()` with `w.wake()` method call
- Update all rustdoc references

## Step 14: Update `RRT`, `TerminationGuard`, and `run_worker_loop` (`rrt.rs`) [COMPLETE]

- Change `waker: OnceLock<Arc<Mutex<Option<WakeFn>>>>` to
  `waker: OnceLock<Arc<Mutex<Option<W::Waker>>>>`
- `subscribe()` returns `SubscriberGuard<W::Waker, W::Event>` instead of `SubscriberGuard<W::Event>`
- `TerminationGuard` becomes `TerminationGuard<W: RRTWaker>` with `waker: Arc<Mutex<Option<W>>>`
- `run_worker_loop` waker param: `Arc<Mutex<Option<W::Waker>>>`
- Update all rustdoc references

## Step 15: Recreate `mio_poll_waker.rs` and update `mio_poll_worker.rs` [COMPLETE]

**`mio_poll_waker.rs`** (recreated):

- Add `#[derive(Debug)] pub struct MioPollWaker(pub mio::Waker);`
- Implement `RRTWaker for MioPollWaker` with `fn wake(&self) { let _ = self.0.wake(); }`
- Full rustdoc with "How It Works" ASCII diagram

**`mio_poll_worker.rs`**:

- Add `type Waker = MioPollWaker;` to `impl RRTWorker`
- Change `create()` return type to `miette::Result<(Self, MioPollWaker)>`
- Replace `Box::new(move || waker.wake())` with `MioPollWaker(waker)`

**`mio_poller/mod.rs`**:

- Re-add `mod mio_poll_waker;` with `#[cfg(any(test, doc))]` visibility
- Re-add `pub use mio_poll_waker::*;`
- Update doc references from `WakeFn` to `RRTWaker`

## Step 16: Update `input_device_impl.rs` and test files [COMPLETE]

**`input_device_impl.rs`**:

- Change `InputSubscriberGuard` from `SubscriberGuard<PollerEvent>` to
  `SubscriberGuard<MioPollWaker, PollerEvent>`

**`rrt_restart_tests.rs`**:

- Add `TestWaker { id: u32 }` struct with `impl RRTWaker`
- Add `type Waker = TestWaker;` to `TestWorker`
- Replace all `WakeFn` references with `TestWaker`
- Replace closure-based waker creation with `TestWaker { id: waker_id }`
- Update `test_waker_swap_on_restart()` to use `.wake()` method calls

**`rrt_restart_pty_tests.rs`**:

- Add `type Waker = MioPollWaker;` to `RestartTestWorker`

## Step 17: Update documentation references across all files [COMPLETE]

- `mod.rs` (~35 `WakeFn` -> `RRTWaker` replacements in doc comments)
  - Text fixes: "closure" -> "implementation", "type alias" -> "trait"
  - ASCII diagram updates: `W::Waker` in type positions, `SubscriberGuard<W::Waker, W::Event>`
  - Code example: added `MyWaker` struct, `type Waker = MyWaker`, `Self::Waker` return
  - Fixed `[DI]:` line that was being misinterpreted as a reference-style link definition
  - Anchor reference: `#why-is-rrtwaker-user-provided`
- `lib.rs` (2 references: table entry and link definition)
- `mio_poller/mod.rs` (2 doc references)

## Step 18: Verify Phase 2 [COMPLETE]

- `./check.fish --check` - passes
- `./check.fish --clippy` - passes
- `./check.fish --doc` - passes
- `./check.fish --test` - passes (including `cargo_rustdoc_fmt` validation tests)

## Step 19: Remove `type Waker` associated type, use RPITIT + trait object erasure (`rrt_di_traits.rs`) [COMPLETE]

- Delete `type Waker: RRTWaker` associated type and its docs from `RRTWorker`
- Change `create()` return from `miette::Result<(Self, Self::Waker)>` to
  `miette::Result<(Self, impl RRTWaker)>`
- Updated docs to explain RPITIT and type erasure via `Box<dyn RRTWaker>`

## Step 20: Simplify `SubscriberGuard` (`rrt_subscriber_guard.rs`) [COMPLETE]

- Remove `W: RRTWaker` generic: `SubscriberGuard<W, E>` -> `SubscriberGuard<E>`
- Change waker field: `Arc<Mutex<Option<W>>>` -> `Arc<Mutex<Option<Box<dyn RRTWaker>>>>`
- Drop impl simplified: `impl<E> Drop for SubscriberGuard<E>`

## Step 21: Update `RRT`, `TerminationGuard`, `run_worker_loop` (`rrt.rs`) [COMPLETE]

- Waker field: `OnceLock<Arc<Mutex<Option<Box<dyn RRTWaker>>>>>`
- `subscribe()` returns `SubscriberGuard<W::Event>` (one generic instead of two)
- `TerminationGuard` loses all generics (was `TerminationGuard<W: RRTWaker>`)
- `run_worker_loop` waker param: `Arc<Mutex<Option<Box<dyn RRTWaker>>>>`
- Boxing uses let-binding coercion (not `as` cast) to avoid `trivial-casts` warning:
  `let boxed: Box<dyn RRTWaker> = Box::new(new_waker);`

## Step 22: Update production implementation (`mio_poll_worker.rs`) [COMPLETE]

- Remove `type Waker = MioPollWaker;`
- Change `create()` return to `miette::Result<(Self, impl RRTWaker)>`
- Add `RRTWaker` to import path

## Step 23: Update `input_device_impl.rs` [COMPLETE]

- Simplify type alias: `SubscriberGuard<MioPollWaker, PollerEvent>` ->
  `SubscriberGuard<PollerEvent>`
- Remove `MioPollWaker` from imports

## Step 24: Update test files [COMPLETE]

**`rrt_restart_tests.rs`**:
- Remove `type Waker = TestWaker;` from `impl RRTWorker for TestWorker`
- Change `create()` return to `miette::Result<(Self, impl RRTWaker)>`
- All `Arc<Mutex<Option<TestWaker>>>` -> `Arc<Mutex<Option<Box<dyn RRTWaker>>>>`
- All waker initialization: `Some(wake_fn)` -> `Some(Box::new(wake_fn))`
  (type annotation on `let` drives the unsized coercion, no `as` cast needed)

**`rrt_restart_pty_tests.rs`**:
- Remove `type Waker = MioPollWaker;` from `impl RRTWorker for RestartTestWorker`
- Change `create()` return to `miette::Result<(Self, impl RRTWaker)>`
- Remove unused `MioPollWaker` import

## Step 25: Update module documentation (`mod.rs`) [COMPLETE]

- Type hierarchy diagram: removed `W::Waker` line, updated waker field to
  `Box<dyn RRTWaker>`, `SubscriberGuard<W::Event>` (one generic)
- Example code: removed `type Waker = MyWaker;`, changed `create()` to return
  `impl RRTWaker`

## Step 26: Verify Phase 3 [COMPLETE]

- `./check.fish --check` - passes
- `./check.fish --clippy` - passes (no `trivial-casts` warnings)

## Files Changed (Summary)

| File                       | Phase 1 Change                                       | Phase 2 Change                                          | Phase 3 Change                                          |
| :------------------------- | :--------------------------------------------------- | :------------------------------------------------------ | :------------------------------------------------------ |
| `rrt_di_traits.rs`         | Collapse 3 traits to 1 + `WakeFn` type alias         | Replace `WakeFn` with `RRTWaker` trait + `type Waker`   | Remove `type Waker`, `create()` returns `impl RRTWaker` |
| `rrt_subscriber_guard.rs`  | Remove `W` generic                                   | Re-add `W: RRTWaker` generic                            | Remove `W` generic again, `Box<dyn RRTWaker>` field     |
| `rrt.rs`                   | `RRT<F>` -> `RRT<W>`, add `SubscribeError`           | Waker field `Option<W::Waker>`, `TerminationGuard<W>`   | Waker field `Box<dyn RRTWaker>`, `TerminationGuard` (no generic) |
| `mio_poll_worker.rs`       | Delete factory, merge `create()` into worker         | Add `type Waker = MioPollWaker`, return `MioPollWaker`  | Remove `type Waker`, return `impl RRTWaker`             |
| `mio_poll_waker.rs`        | **Delete entire file**                               | **Recreate**: `MioPollWaker` newtype + `impl RRTWaker`  | Unchanged (still implements `RRTWaker`)                 |
| `mio_poller/mod.rs`        | Remove waker module, update docs                     | Re-add waker module, update `WakeFn` -> `RRTWaker` docs | Unchanged                                               |
| `input_device_impl.rs`     | Update singleton type and type alias                 | `SubscriberGuard<MioPollWaker, PollerEvent>`            | `SubscriberGuard<PollerEvent>` (remove waker generic)   |
| `rrt_restart_tests.rs`     | Delete TestWaker/TestFactory, update test functions   | Add `TestWaker` + `impl RRTWaker`, `type Waker`        | Remove `type Waker`, `Box<dyn RRTWaker>` in tests       |
| `rrt_restart_pty_tests.rs` | Delete RestartTestFactory, update test               | Add `type Waker = MioPollWaker`                         | Remove `type Waker`, return `impl RRTWaker`             |
| `mod.rs`                   | Major doc update: diagrams, examples, link references | ~35 `WakeFn` -> `RRTWaker` replacements                | Diagram + example: remove `W::Waker`, `Box<dyn>`        |
| `lib.rs`                   | Update crate-level doc references                    | `WakeFn` -> `RRTWaker` in table + link                  | Unchanged                                               |

## Risk Assessment

- **Low risk**: Both phases are mechanical - same behavior, different abstractions. Phase 1: every
  `RRTWaker::wake(&self)` becomes `(wake_fn)()`, every `RRTFactory::create()` becomes
  `RRTWorker::create()`. Phase 2: every `(wake_fn)()` becomes `w.wake()`, `WakeFn` becomes
  `W::Waker`.
- **Test coverage**: Existing tests cover all restart paths, waker lifecycle, panic handling. Phase
  2 adds `TestWaker` struct for verifiable waker identity in tests.
- **io_uring**: Verified compatible in both phases - any `T: Send + Sync + 'static` can implement
  `RRTWaker` (Phase 2) just as it could be captured in a `WakeFn` closure (Phase 1).
- **Breaking change scope**: Internal crate only. No external consumers of these traits.

## What Does NOT Change

- `RRTEvent<E>` enum (unchanged)
- `RRTLiveness` (unchanged)
- `RestartPolicy` (unchanged)
- `Continuation` enum (unchanged)
- `CHANNEL_CAPACITY` constant (unchanged)
- `advance_backoff_delay()` function (unchanged)
- The broadcast channel architecture (unchanged)
- The two-phase setup concept (still valid, just expressed differently)
