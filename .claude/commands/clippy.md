Fix trailing periods in comments, check test coverage, errors and warnings in tests, doctests, build
docs, clippy

# Run clippy-quick tasks

Run all the tasks in @clippy-quick.md

# Fix Comment Punctuation

Comment Punctuation Rules for all the changed files (in the current git working tree): Ensure all
comments end with proper punctuation following these patterns:

1. Single-line standalone comments: Add a period at the end Example:
   ```
   // This is a single line comment.
   ```
2. Multi-line wrapped comments (one logical sentence): Period ONLY on the last line Example:

   ```
   // This is a long line that wraps
   // to the next line.
   ```

3. Multiple independent single-line comments: Each gets its own period Example:
   ```
   // First independent thought.
   // Second independent thought.
   ```

How to identify wrapped vs. independent comments:

- Wrapped: The second line continues the grammatical structure of the first
- Independent: Each line could stand alone as a complete thought

# Documentation and Test Coverage

In all the code that is part of the current git working tree, make sure that there is sufficient
documentation and test code coverage.

- For existing tests, make sure they add value and are not redundant or needless.
- If they are needless, remove them. If there are missing tests, then add them.

# Finally, run cargo fmt

make sure to run `cargo fmt --all`
