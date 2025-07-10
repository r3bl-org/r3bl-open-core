# Quick Start Guide 🚀

@title: Quick Start
@tags: tutorial, beginner

Welcome to our **markdown parser**! This guide will get you started in 5 minutes.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
my-parser = "1.0"
```

## Basic Usage

```rust
use my_parser::parse;

let result = parse("# Hello *world*!")?;
println!("{:?}", result);
```

## Features

- [x] **Fast parsing** ⚡
- [x] **Unicode support** 🌍
- [ ] **Table support** (coming soon)

That's it! Check out our [full documentation](https://docs.example.com) for more details.

> **Note**: This parser handles emojis correctly! 😄🎉✨
