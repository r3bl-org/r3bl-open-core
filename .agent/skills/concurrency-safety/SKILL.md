---
name: concurrency-safety
description: Concurrency safety patterns for R3BL - Chain of Custody, Loud Lock Releases, and Ergonomic Atomics. Use when working with threads, locks, or atomics.
---

# Concurrency Safety Skill

Apply these principles to ensure thread safety, prevent deadlocks, and maintain high-performance, predictable execution in multi-threaded code.

## When to Use

- Proactively when working with `Mutex`, `RwLock`, `Atomic`, or `Condvar`.
- When designing thread-safe modules like `Reactor`, `Monitor`, or `Engine`.
- When refactoring state management to use `AtomicU8Ext`.
- When troubleshooting deadlocks or race conditions.

## Core Principles

### 1. Scoped Access (Deadlock Prevention)

For simple shared state, use closure-based access to structurally prevent deadlocks by making it impossible to hold a lock guard longer than necessary.

**Guidelines:**

- **Closure-Based Access**: Restrict state access to closures. The closure's scope *is* the lock's scope.
- **Recursion Detection**: Use recursion detection (Safety-First) to prevent terminal-hanging deadlocks. Panic with a clear message if a recursive lock is detected.
- **Performance Opt-out**: For hot paths, opt-out of recursion detection at compile-time for zero overhead.
- **Fail-Fast vs. Poison-Safe**: Use Fail-Fast (panic) for normal logic, but Poison-Safe (recover) for cleanup paths to avoid Double Panic Aborts.

### 2. Chain of Custody (Guard Passing)

For complex state machines requiring `Condvar`, ensure linear ownership of locks by passing and returning `MutexGuard` by value.

**Guidelines:**

- Pass `MutexGuard` as a parameter to internal helper functions to guarantee they operate within an existing critical section.
- Return `MutexGuard` from state-reading closures (e.g., `read_state`) to force the caller to be intentional about the lock's lifetime.
- Avoid "Stale Guards" by ensuring the guard is either consumed or explicitly dropped.

### 3. Loud Lock Releases

Make critical section boundaries explicit and easy to audit by using `drop(guard)`.

**Guidelines:**

- **Explicit Release**: Always call `drop(guard)` immediately when a critical section ends, especially before function returns, macro calls, or long-running operations.
- **Auditability**: Explicit drops make it clear to reviewers (and the compiler) exactly where a lock is held, preventing accidental deadlocks during refactoring.
- **No Implicit Drops**: Do not rely on scope-based implicit drops for complex logic or when calling into external code/macros.

### 4. Friction as a Feature (Ergonomic API Design)

Design APIs that make it harder to mismanage concurrency.

**Guidelines:**

- **Type-Safe Subscriptions**: Use specific enums (like `SubscriptionStrategy`) instead of generic types to map thread states to execution paths at compile time.
- **Ergonomic Atomics**: Always use `AtomicU8Ext` (e.g., `get()`, `set()`, `increment()`) instead of raw `load()`/`store()` with manual `Ordering`.

### 5. Visibility for Documentation

Balance encapsulation with discoverability for documentation.

**Guidelines:**

- Make internal fields `pub` if they are targets of public documentation links.
- Rely on the "Barrel Export" pattern (`mod.rs` re-exports) to maintain actual module-level encapsulation while allowing `cargo doc` to resolve links correctly.

## Supporting Files

- `patterns.md` - Good and Bad examples of concurrency safety patterns.

## Related Skills

- `design-philosophy` - Core principles (Cognitive Load, Progressive Disclosure).
- `check-code-quality` - Includes concurrency safety in its final checklist.
