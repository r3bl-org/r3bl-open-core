---
name: write-structured-tracing
description: Standard for writing structured tracing logs behind debug flags.
---

# Write Structured Tracing

This skill documents the project's standard for writing tracing logs (`tracing::info!`, `tracing::debug!`, etc.). It ensures logs are easily filterable, consistently formatted, and don't affect performance when disabled.

## 1. Gating Behind Debug Flags

All `tracing::*!` calls must be gated behind a static boolean flag using the `.then(|| { ... })` pattern. This ensures the tracing macro and any string allocations are completely bypassed when the flag is disabled.

### Naming Convention (`DEBUG_<MODULE_NAME>`)
- **Debug flags:** Name every debug toggle `DEBUG_<MODULE_NAME>` in `SCREAMING_SNAKE_CASE`, matching the module or subsystem it guards (e.g., `DEBUG_SELF_UPGRADE`, `DEBUG_ANALYTICS_CLIENT`, `DEBUG_PTY_MUX`, `DEBUG_VT100_PARSER`, `DEBUG_SCRIPT`, `DEBUG_CHOOSE`, `DEBUG_SYN_HI`, `DEBUG_CLIPBOARD`).
  - Do **not** include redundant crate prefixes (e.g., avoid `DEBUG_TUI_*` for `core/` submodules) or extra verbs like `SHOW_` or `ENABLE_TRACE_`.
  - Related sub-flags extend the module name (e.g., `DEBUG_MD_PARSER` + `DEBUG_MD_PARSER_STDOUT`, `DEBUG_RENDER_PIPELINE` + `DEBUG_RENDER_PIPELINE_EXPANDED`).
- **Info-level telemetry toggles:** Prefix with `INFO_` (e.g., `INFO_DISPLAY_LOG_TELEMETRY`) so the log level is explicit in the constant name.

### Centralized Placement (One Location per Crate)
Define all debug flags centrally in a single file per crate so developers can inspect and toggle every flag in one place, re-exported at the crate root:
- `tui`: `tui/src/tui/mod.rs`
- `cmdr`: `cmdr/src/global_constants.rs`
- `rust-analyzer-mcp-server`: `rust-analyzer-mcp-server/src/constants.rs` (`debug_flags`)

**Choosing the right flag (Scope Specificity):**
Make sure the flag you use is specifically scoped to the module or subsystem you are debugging. This scope can be narrow or broad depending on the requirements.
- Do NOT reuse a broad orchestrator flag (e.g., `DEBUG_PTY_MUX`) for a microscopic, high-volume subsystem (like a byte-stream parser). Doing so would spam the orchestrator logs.
- If a suitable module-specific flag does not exist, create a new one (e.g., `DEBUG_VT100_PARSER`) in the crate's central toggle file to ensure developers can isolate and filter logs effectively without overwhelming the IO overhead.

```rust
DEBUG_TUI.then(|| {
    // ... tracing call ...
});
```

## 2. The `// % is Display, ? is Debug.` Comment

You **MUST** add the exact line comment `// % is Display, ? is Debug.` directly above every `tracing::*!` invocation. This serves as a quick syntax reminder.

## 3. Structured Fields and `message`

Do not use unstructured string formatting (e.g., `tracing::info!("Hello {}", name)`). Instead, use structured fields with an explicit `message` key that identifies the context (e.g., the struct and method name).

```rust
crate::DEBUG_TUI.then(|| {
    // % is Display, ? is Debug.
    tracing::info! {
        message = "ComponentName::method_name",
        status = "Something happened",
    };
});
```

## 4. Using `inline_string!` vs `format!` for Complex Formatting

When you need to format complex strings within a tracing field, bind it to a field using the `%` (Display) modifier and follow this rule:
- If the resulting string will be **<= 16 bytes**, use the `inline_string!` macro to stack-allocate it.
- If the resulting string will be **> 16 bytes**, use the standard `format!` macro (since `inline_string!` would spill to the heap and allocate anyway).

```rust
crate::DEBUG_TUI.then(|| {
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
DEBUG_PTY_MUX.then(|| {
    // % is Display, ? is Debug.
    tracing::debug! {
        message = "PTYMux::run_event_loop",
        input_event = %format!("{:?}", input_event)
    };
});
```
