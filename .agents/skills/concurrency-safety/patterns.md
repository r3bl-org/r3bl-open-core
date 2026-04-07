# Concurrency Safety Patterns

Compare these patterns to ensure your code follows R3BL standards.

## 1. Scoped Access (Deadlock Prevention)

For simple shared state (e.g., global settings, single statistics, caches), we prefer the **Scoped Access** pattern using `ScopedMutex<S>`. This pattern structurally prevents deadlocks by making it impossible to hold a lock guard longer than the execution of a single closure.

### Why Scoped Access?
In Rust, it is easy to accidentally hold a `MutexGuard` across a long-running or blocking operation, or even attempt to re-lock the same mutex. `ScopedMutex` hides the `.lock()` method and only provides access via closures.

```rust
// ✅ GOOD: Scoped access for simple state
fn update_cache(cache: &ScopedMutex<HashMap<String, String>>) {
    cache.write(|map| {
        map.insert("key".to_string(), "value".to_string());
    }); // Lock released immediately when closure returns.
}
```

### Recursion Detection (Safety-First)
By default, `ScopedMutex` includes **Recursion Detection** to prevent terminal-hanging deadlocks. If a recursive lock is detected on the same thread, it will panic with a clear message instead of hanging.

```rust
// ❌ BAD: Recursive deadlock (detected and results in panic)
SAFE.write(|_| {
    SAFE.read(|_| {}); // Panic: "Recursive lock detected on ScopedMutex!"
});
```

### Performance Opt-out (Performance-Critical)
For performance-critical hot paths (like a render-loop cache), you can opt-out of
recursion detection at compile-time for zero overhead by setting the
`POLICY` const generic to `DeadlockPreventionPolicy::OptOut`.

```rust
// ✅ GOOD: Performance opt-out for hot paths
static HOT_PATH: ScopedMutex<i32, { DeadlockPreventionPolicy::OptOut }> = ScopedMutex::new(0);
```

---

## 2. Chain of Custody (Guard Passing)

For complex state machines or thread coordination (e.g., RRT engine), we use the `Monitor<S>` pattern. Unlike `ScopedMutex`, a `Monitor` **must** return the guard because primitives like `Condvar::wait()` need to consume and then re-acquire the lock.

To manage this safely, we use the **Chain of Custody** pattern.

### Chain of Custody Protocol
1.  The guard is returned by value (e.g., via `monitor.lock()`).
2.  The caller becomes the temporary "custodian" of the lock.
3.  The caller must eventually "return" the guard to the `Monitor` by dropping it or passing it back into another method (like `monitor.wait(guard)`).

```rust
// ✅ GOOD: Chain of custody for complex coordination
fn coordinate(monitor: &Monitor<State>) {
    let mut guard = monitor.lock(); // Caller takes custody.
    while !guard.is_ready() {
        guard = monitor.wait(guard); // Custody passed back and forth.
    }
    // Perform complex multi-step mutation...
    drop(guard); // Explicitly release custody.
}
```

---

## 3. Comparison Summary

| Feature             | `ScopedMutex` (Scoped Access) | `Monitor` (Chain of Custody)    |
| :------------------ | :---------------------------- | :------------------------------ |
| **Primary Goal**    | Simple shared state           | Complex state machines          |
| **Synchronization** | `Mutex` only                  | `Mutex` + `Condvar`             |
| **Access Pattern**  | Closure-based                 | Guard-based (move-by-value)     |
| **Deadlock Safety** | Structural (via closures)     | Protocol-based (chain of custody) |
| **Use Case**        | Global settings, caches       | RRT engine, thread coordination |

---

## 4. Locking and Mutex Poisoning (Drop-Safety)

### Normal Paths (Fail-Fast)
For standard application logic, we prefer the "Fail-Fast" approach. If a mutex is poisoned, it means a previous thread panicked while holding the lock. Failing fast prevents the system from operating on corrupted state.

```rust
// ✅ GOOD: Standard fail-fast for active logic
let val = scoped_mutex.read(|&s| s); // internally calls .unwrap()
```

### Cleanup Paths (Poison-Safe)
For cleanup and terminal restoration paths (e.g., `drop()`, terminal restoration), we **must** be "Poison-Safe". A panic in a cleanup path is a "Double Panic", which aborts the process immediately.

Always use `lock_raw()` and `into_inner()` to attempt restoration even if the mutex is poisoned.

```rust
// ✅ GOOD: Poison-safe for cleanup
pub fn restoration() {
    SAVED_TERMIOS.lock_raw(|result| {
        let mut guard = match result {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("Mutex poisoned, attempting restoration anyway");
                poisoned.into_inner()
            }
        };
        // ... perform restoration ...
    });
}
```

---

## 5. Ergonomic Atomics (AtomicU8Ext)

Manual `Ordering` is error-prone. Use `AtomicU8Ext` for clear, semantic operations with built-in `SeqCst` safety.

```rust
// ✅ GOOD: AtomicU8Ext
use crate::core::common::AtomicU8Ext;
let val = atomic.get();
atomic.increment();
```
