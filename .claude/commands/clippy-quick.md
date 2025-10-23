Fix errors and warnings in tests, doctests, build docs, clippy

1. make sure all the tests pass (`cargo nextest run`)
2. can you fix any failing doc tests (`cargo test --doc`)
3. make sure all the docs build (`cargo doc --no-deps`)
4. make sure (`cargo clippy --all-targets`) has no warnings
5. for all the code in the staged & unstaged area of the working tree, or code that clippy is
   complaining about, can you use reference style links in rustdocs wherever possible for symbols
   enclosed in backticks and add all the references at the bottom of the rustdoc comment block (not
   in the middle).

   For example this is good:

   ```
   /// The module [`char_ops`] does XYZ.
   ///
   /// Bla bla bla... [`other_symbol`].
   ///
   /// [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops
   /// [`other_symbol`]: crate::some::other::path::other_symbol
   ```

   And this is bad:

   ```
   /// The module [`char_ops`] does XYZ.
   /// [`char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops
   ///
   /// Bla bla bla... [`other_symbol`].
   /// [`other_symbol`]: crate::some::other::path::other_symbol
   ```

6. for all the code in the staged & unstaged area of the working tree, or code that clippy is
   complaining about, if the rustdocs have markdown tables, make sure that they are properly column
   aligned using whitespaces
7. make sure to run `cargo fmt --all`
