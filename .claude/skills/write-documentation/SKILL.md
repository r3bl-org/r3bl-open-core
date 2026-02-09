---
name: write-documentation
description: Write and format Rust documentation correctly. Apply proactively when writing code with rustdoc comments (//! or ///). Covers voice & tone, prose style (opening lines, explicit subjects, verb tense), structure (inverted pyramid), intra-doc links (crate:: paths, reference-style), constant conventions (binary/byte literal/decimal), and formatting (cargo rustdoc-fmt). Also use retroactively via /fix-intradoc-links, /fix-comments, or /fix-md-tables commands.
---

# Writing Good Rust Documentation

This consolidated skill covers all aspects of writing high-quality rustdoc:

1. **Voice & Tone** - Serious, meaningful, precise, and fun
2. **Prose Style** - Opening lines, explicit subjects, verb tense
3. **Structure** - Inverted pyramid principle
4. **Links** - Intra-doc link patterns
5. **Constants** - Human-readable numeric literals
6. **Formatting** - Markdown tables and cargo rustdoc-fmt

## When to Use

### Proactively (While Writing Code)

- Writing new code that includes `///` or `//!` doc comments
- Creating new modules, traits, structs, or functions
- Adding links to other types or modules in documentation
- Defining byte/u8 constants

### Retroactively (Fixing Issues)

- `/fix-intradoc-links` - Fix broken links, convert inline to reference-style
- `/fix-comments` - Fix constant conventions in doc comments
- `/fix-md-tables` - Fix markdown table formatting
- `/docs` - Full documentation check and fix

---

## Voice & Tone

**r3bl is serious & meaningful & precise. r3bl is also fun.**

Documentation should be rigorous about content, playful about presentation:

| Aspect | Serious & Precise | Fun |
|--------|-------------------|-----|
| **Technical accuracy** | Correct terminology, proper distinctions | — |
| **Links** | Intra-doc links, authoritative sources | — |
| **Visual aids** | ASCII diagrams, tables | Emoji for scannability |
| **Language** | Clear, unambiguous | Literary references, personality |

### Examples

**Emoji for visual scanning** (semantic, not decorative):
```rust
//! 🐧 **Linux**: Uses `epoll` for I/O multiplexing
//! 🍎 **macOS**: Uses `kqueue` (with PTY limitations)
//! 🪟 **Windows**: Uses IOCP for async I/O
```

**Severity with visual metaphors:**
```rust
//! 1. 🐢 **Multi-threaded runtime**: Reduced throughput but still running
//! 2. 🧊 **Single-threaded runtime**: Total blockage — nothing else runs
```

**Literary references with layered meaning:**
```rust
//! What's in a name? 😛 The three core properties:
```
The 😛 is a visual pun on "tongue in cheek" — Shakespeare's Juliet argues names *don't* matter,
but here we use the quote to explain why RRT's name *does* matter. The emoji signals the irony.

**Rule:** Emoji must have semantic meaning (OS icons, severity levels). Never use random 🚀✨🎉 for "excitement."

### Unicode Over Emoji in Diagrams

For ASCII art diagrams in rustdoc, **use standard Unicode characters** instead of emoji. Emoji
require special font support (Nerd Fonts, emoji fonts) and may not render correctly on all
systems. Standard Unicode box-drawing and symbol characters render reliably everywhere.

#### Box-Drawing Characters

See [`docs/boxes.md`] for the complete reference. Common patterns:

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                         Box with header                                 │
├─────────────────────────────────────────────────────────────────────────┤
│   Content here                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

#### Arrows

| Use | Instead of | Unicode |
|-----|------------|---------|
| `→` | `➡️` | U+2192 RIGHTWARDS ARROW |
| `←` | `⬅️` | U+2190 LEFTWARDS ARROW |
| `▼` | `⬇️` | U+25BC BLACK DOWN-POINTING TRIANGLE |
| `▲` | `⬆️` | U+25B2 BLACK UP-POINTING TRIANGLE |
| `►` | `▶️` | U+25BA BLACK RIGHT-POINTING POINTER |
| `◄` | `◀️` | U+25C4 BLACK LEFT-POINTING POINTER |

#### Status/Result Indicators

| Use | Instead of | Unicode | Meaning |
|-----|------------|---------|---------|
| `✓` | `✅` | U+2713 CHECK MARK | Success/yes |
| `✗` | `❌` | U+2717 BALLOT X | Failure/no |
| `✘` | `❌` | U+2718 HEAVY BALLOT X | Failure/no (bold) |

#### Example: Before and After

```rust
// ❌ Bad: Emoji may not render correctly
//! Timeline: create ──► spawn ──► ❌ fails

// ✓ Good: Standard Unicode renders everywhere
//! Timeline: create ──► spawn ──► ✗ fails
```

**Exception:** OS-identifying emoji (🐧 🍎 🪟) are acceptable in prose because they're semantic
and commonly supported. But in ASCII art diagrams, stick to standard Unicode.

[`docs/boxes.md`]: ../../../docs/boxes.md

---

## Prose Style

Doc comments should read naturally and have clear subjects. Avoid abrupt sentence starts.

### Opening Lines by Item Type

The first line/paragraph of a doc comment should describe **what the item IS**, not what it does.
Follow Rust std conventions.

**IMPORTANT: The first paragraph must be separate.** Rustdoc uses it as the **summary** in:
- Module listings (each item shows only its first paragraph)
- IDE tooltips and autocomplete
- Search results

```rust
// ❌ Bad: Summary and details merged
/// A trait for creating workers. This trait solves the chicken-egg problem.

// ✅ Good: Summary is separate paragraph
/// A trait for creating workers.
///
/// This trait solves the chicken-egg problem.
```

#### Structs — Noun Phrase

Start with "A/An [noun]..." describing what it is:

```rust
// From std:
/// A contiguous growable array type, written as `Vec<T>`, short for 'vector'.
pub struct Vec<T> { ... }

/// A UTF-8–encoded, growable string.
pub struct String { ... }

/// A mutual exclusion primitive useful for protecting shared data.
pub struct Mutex<T> { ... }

// Our style:
/// A thread-safe container for managing worker thread lifecycle.
pub struct ThreadSafeGlobalState<F> { ... }

/// An offscreen buffer for testing terminal rendering.
pub struct OffscreenBuffer { ... }
```

#### Enums — What It Represents

Start with "A/An [noun]..." or "The [type]...":

```rust
// From std:
/// An `Ordering` is the result of a comparison between two values.
pub enum Ordering { Less, Equal, Greater }

/// An IP address, either IPv4 or IPv6.
pub enum IpAddr { V4(...), V6(...) }

// Our style:
/// An indication of whether the worker thread is running or terminated.
pub enum LivenessState { Running, Terminated }

/// A decision about whether the worker thread should shut down.
pub enum ShutdownDecision { ContinueRunning, ShutdownNow }
```

#### Traits — "A trait for..."

```rust
// Our style:
/// A trait for creating the coupled [`Worker`] + [`Waker`] pair atomically.
pub trait RRTFactory { ... }

/// A trait for implementing the blocking I/O loop on the dedicated RRT thread.
pub trait RRTWorker { ... }
```

#### Methods & Functions — Third-Person Verb

Start with what the method/function **does** using third-person:

```rust
// From std:
/// Constructs a new, empty `Vec<T>`.
pub fn new() -> Vec<T> { ... }

/// Returns the number of elements in the vector.
pub fn len(&self) -> usize { ... }

/// Appends an element to the back of a collection.
pub fn push(&mut self, value: T) { ... }

/// Returns the contained `Some` value, consuming the `self` value.
pub fn unwrap(self) -> T { ... }

// Our style:
/// Creates new thread state with fresh liveness tracking.
pub fn new(waker: W) -> Self { ... }

/// Checks if the thread should self-terminate.
pub fn should_self_terminate(&self) -> ShutdownDecision { ... }
```

#### Associated Types — "The type of..." or "The type..."

Follow the Rust std convention (e.g., `Iterator::Item`, `Future::Output`):

```rust
// From std:
/// The type of the elements being iterated over.
type Item;

/// The type of value produced on completion.
type Output;

// Our style (user-provided types use "Your type"):
/// The type broadcast from your [`Worker`] to async subscribers.
type Event;

/// Your type implementing one iteration of the blocking I/O loop.
type Worker: RRTWorker<Event = Self::Event>;

/// Your type for interrupting the blocked dedicated RRT worker thread.
type Waker: RRTWaker;
```

**Pattern:** Use "The type [verb]..." or "Your concrete type [verb]..." where the verb
describes what the type does:
- "The concrete type broadcast..." (Event — gets broadcast)
- "Your concrete type implementing..." (Worker — user provides this)
- "Your concrete type for..." (Waker — user provides this)

**When to use "Your concrete type":** For associated types that the user must provide —
types with trait bounds like `: RRTWorker`. The word "concrete" emphasizes they provide
an actual struct/enum, not just satisfy an abstract contract.

**When to use "of":** Only when describing what a type *contains* rather than what it *is*:
- std's `Iterator::Item`: "The type **of the elements**..." — Item contains elements
- std's `Future::Output`: "The type **of value**..." — Output contains a value

**Parenthetical clarifiers:** When context is needed, use parentheticals:
```rust
/// Your concrete type (that implements this method) is an injected dependency...
```

**Gold standard:** See [`RRTFactory`] in `tui/src/core/resilient_reactor_thread/types.rs`
for a complete example of complex trait documentation with associated types.

[`RRTFactory`]: crate::core::resilient_reactor_thread::RRTFactory

#### Constants — Noun Phrase

```rust
/// Capacity of the broadcast channel for events.
pub const CHANNEL_CAPACITY: usize = 4_096;

/// ESC byte (0x1B in hex).
pub const ANSI_ESC: u8 = 27;
```

#### Quick Reference Table

| Item Type | Pattern | Example Opening |
|-----------|---------|-----------------|
| **Struct** | `A/An [noun]...` | `A thread-safe container for...` |
| **Enum** | `A/An [noun]...` | `An indication of whether...` |
| **Trait** | `A trait for...` | `A trait for creating...` |
| **Associated Type** (user-provided) | `Your concrete type [verb]...` | `Your concrete type implementing...` |
| **Associated Type** (framework) | `The concrete type [verb]...` | `The concrete type broadcast...` |
| **Method** | Third-person verb | `Returns the...`, `Creates a...` |
| **Function** | Third-person verb | `Constructs a new...`, `Checks if...` |
| **Constant** | Noun phrase | `Capacity of the...`, `ESC byte...` |

### Module-Level Docs for Single-Type Files

When a file contains primarily one struct, enum, or trait, keep module docs minimal — just
identify the file's purpose and link to the main type:

#### Single Type — Link to It

```rust
//! Thread-safe global state manager for the Resilient Reactor Thread pattern. See
//! [`ThreadSafeGlobalState`] for details.
```

```rust
//! Shared state container for the Resilient Reactor Thread pattern. See [`ThreadState`].
```

```rust
//! [RAII] subscription guard for the Resilient Reactor Thread pattern. See
//! [`SubscriberGuard`].
//!
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
```

#### Multiple Types — Bullet List

When a file contains multiple related types, use a brief intro + bullet list:

```rust
//! Core traits for the Resilient Reactor Thread (RRT) pattern.
//!
//! - [`RRTFactory`]: Creates coupled worker thread + waker
//! - [`RRTWorker`]: Work loop running on the thread
//! - [`RRTWaker`]: Interrupt a blocked thread
//!
//! See [module docs] for the full RRT pattern explanation.
//!
//! [module docs]: super
```

```rust
//! Thread liveness tracking for the Resilient Reactor Thread pattern. See
//! [`ThreadLiveness`], [`LivenessState`], and [`ShutdownDecision`].
```

**Why minimal?** The detailed documentation belongs on the types themselves (inverted pyramid).
Module docs just help readers navigate to the right type. Don't duplicate content.

### Follow-Up Sentences Need Explicit Subjects

After the opening line, subsequent sentences should use explicit subjects — don't start with
verbs that leave the subject ambiguous:

#### ❌ Bad: Abrupt Starts

```rust
/// A trait for interrupting blocked threads.
///
/// Called by `SubscriberGuard::drop()` to signal shutdown.
```

What's "called"? The trait? A method? The reader must guess.

#### ✅ Good: Explicit Subjects

```rust
/// A trait for interrupting blocked threads.
///
/// [`SubscriberGuard::drop()`] calls [`wake()`] on implementors of this trait to signal
/// shutdown.
```

Now it's clear: the *method* is what's being called, on *implementors* of the trait.

**Note:** Traits themselves aren't "called" — methods are. Say what valid actions
a trait can take: "This trait **solves**...", "This trait **requires**...",
"This trait **defines**...". Don't say "This trait is called...".

### Common Patterns to Fix

| Abrupt Start | Fix With Explicit Subject |
|--------------|---------------------------|
| `Called by...` | `[`Foo::bar()`] calls this method...` or `This method is called by...` |
| `Returned by...` | `This enum is returned by...` |
| `Used to...` | `This struct is used to...` |
| `Manages...` | `This struct manages...` |
| `Centralizes...` | `This module centralizes...` |
| `Solves...` | `This trait solves...` |

### Method Doc Verb Tense

Methods should use **third-person** verbs (like Rust std docs), not imperative:

| ❌ Imperative | ✅ Third-Person |
|---------------|-----------------|
| `Create a new buffer.` | `Creates a new buffer.` |
| `Return the length.` | `Returns the length.` |
| `Check if empty.` | `Checks if empty.` |
| `Subscribe to events.` | `Subscribes to events.` |

**Why third-person?** It reads naturally as "This method *creates*..." without needing to say
"This method". Imperative form ("Create...") sounds like a command to the reader.

### Self-Reference in Different Contexts

| Context | Self-Reference |
|---------|----------------|
| Trait doc | `This trait...` |
| Struct doc | `This struct...` |
| Enum doc | `This enum...` |
| Module doc (`//!`) | `This module...` |
| Method doc | Implicit (verb alone) or `This method...` |
| Associated type doc | `This type...` |

### Section Headings for Reference Implementations

Use `# Example` (not `# Concrete Implementation`) when linking to reference implementations:

```rust
// ❌ Bad: Sounds like THE canonical implementation
/// # Concrete Implementation
///
/// See [`MioPollWorker`] for a concrete implementation.

// ✅ Good: Idiomatic Rust, implies there could be others
/// # Example
///
/// See [`MioPollWorker`] for an example implementation.
```

**Why `# Example`?**
- Matches Rust std lib conventions
- "example implementation" signals "one of potentially many"
- "concrete implementation" sounds like THE canonical choice

---

## Part 1: Structure (Inverted Pyramid)

Structure documentation with high-level concepts at the top, details below:

```text
╲────────────╱
 ╲          ╱  High-level concepts - Module/trait/struct documentation
  ╲────────╱
   ╲      ╱  Mid-level details - Method group documentation
    ╲────╱
     ╲  ╱  Low-level specifics - Individual method documentation
      ╲╱
```

**Avoid making readers hunt through method docs for the big picture.**

### Placement Guidelines

| Level | What to Document | Example Style |
|-------|------------------|---------------|
| **Module/Trait** | Why, when, conceptual examples, workflows, ASCII diagrams | Comprehensive |
| **Method** | How to call, exact types, parameters | Brief (IDE tooltips) |

### Reference Up, Not Down

```rust
/// See the [module-level documentation] for complete usage examples.
///
/// [module-level documentation]: mod@crate::example
pub fn some_method(&self) -> Result<()> { /* ... */ }
```

---

## Part 2: Intra-doc Links

### Golden Rules

1. **Use `crate::` paths** (not `super::`) - absolute paths are stable
2. **Use reference-style links** - keep prose clean
3. **Place all link definitions at bottom** of comment block
4. **Include `()` for functions/methods** - distinguishes from types

### Link Source Priority

When deciding local vs external links, follow this priority:

| Priority | Source | Link Style | Example |
|----------|--------|------------|---------|
| 1 | Code in this monorepo | `crate::` path | `[`Foo`]: crate::module::Foo` |
| 2 | Dependency in Cargo.toml | Crate path | `[`mio`]: mio` |
| 3 | OS/CS/hardware terms | External URL | `[`epoll`]: https://man7.org/...` |
| 4 | Pedagogical/domain terms | Wikipedia URL | `[design pattern]: https://en.wikipedia.org/...` |
| 5 | Non-dependency crates | docs.rs URL | `[`rayon`]: https://docs.rs/rayon` |

**Key principle:** If it's in Cargo.toml, use local links (validated, offline-capable, version-matched).

### Link All Symbols for Refactoring Safety

**Every codebase symbol in backticks must be a link.** This isn't just style—it's safety.

When you rename, move, or delete a symbol:
- **With links**: `cargo doc` fails with a clear error pointing to the stale reference
- **Without links**: The docs silently rot, referencing symbols that no longer exist

| Docs say | Symbol renamed to | With link | Without link |
|----------|-------------------|-----------|--------------|
| `` [`Parser`] `` | `Tokenizer` | ❌ Build error | ✅ Silently stale |
| `` [`process()`] `` | `handle()` | ❌ Build error | ✅ Silently stale |

**Rule:** If it's a symbol from your codebase and it's in backticks, make it a link.

```rust
// ❌ Bad: Will silently rot when Parser is renamed
/// Uses `Parser` for tokenization.

// ✅ Good: cargo doc will catch if Parser is renamed
/// Uses [`Parser`] for tokenization.
///
/// [`Parser`]: crate::Parser
```

### Quick Reference

| Link To | Pattern |
|---------|---------|
| Struct | `[`Foo`]: crate::Foo` |
| Function | `[`process()`]: crate::process` |
| Method | `[`run()`]: Self::run` |
| Module | `[`parser`]: mod@crate::parser` |
| Section heading | `[`docs`]: mod@crate::module#section-name` |
| Dependency crate | `[`tokio::spawn()`]: tokio::spawn` |

### ✅ Good: Reference-Style Links

```rust
/// This struct uses [`Position`] to track cursor location.
///
/// The [`render()`] method updates the display.
///
/// [`Position`]: crate::Position
/// [`render()`]: Self::render
```

### ❌ Bad: Inline Links

```rust
/// This struct uses [`Position`](crate::Position) to track cursor location.
```

### ❌ Bad: No Links

```rust
/// This struct uses `Position` to track cursor location.
```

### Linking to Dependency Crates

For crates listed in your `Cargo.toml` dependencies, **use direct intra-doc links** instead of
external hyperlinks to docs.rs. Rustdoc automatically resolves these when the dependency is built.

| Link To | Pattern |
|---------|---------|
| Crate root | `[`crossterm`]: ::crossterm` |
| Type in crate | `[`mio::Poll`]: mio::Poll` |
| Function in crate | `[`tokio::io::stdin()`]: tokio::io::stdin` |
| Macro in crate | `[`tokio::select!`]: tokio::select` |

#### ✅ Good: Direct Dependency Links

```rust
//! **UI freezes** on terminal resize when using [`tokio::io::stdin()`].
//! Internally, cancelling a [`tokio::select!`] branch doesn't stop the read.
//! However, the use of [Tokio's stdin] caused the first two issues.
//!
//! [`tokio::select!`]: tokio::select
//! [`tokio::io::stdin()`]: tokio::io::stdin
//! [Tokio's stdin]: tokio::io::stdin
```

```rust
/// Uses [`mio::Poll`] to efficiently wait on file descriptor events.
///
/// [`mio::Poll`]: mio::Poll
```

```rust
//! Use [`crossterm`]'s `enable_raw_mode` for terminal input.
//!
//! [`crossterm`]: ::crossterm
```

#### ❌ Bad: External docs.rs Links for Dependencies

```rust
/// Uses [mio::Poll](https://docs.rs/mio/latest/mio/struct.Poll.html) to wait.
```

Don't use docs.rs URLs for crates that are **already in your `Cargo.toml`**.

**Why direct links are better for dependencies:**
- Clickable in local `cargo doc` output (works offline)
- Version-matched to your actual dependency version
- Validated by rustdoc (broken links caught at build time)
- Consistent style with internal crate links

#### ✅ OK: External docs.rs Links for Non-Dependencies

For crates that are **not** in your `Cargo.toml`, external links are fine:

```rust
/// This is similar to how [rayon](https://docs.rs/rayon) handles parallel iteration.
```

Since `rayon` isn't a dependency, there's no local documentation to link to.

#### ✅ OK: External Links for OS/CS/Hardware Terminology

For operating system concepts, computer science terminology, or hardware references that **aren't Rust crates**,
use external URLs (man pages, Wikipedia, specs):

```rust
//! Uses [`epoll`] for efficient I/O multiplexing on Linux.
//! Implements the [`Actor`] pattern for message passing.
//! Reads from [`stdin`] which is a [`file descriptor`].
//!
//! [`epoll`]: https://man7.org/linux/man-pages/man7/epoll.7.html
//! [`Actor`]: https://en.wikipedia.org/wiki/Actor_model
//! [`stdin`]: std::io::stdin
//! [`file descriptor`]: https://en.wikipedia.org/wiki/File_descriptor
```

**Common external link targets:**

| Type | URL Pattern | Example |
|------|-------------|---------|
| Linux syscalls/APIs | `man7.org/linux/man-pages/` | `epoll`, `signalfd`, `io_uring` |
| BSD APIs | `man.freebsd.org/` | `kqueue` |
| CS concepts | `en.wikipedia.org/wiki/` | `Actor model`, `Reactor pattern` |
| Pedagogical terms | `en.wikipedia.org/wiki/` | `design pattern`, `RAII`, `file descriptor` |
| Specs/RFCs | Official spec sites | ANSI escape codes, UTF-8 |

**Key distinction:**
- `mio` (Rust crate in Cargo.toml) → `[`mio`]: mio` (local)
- `epoll` (Linux kernel API) → `[`epoll`]: https://man7.org/...` (external)

### Pedagogical Links for Inclusivity

Link domain-specific terminology to external references (typically Wikipedia) even when the
concept seems "obvious." This makes documentation accessible to readers of all backgrounds —
not everyone comes from a CS degree or has the same experience level.

**Rule:** If a term has a formal definition that would help a newcomer understand the docs,
link it. The cost of an extra link is near zero; the cost of excluding a reader is high.

```rust
// ✅ Good: Links pedagogical terms for inclusivity
//! This [design pattern] avoids all of this and allows async code to...
//! Resources are cleaned up via [RAII] when the guard is dropped.
//!
//! [design pattern]: https://en.wikipedia.org/wiki/Software_design_pattern
//! [RAII]: https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization
```

```rust
// ❌ Bad: Assumes reader already knows these terms
//! This design pattern avoids all of this and allows async code to...
//! Resources are cleaned up via RAII when the guard is dropped.
```

**Common pedagogical link targets:**

| Term | URL |
|------|-----|
| design pattern | `https://en.wikipedia.org/wiki/Software_design_pattern` |
| RAII | `https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization` |
| file descriptor | `https://en.wikipedia.org/wiki/File_descriptor` |
| dependency injection | `https://en.wikipedia.org/wiki/Dependency_injection` |
| inversion of control | `https://en.wikipedia.org/wiki/Inversion_of_control` |
| Actor model | `https://en.wikipedia.org/wiki/Actor_model` |
| Reactor pattern | `https://en.wikipedia.org/wiki/Reactor_pattern` |

> **Note:** The link source priority is also documented in `link-patterns.md`. This redundancy is
> intentional—SKILL.md content is loaded when the skill triggers, ensuring reliable application
> during doc generation. Supporting files require explicit reads and serve as detailed reference.

---

## Part 3: Constant Conventions

Use human-readable numeric literals for byte constants:

| Type | Format | Example |
|------|--------|---------|
| **Bitmasks** (used in `&`, `\|`, `^`) | Binary | `0b0110_0000` |
| **Printable ASCII** | Byte literal | `b'['` |
| **Non-printable bytes** | Decimal | `27` |
| **Comments** | Show hex | `// (0x1B in hex)` |

### ✅ Good: Human-Readable

```rust
/// ESC byte (0x1B in hex).
pub const ANSI_ESC: u8 = 27;

/// CSI bracket byte: `[` (91 decimal, 0x5B hex).
pub const ANSI_CSI_BRACKET: u8 = b'[';

/// Mask to convert control character to lowercase (0x60 in hex).
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0b0110_0000;
```

### ❌ Bad: Hex Everywhere

```rust
pub const ANSI_ESC: u8 = 0x1B;
pub const ANSI_CSI_BRACKET: u8 = 0x5B;
pub const CTRL_TO_LOWERCASE_MASK: u8 = 0x60;
```

**For detailed conventions, see `constant-conventions.md` in this skill.**

---

## Part 4: Formatting

### Run cargo rustdoc-fmt

```bash
# Format specific file
cargo rustdoc-fmt path/to/file.rs

# Format all git-changed files
cargo rustdoc-fmt

# Format entire workspace
cargo rustdoc-fmt --workspace
```

**What it does:**
- Formats markdown tables with proper column alignment
- Converts inline links to reference-style
- Preserves code examples

**If not installed:**
```bash
cd build-infra && cargo install --path . --force
```

### Markdown Table Alignment

**Always use left-aligned columns** in markdown tables. This is the default and most readable
alignment for technical documentation.

#### Alignment Syntax

```markdown
| Left-aligned | Left-aligned | Left-aligned |
| :----------- | :----------- | :----------- |
| data         | data         | data         |
```

The `:` on the left side of the dashes indicates left alignment. While the `:` is optional for
left alignment (it's the default), **always include it explicitly** for consistency.

#### ✅ Good: Left-Aligned (Default)

```markdown
| Item Type | Pattern | Example |
| :-------- | :------ | :------ |
| Struct    | `A/An`  | `A thread-safe container...` |
| Trait     | `A trait for` | `A trait for creating...` |
```

Renders as:

| Item Type | Pattern | Example |
| :-------- | :------ | :------ |
| Struct    | `A/An`  | `A thread-safe container...` |
| Trait     | `A trait for` | `A trait for creating...` |

#### ❌ Avoid: Center or Right Alignment

```markdown
| Item Type | Pattern | Example |
| :-------: | ------: | :-----: |
| Struct    | `A/An`  | `A thread-safe container...` |
```

Center (`:---:`) and right (`---:`) alignment are harder to scan and rarely appropriate for
technical docs. Use them only when the content semantically requires it (e.g., numeric columns
that should right-align for decimal alignment).

#### Why Left-Align?

- **Scannability** — Eyes naturally start at the left margin
- **Consistency** — All tables look the same throughout the codebase
- **Prose readability** — Technical descriptions flow better left-to-right
- **Code snippets** — Backtick content is easier to read left-aligned

### Verify Documentation Builds

```bash
./check.fish --doc
# (runs: cargo doc --no-deps)

./check.fish --test
# (runs: cargo test --doc)
```

---

## Code Examples in Docs

**Golden Rule:** Don't use `ignore` unless absolutely necessary.

| Scenario | Use |
|----------|-----|
| Example compiles and runs | ` ``` ` (default) |
| Compiles but shouldn't run | ` ```no_run ` |
| Can't make it compile | Link to real code instead |
| Macro syntax | ` ```ignore ` with HTML comment explaining why |

### Linking to Test Modules and Functions

```rust
/// See [`test_example`] for actual usage.
///
/// [`test_example`]: crate::tests::test_example
```

Make test module visible to docs:
```rust
#[cfg(any(test, doc))]
pub mod tests;
```

#### Platform-Specific Test Modules

**When you see this warning:**
> "unresolved link to `crate::path::test_module`"
>
> And the module is `#[cfg(test)]` only

**Don't give up on links** — Add conditional visibility instead of using plain text:

```rust
// Before (links won't resolve):
#[cfg(test)]
mod backend_tests;

// After (links resolve in docs):
#[cfg(any(test, doc))]
pub mod backend_tests;
```

#### Cross-Platform Docs for Platform-Specific Code

For code that only runs on specific platforms (e.g., Linux) but should have docs generated on **all
platforms** (so developers on macOS can read them locally):

```rust
// ❌ Broken: Docs won't generate on macOS!
#[cfg(all(target_os = "linux", any(test, doc)))]
pub mod linux_only_module;

// ✅ Fixed: Docs generate on all platforms, tests run only on Linux
#[cfg(any(doc, all(target_os = "linux", test)))]
pub mod linux_only_module;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod linux_only_module;

// Re-exports also need the doc condition
#[cfg(any(target_os = "linux", doc))]
pub use linux_only_module::*;
```

**Key insight:** The `doc` cfg flag doesn't override other conditions—it's just another flag. Use
`any(doc, ...)` to make documentation an **alternative path**, not an additional requirement:

| Pattern | Meaning | Docs on macOS? |
|:--------|:--------|:---------------|
| `all(target_os = "linux", any(test, doc))` | Linux AND (test OR doc) | ❌ No |
| `any(doc, all(target_os = "linux", test))` | doc OR (Linux AND test) | ✅ Yes |

**Apply at all levels** — If linking to a nested module, both parent and child modules need
the visibility change. See `organize-modules` skill for complete patterns and examples.

#### ⚠️ Unix Dependency Caveat

The `cfg(any(doc, ...))` pattern assumes the module's code **compiles on all platforms**. When
the module uses Unix-only APIs (e.g., `mio::unix::SourceFd`, `signal_hook`, `std::os::fd::AsRawFd`),
use `cfg(any(all(unix, doc), ...))` instead to restrict doc builds to Unix platforms where the
dependencies exist.

**Three-tier platform hierarchy for cfg doc patterns:**

| Module dependencies | Pattern | Docs on Linux | Docs on macOS | Docs on Windows |
| :------------------ | :------ | :------------ | :------------ | :-------------- |
| Platform-agnostic (pure Rust, cross-platform deps) | `cfg(any(doc, ...))` | ✅ | ✅ | ✅ |
| Unix APIs (`mio::unix`, `signal_hook`, `std::os::fd`) | `cfg(any(all(unix, doc), ...))` | ✅ | ✅ | excluded |
| Linux-only APIs (hypothetical) | `cfg(any(all(target_os = "linux", doc), ...))` | ✅ | excluded | excluded |

**Example — Unix-restricted doc build:**

```rust
// Module uses mio::unix::SourceFd, signal_hook — Unix-only APIs.
// Dependencies in Cargo.toml are gated with cfg(unix).
// Doc builds are restricted to Unix where the dependencies exist.
#[cfg(any(all(unix, doc), all(target_os = "linux", test)))]
pub mod input;
#[cfg(all(target_os = "linux", not(any(test, doc))))]
mod input;

// Re-export also needs the unix-gated doc condition
#[cfg(any(target_os = "linux", all(unix, doc)))]
pub use input::*;
```

**Rule of thumb:** Match your `doc` cfg guard to your dependency's `cfg` guard. If the dep uses
`cfg(unix)`, gate docs with `all(unix, doc)`. If the dep uses `cfg(target_os = "linux")`, gate
docs with `all(target_os = "linux", doc)`.

---

## Checklist

Before committing documentation:

- [ ] Opening lines describe what the item IS (traits: "A trait for...", structs: "A/An X that...")
- [ ] First paragraph is separate (used as summary in module listings, IDE tooltips, search)
- [ ] Follow-up sentences use explicit subjects ("This trait...", "This struct...")
- [ ] Methods use third-person verbs (Creates, Returns, Checks — not Create, Return, Check)
- [ ] ASCII diagrams use standard Unicode (`✗` `→` `▼`) not emoji (`❌` `➡️` `⬇️`)
- [ ] Markdown tables use left-aligned columns (`:---`)
- [ ] High-level concepts at module/trait level (inverted pyramid)
- [ ] All links use reference-style with `crate::` paths
- [ ] All link definitions at bottom of comment blocks
- [ ] Constants use binary/byte literal/decimal (not hex)
- [ ] Hex shown in comments for cross-reference
- [ ] Markdown tables formatted (`cargo rustdoc-fmt`)
- [ ] No broken links (`./check.fish --doc`)
- [ ] All code examples compile (`./check.fish --test`)

---

## Supporting Files

| File | Content | When to Read |
|------|---------|--------------|
| `link-patterns.md` | Link source rubric + 15 detailed patterns | Choosing local vs external links, modules, private types, test functions, fragments |
| `constant-conventions.md` | Full human-readable constants guide | Writing byte constants, decision guide |
| `examples.md` | 5 production-quality doc examples | Need to see inverted pyramid in action |
| `rustdoc-formatting.md` | cargo rustdoc-fmt deep dive | Installing, troubleshooting formatter |

---

## Related Commands

| Command | Purpose |
|---------|---------|
| `/docs` | Full documentation check (invokes this skill) |
| `/fix-intradoc-links` | Fix only link issues |
| `/fix-comments` | Fix only constant conventions |
| `/fix-md-tables` | Fix only markdown tables |

---

## Related Skills

- `check-code-quality` - Includes doc verification step
- `organize-modules` - Re-export chains, conditional visibility for doc links
- `run-clippy` - May suggest doc improvements
