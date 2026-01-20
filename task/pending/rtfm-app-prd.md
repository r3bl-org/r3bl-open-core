<!-- cspell:words reimagines Kerrisk sektion mankier coreutils groff mandoc ollama errno -->

# rtfm - TUI Man Page Reader (PRD)

**Status:** WIP / Research Phase
**Crate:** r3bl_cmdr
**Goal:** Low cognitive load approach to reading documentation using inverted pyramid
and progressive disclosure, enhanced by small local LLMs.

---

## Vision

Traditional man pages are comprehensive but overwhelming. `rtfm` reimagines
documentation consumption by:

1. **Inverted pyramid** — Most important information first, details on demand
2. **Progressive disclosure** — Start simple, drill down as needed
3. **LLM-assisted digestion** — Small local models summarize and restructure content
4. **TUI-native** — Fast, keyboard-driven, works in any terminal

---

## Research: How Man Pages Work

### Section Numbers

The `(N)` in references like `read(2)` indicates the manual section:

| Section | Contents                     | Examples                              |
| :------ | :--------------------------- | :------------------------------------ |
| 1       | User commands                | `ls(1)`, `grep(1)`, `git(1)`          |
| 1p      | POSIX user commands          | `read(1p)` (POSIX spec)               |
| 2       | System calls (kernel API)    | `read(2)`, `write(2)`, `open(2)`      |
| 3       | Library functions            | `printf(3)`, `malloc(3)`, `strlen(3)` |
| 3p      | POSIX library functions      | `read(3p)` (POSIX spec)               |
| 4       | Special files                | `/dev/null(4)`, `/dev/tty(4)`         |
| 5       | File formats                 | `passwd(5)`, `fstab(5)`               |
| 6       | Games                        | `fortune(6)`                          |
| 7       | Concepts/overviews           | `epoll(7)`, `socket(7)`, `signal(7)`  |
| 8       | System administration        | `mount(8)`, `systemctl(8)`            |

**Why sections matter:** Same name can exist in multiple sections:
- `printf(1)` — Shell command
- `printf(3)` — C library function
- `read(1)` — Shell builtin (bash)
- `read(2)` — System call (kernel)

### Man Page Hosting & URL Patterns

#### man7.org (Primary for Linux)

Maintained by Michael Kerrisk (author of "The Linux Programming Interface").
Official upstream for Linux man-pages project.

**URL Pattern:**
```
https://man7.org/linux/man-pages/man{SECTION}/{NAME}.{SECTION}.html
```

**Examples:**

| Reference              | URL                                                        |
| :--------------------- | :--------------------------------------------------------- |
| `read(2)`              | `man7.org/linux/man-pages/man2/read.2.html`                |
| `signalfd(2)`          | `man7.org/linux/man-pages/man2/signalfd.2.html`            |
| `epoll(7)`             | `man7.org/linux/man-pages/man7/epoll.7.html`               |
| `io_uring(7)`          | `man7.org/linux/man-pages/man7/io_uring.7.html`            |
| `io_uring_enter(2)`    | `man7.org/linux/man-pages/man2/io_uring_enter.2.html`      |
| `io_uring_prep_read(3)`| `man7.org/linux/man-pages/man3/io_uring_prep_read.3.html`  |

**What man7.org hosts:**

| Section | Hosted? | Notes                                           |
| :------ | :------ | :---------------------------------------------- |
| 1       | Partial | Only commands from man-pages project (e.g., ls) |
| 2       | Yes   | System calls (kernel API)                         |
| 3       | Yes   | C library functions                               |
| 4       | Yes   | Special files                                     |
| 5       | Yes   | File formats                                      |
| 7       | Yes   | Concepts/overviews                                |
| 8       | Partial | Some admin commands                             |

**What man7.org does NOT host:**
- Shell builtins (`read`, `cd`, `export`) — documented by shell (`man bash`)
- Third-party tools (`git`, `docker`) — documented by their projects

#### Other Man Page Hosts

| Site       | URL Pattern                        | Example for `kqueue(2)`                              |
| :--------- | :--------------------------------- | :--------------------------------------------------- |
| FreeBSD    | `?query={name}&sektion={N}`        | `man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2` |
| die.net    | `/man/{N}/{name}`                  | `linux.die.net/man/2/read`                           |
| mankier    | `/{N}/{name}`                      | `mankier.com/2/read`                                 |
| Arch Wiki  | Links to man7.org                  | —                                                    |

### Shell Builtins vs Standalone Commands

Shell builtins are documented differently:

```bash
# Builtins - part of the shell
help read              # Bash builtin help
man bash               # Search for "read [" in SHELL BUILTIN COMMANDS

# Standalone commands - have their own man pages
man ls                 # From coreutils
man grep               # From GNU grep
```

**Common builtins (no man page on man7.org):**
- `read`, `cd`, `export`, `source`, `alias`, `echo` (also exists as `/bin/echo`)

### Man Page Internal Structure

Typical sections within a man page:

| Section      | Purpose                                      |
| :----------- | :------------------------------------------- |
| NAME         | One-line description                         |
| SYNOPSIS     | Usage syntax                                 |
| DESCRIPTION  | Detailed explanation                         |
| OPTIONS      | Command-line flags                           |
| RETURN VALUE | What the function/syscall returns            |
| ERRORS       | Possible error codes (errno values)          |
| EXAMPLES     | Usage examples                               |
| SEE ALSO     | Related man pages                            |
| BUGS         | Known issues                                 |
| HISTORY      | When introduced, changes                     |

---

## rtfm App Design Ideas

### Inverted Pyramid for Man Pages

Transform the traditional structure into progressive disclosure:

```
Level 0: One-liner (NAME)
    ↓
Level 1: Synopsis + most common use case
    ↓
Level 2: Core options/parameters (80% use cases)
    ↓
Level 3: Full description with edge cases
    ↓
Level 4: Errors, bugs, history (rarely needed)
```

### LLM Integration Ideas

1. **Summarization** — Condense DESCRIPTION into 2-3 key points
2. **Example generation** — Create practical examples from SYNOPSIS
3. **Cross-reference** — Explain SEE ALSO relationships
4. **Error translation** — Human-readable explanations of errno values
5. **Comparison** — "How is epoll different from poll?"

### Keyboard Navigation

```
j/k     — Navigate within page
h/l     — Previous/next level of detail
/       — Search within page
]       — Jump to SEE ALSO link
?       — Ask LLM a question about current page
```

### Data Sources

1. **Local man pages** — `man -w` to find paths, parse with `mandoc` or `groff`
2. **Online fallback** — Fetch from man7.org if local not available
3. **Cache** — Store parsed/summarized content for fast access

---

## Open Questions

- [ ] Which local LLM? (llama.cpp, ollama, etc.)
- [ ] How to handle platform differences (Linux vs macOS vs BSD)?
- [ ] Should we pre-process and cache summaries?
- [ ] Integration with existing r3bl_cmdr infrastructure?

---

## References

- [Linux man-pages project](https://www.kernel.org/doc/man-pages/)
- [man7.org](https://man7.org/linux/man-pages/)
- [mandoc](https://mandoc.bsd.lv/) — Man page parser
- [The Linux Programming Interface](https://man7.org/tlpi/) — Michael Kerrisk
