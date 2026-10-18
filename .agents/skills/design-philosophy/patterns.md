# Design Philosophy Patterns

## Cognitive Load Patterns

### Bad: Too Many Concepts at Once

```rust
fn process(data: &[u8], offset: usize, len: usize, flags: u32, mode: u8) -> Result<Vec<u8>, Error>
```

### Good: Group Related Concepts

```rust
fn process(input: InputBuffer, options: ProcessOptions) -> Result<OutputBuffer, ProcessError>
```

---

## Progressive Disclosure Patterns

### Bad: Everything Exposed

```rust
pub struct Parser {
    pub buffer: Vec<u8>,
    pub position: usize,
    pub state: ParserState,
    pub error_recovery_mode: bool,
    // ... 10 more fields
}
```

### Good: Minimal Public Surface

```rust
pub struct Parser { /* private fields */ }

impl Parser {
    pub fn new() -> Self { ... }
    pub fn parse(&mut self, input: &str) -> Result<Ast, ParseError> { ... }
    // Advanced users can access more:
    pub fn with_options(options: ParserOptions) -> Self { ... }
}
```

### Type-Centric Co-location (Associated Functions over Floating Functions)

#### Bad: Free-Floating Helper Functions for Domain Types

Floating functions clutter the module namespace, complicate barrel re-exports, and hide lifecycle operations from IDE autocomplete.

```rust
pub enum CsiBufferKind<'a> {
    SingleChar(u8),
    Parameterized(&'a [u8]),
    Invalid,
}

// Floating classifier in the module namespace:
pub fn classify_csi_buffer(buffer: &[u8]) -> CsiBufferKind<'_> { ... }

// Floating decoder for an internal shape:
fn decode_csi_event(params: &[u16], final_byte: u8) -> Option<VT100InputEventIR> { ... }
```

#### Good: Associated Functions & Methods on Domain Types

Co-locating classification, construction, and decoding directly with the types encapsulates behavior, cleans up module exports, and enables IDE discoverability via `Type::` or `instance.`.

```rust
pub enum CsiBufferKind<'a> {
    SingleChar(u8),
    Parameterized(&'a [u8]),
    Invalid,
}

impl<'a> CsiBufferKind<'a> {
    // Associated constructor / classifier:
    #[must_use]
    pub fn classify(buffer: &'a [u8]) -> Self { ... }
}

impl CsiParamShape {
    // Associated classifier:
    #[must_use]
    fn classify(params: &[u16], final_byte: u8) -> Self { ... }

    // Associated decoder:
    #[must_use]
    fn decode(self, final_byte: u8) -> Option<VT100InputEventIR> { ... }
}
```

#### When to Keep Floating

Reserve floating functions for top-level pipeline coordinators (orchestrators) that return generic envelopes or cross-cutting intermediate representations, or functions returning standard library / primitive types:

```rust
// GOOD as floating: Top-level entry point coordinating multiple parsers into generic IR:
pub fn parse_keyboard_sequence(buffer: &[u8]) -> Option<ParsedInputEventIR> {
    if let Some(event) = csi_u::parse_csi_u_sequence(buffer) {
        return Some(event);
    }
    match CsiBufferKind::classify(buffer) { ... }
}

// GOOD as floating: Primitive or standard library return type (orphan rule):
pub fn strip_csi_numeric_prefix(chunk: &[u8]) -> Option<&[u8]> { ... }
```

---

## Make Illegal States Unrepresentable

### Bad: Runtime Validation

```rust
struct Range {
    start: usize,
    end: usize,  // Must be >= start, but not enforced!
}

fn validate_range(r: &Range) -> bool {
    r.end >= r.start
}
```

### Good: Compile-Time Guarantee

```rust
struct Range {
    start: usize,
    length: usize,  // Cannot be negative, end is always start + length
}

impl Range {
    fn end(&self) -> usize { self.start + self.length }
}
```

### Bad: Stringly Typed

```rust
fn set_color(color: &str) { ... }  // "red"? "RED"? "#ff0000"? "rgb(255,0,0)"?
```

### Good: Type-Safe

```rust
enum Color { Red, Green, Blue, Rgb(u8, u8, u8) }
fn set_color(color: Color) { ... }
```

### Bad: Boolean Soup / Boolean Blindness

```rust
// Unclear what true/false means at call sites and obscures actual domain behavior
pub fn is_file_extension_default(&self) -> bool { ... }

if buffer.is_file_extension_default() {
    render_r3bl();
} else {
    render_syntect();
}
```

### Good: Self-Documenting Domain Enums

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxHighlightPipeline<'a> {
    /// R3BL custom Markdown parser & AST renderer
    R3BLMarkdown,
    /// Syntect syntax highlighter with language extension
    Syntect(&'a str),
    /// Plain text rendering without syntax highlighting
    PlainText,
}

