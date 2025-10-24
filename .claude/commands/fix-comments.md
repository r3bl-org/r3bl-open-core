In the file currently open in the IDE, or the files in the curent working tree or for files that
have documentation warnings from (cargo doc --no-deps), apply the following fixes:

1. In all the rustdoc comments use reference style links for symbols that are enclosed in backticks
   (where this is possible). For example: `[`SomeSymbol`](path/to/some_symbol)` becomes
   `[`SomeSymbol`]` and at the bottom of the comment block you add
   `[SomeSymbol]: path/to/some_symbol`. This makes the comments much more readable. Follow these
   guidelines:
   - When adding reference style links, ensure that all the added links are at the bottom of the
     comment block.
   - Once complete, verify that all links are correct by running `cargo doc --no-deps` and checking
     the generated documentation.

2. Make sure that any markdown tables in this file is properly formatted with columns aligned using
   the right amount of whitespaces.
