---
name: write-structured-tracing
description: Standard for writing structured tracing logs behind debug flags.
---

# Write Structured Tracing

This skill documents the project's standard for writing tracing logs (`tracing::info!`, `tracing::debug!`, etc.). It ensures logs are easily filterable, consistently formatted, and don't affect performance when disabled.

## 1. Gating Behind Debug Flags

All `tracing::*!` calls must be gated behind a specific debug flag from `tui/src/tui/mod.rs` (or similar location) using the `.then(|| { ... })` pattern. This ensures the tracing macro and any string allocations are completely bypassed when the flag is disabled.

```rust
crate::DEBUG_TUI_MOD.then(|| {
    // ... tracing call ...
});
```

## 2. The `// % is Display, ? is Debug.` Comment

You **MUST** add the exact line comment `// % is Display, ? is Debug.` directly above every `tracing::*!` invocation. This serves as a quick syntax reminder.

## 3. Structured Fields and `message`

Do not use unstructured string formatting (e.g., `tracing::info!("Hello {}", name)`). Instead, use structured fields with an explicit `message` key that identifies the context (e.g., the struct and method name).

```rust
crate::DEBUG_TUI_MOD.then(|| {
    // % is Display, ? is Debug.
    tracing::info! {
        message = "ComponentName::method_name",
        status = "Something happened",
    };
});
```

## 4. Using `inline_string!` for Complex Formatting

When you need to format complex strings within a tracing field, use the `inline_string!` macro and bind it to a field using the `%` (Display) modifier.

```rust
crate::DEBUG_TUI_MOD.then(|| {
    // % is Display, ? is Debug.
    tracing::info! {
        message = "AppNoLayout::handle_event",
        input_event = %inline_string!(
            "{a} {b:?}",
            a = glyphs::USER_INPUT_GLYPH,
            b = input_event
        )
    };
});
```

## Example Transformation

**Bad (Unstructured and Un-gated):**
```rust
tracing::debug!("Received input event: {:?}", input_event);
```

**Good (Structured and Gated):**
```rust
crate::DEBUG_TUI_PTY_MUX.then(|| {
    // % is Display, ? is Debug.
    tracing::debug! {
        message = "PTYMux::run_event_loop",
        input_event = %inline_string!("{:?}", input_event)
    };
});
```
