# Code Style Patterns and Examples

This document provides detailed examples of good vs bad patterns for code style enforcement.

## Comment Punctuation Patterns

### Single-line Standalone Comments

**✅ Good:**
```rust
// Initialize the buffer with default values.
let mut buffer = Vec::new();

// Process each line in the input.
for line in lines {
    // Skip empty lines.
    if line.is_empty() {
        continue;
    }
}
```

**❌ Bad:**
```rust
// Initialize the buffer with default values
let mut buffer = Vec::new();

// Process each line in the input
for line in lines {
    // Skip empty lines
    if line.is_empty() {
        continue;
    }
}
```

---

### Multi-line Wrapped Comments

**✅ Good (One Logical Sentence):**
```rust
// This function performs complex validation by checking
// multiple conditions across different data structures.
fn validate(data: &Data) -> bool {
    // Process the validation by iterating through all
    // the fields and ensuring they meet requirements.
    data.fields.iter().all(|f| f.is_valid())
}
```

**❌ Bad (Period on Every Line):**
```rust
// This function performs complex validation by checking.
// multiple conditions across different data structures.
fn validate(data: &Data) -> bool {
    // Process the validation by iterating through all.
    // the fields and ensuring they meet requirements.
    data.fields.iter().all(|f| f.is_valid())
}
```

**❌ Bad (No Period at All):**
```rust
// This function performs complex validation by checking
// multiple conditions across different data structures
fn validate(data: &Data) -> bool {
    data.fields.iter().all(|f| f.is_valid())
}
```

---

### Multiple Independent Comments

**✅ Good:**
```rust
// Set up the initial state.
let mut state = State::new();

// Configure logging.
env_logger::init();

// Start the main loop.
loop {
    // Handle incoming events.
    match rx.recv() {
        // Process data events.
        Ok(Event::Data(d)) => process(d),

        // Terminate on shutdown signal.
        Ok(Event::Shutdown) => break,

        // Log errors and continue.
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

**❌ Bad:**
```rust
// Set up the initial state
let mut state = State::new();

// Configure logging
env_logger::init();

