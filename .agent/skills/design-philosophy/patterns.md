<!-- cspell:words Stringly -->

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

## Quick Reference

| Principle | Ask Yourself |
|-----------|--------------|
| Cognitive Load | "How many concepts must a reader hold to understand this?" |
| Progressive Disclosure | "Can a beginner use the simple path without seeing complexity?" |
| Illegal States | "Can the type system prevent this bug?" |
| Abstraction Worth | "Does this abstraction make the code easier or harder to understand?" |
| ADT Const Params | "Can I use a const enum to replace a trait or runtime field?" |
