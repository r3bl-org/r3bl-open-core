# Technical Terminology Precision

To maintain low cognitive load and technical accuracy across the R3BL codebase, we use a
standardized mental model for the lifecycle of code and its generic components.

## The Lifecycle Map

Use this map to choose the correct terms in documentation prose and comments.

| Concept | Mapping | Role |
| :--- | :--- | :--- |
| **Declaration** | **Header** | The "What": Identifier, Types, and **Parameters** (placeholders). |
| **Definition** | **Body** | The "How": Implementation, Logic, and Storage. |
| **Usage** | **Call Site** | The "Where": **Arguments** (values) and Execution. |

## Parameters vs. Arguments

The distinction is between the **slot** (declaration) and the **filler** (usage).

### 1. Parameters (The Slots)
- Belong to the **Header** and **Body**.
- They are defined in the **Header** and used as placeholders within the **Body**.
- **Type Parameters**: `T` in `Vec<T>`.
- **Const Parameters**: `POLICY` in `ScopedMutex<S, const POLICY: ...>`.

### 2. Arguments (The Fillers)
- Belong to the **Call Site**.
- Are the concrete Types or Values provided by the user.
- **Type Arguments**: `u8` in `Vec<u8>`.
- **Const Arguments**: `{ OptOut }` in `ScopedMutex<i32, { OptOut }>`.

## Perspective: Variable vs. Expression

A single line of code often represents multiple concepts simultaneously. Let's examine
this line `let a = String::new();`
- Variable `a`: This line is its Declaration (creating the variable `a`) and its
  Definition (providing the value via the assigned expression).
- Expression `String::new()`: This line is a Usage (Call Site).

We can break this line into two to make this more explicit:
`let a; a = String::new();`

## Parameters vs. Arguments

### In a Function
```rust
// DECLARATION / HEADER
// 'val' is a PARAMETER
fn process(val: u32) {
    // DEFINITION / BODY
    println!("{}", val);
}

// USAGE / CALL SITE
// '10' is an ARGUMENT
process(10);
```

### In Generics (Type Theory)
- **Generic over type `T`**: `T` is the **Type Parameter**.
- **Generic over value `V`**: `V` is the **Const Parameter**.
- When you instantiate `MyStruct<i32>`, `i32` is the **Type Argument**.

## Gold Standard Reference

For a production-quality example of how to apply these rules to complex code (using ADT Const
Params, Type Families, and explicit terminology headings), see:

[`tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs`]

[`tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs`]: ../../../tui/src/core/common/scoped_mutex/scoped_mutex_public_api.rs