// Start the main loop
loop {
    // Handle incoming events
    match rx.recv() {
        // Process data events
        Ok(Event::Data(d)) => process(d),

        // Terminate on shutdown signal
        Ok(Event::Shutdown) => break,

        // Log errors and continue
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

---

### Distinguishing Wrapped vs Independent

**Example 1: Wrapped Comment (Continues Grammatical Structure)**

```rust
// Calculate the checksum by iterating through all bytes
// and applying the XOR operation to each one.
let checksum = bytes.iter().fold(0u8, |acc, &b| acc ^ b);
```

Test: Can you combine into one sentence?
→ "Calculate the checksum by iterating through all bytes and applying the XOR operation to each one."
→ ✅ Yes, so it's wrapped. Period only at the end.

**Example 2: Independent Comments (Separate Thoughts)**

```rust
// First, validate the input data.
// Then, transform it into the output format.
// Finally, write it to the file.
process_data(&input, &output)?;
```

Test: Can each line stand alone?
→ "First, validate the input data." ✅ Complete thought
→ "Then, transform it into the output format." ✅ Complete thought
→ "Finally, write it to the file." ✅ Complete thought
→ Each is independent. Each gets a period.

---

## Clippy Lint Categories and Responses

### Correctness Lints (Must Fix)

These indicate potential bugs or logic errors.

**Example: suspicious_else_formatting**

```rust
// ❌ Bad - suspicious else formatting
if x > 0 {
    do_something();
}
    else {  // Clippy warning: suspicious indentation
    do_other();
}

// ✅ Good
if x > 0 {
    do_something();
} else {
    do_other();
}
```

**Example: clone_on_copy**

```rust
// ❌ Bad - cloning a Copy type
let x = 42;
let y = x.clone();  // Clippy warning: i32 is Copy, not Clone

// ✅ Good
let x = 42;
let y = x;  // Just copy it
```

---

### Performance Lints (Should Fix)

These indicate inefficient code patterns.

**Example: unnecessary_to_owned**

```rust
// ❌ Bad - unnecessary allocation
fn process(items: &[String]) {
    for item in items.to_vec() {  // Clippy: unnecessary allocation
        println!("{}", item);
    }
}

// ✅ Good
fn process(items: &[String]) {
    for item in items {
        println!("{}", item);
    }
}
```

**Example: single_char_pattern**

```rust
// ❌ Bad - using string for single char
let trimmed = s.trim_start_matches("x");  // Clippy: use char

// ✅ Good
let trimmed = s.trim_start_matches('x');
```

---

### Style Lints (Should Fix for Idiomatic Code)

These suggest more idiomatic Rust patterns.

**Example: match_bool**

```rust
// ❌ Bad - matching on bool
match flag {
    true => do_this(),
    false => do_that(),
}

// ✅ Good
if flag {
    do_this()
} else {
    do_that()
}
```

**Example: needless_return**

```rust
// ❌ Bad - unnecessary return
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// ✅ Good
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

---

### Complexity Lints (Consider Refactoring)

These indicate code that's hard to understand or maintain.

**Example: cognitive_complexity**

```rust
// ❌ Bad - too complex
fn process(data: &[Data]) -> Result<Vec<Output>> {
    let mut results = Vec::new();
    for item in data {
        if item.valid {
            if item.category == Category::A {
                if item.value > 100 {
                    // Deep nesting...
                    results.push(transform_a(item)?);
                }
            } else {
                // More complexity...
            }
        }
    }
    Ok(results)
}

// ✅ Good - refactored with early returns
fn process(data: &[Data]) -> Result<Vec<Output>> {
    data.iter()
        .filter(|item| item.valid)
        .map(|item| process_item(item))
        .collect()
}

fn process_item(item: &Data) -> Result<Output> {
    match item.category {
        Category::A if item.value > 100 => transform_a(item),
        Category::B => transform_b(item),
        _ => Ok(Output::default()),
    }
}
```

---

## cargo fmt Formatting Rules

### Indentation

**Always use 4 spaces (never tabs):**

```rust
// ✅ Good
fn example() {
    if condition {
        do_something();
    }
}

// ❌ Bad (tabs)
fn example() {
→   if condition {
→   →   do_something();
→   }
}
```

---

### Line Length

**Default 100 characters, but configurable:**

```rust
// ✅ Good - respects line length
let result = some_function_with_many_params(
    param1,
    param2,
    param3,
);

// ❌ Bad - exceeds line length
let result = some_function_with_many_params(param1, param2, param3, param4, param5, param6);
```

---

### Import Organization

**Group imports logically:**

```rust
// ✅ Good - organized imports
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};

use external_crate::SomeType;

use crate::internal::Module;

// ❌ Bad - unorganized
use crate::internal::Module;
use std::collections::HashMap;
use external_crate::SomeType;
use std::io::{self, Read};
use std::fs::File;
```

**cargo fmt will automatically organize imports when configured with `group_imports = "StdExternalCrate"`**

---

### Trailing Commas

**Use trailing commas in multi-line contexts:**

```rust
// ✅ Good - trailing comma
let items = vec![
    1,
    2,
    3,  // Trailing comma makes diffs cleaner
];

// ❌ Bad - no trailing comma
let items = vec![
    1,
    2,
    3
];
```

---

## Module Organization Quick Checks

When running clippy, also verify module organization follows these patterns:

### Private Modules + Public Re-exports

**✅ Good:**
```rust
// mod.rs
mod constants;  // Private
mod types;      // Private
mod helpers;    // Private

pub use constants::*;  // Public API
pub use types::*;
pub use helpers::*;
```

**❌ Bad:**
```rust
// mod.rs
pub mod constants;  // Exposes internal structure
pub mod types;
pub mod helpers;
```

---

### Conditional Visibility

**✅ Good (for docs and tests):**
```rust
// mod.rs
#[cfg(any(test, doc))]
pub mod internal_parser;
#[cfg(not(any(test, doc)))]
mod internal_parser;

pub use internal_parser::*;
```

**❌ Bad:**
```rust
// mod.rs
pub mod internal_parser;  // Always public
pub use internal_parser::*;
```

---

### Rustfmt Skip for Manual Alignment

**When to use:**
- Large mod.rs files with many exports
- Deliberately structured code alignment for clarity

**✅ Good:**
```rust
// mod.rs

// Skip rustfmt for rest of file.
// https://stackoverflow.com/a/75910283/2085356
#![cfg_attr(rustfmt, rustfmt_skip)]

// Private modules
mod constants;
mod types;

// Public API
pub use constants::*;
pub use types::*;
```

---

## Summary Checklist

When running the `run-clippy` skill:

- [ ] `cargo clippy --all-targets` shows no warnings
- [ ] Comment punctuation follows rules (single-line: period, wrapped: period at end, independent: each gets period)
- [ ] Module organization follows private + re-export pattern where appropriate
- [ ] Rustdoc reference-style links properly formatted (if applicable)
- [ ] Tests pass after auto-fixes: `cargo test --all-targets`
- [ ] Code formatted: `cargo fmt --all`

All green? Ready to commit! ✅
