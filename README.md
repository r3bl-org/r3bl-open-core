# r3bl_rs_utils

This library provides utility functions:

1. Functions to unwrap deeply nested objects Inspired by by Kotlin scope functions.
2. Non binary tree data structure inspired by memory arenas, that is threadsafe and supports
   parallel tree walking.
3. Capabilities to make it easier to build TUIs (Text User Interface apps) in Rust.

The equivalent of this library is available for TypeScript and is called
[r3bl-ts-utils](https://github.com/r3bl-org/r3bl-ts-utils/).

## Usage

Please add the following to your `Cargo.toml`:

```toml
[dependencies]
r3bl_rs_utils = "0.5.0"
```

## Stability

🧑‍🔬 This library is in early development.

1. There are extensive integration tests for code that is production ready.
2. Everything else is marked experimental in the source.

Please report any issues to the [issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
And if you have any feature requests, feel free to add them there too 👍.
