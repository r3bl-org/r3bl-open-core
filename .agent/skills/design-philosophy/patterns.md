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
trait Processor<T, U, E> where T: Input, U: Output, E: Error {
    fn process(&self, input: T) -> Result<U, E>;
}
// Used exactly once, for one concrete type
```

### Good: Concrete Until Proven Otherwise

```rust
fn process_input(input: &str) -> Result<Output, ProcessError> { ... }
// Abstract only when you have 2+ concrete use cases
```

---

## Quick Reference

| Principle | Ask Yourself |
|-----------|--------------|
| Cognitive Load | "How many concepts must a reader hold to understand this?" |
| Progressive Disclosure | "Can a beginner use the simple path without seeing complexity?" |
| Illegal States | "Can the type system prevent this bug?" |
| Abstraction Worth | "Does this abstraction make the code easier or harder to understand?" |