match buffer.get_syntax_highlight_pipeline() {
    SyntaxHighlightPipeline::R3BLMarkdown => render_r3bl(),
    SyntaxHighlightPipeline::Syntect(ext) => render_syntect(ext),
    SyntaxHighlightPipeline::PlainText => render_plain_text(),
}
```

### Mutator Returns: `Option<T>` vs `Option<()>` vs Domain Enums

```rust
// ❌ Bad: Boolean blindness on mutator returns
pub fn delete_at_col(&mut self, row: CRow, col: CCol) -> bool;
pub fn format_table_node(node: &mut DocNode) -> bool;

// ℹ️ Note: &mut self mutators that delete text CANNOT return borrowed slices (&str)
// or metadata offsets (DocSeg) because the borrow checker forbids returning a &str
// borrowing from self across a &mut self mutation, and deleted bytes no longer exist.
// Returning deleted text payload unavoidably requires an owned Option<String> allocation.

// ✅ Good: Option<T> when payload is removed and genuinely consumed by callers
pub fn remove_line(&mut self, row: CRow) -> Option<LineMetadata>;

// ✅ Good: Zero-allocation Option<()> ONLY when callers short-circuit/chain with `?`
pub fn delete_at_col(&mut self, row: CRow, col: CCol) -> Option<()>;

// Caller usage (chaining with `?` or early return):
buffer.delete_at_col(row, col)?;

// ❌ Bad (Anti-Pattern): Returning Option<()> when callers do NOT chain with `?`
// An unchanged CST node is not a failure (`None`), and sequential formatting passes
// must not short-circuit!
pub fn format_table_node(node: &mut DocNode) -> Option<()>;

// ✅ Good: Domain Enum when callers do NOT chain with `?`
// Do NOT add `.is_modified() -> bool` helpers on domain enums; compare directly!
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatStatus {
    Modified,
    Unchanged,
}

pub fn format_table_node(node: &mut DocNode) -> FormatStatus;

if format_table_node(node) == FormatStatus::Modified {
    // Handle modification explicitly
}
```

### When `bool` Is Acceptable (`std` & Third-Party Compatibility Only)

Raw `bool` is prohibited in domain logic and is only permitted at `std`, `clippy`, or third-party macro boundaries:

```rust
// ✅ Acceptable: std collection convention (paired with `len()`, required by clippy::len_without_is_empty)
pub fn is_empty(&self) -> bool { self.len() == 0 }

// ✅ Acceptable: std iterator adaptor closures (`filter`, `any`, `all`, `retain`)
items.retain(|item| item.status == FormatStatus::Modified);

// ✅ Acceptable: Third-party derive macro contracts (e.g. clap CLI flags)
#[derive(clap::Parser)]
pub struct CLIArg {
    #[arg(long)]
    pub verbose: bool,
}
```

---

## Encapsulation & Testing Hygiene

### Bad: Breaking Encapsulation for Tests

Making fields or methods overly public (`pub` or `pub(crate)`) simply so they can be accessed in test code leaks internal state and breaks the encapsulation boundary for production code.

```rust
pub struct Connection {
    // ❌ Bad: Made pub(crate) just so tests can inspect or manipulate it
    pub(crate) internal_state: State, 
}
```

### Good: Test-Only Accessors (`#[cfg(test)]`)

Keep fields strictly scoped (e.g. private or `pub(in crate::my_module)`) for production code, and provide dedicated accessor methods annotated with `#[cfg(test)]`. This ensures the escape hatch is compiled completely out of the production binary while allowing tests to observe or manipulate internal state. Name these methods with a `_for_testing` suffix to make their intent explicit.

```rust
pub struct Connection {
    // ✅ Good: Strictly encapsulated in production
    internal_state: State,
}

impl Connection {
    // ✅ Good: Explicit test-only escape hatch
    #[cfg(test)]
    pub(crate) fn state_for_testing(&self) -> &State {
        &self.internal_state
    }
}
```

---

## Abstraction Patterns

### Bad: Abstraction Adds Complexity

```rust
trait Strategy { fn execute(); }
struct Fast;
impl Strategy for Fast { fn execute() { /* ... */ } }
struct Safe;
impl Strategy for Safe { fn execute() { /* ... */ } }

struct Processor<S: Strategy> { _marker: PhantomData<S> }
```

### Good: Concrete Until Proven Otherwise

```rust
fn process_input(input: &str) -> Result<Output, ProcessError> { ... }
// Abstract only when you have 2+ concrete use cases
```

## Type-Safe Error Handling

