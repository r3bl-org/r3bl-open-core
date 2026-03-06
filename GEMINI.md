# Gemini CLI Configuration (Single Source of Truth)

# 1. Inherit Core Repository Rules
@CLAUDE.md

**System Directive: SSOT**
You are operating in the current repository. Your entire behavior, coding style, and toolset are governed by existing Claude Code configurations. You must strictly obey all rules, formatting guidelines, and architectural constraints defined in `CLAUDE.md` (or the repository's primary Claude configuration file) as if they were written directly for you.

# 2. Tooling & Capabilities (Dual-Engine)
You have two strict sets of tools at your disposal. Use them appropriately:

**A. Semantic Rust Tools (MCP):** You are connected to powerful Rust MCP servers. You must actively use these built-in tools for all code navigation (go-to-definition, finding references), deep architectural analysis (call graphs), and precise compiler-driven refactoring.

*   **The Data Diet Rule:** Some of your tools (especially from Narsil) generate massive, voluminous outputs like call graphs and ASTs. **You must protect your context window.**
    *   **Narsil Tools:** Whenever you request a graph, search, or trace, you MUST pass strict parameters to limit the output. Never dump unbounded, raw codebase graphs into your memory. If you need more info, take it step-by-step.
    *   **Rust-Refactor Tools:** These tools are surgical. Always provide precise `file_path`, `line`, and `character` coordinates to ensure fast, focused responses.
*   **Mandatory Limits Reference (Narsil):**
    | Tool Category | Key Tools | Mandatory Limit Parameter | Recommended Default |
    | :--- | :--- | :--- | :--- |
    | **Search** | `search_code`, `semantic_search` | `max_results` | 5 to 10 |
    | **Symbols** | `workspace_symbol_search` | `limit` | 10 to 20 |
    | **Graphs** | `get_code_graph` | `depth` | 1 |
    | **Structure** | `get_project_structure` | `max_depth` | 2 |
    | **Snippets** | `get_excerpt` | `max_lines` | 50 |
    | **Security** | `scan_security` | `max_findings` | 20 |

**B. Local Workflows (.claude/):** For repo-specific workflows (like running clippy, formatting, or analyzing logs), your capabilities are defined in the `.claude/` directory. Whenever I ask you to run a codebase skill, agent, or check quality, you must:
1. Look inside the `.claude/` directory.
2. Read the markdown instructions inside that folder.
3. Execute the underlying shell/scripts exactly as instructed.

# 3. Local Skills, Agents & Commands
**Execution Rule:** Do not look for Gemini-specific native skills. Your capabilities are defined strictly in the `.claude/` directory.

**Proactive Usage:** You MUST autonomously identify when a task matches a skill, agent, or command below. Before acting, read the corresponding markdown file to load the expert instructions into your context.

**Supporting Files:** When loading a skill, you MUST also check for and read any supporting `.md` files in that skill's directory (e.g., `patterns.md`, `reference.md`, `examples.md`) to understand the full context of the expert guidance.

### Skills (`.claude/skills/`)
| Skill Name | Purpose |
| :--- | :--- |
| **check-code-quality** | Full check (typecheck, build, clippy, tests, docs) |
| **run-clippy** | Linting, formatting, and style checks |
| **write-documentation** | Rustdoc formatting, link fixing, inverted pyramid style |
| **design-philosophy** | Cognitive load, type safety, illegal state design |
| **organize-modules** | Module structure, barrel exports, re-exports |
| **check-bounds-safety** | Safe terminal cursor and viewport calculations |
| **release-crate** | Crate versioning and publishing workflow |
| **analyze-log-files** | Processing logs with ANSI escape codes |
| **analyze-performance** | Performance regression and flamegraph analysis |

### Agents (`.claude/agents/`)
| Agent Name | Purpose |
| :--- | :--- |
| **test-runner** | Expert in running tests and fixing failures |
| **clippy-runner** | Expert in linting and fixing style issues |
| **code-formatter** | Expert in bulk code formatting |
| **perf-checker** | Expert in performance regression analysis |

### Slash Commands (`.claude/commands/`)
When a user uses a `/command`, or when you identify a matching workflow, read its definition in `.claude/commands/`.

| Command | Purpose |
| :--- | :--- |
| **/check** | Runs the full `check-code-quality` checklist |
| **/clippy** | Runs clippy and enforces style standards |
| **/docs** | Formats and builds documentation |
| **/release** | Executes the full crate release workflow |
| **/r3bl-task** | Manages long-running task documentation |
| **/fix-comments** | Standardizes constant formatting in doc comments |
| **/fix-intradoc-links** | Resolves broken or un-idiomatic doc links |
| **/fix-md-tables** | Standardizes markdown table formatting |
| **/analyze-logs** | Strips ANSI codes and processes log files |
| **/check-regression** | Performs flamegraph-based performance checks |
| **/boxes** | Provides the approved Unicode box-drawing set |

# 4. The Global Context Guardrail
**System Directive: Surgical Mode**
You do not have the full codebase in memory. You must actively use your search and file-reading tools to gather local context.

**Rule:** If a request requires system-wide knowledge, global refactoring, or sweeping architectural changes, **DO NOT GUESS**. Stop immediately and reply exactly with:
> "I need global context for this. Please run your request again, starting your prompt with `@.`"
