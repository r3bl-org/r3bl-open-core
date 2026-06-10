---
name: ansi-escape-codes
description: Conventions for documenting ANSI/VT100 escape sequences in rustdocs and comments.
---

# ANSI Escape Code Documentation

Whenever an ANSI/VT100 escape sequence is documented in rustdocs or inline comments, it must prioritize readability by following these conventions.

## The Standard: Spaced `ESC [` Notation

Sequences must be written using the literal `ESC` string followed by spaced tokens, enclosed in a single pair of backticks.

### Rules

1. **Use `ESC [` everywhere:** Replace `CSI`, unspaced `ESC[`, and shorthand modes like `?25h` with the fully expanded, spaced format.
   - *Avoid:* `[`CSI`]` `?25h`
   - *Avoid:* `ESC[?25h`
   - *Prefer:* `ESC [ ? 25 h`

2. **Explicitly state enable/disable pairs:** Do not use slashes or shorthand to combine enable/disable sequences. Expand them fully.
   - *Avoid:* `?25h`/`l`
   - *Prefer:* `ESC [ ? 25 h` (enable) / `ESC [ ? 25 l` (disable)

3. **Optional CSI Context:** If a module specifically deals with CSI parsing, you may optionally include a brief note explaining the term, but the sequence itself must still use the spaced format.
   - *Example:*
      ```rust
      /// Uses `ESC [ ? 25 h`. Note: `ESC [` is also known as the Control
      /// Sequence Introducer ([`CSI`]).
      ///
      /// [`CSI`]: crate::CsiSequence
      ```

4. **Linkify and Backtick Standard Terms:** Standard VT100/ANSI acronyms (like `DECAWM`, `DECTCEM`, `SGR`) must be enclosed in backticks and linked to an external reference URL using rustdoc's reference link syntax.
   - *Example:*
      ```rust
      /// Characters automatically wrap to the next line ([`DECAWM`] `ESC [ ? 7 h`).
      ///
      /// [`DECAWM`]: https://vt100.net/docs/vt510-rm/DECAWM.html
      ```