### Bad: Stringly-Typed Errors (No type information)

```rust
fn process() -> Result<(), String> {
    Err("Something went wrong".to_string())
}
```

### Good: Custom Error Enums (with `thiserror` and `miette`)

```rust
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum MyError {
    #[error("Something went wrong: {0}")]
    #[diagnostic(
        code(r3bl_tui::feature::description),
        help("Provide a helpful hint to the user")
    )]
    InternalError(String),
}

fn process() -> Result<(), MyError> { ... }
```

### Domain Control Flow vs Standard Errors

When your architecture requires custom control flow enums (like `ControlFlowLimited`), do not pollute your "happy path" with nested manual error mappings. Instead, isolate the standard Rust error handling (`Result` and `?`) in a closure, and convert it at the boundary using `From` and `.into()`.

**Bad: The Staircase of Doom**
```rust
// Forces manual unpacking and multiple return paths.
LineStateControlSignal::Line(buf) => {
    output_device.write(|term| {
        if let Err(err) = line_state.print_data(buf, term) {
            return ControlFlowLimited::ReturnError(err);
        }
        if let Err(err) = term.flush() {
            return ControlFlowLimited::ReturnError(err.into());
        }
        ControlFlowLimited::Continue
    })
}
```

**Good: The Closure Sandbox**
```rust
// 1. Implement the bridge
impl<E> From<Result<(), E>> for ControlFlowLimited<E> {
    fn from(res: Result<(), E>) -> Self {
        match res {
            Ok(_) => Self::Continue,
            Err(e) => Self::ReturnError(e),
        }
    }
}

// 2. Use `?` safely inside the closure sandbox
LineStateControlSignal::Line(buf) => {
    output_device.write(|term| {
        line_state.print_data(buf, term)?;
        term.flush()?;
        Ok(()) // Return a standard Result
    }).into()  // Bridge it to the custom ControlFlow
}
```

---

## Modern Rust: ADT Const Params

Control behavior with enums in const generics instead of traits or runtime fields. Use this when writing **new code** or **refactoring existing code** that choose between strategies at compile-time. This is unlocked by the [`adt_const_params`](tui/src/lib.rs:17) feature.

### Bad: Trait-based Strategy (High Boilerplate)

```rust
trait Strategy { fn execute(); }
struct Fast;
impl Strategy for Fast { fn execute() { /* ... */ } }
struct Safe;
impl Strategy for Safe { fn execute() { /* ... */ } }

struct Processor<S: Strategy> { _marker: PhantomData<S> }
```

### Bad: Runtime Field (Runtime Overhead)

```rust
enum Mode { Fast, Safe }
struct Processor { mode: Mode }

impl Processor {
    fn run(&self) {
        match self.mode { // Branching happens at runtime
            Mode::Fast => { /* ... */ }
            Mode::Safe => { /* ... */ }
        }
    }
}
```

### Good: ADT Const Generic (Zero-Cost & Low Boilerplate)

Real-world example: [`ScopedMutex`](tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs:279).

```rust
#[derive(PartialEq, Eq, std::marker::ConstParamTy)]
enum Mode { Fast, Safe }

struct Processor<const M: Mode>;

impl<const M: Mode> Processor<M> {
    fn run(&self) {
        // Compiler prunes the branches that don't match M
        match M {
            Mode::Fast => { /* ... */ }
            Mode::Safe => { /* ... */ }
        }
    }
}
```

---

## Constant Design Patterns

### Bad: Redundant Duplicate Literals Across Types

Duplicating literal values across byte slices, string slices, and length constants creates multiple sources of truth, invites desynchronization bugs, and increases cognitive load.

```rust
// ❌ Bad: Three separate hardcoded literals for the same sequence.
pub const MOUSE_SGR_PREFIX: &[u8] = b"\x1b[<";
pub const MOUSE_SGR_PREFIX_STR: &str = "\x1b[<";
pub const MOUSE_SGR_PREFIX_LEN: usize = 3;

// ❌ Bad: Separate hardcoded byte literal ignoring existing string macro.
define_ansi_const!(@sgr_str : SGR_RESET_STR = ["0m"] => ...);
pub const SGR_RESET_BYTES: &[u8] = b"\x1b[0m";
```

### Good: Constant Derivation Hierarchy

Rust const evaluation has two distinct constraints:
1. `&str::chars()` is not a `const fn`. Extracting a `char` from `&str` at compile time is clumsy.
2. Concatenating `&[u8]` byte slices is not supported in `const` contexts without heap allocations. By contrast, `&str` can be concatenated at compile time (`formatcp!`, `define_ansi_const!`), and converting `&str` to `&[u8]` via `.as_bytes()` is a zero-cost `const fn`.

