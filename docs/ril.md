# Rust Innovation Lab (RIL) Membership Application: r3bl_tui

> **Submitted Application**: [Rust Foundation Innovation Lab (RIL) Issue #15](https://github.com/rustfoundation/rust-innovation-lab/issues/15) (Submitted August 28, 2026)

## Section 1: About the project and about you

### Field 1: Elevator Pitch (`about-project`)
*Give us the project's elevator pitch! What does it do? What are its aims/goals? How will being part of the Rust Innovation Lab help achieve that?*

#### What is `r3bl_tui`?
`r3bl_tui` is a modern, production-grade, purely asynchronous TUI and CLI infrastructure framework built from the ground up in Rust and Tokio (part of the R3BL open-source ecosystem with 2.7M+ crates.io downloads). It provides foundational systems-level terminal emulation, layout, rendering, and text editing infrastructure for next-generation developer tooling, AI coding harnesses/agents, and remote cloud workflows.

#### Core Architectural Innovation
While modern web and desktop UI architectures have evolved dramatically, terminal applications have largely remained constrained by 1990s-era paradigms relying on platform-specific blocking operations (e.g., POSIX `readline()` or Windows `ReadConsole()`). `r3bl_tui` replaces this legacy model with:
- **Purely Asynchronous & Non-Blocking**: Built on Tokio, ensuring the main event loop never blocks, making concurrent I/O, streaming LLM tokens, and background process orchestration completely non-blocking.
- **Immediate-Mode Reactive UI**: Unidirectional Elm-style state management with a clean separation of state mutation and rendering. State changes trigger deterministic re-rendering with zero thread locks.
- **Cross-Platform & SSH-Optimized**: Native support across Linux, macOS, and Windows. Highly optimized for remote cloud administration over SSH by calculating and painting only offscreen buffer diffs.
- **Declarative Layout Engine**: Flexbox-style layouts, CSS-like styling cascades, and declarative React/JSX-like macros.

#### Making Illegal States Unrepresentable via Algebraic Data Types (ADTs) & the Type System
A core engineering philosophy in `r3bl_tui` is leveraging Rust's rich type system and **Algebraic Data Types (ADTs)** to eliminate entire classes of runtime defects at compile time:
- **Exhaustive State Representation (Sum Types / Enums)**: Core events, terminal rendering operations (`RenderOp`), parser elements (`MdElement`), and component lifecycles are modeled as strict ADTs. This guarantees exhaustive compile-time pattern matching and prevents unhandled or illegal state transitions.
- **Eliminating Domain Mangling (Newtype Wrappers)**: Distinct, strongly-typed domain wrappers prevent accidental mixing of orthogonal units—for instance, making it impossible to pass a `RowIndex` where a `ColIndex` is expected, or to conflate `Width` and `Height`.
- **Canvas Memory vs. Viewport Display Separation**: Strict type-level separation between memory storage space (`StorageCoordinate` / `usize`) and physical screen display space (`ScreenCoordinate` / `u16`). Explicit conversion traits (`ViewportToCanvasExt`, `CanvasToViewportExt`) govern transformations, preventing silent downcast truncations, integer overflows, and rendering desynchronization across terminal resizing.
- **Compile-Time Elimination of Off-by-One Errors**: Coordinate arithmetic, boundary clipping, and buffer indexing enforce strict invariants at the type and trait level, eradicating the classic off-by-one errors common in terminal coordinate math.

#### Virtual Terminal Emulation & Systems-Level Innovations
Recent deep systems engineering in `r3bl_tui` has introduced groundbreaking capabilities for terminal multiplexing and virtual buffer management:
- **Dual-Tier Offscreen Buffer (`OfsBuf`) Backing Store**:
  - `Flat2DArray` with SIMD acceleration for alternate-screen fullscreen applications requiring ultra-low-latency screen swaps and maximum cache locality.
  - `GrowableBuffer` for primary terminal buffers supporting native scrollback history and 2D navigation.
- **Continuous 2D Panning & Virtual Terminal Geometry (`PTYMux`)**: Full VT-100 ANSI emulation and PTY multiplexing with configurable virtual terminal widths, allowing continuous horizontal and vertical panning across wide process outputs (compilers, build logs, interactive TUIs).
- **Zero-Flicker Process Switching**: Per-process virtual offscreen buffers enable instant switching across interactive shells (bash/zsh/fish), TUIs (vim/htop), and CLI tools while maintaining live background updates.
- **Applet State Persistence**: Systems-level state management allowing processes built using `r3bl_tui` to persist their reactive state across lifecycles and share it across instances/processes.

#### High-Performance Markdown Engine, Custom Syntax Highlighting & Editor
Terminal-based documentation and collaborative code execution require first-class text processing. `r3bl_tui` includes a complete, deeply integrated text stack:
- **Zero-Copy Markdown Parser (`nom` + `ZeroCopyGapBuffer`)**: A fast, zero-copy Markdown AST parser operating on a null-padded gap buffer invariant, eliminating redundant allocations and safely parsing documents with frontmatter metadata (KV/KCSV), headings, smart lists, nested inline fragments (links, bold, italic), and fenced code blocks.
- **Real-Time Custom Syntax Highlighting (`md_parser_syn_hi`)**: A hybrid highlighting engine combining Markdown AST styling with Syntect code-block highlighting, supporting customizable stylesheets and truecolor (24-bit RGB) ANSI styling pipelines.
- **Production-Grade Markdown Editor Component (`EditorEngine`)**:
  - **Grapheme-Cluster Safe (`gc_string`)**: Immune to cursor drift and rendering artifacts when handling emojis, multi-byte Unicode sequences, and East Asian wide characters.
  - **Anchor-and-Line Selection Model**: Precise multi-line text selection, viewport bounds checking, and native clipboard integration.
  - **Transactional Undo/Redo**: Full history buffer (`EditorHistory`) with smooth viewport scrolling and live syntax highlighting on every keystroke.

#### Aims & Goals
1. **Modernize Developer & AI Tooling**: Provide the foundational terminal infrastructure required for the modern era of AI coding harnesses, local-first collaborative Markdown execution, and remote cloud infrastructure management.
2. **Standardize Rust TUI Architecture**: Deliver a rock-solid, memory-safe, ergonomic foundation that lets Rust developers build sophisticated terminal applications as easily as building modern web apps.

#### Why the Rust Innovation Lab?
`r3bl_tui` demonstrates the pinnacle of what Rust makes possible—fearless concurrency, zero-cost abstractions, memory safety, SIMD vectorization, and low-level cross-platform systems engineering. Joining the Rust Innovation Lab will provide:
- **Neutral Stewardship & Governance**: Establish long-term community stewardship and formal governance to grow `r3bl_tui` into an enduring standard for the Rust ecosystem.
- **Rust Ecosystem Alignment & Reach**: Deepen collaboration with the wider Rust developer community, crate authors, and working groups to drive adoption and interoperability.
- **Fiscal & Administrative Sponsorship**: Provide the operational stability needed to accelerate roadmap milestones like enhanced accessibility, expanded PTY capabilities, and headless virtual terminal testing harnesses.

---

### Field 2: Key Personnel (`project-people`)
*Who are the project's key personnel? List their names and contact info (one person per line).*

[Nazmul Idris (Founder, Principal Architect & Maintainer)](https://nazmulidris.com) - idris@developerlife.com

---
---

## Section 2: Governance

### Field 1: Governing Document (`governing-document`)
*Do you have a governing document?*

**Option**: `No`

*(Note: The Rust Foundation assists projects in adopting/drafting a standard RIL governance charter).*

---

### Field 2: Project Involvement (`project-involvement`)
*Who is involved in the project? We tend to use the term 'members' to describe these people. You might call them participants, contributors, or something else.*

The project community consists of:
- **Lead Architect & Maintainer**: Nazmul Idris (responsible for core architectural design, systems implementation, release management, documentation, and technical education).
- **Community Contributors**: Open-source contributors who submit pull requests, bug fixes, feature requests, and documentation improvements across the `r3bl-open-core` repository.
- **Ecosystem Users & Developers**: Downstream developers and systems engineers using `r3bl_tui` crates (2.7M+ total downloads) across Linux, macOS, and Windows who provide active feedback on the use cases they need, API ergonomics, terminal emulator compatibility, and performance.

---

### Field 3: Project Leadership (`project-lead`)
*Who is in charge of the project? We tend to use the term 'leadership' to describe these people. They are the ones with decision-making power over the project's direction. You might call them a Board.*

Decision-making power currently rests with the project founder and lead maintainer, Nazmul Idris. 

As part of joining the Rust Innovation Lab, our goal is to transition to a formal **Project Leadership / Steering Team** model by inviting trusted, active community contributors and key ecosystem stakeholders into formal decision-making roles to ensure shared governance.

---

### Field 4: Leadership Appointments & Removals (`leadership-appointments`)
*What is the process for appointing/removing leadership?*

- **Appointment**: Any contributor who demonstrates sustained technical contribution, deep architectural alignment, and commitment to community values may be nominated and appointed to leadership through consensus of the existing steering team.
- **Stepping Down / Removal**: Leaders may step down voluntarily at any time, transitioning to an emeritus status. If a leader becomes inactive for an extended period or violates the Code of Conduct, removal will follow the established RIL governance procedures under Rust Foundation oversight.

---

### Field 5: How Leadership Runs the Project (`running-project`)
*Give an outline description of how leadership runs the project. How do ideas get turned into proposals for decision-making? Do you vote on decisions, or reach consensus in some other way? Are there leadership meetings? How often?*

- **Idea to Proposal**: New features, architectural refactors, and breaking changes originate as GitHub Discussions or RFC-style Tracking Issues with detailed technical plans (e.g., coordinate unit migrations, offscreen buffer redesigns, or PTY subsystem enhancements).
- **Consensus & Review**: Technical decisions are made through open consensus. Major systems changes require prototypes, benchmark validations, and public PR reviews before landing.
- **Meetings & Communication**: Collaboration is primarily asynchronous via GitHub Issues/PRs and community chat. As the leadership team expands under RIL, regular monthly/quarterly syncs will be established for roadmap planning and milestone reviews.

---

### Field 6: Project Sustainability & Wind-Down (`project-sustainability`)
*What will happen if the project is no longer sustainable and needs to be wound down?*

If the project ever becomes unsustainable for the existing team:
1. **Maintainer Handover**: We will work with the Rust Foundation to identify new maintainers from the active contributor community or broader Rust ecosystem to take over stewardship.
2. **Archival & Open Preservation**: If no successor is found, the repositories, crates.io packages, and documentation will be permanently archived under the Apache 2.0 license with clear deprecation notices. All assets and intellectual property will remain publicly accessible under the ongoing stewardship of the Rust Foundation.

---

### Field 7: Code of Conduct (`code-of-conduct`)
*Do you have a Code of Conduct?*

**Option**: `Yes`

---

### Field 8: Code of Conduct URL (`code-of-conduct-url`)
*If you have a Code of Conduct, please provide a URL where we can read it.*

`https://github.com/r3bl-org/r3bl-open-core/blob/main/CODE_OF_CONDUCT.md`

---

### Field 9: Project Management Support Required (`project-management-support`)
*What level of project management/support do you envisage your project requiring from the Foundation?*

We anticipate lightweight-to-moderate administrative and strategic support:
- **Governance & Bylaws**: Guidance on formalizing a project charter and steering team framework under the RIL structure.
- **Marketing & Community Outreach**: Amplifying major releases, architectural breakthroughs, and RFCs through official Rust Foundation communication channels (blogs, newsletters, and announcements).
- **Ecosystem Connections**: Facilitating connections with other Rust projects, working groups, and industry partners building terminal tooling, cloud infrastructure, or AI developer harnesses.
- **Fiscal & Legal Sponsorship**: Managing fiscal sponsorship, trademark protection, and non-profit reporting if external sponsorship or grant funding is secured in the future.

---
---

## Section 3: Financial

### Field 1: Project Funding (`project-funding`)
*Is your project funded, do you expect it to be funded in the near future, or can you sustain your own funding requirements (even if your funding requirements are $0)?*

**Option**: `Yes`

*(Note: The project operates with a $0 operational budget requirement and is self-sustaining).*

---

### Field 2: Money Management (`money-management`)
*If you have funding already, describe how you currently manage the money. If there is no money to manage because it is not required, say so here and then say "No" to a budget below.*

The project currently has a $0 direct operational funding requirement and is fully self-sustaining. All development, CI/CD workflows, documentation hosting, and crate distribution are handled through open-source infrastructure (GitHub Actions, crates.io, docs.rs) and the maintainer's personal developer workstations. 

There are currently no dedicated project funds or bank accounts to manage. If external grant or sponsor funding is secured in the future, we would look to the Rust Foundation's fiscal sponsorship infrastructure to manage accounts, disbursements, and financial reporting.

---

### Field 3: Budget (`budget`)
*Do you have a budget?*

**Option**: `No`

---
---

## Section 4: Trademark and Branding

### Field 1: Existing Trademarks (`trademarks`)
*Please list and describe any trademarks you already have, and what territories they are trademarked in.*

None registered formally. The project currently relies on common-law open-source trademark recognition for the names **"R3BL"** and **"r3bl_tui"** across the global developer and Rust open-source community.

---

### Field 2: Future Trademarks (`future-trademarks`)
*Please list and describe anything you'd like to trademark in future, and what territories you want to apply for the trademark in.*

Under the stewardship of the Rust Foundation / RIL, we would welcome exploring trademark registration for the wordmark and logo marks for **"R3BL"** and **"r3bl_tui"** (principally in the United States and EU/international territories) to protect the project name against brand confusion or unauthorized commercial appropriation.

---

### Field 3: Branding Assets (`branding-assets`)
*What branding assets do you already have, if any? (Logo files, fonts, a color palette, etc.)*

- **Vector Logos & Icons**: Dedicated SVG logo and icon assets for `r3bl_tui`, `r3bl-term`, and `r3bl-cmdr` located in the repository.
- **Color Palette & Styling**: Standardized ANSI/Truecolor theme palettes and styling constants integrated directly into the `r3bl_tui` rendering pipeline.
- **Web & Social Assets**: Consistent visual banners, avatars, and headers across GitHub, [developerlife.com](https://developerlife.com), and the [YouTube channel](https://youtube.com/@developerlifecom).

---

### Field 4: Brand Usage Framework (`brand-usage-framework`)
*Do you already have a framework for the usage of your trademark(s) and branding asset(s)? Please briefly describe here, and attach any relevant documents.*

Currently, the project operates under a standard open-source convention allowing community members to freely use the name and logo to reference, build upon, and link to the project, provided it does not imply official endorsement or ownership. 

As part of the RIL onboarding, we intend to adopt the Rust Foundation's standard Trademark and Brand Usage Policy guidelines.

---
---

## Section 5: Infrastructure

### Field 1: Project Website (`project-website`)
*Does your project have a website?*

**Option**: `Yes`

---

### Field 2: Code Repositories (`project-code`)
*Does your project have code repositories?*

**Option**: `Yes`

---

### Field 3: Mailing List (`project-mailing-list`)
*Does your project have a mailing list?*

**Option**: `No`

---

### Field 4: Forums (`project-forums`)
*Does your project have forums?*

**Option**: `Yes`

---

### Field 5: Chat Platforms (`project-chat-platforms`)
*Does your project have any chat platforms?*

**Option**: `No`

---

### Field 6: Collaboration Spaces (`project-collaboration-spaces`)
*Does your project have any other collaboration spaces?*

**Option**: `Yes`

---

### Field 7: Infrastructure Information (`infrastructure-information`)
*If you answered "Yes" to any of the above infrastructure questions, please provide URLs and any relevant access information here.*

- **Code Repositories & Collaboration**:
  - Monorepo & Collaboration Spaces (Issues, Projects, PRs): https://github.com/r3bl-org/r3bl-open-core
  - Crates.io Registry: https://crates.io/crates/r3bl_tui (and ecosystem crates under `r3bl_*`)
- **Forums / Community Discussions**:
  - GitHub Discussions: https://github.com/r3bl-org/r3bl-open-core/discussions
- **Websites & Documentation**:
  - Project & Personal Site: https://nazmulidris.com
  - Technical Blog & Deep-Dives: https://developerlife.com
  - API Documentation: https://docs.rs/r3bl_tui/latest/r3bl_tui/

---
---

## Section 6: Licensing and Data

### Field 1: Open License (`project-license`)
*Is the project's work made available under an open license?*

**Option**: `Yes`

---

### Field 2: Open Source License List (`project-license-list`)
*If your project is under an open source license, which license(s) is it?*

`Apache-2.0`

---

### Field 3: Contributor License Agreement (`project-cla`)
*Do you operate a Contributor License Agreement (CLA) system?*

**Option**: `No`

---

### Field 4: CLA Details (`project-cla-details`)
*If you answered "yes", please summarize the process and the license conditions below, and attach a copy of the full license. If "no", how will you ensure the Rust Foundation is able to use/distribute the intellectual property contained within your project?*

All source code and assets are licensed under the **Apache License, Version 2.0**. 

Under **Section 5 of the Apache 2.0 License** (*"Submission of Contributions"*), any contribution submitted for inclusion in the project is automatically subject to the terms and conditions of the Apache 2.0 license with full patent and copyright grants to the project and its distributors, without any additional terms. Contributions are accepted publicly via GitHub Pull Requests where the Apache 2.0 licensing terms are explicitly stipulated in `CONTRIBUTING.md`.

Furthermore, as part of joining the Rust Innovation Lab, we are happy to implement a **Developer Certificate of Origin (DCO)** sign-off check or adopt any contributor licensing workflow recommended by the Rust Foundation.

---

### Field 5: Personally Identifiable Information (`project-pii`)
*Does your project collect or store Personally Identifiable Information (PII)?*

**Option**: `No`

*(r3bl_tui is a local-first terminal infrastructure library and framework. It contains zero telemetry, zero analytics tracking, and collects no user data whatsoever).*

---

### Field 6: Collecting PII (`project-pii-details`)
*If you answered "yes" on PII, please describe the types of personal data you collect, your reasons for collecting it, and the retention period for the data. Have you done a Data Protection Impact Assessment (or similar)?*

*(Leave blank)*

---

### Field 7: Privacy Policy (`project-privacy-policy`)
*Do you have a privacy policy?*

**Option**: `No`

---

### Field 8: Privacy Policy URL (`privacy-policy-url`)
*If you have a Privacy Policy, please provide a URL where we can read it.*

*(Leave blank)*

---
---

## Section 7: Information Checklist

### Field 1: Information Checklist (`information-checklist`)
*Please check the box for each document you provided a URL for, or indicated you will provide to us.*

- [ ] Governing Document
- [x] Code of Conduct
- [ ] Guidelines for trademark and/or branding usage
- [ ] Contributor License Agreement (CLA)
- [ ] Privacy Policy
