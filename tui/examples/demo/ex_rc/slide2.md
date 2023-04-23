# 📑 Design document

## 🦀 Text User Interface engine for Rust

You can build fully async _TUI_ (text user interface) apps with a modern API that brings the best of
the web frontend development ideas to TUI apps written in Rust:

1. Reactive & unidirectional data flow architecture from frontend web development (`React`, `Redux`).
2. Responsive design w/ `CSS`, flexbox like concepts.
3. Declarative style of expressing styling and layouts.

### 🔦 Framework highlights

- An easy to use and approachable API that is inspired by `React`, `JSX`, `CSS`, and `Redux`. Lots of
  components and things are provided for you so you don't have to build them from scratch. This is a
  full featured component library including:
    - Redux for state management (fully async, concurrent & parallel).
    - CSS like declarative styling engine.
    - CSS flexbox like *declarative layout* engine which is fully responsive. You can resize your
      terminal window and everything will be laid out correctly.
    - A terminal independent underlying rendering and painting engine (can use crossterm or termion or
      whatever you want).
    - Markdown text editor w/ syntax highlighting support, metadata (tags, title, author, date), smart
      lists. This uses a custom Markdown parser and custom syntax highligther. Syntax highlighting for
      code blocks is provided by the syntect crate.
    - Modal dialog boxes. And autocompletion dialog boxes.
    - Lolcat (**color gradients**) implementation w/ a rainbow color-wheel palette. All the color output
      is sensitive to the capabilities of the terminal. Colors are gracefully downgraded from
      truecolor, to [ANSI256](https://www.ditig.com/256-colors-cheat-sheet), to grayscale.
    - Support for Unicode grapheme clusters in strings. You can safely use emojis, and other Unicode
      characters in your TUI apps.
    - Support for mouse events.
- The entire TUI framework itself supports concurrency & parallelism (user input, rendering, etc.
  are generally _non-blocking_).
- It is fast! There are no needless re-renders, or flickering. Animations and color changes are
  smooth (check this out for yourself by running the examples). You can even build your TUI in
  layers (like `z-order` in a browser's `DOM`).

## 🌱 Getting started

```bash
cd tui/examples
cargo run --release --example demo
clear
pushd tui
tail -f -s 5 log.txt | lolcat
rm log.txt
touch log.txt
popd
```

### 🌍 Life of an input event

There is a clear separation of concerns in this module. To illustrate what goes where, and how
things work let's look at an example that puts the main event loop front and center & deals w/ how
the system handles an input event (key press or mouse).

- The diagram below shows an app that has 3 Components for (flexbox like) layout & (CSS like)
  styling.
- Let's say that you run this app (by hypothetically executing `cargo run`).
- And then you click or type something in the terminal window that you're running this app in.

```text
🧍🖱️  ⌨️
input → [TerminalWindow]
event       ↑      ↓               [ComponentRegistry] creates
            ┊   [App] ───────────■ [Component]s at 1st render
            ┊      │
            ┊      │        ┌──────■ id=1 has focus
            ┊      │        │
            ┊      ├→ [Component] id=1 ───┐
            ┊      ├→ [Component] id=2    │
            ┊      └→ [Component] id=3    │
         default                          │
         handler  ←───────────────────────┘
```
