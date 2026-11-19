# Article: Const Expressions & Witness Tokens: Safely Capturing `Cargo.toml` at the Library/Binary Boundary

## Metadata
- **Target Site**: [developerlife.com](https://developerlife.com)
- **Status**: Draft Abstract & Outline
- **Topic**: Rust Macros, Const Expressions, Type-Driven Design, Zero-Cost Abstractions
- **Foundational Companion**: [`task/two-static-meanings-article.md`](two-static-meanings-article.md) (Part 1: Demystifying `'static` lifetime annotations vs trait bounds)

---

## What This Article Is About (Scope & Thesis)

This article provides an **advanced architectural blueprint** for safely capturing a binary's
`Cargo.toml` package metadata from a reusable library crate:

- **The Problem**: Calling `env!("CARGO_PKG_*")` inside a library crate captures the *library's*
  identity rather than the *consuming binary's* identity. Furthermore, `CARGO_PKG_*` variables
  do not exist at runtime (`std::env::var`), and parsing SemVer versions at application startup
  incurs runtime overhead and risks startup panics.
- **The Solution**: A declarative macro (`upgrade_journey!`) expands at the binary's call site,
  feeding Cargo's `CARGO_PKG_*` strings into an inline `const { ... }` block that evaluates
  `CrateVersion::from_cargo_parts` at compile time.
- **The Core Thesis**: Combining **call-site macro expansion**, **inline `const { ... }` blocks**,
  and the **Witness Token Pattern** shifts version validation entirely into `cargo build`. Illegal
  versions fail the compiler immediately, producing an infallible, owned witness token at runtime
  with zero overhead.

---

## Codebase Reference & Prior Art
- **Canonical In-Code Documentation**: [`tui/src/core/resilient_reactor_thread/rrt.rs#L368-L399`](file:///home/nazmul/github/roc-binaries/tui/src/core/resilient_reactor_thread/rrt.rs#L368-L399)
  - Generated Rustdoc anchor: `#const-expression-vs-const-declaration-vs-static-declaration`
  - Explains the critical differences between:
    1. **`const` expression**: Value computed by the compiler at compile time (`1 + 2`, `const { ... }`).
    2. **`const fn`**: Function callable within a const evaluation context.
    3. **`const` declaration**: Inlined at every use site (like macro expansion; address cannot be taken).
    4. **`static` declaration**: Single instance with a fixed memory address.
  - Highlights use-site divergence (e.g., mutating a `const` mutex creates fresh inlined instances and loses mutations, whereas `static` mutates the single shared instance).
- **Macro Implementation**: [`cmdr/src/self_upgrade/upgrade_journey/upgrade_journey_macro.rs#L83-L116`](file:///home/nazmul/github/roc-binaries/cmdr/src/self_upgrade/upgrade_journey/upgrade_journey_macro.rs#L83-L116)
- **Compile-Time Const Validation**: [`cmdr/src/self_upgrade/jsonc_manifest/crate_version.rs#L86-L100`](file:///home/nazmul/github/roc-binaries/cmdr/src/self_upgrade/jsonc_manifest/crate_version.rs#L86-L100)

---

## Where `const expr` and `'static` Intersect

This architecture forms the direct practical bridge between compile-time `const` evaluation and
thread-safe `'static` type bounds:

```text
┌────────────────────────────────────────────────────────────────────────┐
│                        COMPILE-TIME CONTEXT                            │
├────────────────────────────────────────────────────────────────────────┤
│ 1. env!("CARGO_PKG_*")                                                 │
│    └─► Produces compile-time string literals with type: &'static str   │
│                                                                        │
│ 2. const { CrateVersion::from_cargo_parts(...) }                       │
│    └─► Evaluates const fn at compile time; assert!() fails cargo build │
│        if pre-release or out of range                                  │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼ Emits at zero runtime cost
┌────────────────────────────────────────────────────────────────────────┐
│                          RUNTIME CONTEXT                               │
├────────────────────────────────────────────────────────────────────────┤
│ 3. CrateVersion (Witness Token)                                        │
│    └─► An owned struct (holding u16 fields, zero borrows)              │
│    └─► Satisfies T: 'static trait bound ("won't dangle")               │
│                                                                        │
│ 4. std::thread::spawn(move || ...)                                     │
│    └─► Requires F: Send + 'static. Because CrateVersion is owned, it   │
│        safely crosses the thread boundary into the upgrade journey     │
│        background thread with guaranteed thread safety!                │
└────────────────────────────────────────────────────────────────────────┘
```

1. **Input is `'static` by Promotion**: `env!("CARGO_PKG_*")` produces `&'static str` literals
   baked into `.rodata`.
2. **Computation is a `const expr`**: The inline `const { ... }` block forces the compiler to
   execute validation during `cargo build`, turning runtime panics into compiler errors.
3. **Output Satisfies `'static` by Ownership**: The `const` block produces an owned `CrateVersion`
   struct. Because it owns its fields and has no borrowed references, it implements `T: 'static`.
4. **Execution Crosses Thread Boundaries**: The self-upgrade engine spawns a dedicated OS thread
   (`std::thread::spawn`), which mandates `Send + 'static`. The witness token satisfies this bound
   cleanly, providing an unbroken chain from compile-time validation to multithreaded runtime
   execution.

---

## Abstract

When designing reusable Rust libraries that embed into downstream binary crates—such as
CLI self-upgraders, telemetry harnesses, or diagnostic frameworks—capturing package
identity from `Cargo.toml` (`CARGO_PKG_NAME`, `CARGO_PKG_VERSION`) presents a subtle
architectural challenge:

1. **The Library Boundary Trap**: Calling [`env!("CARGO_PKG_NAME")`] inside a library
   function captures the *library's* package metadata, permanently baking the wrong name
   and version into the downstream executable.
2. **Missing at Runtime**: Cargo environment variables exist strictly during `cargo build`.
   They cannot be fetched via `std::env::var` when users run the deployed binary.
3. **Runtime Fallibility & Panics**: Parsing SemVer strings at application startup introduces
   unnecessary runtime parsing overhead, `Result` unwrapping, and the risk of runtime panics
   if a pre-release or malformed version string is supplied in `Cargo.toml`.

This article demonstrates how to combine **declarative macros (`macro_rules!`)**, Rust's
inline **`const { ... }` expressions**, and the **Witness Token Pattern** to achieve
zero-overhead, compile-time verified package identity capture. By evaluating
[`CrateVersion::from_cargo_parts`] inside an inline `const` block within the
[`upgrade_journey!`] macro, downstream binary crates capture their own `Cargo.toml` variables
at the call site. Furthermore, any invalid version or disallowed pre-release suffix fails
at compile time (`cargo build`) with a clear assertion message—guaranteeing that the
constructed version token is an infallible witness at runtime.

---

## Article Outline & Proposed Sections

### 1. The Architectural Dilemma: Library vs. Binary Boundaries
- Multi-crate workspace architecture: `r3bl-cmdr` provides a generic self-upgrade engine
  used by downstream CLI binaries (`edi`, `giti`, `r3bl-rust-analyzer-mcp-server`).
- The naive approach: Calling `env!("CARGO_PKG_NAME")` in `UpgradeConfig::new()`.
- The compilation pitfall: `env!` expands when compiling the library crate, baking
  `"r3bl-cmdr"` into every dependent binary!
- Why runtime environment queries (`std::env::var`) fail in distributed production binaries.

### 2. Step 1: Call-Site Macro Expansion
- Why declarative macros (`macro_rules!`) solve the lexical boundary:
  - Macros expand inside the **consuming binary's crate context**.
  - `env!("CARGO_PKG_NAME")` inside the macro resolves against the calling binary's
    `Cargo.toml`.
- Eliminating caller boilerplate: Downstream binaries invoke a single clean macro
  (`upgrade_journey!(channel, args)`) without manually re-typing version and package names.

### 3. Step 2: The 4-Way `const` Distinction & Why Inline `const { ... }` Blocks Matter
- Drawing from [`rrt.rs#L368-L399`](file:///home/nazmul/github/roc-binaries/tui/src/core/resilient_reactor_thread/rrt.rs#L368-L399):
  - Why not a `static` declaration? (`static V: CrateVersion`): Requires a fixed global
    memory address in `.data`/`.bss`, inappropriate for a local configuration value.
  - Why not a `const` declaration? (`const V: CrateVersion`): Inlines a fresh copy at every
    use site and forces naming a top-level or scoped item (`const V = ...`), which is noisy
    and awkward inside a macro body.
  - Why **inline `const { ... }` blocks (Rust 1.79+)** are the ideal solution:
    Evaluates expressions at compile time directly in expression position, producing an
    ephemeral owned value without declaring any named items or global addresses.

### 4. Step 3: The Witness Token Pattern & Compile-Time Panics
- What is a Witness Token?
  - A type whose very existence proves that specific validation rules have already been
    verified.
  - Making illegal states unrepresentable: `CrateVersion` cannot be instantiated with
    invalid parts.
- Compile-time assertion mechanics:
  ```rust
  pub const fn from_cargo_parts(major: &str, minor: &str, patch: &str, pre: &str) -> Self {
      assert!(
          pre.is_empty(),
          "CARGO_PKG_VERSION_PRE must be empty (pre-release versions are disallowed)"
      );
      // const-compatible parsing using u16::from_str_radix
      ...
  }
  ```
- Contrast the developer experience:
  - *Runtime failure*: Binary boots up in production and panics on a customer's machine.
  - *Compile-time failure*: `cargo build` immediately aborts with an evaluation error:
    `"evaluation of constant value failed: CARGO_PKG_VERSION_PRE must be empty"`.

### 5. Deep Dive: `upgrade_journey!` Implementation
- Step-by-step walkthrough of the real implementation from `r3bl-cmdr`:
  ```rust
  #[macro_export]
  macro_rules! upgrade_journey {
      ($user_feedback_channel:expr, $upgrade_cli_args:expr) => {{
          let crate_name = ::core::env!("CARGO_PKG_NAME");

          // XMARK: Use const expression for compile time (witness) validation
          let current_version = const {
              $crate::CrateVersion::from_cargo_parts(
                  ::core::env!("CARGO_PKG_VERSION_MAJOR"),
                  ::core::env!("CARGO_PKG_VERSION_MINOR"),
                  ::core::env!("CARGO_PKG_VERSION_PATCH"),
                  ::core::env!("CARGO_PKG_VERSION_PRE"),
              )
          };
          ...
      }};
  }
  ```
- Handling `const`-compatible parsing: Why `str::parse()` and `Result::unwrap()` are not
  allowed in `const fn`, and how `u16::from_str_radix` solves it cleanly.

### 6. Summary of Key Design Principles
- **Shift verification left**: Move data validation from runtime initialization to compile
  time.
- **Macros for call-site context**: Use `macro_rules!` when compilation environment variables
  must bind to the consumer, not the producer.
- **`const` blocks as zero-cost gates**: Use `const { ... }` to enforce that parsing logic
  executes during `cargo build`, turning runtime panics into compiler errors.