Therefore, constants follow a strict derivation hierarchy based on whether they are atomic primitives or composed sequences.

#### 1. Atomic Primitives (Single byte/char): Derive from `u8`

For foundational 1-byte protocol building blocks (`NUL`, `b';'`, `b':'`, `b'<'`, `b'M'`), define the `u8` byte value first and derive `char` and `&str` outward using helper functions:

```rust
// ✅ Good: Define u8 first for atomic primitives.
pub const NUL: u8 = 0;
pub const NUL_CHAR: char = ascii_byte_to_char(NUL);
pub const NUL_STR: &str = ascii_byte_to_str(&[NUL]);

pub const ANSI_PARAM_SEPARATOR: u8 = b';';
pub const ANSI_PARAM_SEPARATOR_CHAR: char = ascii_byte_to_char(ANSI_PARAM_SEPARATOR);
pub const ANSI_PARAM_SEPARATOR_STR: &str = ascii_byte_to_str(&[ANSI_PARAM_SEPARATOR]);
```

Do not invert this by defining `NUL_STR: &str = "\0"` and indexing `as_bytes()[0]`.

##### Helper Functions for Atomic Primitives

Defined in `tui/src/core/ansi/constants/mod.rs`:

- `ascii_byte_to_char(byte: u8) -> char`:
  - **Purpose**: Converts a foundational `u8` byte (`0..=255`) to a `char` at compile time.
  - **Why**: The ISO-8859-1 / ASCII range (`U+0000`..`U+00FF`) maps 1:1 to Unicode without surrogates. Encapsulating this in a helper satisfies Clippy's `as_conversions` lint in one audited place.
  - **When to use**: Whenever deriving a single-character constant (`_CHAR`) from an atomic `u8` constant.
- `ascii_byte_to_str(bytes: &'static [u8]) -> &'static str`:
  - **Purpose**: Converts a static byte slice to a `&'static str` at compile time.
  - **Why**: Performs compile-time UTF-8 validation via `core::str::from_utf8`.
  - **When to use**: Whenever deriving a 1-character string slice (`_STR`) from an atomic `u8` constant (e.g. `ascii_byte_to_str(&[NUL])`). Do not use this for multi-byte composed sequences; composed sequences start as `&str` instead.

#### 2. Composed Sequences (Multi-byte strings): Derive from `&str`

For multi-byte sequences composed of prefixes, markers, or parameters, define the `&str` constant first and derive byte slice and length outward:

```rust
// ✅ Good: Define &str first for composed sequences.
pub const MOUSE_SGR_PREFIX_STR: &str =
    const_format::formatcp!("{CSI_START}{MOUSE_SGR_MARKER_CHAR}");
pub const MOUSE_SGR_PREFIX: &[u8] = MOUSE_SGR_PREFIX_STR.as_bytes();
pub const MOUSE_SGR_PREFIX_LEN: usize = MOUSE_SGR_PREFIX.len();

define_ansi_const!(@sgr_str : SGR_RESET_STR = ["0m"] => ...);
pub const SGR_RESET_BYTES: &[u8] = SGR_RESET_STR.as_bytes();
```

#### Decision Matrix

| Scenario | Base Type | Conversion Mechanism | Resulting Constants |
| :--- | :--- | :--- | :--- |
| **Atomic Primitive** (Single byte/char, e.g. `NUL`, `;`, `:`, `<`, `M`) | `u8` | `ascii_byte_to_char(BYTE)`<br>`ascii_byte_to_str(&[BYTE])` | `FOO: u8`<br>`FOO_CHAR: char`<br>`FOO_STR: &str` |
| **Composed Sequence** (Multi-byte strings, e.g. `ESC [ <`, `ESC [ 0 m`, `ESC ]`) | `&str` | `formatcp!` / `define_ansi_const!`<br>`.as_bytes()`<br>`.len()` | `FOO_STR: &str`<br>`FOO: &[u8]`<br>`FOO_LEN: usize` |

---

## Quick Reference

| Principle              | Ask Yourself                                                                    |
| :--------------------- | :------------------------------------------------------------------------------ |
| Cognitive Load         | "How many concepts must a reader hold to understand this?"                      |
| Progressive Disclosure | "Can a beginner use the simple path without seeing complexity?"                 |
| Illegal States         | "Can the type system prevent this bug?"                                         |
| Abstraction Worth      | "Does this abstraction make the code easier or harder to understand?"           |
| ADT Const Params       | "Can I use a const enum to replace a trait or runtime field?"                   |
| Type Co-location       | "Is this floating function constructing or decoding a co-located domain type?"   |
| Constant Hierarchy     | "Is this atomic (base u8) or a composed sequence (base &str)?"                  |
