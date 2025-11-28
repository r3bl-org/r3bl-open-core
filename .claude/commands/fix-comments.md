# Fix Constant Conventions in Comments

Invoke the `write-documentation` skill, focusing on **Part 3: Constant Conventions**.

Fix numeric literals in byte constants: use binary for bitmasks (`0b0110_0000`), byte literals for printable chars (`b'['`), decimal for non-printables (`27`). Add hex in comments. See `constant-conventions.md` for details.
