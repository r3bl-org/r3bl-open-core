# Task: Document Academic Research on Type Safety at Scale (Theoretical Foundations and Empirical Benchmarks)

## Overview

In `r3bl_tui`, coordinate systems and viewport camera abstractions enforce domain
boundaries between canvas-absolute positions and viewport-relative positions using Rust's
Newtype and Typestate patterns.

This task incorporates formal academic research and empirical performance studies directly
into the codebase documentation, grounding the design in programming languages research
while separating two fundamental concepts:

1. **Theoretical Foundations (Design Patterns)**:
    - **Will Crichton ([FUNARCH 2023 paper], [Stanford CS 242])**: Explores how modern
      type systems (specifically Rust) implement the Typestate pattern, State Machine
      pattern, and The Witness pattern. Defines four core principles: State as Type,
      Restricted Transitions, Type Transformation (consuming `self`), and Invalidation
      (consuming `self` prevents reusing stale or illegal states).
    - **Alexis King ([Parse, don't validate])**: Guides boundary parsing where raw input
      data is parsed into types that make illegal states unrepresentable at compile time.

2. **Empirical Benchmarks (Performance and Faultlessness at Scale)**:
    - **Leon Heuer, Falk Woldmann Lu, and Jan Haase ([FUNARCH 2026 paper])**: Experience
      report investigating Newtype and Typestate patterns in production Rust software.
      Demonstrates that combining newtypes with "Parse, don't validate" elevates software
      faultlessness and eliminates invalid runtime states at low structural cost.
      Criterion benchmarks confirm these compile-time abstractions incur zero runtime
      performance penalty (execution-time differences remained within the +/- 2% noise
      margin).

### Key Design Decisions

1. **Single Source of Truth**: Centralize full bibliographic citations, DOIs, and external
   URLs exclusively in `tui/src/core/coordinates/canvas/mod.rs`.
2. **Distinct Conceptual Anchor Links**: Provide deep intra-doc anchor links so callers
   can reference:
    - `[academic research on type safety at scale]`: Main section overview.
    - `[theoretical foundations]`: Crichton and King's pattern theories.
    - `[empirical benchmarks]`: Heuer et al.'s empirical faultlessness and benchmark data.
3. **No Redundant External URLs**: Dependent modules (`coordinates/mod.rs` and
   `viewport.rs`) inline pattern benefits and link directly to `canvas/mod.rs` using
   intra-doc links rather than repeating external links.
4. **Reframing Primitive Obsession**: Update `tui/src/core/coordinates/mod.rs:56` from
   "Type Safety Over Convenience" to "Type Safety Over Ambiguous Primitives", emphasizing
   that raw `usize`/`u16` types represent ambiguity rather than genuine convenience.

---

## Implementation Plan

### [x] Phase 1: Centralize Citations in `canvas/mod.rs`

- [x] Add `## Academic Research on Type Safety at Scale` to
      `tui/src/core/coordinates/canvas/mod.rs`.
- [x] Add `### Theoretical Foundations` detailing Crichton's typestate principles and
      King's "Parse, don't validate".
- [x] Add `### Empirical Benchmarks` detailing Heuer, Lu, and Haase's empirical findings
      and Criterion benchmarks.
- [x] Add external reference-style links at the bottom of the module comment block.
- [x] Ensure deterministic heading anchors (`#academic-research-on-type-safety-at-scale`,
      `#theoretical-foundations`, `#empirical-benchmarks`).

### [x] Phase 2: Cross-Link in `viewport.rs`

- [x] Update `Viewport` struct doc comment in
      `tui/src/core/coordinates/canvas/viewport.rs`.
- [x] Inline the pattern benefits (Parse Don't Validate, Newtype, Typestate, runtime
      faultlessness, zero performance penalty).
- [x] Replace duplicate external paper URLs with intra-doc links to
      `[academic research on type safety at scale]`, `[theoretical foundations]`, and
      `[empirical benchmarks]`.

### [x] Phase 3: Cross-Link and Reframe in `coordinates/mod.rs`

- [x] Rename section `## 2. **Type Safety Over Convenience**` to
      `## 2. **Type Safety Over Ambiguous Primitives**`.
- [x] Update opening explanation to emphasize eliminating ambiguous raw primitives.
- [x] Inline pattern benefits and link to `[academic research on type safety at scale]`
      (`[theoretical foundations]`, `[empirical benchmarks]`).
- [x] Replace duplicate external URLs with intra-doc links targeting `canvas/mod.rs`.

### [x] Phase 4: Quality & Verification (Coordinates Subsystem)

- [x] Run `cargo rustdoc-fmt` on all modified files.
- [x] Run `cargo doc -p r3bl_tui --no-deps` to verify all intra-doc links resolve with
      zero warnings.
- [x] Run `./check.fish --check` to verify compilation.
- [x] Perform line-by-line `git diff` audit to enforce surgical changes and formatting
      rules.

### [x] Phase 5: Feature in Crate Entry Point (`lib.rs`) & Generate `README.md`

- [x] Add mathematically and empirically validated type safety highlight bullet to
      `# Framework highlights` in `tui/src/lib.rs`.
- [x] Remove outdated "Future Unification & Advanced Patterns" roadmap text in
      `tui/src/lib.rs` (unification already completed).
- [x] Add `## Academic Research on Type Safety at Scale` with `### Theoretical Foundations`
      and `### Empirical Benchmarks` under `# Canvas vs Viewport Architecture` in `tui/src/lib.rs`.
- [x] Update `# Table of contents` in `tui/src/lib.rs` to reflect new sections.
- [x] Add reference link definitions at bottom of `tui/src/lib.rs` (including `[`Deref`]`,
      intra-doc canvas links, and external paper URLs).
- [x] Run `cargo rustdoc-fmt tui/src/lib.rs`.
- [x] Run `cargo doc -p r3bl_tui --no-deps` to verify zero warnings.
- [x] Run `cargo readme > README.md` in `tui/` directory to regenerate `tui/README.md`.

### [x] Phase 6: Mandatory Manual Review

- [x] `tui/src/core/coordinates/canvas/mod.rs`
- [x] `tui/src/core/coordinates/canvas/viewport.rs`
- [x] `tui/src/core/coordinates/mod.rs`
- [x] `tui/src/lib.rs`
- [x] `tui/README.md`

<!-- cspell:words FUNARCH Heuer Woldmann Haase Crichton -->

