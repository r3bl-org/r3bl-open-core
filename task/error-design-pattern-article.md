# Task: Error Design Pattern Tutorial

<!-- BEGIN mktoc -->

- [Task: Error Design Pattern Tutorial](#task-error-design-pattern-tutorial)
- [Overview](#overview)
    - [Narrative Arc](#narrative-arc)
    - [Tone & Style Rules](#tone--style-rules)
    - [Workspace Crate Showcase](#workspace-crate-showcase)
- [Tutorial Blueprint](#tutorial-blueprint)
    - [Section 1: Real-World Origin](#section-1-real-world-origin)
    - [Section 2: Problem Domain](#section-2-problem-domain)
    - [Section 3: Mental Model](#section-3-mental-model)
    - [Section 4: Solution Patterns](#section-4-solution-patterns)
    - [Section 5: Decision Reference](#section-5-decision-reference)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: Blueprint & Narrative Alignment](#phase-1-blueprint--narrative-alignment)
    - [Phase 2: Draft & Verify Tutorial](#phase-2-draft--verify-tutorial)
    - [Phase 3: Community Publicity](#phase-3-community-publicity)

<!-- END mktoc -->

## Overview

This task file contains the blueprint and authoring instructions for generating a
[developerlife.com](https://developerlife.com) tutorial on designing clear `Error`,
`Result`, `Outcome`, and `Report` types in multi-crate Rust workspaces.

**Real-World Origin:** Anchor the tutorial in our firsthand experience building and
evolving [`r3bl-open-core`](https://github.com/r3bl-org/r3bl-open-core). Early on,
standardizing on `CommonResult<T>` (aliasing `miette::Result<T>`) and a shared
`CommonError` type let us move fast while prototyping. As `r3bl-open-core` expanded into a
five-crate workspace spanning reusable async engines, a JSON-RPC Model Context Protocol
(`MCP`) server, a CST-based Cargo subcommand formatter, shared wire schemas, and async
developer applications, leaning on a single catch-all `Result` across every layer created
growing friction. Refactoring all five crates surfaced four distinct semantic roles and
clear boundary rules that apply to any growing Rust system.

**Target Audience:** Intermediate-to-advanced Rust developers building multi-crate
workspaces that combine reusable libraries with servers, developer toolchains, shared
schemas, or applications.

### Narrative Arc

Follow a strict five-stage progression where concepts are earned from real codebase
pressures rather than stated upfront as axioms:

```text
┌──────────────────────┐    ┌──────────────────────┐    ┌──────────────────────┐
│ 1. Real-World Origin │───►│  2. Problem Domain   │───►│   3. Mental Model    │
│ (ROC evolution &     │    │ (4 pressures that    │    │ (Library vs. edge +  │
│  5-crate ecosystem)  │    │  emerge at scale)    │    │  4 semantic roles)   │
└──────────────────────┘    └──────────────────────┘    └──────────┬───────────┘
                                                                   │
                            ┌──────────────────────┐    ┌──────────▼───────────┐
                            │ 5. Decision Reference│◄───│ 4. Solution Patterns │
                            │ (ASCII decision tree │    │ (Production ROC code │
                            │  & lookup table)     │    │  & #[source] rules)  │
                            └──────────────────────┘    └──────────────────────┘
```

### Tone & Style Rules

- **Constructive, grounded engineering voice:** Write from lived experience evolving
  `r3bl-open-core` ("Early in `r3bl-open-core`, we reached for..."). Avoid aggressive,
  hyperbolic, or judgmental language ("error handling fails", "sloppiness", "fake",
  "pointless", "mess", "dead").
- **Domain-agnostic framing:** Make clear that these patterns apply to any Rust
  architecture (reusable libraries, HTTP/gRPC/JSON-RPC servers, CLI/build toolchains,
  shared protocol schemas, and interactive UIs), not solely terminal apps.
- **Formatting guardrails:**
    - Use ASCII / Unicode box-drawing diagrams with font-safe glyphs from `docs/boxes.md`
      (never Mermaid diagrams).
    - Use ASCII hyphens (`-`) only for list bullets, table borders, code, and compound
      words (`multi-crate`, `type-erased`, `JSON-RPC`). Never use em-dashes, en-dashes, or
      hyphens to connect clauses or sentences.
    - Never use `$` or LaTeX math delimiters.

### Workspace Crate Showcase

Ground every pattern in production code from `r3bl-open-core` and link directly to GitHub
source files, `crates.io`, and `docs.rs` so readers can inspect real implementations and
discover all five workspace crates:

- **`r3bl_tui`** (Library):
    - **Role:** Async TUI/CLI engine, process & `PTY` orchestration, HTTP/git scripting,
      network I/O, and zero-copy gap buffer.
    - **Links:** [GitHub](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui) ·
      [crates.io](https://crates.io/crates/r3bl_tui) · [docs.rs](https://docs.rs/r3bl_tui)
- **`r3bl-rust-analyzer-mcp-server`** (Library + Binary: `rust-analyzer-mcp-server`):
    - **Role:** Long-running JSON-RPC `MCP` server wrapping `rust-analyzer` LSP for AI
      coding agents.
    - **Links:**
      [GitHub](https://github.com/r3bl-org/r3bl-open-core/tree/main/rust-analyzer-mcp-server)
      · [crates.io](https://crates.io/crates/r3bl-rust-analyzer-mcp-server) ·
      [docs.rs](https://docs.rs/r3bl-rust-analyzer-mcp-server)
- **`r3bl-build-infra`** (Library + Binary: `cargo-rustdoc-fmt`):
    - **Role:** Rust workspace developer tooling and lossless CST-based rustdoc formatter
      subcommand.
    - **Links:**
      [GitHub](https://github.com/r3bl-org/r3bl-open-core/tree/main/build-infra) ·
      [crates.io](https://crates.io/crates/r3bl-build-infra) ·
      [docs.rs](https://docs.rs/r3bl-build-infra)
- **`r3bl-cmdr`** (Library + Binaries: `giti`, `edi`, `env-source`):
    - **Role:** Async developer tools for interactive Git workflows, Markdown editing,
      shell env loading, and self-upgrade.
    - **Links:** [GitHub](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) ·
      [crates.io](https://crates.io/crates/r3bl-cmdr) ·
      [docs.rs](https://docs.rs/r3bl-cmdr)
- **`r3bl_analytics_schema`** (Library):
    - **Role:** Pure wire-protocol schema shared between client binaries (`r3bl-cmdr`) and
      backend services.
    - **Links:**
      [GitHub](https://github.com/r3bl-org/r3bl-open-core/tree/main/analytics_schema) ·
      [crates.io](https://crates.io/crates/r3bl_analytics_schema) ·
      [docs.rs](https://docs.rs/r3bl_analytics_schema)

---

## Tutorial Blueprint

### Section 1: Real-World Origin

- **Hook (Anchored in `r3bl-open-core`):** Tell the candid story of how error handling
  evolved in `r3bl-open-core`:
    1. Starting with `CommonResult<T> = miette::Result<T>` and `CommonError` was pragmatic
       and productive when the workspace was small.
    2. As the monorepo grew into five published crates spanning foundational libraries, a
       JSON-RPC `MCP` server, a Cargo subcommand formatter, shared telemetry schemas, and
       user-facing binaries, using one type-erased `Result` convention everywhere began
       creating friction across crate boundaries.
- **Introduce the `r3bl-open-core` Crate Map:** Present the five-crate overview from
  `Workspace Crate Showcase` above so readers see the diversity of targets (libraries,
  background protocol servers, build tools, schemas, and interactive binaries) and have
  direct links to explore the repo.

### Section 2: Problem Domain

Consolidate problem space and concrete `r3bl-open-core` scenarios into one unified section
showing the four organic pressures that emerged as the workspace scaled:

1. **Cross-crate callers need to inspect and compose failures programmatically:**
    - Returning `miette::Result<T>` from library helpers (`r3bl_tui`, `r3bl-build-infra`,
      `r3bl-cmdr`) erases concrete failure modes into `miette::Report`.
    - Callers (whether a JSON-RPC handler, a self-upgrade state machine, or a unit test)
      lose exhaustive `match` branching and structured `#[source]` chaining, falling back
      to string checks or formatted string wrappers (`Other(format!("{err:#}"))`).
2. **Executed operations return domain states and boundary summaries beyond `Result`:**
    - Subprocess pipelines (`tui/src/core/script/command_impl/`), state-machine steps
      (`cmdr/src/self_upgrade/`), and protocol/operator summaries represent multi-way
      outcomes that are not binary `Ok`/`Err` values.
    - Naming an OS exit classifier `CommandOutputResult`, a tuple alias
      `ResultAndCommand`, and a formatted summary `CommandRunResult` produced nested
      signatures like `CommonResult<CommandRunResult<Details>>` where
      `std::result::Result` wrapped a domain enum.
3. **Third-party crate errors and implicit `#[from]` conversions create boundary
   friction:**
    - Returning raw `reqwest::Error`, `serde_json::Error`, or `std::io::Error` runs into
      Rust's orphan rule (foreign errors do not implement `miette::Diagnostic`), leaking
      `.into_diagnostic()` calls and upstream types into public APIs.
    - Using `#[from]` obscures which variant bare `?` constructs at call sites and hits
      duplicate `From<Upstream>` compiler limits whenever two steps in one workflow share
      `std::io::Error` or `reqwest::Error` (e.g., `WorkspaceRootError` in
      `build-infra/src/common/workspace_utils.rs`).
4. **Infallible helpers and hot paths pay avoidable ergonomic and allocation costs:**
    - Wrapping infallible helpers in `CommonResult<()>` forces callers to propagate `?`,
      call `.ok()`, or handle unreachable `Err` branches.
    - Constructing `miette!("...")` errors inside tight buffer and coordinate checks
      (`OfsBuf`, `ZeroCopyGapBuffer`) heap-allocates formatted strings on hot paths.

### Section 3: Mental Model

Derive two domain-agnostic organizing principles directly from the four pressures in
Section 2:

1. **Principle 1: Reusable Library vs. System-Edge Boundary:**
    - **Libraries** (`r3bl_tui`, `r3bl_build_infra`, `r3bl_rust_analyzer_mcp_server` lib,
      internal `r3bl_cmdr` modules) return concrete typed errors deriving
      `#[derive(Debug, thiserror::Error, miette::Diagnostic)]`, or bare `T` when
      infallible (such as pure data crates like `r3bl_analytics_schema`).
    - **System edges** (`main()`, JSON-RPC/HTTP server dispatchers, Cargo subcommands,
      CLI/app entry points, dyn-compatible plugin/UI traits, `#[test]`) convert typed
      errors into protocol responses or `miette::Result<T>`. Because `miette::Report`
      implements `From<E: miette::Diagnostic>`, bare `?` bridges typed library errors into
      application edges with zero `.into_diagnostic()` boilerplate.
2. **Principle 2: The 4 Semantic Roles (`*Error`, `*Result`, `*Outcome`, `*Report`):**
    - Show how each pressure in Section 2 maps to a distinct question and type suffix:
        - **`*Error`**: _What went wrong when a fallible step failed?_ (Typed struct/enum
          returned in `Err(...)` for programmatic matching and cause chains).
        - **`*Result`**: _Did this fallible call succeed or fail?_ (Reserved exclusively
          for `std::result::Result<T, E>` or thin aliases like `miette::Result<T>` /
          `FsOpResult<T>`; omit `Result` altogether when infallible. Pair fallible free
          functions and inherent methods with a `try_` prefix such as
          `try_get_workspace_root` or `try_from_seed_file` so fallibility is visible at
          every call site, reserving unprefixed names for infallible calls, `main`,
          traits, and unconditional `Err` constructors).
        - **`*Outcome`**: _What structured domain state did an executed operation produce
          for downstream code?_ (Unformatted domain enum or struct inspected by callers).
        - **`*Report`**: _What formatted diagnostic or summary artifact crosses an
          external boundary?_ (Presentation or boundary payload for human operators,
          structured logs, or wire/UI edges, never parsed back into domain control flow).
    - Include a 4-row comparison table (Role, Suffix, Question Answered, Rust Shape &
      Traits, Production `r3bl-open-core` Examples) and an ASCII data-flow diagram.

### Section 4: Solution Patterns

Walk through four concrete implementation patterns with verified code and direct GitHub
links into `r3bl-open-core`:

1. **Subsection 4.1: Choosing `*Error` Shapes by Cardinality & Newtype Wrappers**
    - Derive `#[derive(Debug, thiserror::Error, miette::Diagnostic)]` declaratively.
    - Match the Rust type shape to failure-mode cardinality (avoiding 1-variant enums):
        - **Unit struct (1 mode, no payload):** `MdParseError` in
          `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs`.
        - **Newtype / field struct (1 mode, with cause or context):** `HttpClientError` in
          `tui/src/core/script/http_client.rs` and `InvalidHexColorError` in
          `tui/src/core/ansi/color/rgb_value.rs`. Detail how the newtype wrapper works
          cleanly with Rust's orphan rule, adds domain context while preserving
          `Error::source()`, distinguishes failure domains, and insulates public APIs from
          dependency updates.
        - **Multi-variant enum (2+ modes):** `DownloadFileError` in
          `tui/src/core/script/download.rs`, `GetWorkspaceRootError` in
          `build-infra/src/common/workspace_utils.rs`, and `McpServerError` in
          `rust-analyzer-mcp-server/src/error.rs` (paired with JSON-RPC error mapping in
          `rust-analyzer-mcp-server/src/mcp/server.rs`).
        - **Zero-allocation `Copy` enums for hot paths:** `OfsBufBoundsError` in
          `tui/src/tui/terminal_lib_backends/ofs_buf/core.rs` and `GapBufferError` in
          `tui/src/tui/editor/zero_copy_gap_buffer/mod.rs`.
2. **Subsection 4.2: Explicit `#[source]` + `.map_err(...)` Over Implicit `#[from]`**
    - Present the two-part mechanics:
        - `#[source]` implements `std::error::Error::source(&self)` so `miette` renders
          the full `╰─▶` cause chain and callers can walk `.source()`.
        - `#[from]` equals `#[source]` **plus** an implicit `impl From<Inner> for Outer`.
    - Compare omitting both attributes (cause chain disappears unless a field is named
      `source`) vs. keeping `#[source]` without `#[from]` (identical diagnostics and cause
      chains, explicit `.map_err(...)` at call sites).
    - Show side-by-side code from `GetWorkspaceRootError` (`r3bl-build-infra`) and
      `DownloadFileError` (`r3bl_tui`) illustrating call-site clarity, zero duplicate
      `From<std::io::Error>` / `From<reqwest::Error>` collisions, intentional mapping
      during refactors, and uniform syntax across single-field and context-carrying
      variants.
3. **Subsection 4.3: Composing `*Outcome`, `*Error`, `*Result`, and `*Report` Together**
    - Walk through the end-to-end process execution pipeline in
      `tui/src/core/script/command_impl/`:
        - `ProcessExecutionOutcome` (`process_execution_outcome.rs`) classifying
          `io::Result<Output>` into `Success(Output)`, `NonZeroExit(Output)`, or
          `SpawnFailed(IoError)`.
        - `CommandRunError` (`command_run.rs`) for fallible `cmd.run().await` /
          `try_run` calls.
        - `CommandRunOutcome<T>` (`command_run.rs`)
          replacing a positional `(Result<T, E>, Command)` tuple with named
          `{ result, command }` fields.
        - `CommandRunReport<T>` (`command_run_report.rs`) formatting `Noop | Run | Fail`
          for display and `tracing` output
          (`Result<CommandRunReport<Details>, GitiBranchError>`, converted to
          `miette::Report` strictly in `cmdr/src/bin/giti.rs`).
    - Show `StartOutcome` in `cmdr/src/self_upgrade/upgrade_journey/start_outcome.rs` as a
      domain state-machine `*Outcome`, and show returning bare `T` on infallible helpers.
4. **Subsection 4.4: System Edges Across Servers, Subcommands, Apps & Dyn Traits**
    - Compare how typed library errors connect cleanly to four different system edges in
      `r3bl-open-core`:
        - **JSON-RPC `MCP` server edge:** `rust-analyzer-mcp-server/src/main.rs` and
          `rust-analyzer-mcp-server/src/mcp/server.rs`.
        - **Cargo subcommand edge:** `build-infra/src/bin/cargo-rustdoc-fmt.rs`.
        - **Async application binaries:** `cmdr/src/bin/giti.rs`, `cmdr/src/bin/edi.rs`,
          and `cmdr/src/bin/env-source.rs`.
        - **Dyn-compatible trait boundaries:** `App`, `Component`, and `SurfaceRender` in
          `tui/src/tui/terminal_window/`.

### Section 5: Decision Reference

- Provide a bookmarkable ASCII decision flowchart and a concise summary table mapping
  everyday design questions ("Can this function fail?", "Should it carry a `try_`
  prefix?", "Is this a library or a system edge?", "How many failure modes exist?", "Is
  the caller branching in code or rendering at a boundary?") to the exact Rust pattern and
  naming convention.

---

## Implementation Plan

### Phase 1: Blueprint & Narrative Alignment

- [x] Refine constructive tone, consolidate problem domain with real scenarios, generalize
      beyond CLI/TUI, and anchor the narrative in `r3bl-open-core` across all five crates.
- [ ] **Mandatory manual review:** Verify the tutorial blueprint and authoring
      instructions before drafting the article.
    - [ ] `task/error-design-pattern-article.md`

### Phase 2: Draft & Verify Tutorial

- [ ] Confirm target output file path (or `developerlife.com` repo path) for the tutorial
      markdown draft.
- [ ] Verify all extracted code snippets, type signatures, and GitHub / `crates.io` /
      `docs.rs` URLs against `r3bl-open-core` `main`.
- [ ] Draft Sections 1 through 5 following the `Tutorial Blueprint` and
      `Tone & Style Rules`.
- [ ] Format markdown with `prettier --write` and audit for link accuracy, font-safe ASCII
      diagrams, and prose rules.
- [ ] **Mandatory manual review:** Verify every file created or modified in this phase.
    - [ ] `task/error-design-pattern-article.md`

### Phase 3: Community Publicity

Once the tutorial is live on `developerlife.com`, draft channel-tailored copy and share
across developer communities:

- [ ] **This Week in Rust (TWiR):**
    - Pull latest `main` in `/home/nazmul/github/this-week-in-rust`, create a feature
      branch, add the `developerlife.com` tutorial entry under
      `Updates from Rust Community` (`Rust Walkthroughs` or `Observations/Thoughts`) in
      the upcoming draft issue, and open a PR to `rust-lang/this-week-in-rust`.
- [ ] **Hacker News (HN):**
    - Submit the `developerlife.com` URL with a concise, factual title (e.g.,
      `Designing Error, Outcome, and Report Types in Multi-Crate Rust Workspaces`) and
      prepare a concise first comment summarizing the `r3bl-open-core` context and GitHub
      repo links.
- [ ] **Reddit:**
    - **`r/rust` (Primary):** Follow Rule 2 (descriptive title, no `[Blog]` tag) and Rule
      6 (at most 1 self-promotion post per week) with `Article` / `Tutorial` flair; pair
      the link with a high-signal technical summary covering the 4 semantic roles,
      `#[source]` vs. `#[from]`, and direct `r3bl-open-core` source links.
    - **`r/programming` & `r/softwarearchitecture`:** Share the `developerlife.com`
      article focusing on multi-layer error, domain outcome, and boundary report design in
      typed systems.
- [ ] **LinkedIn:**
    - Draft an engineering post anchored in the `r3bl-open-core` evolution across all five
      crates (`r3bl_tui`, `r3bl-rust-analyzer-mcp-server`, `r3bl-build-infra`,
      `r3bl-cmdr`, `r3bl_analytics_schema`), highlighting the 4-role mental model and
      linking to both the `developerlife.com` tutorial and the GitHub monorepo.
- [ ] **Mandatory manual review:** Review all drafted submission blurbs and social posts
      before publishing.
    - [ ] `task/error-design-pattern-article.md`
