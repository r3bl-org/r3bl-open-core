# Getting Started with Rust 🦀

Welcome to the world of **Rust programming**! This guide will walk you through the basics of setting up your development environment and writing your first Rust program.

## Table of Contents

1. [Installation](#installation)
2. [Your First Program](#your-first-program)
3. [Basic Concepts](#basic-concepts)
4. [Resources](#resources)

## Installation

To get started with Rust, you'll need to install the Rust toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your terminal or run:

```bash
source ~/.cargo/env
```

### Verify Installation

Check that everything is working:

```bash
rustc --version
cargo --version
```

## Your First Program

Let's create the classic "Hello, World!" program:

1. Create a new project:
   ```bash
   cargo new hello_world
   cd hello_world
   ```

2. Edit `src/main.rs`:
   ```rust
   fn main() {
       println!("Hello, world! 🌍");
   }
   ```

3. Run your program:
   ```bash
   cargo run
   ```

## Basic Concepts

### Variables

In Rust, variables are **immutable by default**:

```rust
let x = 5;
let mut y = 10; // mutable variable
y += 1;
```

### Functions

Functions in Rust use the `fn` keyword:

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b // no semicolon = return value
}
```

### Ownership

Rust's unique feature is its *ownership system*:

- Each value has an **owner**
- There can only be **one owner** at a time
- When the owner goes out of scope, the value is **dropped**

## Resources

Here are some helpful links:

- [The Rust Book](https://doc.rust-lang.org/book/) - Official documentation
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) - Learn by doing
- [Rustlings](https://github.com/rust-lang/rustlings) - Interactive exercises

> **Note**: Remember to practice regularly! The more you code in Rust, the more comfortable you'll become with its concepts.

---

*Happy coding!* 🚀
