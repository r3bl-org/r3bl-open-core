<p align="center">
  <img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg" height="128px">
</p>

# R3BL suite of TUI apps and library focused on developer productivity

We are building a **powerful suite of command line apps in Rust**, featuring text user interfaces
(TUIs) that unlock new levels of productivity within your terminal.

We lean in to the **terminal as a place of productivity** for software engineers and build
interlinked apps that all work with each other to make you more productive.

## Folder structure and code organization

This repo is organized as a monorepo. Many subfolders are Rust crates. There's a `Cargo.toml` file
at the top level which provides a workspace that allows us to build all the crates in this repo at
the same time.


### R3BL product offerings

- [**cmdr** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr)
  ([r3bl-cmdr crate](https://crates.io/crates/r3bl-cmdr)): Suite of TUI apps focused on developer
  productivity (early access previews 🐣). edi and giti are the first two apps in this suite. **We support Linux, macOS and Windows**.
  - [**🐱 giti** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr/src/giti) is an
    interactive git CLI app designed to give you more confidence and a better experience when
    working with git version control.
  - [**🦜 edi** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr/src/edi) is a
    Markdown editor for the terminal and cloud. It lets you edit Markdown files in your in style
    (supports emoji, has color gradient headers, and more).

### R3BL component offerings

- [**tui** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui)
  ([r3bl_tui crate](https://crates.io/crates/r3bl_tui)): TUI library to build modern apps inspired
  by Elm, with Flexbox, CSS, editor component, emoji support, and more
- [**tuify** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/tuify)
  ([r3bl_tuify crate](https://crates.io/crates/r3bl_tuify)): single and multi-select TUI components
  used in giti.

### R3BL library offerings

- [**ansi_color** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/ansi_color): Rust
  crate to generate formatted ANSI 256 (8-bit) and truecolor (24-bit) color output to stdout. On
  macOS, the default Terminal.app does not support truecolor, so ANSI 256 colors are used instead.
  ([r3bl_ansi_color crate](https://crates.io/crates/r3bl_ansi_color))
- [**core** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/core)
  ([r3bl_rs_utils_core crate](https://crates.io/crates/r3bl_rs_utils_core))
- [**macro** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/macro)
  ([r3bl_rs_utils_macro crate](https://crates.io/crates/r3bl_rs_utils_macro))
- [**redux** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/redux)
  ([r3bl_redux crate](https://crates.io/crates/r3bl_redux))
- [**utils** folder](https://github.com/r3bl-org/r3bl-open-core/tree/main/utils)
  ([r3bl_rs_utils crate](https://crates.io/crates/r3bl_rs_utils))
- [**simple_logger**](https://github.com/r3bl-org/r3bl-open-core/tree/main/simple_logger)([r3bl_simple_logger crate](https://crates.io/crates/r3bl_simple_logger))

## Learn more

To learn more about this library, please read how it was built (on
[developerlife.com](https://developerlife.com)):

1.  [Build a non-binary tree that is thread safe using Rust](https://developerlife.com/2022/02/24/rust-non-binary-tree/)
2.  [Guide to Rust procedural macros](https://developerlife.com/2022/03/30/rust-proc-macro/)

You can also find all the 🦀 Rust related content on developerlife.com
[here](https://developerlife.com/category/Rust/).
