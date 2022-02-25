//! This is an experimental module that isn't ready yet. It is the first step towards creating a TUI
//! library that can be used to create sophisticated TUI applications. This is similar to Ink
//! library for Node.js & TypeScript (that uses React and Yoga)

// Include these modules:
pub mod components;
pub mod tui_types;

// Module re-exports:
// <https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#documentation-comments-as-tests>

// Re-export the following modules:
pub use components::*;
pub use tui_types::*;
