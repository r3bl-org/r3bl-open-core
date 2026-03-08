# Task: Add lychee link checker to detect link rot

## Overview

External URLs in rustdoc comments rot over time (sites restructure, repos rename, branches
get deleted). `cargo doc --no-deps` validates intra-doc links but not external HTTP/HTTPS
URLs. Adding `lychee` to the toolchain formalizes link rot detection.

Design decision: integrate into `check.fish --full` scoped to git-modified files (staged +
unstaged). Lychee errors are aggregated into the pass/fail result like every other check.
No standalone `--lychee` flag — link checking only runs as part of `--full`.

### What to fix vs ignore

lychee will report errors that fall into categories:

- **Real 404s**: Fix the URL (find the new location via WebFetch or web search).
- **`file://` URIs**: Example paths in docs (e.g., `osc_hyperlink.rs`). Exclude in config.
- **Test fixture URLs**: Like `http://test.com/` in test code. Exclude in config.
- **Rate limit 429s**: Accept as non-error in config.
- **Redirects**: Informational only. docs.rs short URLs always 302 (by design).

Note: Sites like `medium.com` that block automated requests (403 Forbidden) are **not**
excluded. We prefer to avoid linking to them and will replace them with more
stable/accessible alternatives when found.

## Implementation plan

### Phase 1: Installation and config

- [x] Add `lychee` to `run.fish` `install-cargo-tools` via `install_cargo_tool` pattern
- [x] Create `lychee.toml` at repo root with exclusions:
  - Exclude `^file://` (example URIs in docs)
  - Exclude `^https?://test\\.com` (test fixture)
  - Exclude `^https?://link1\\.com` (example URL in test/doc)
  - Accept 429 (rate limit) as non-error
  - Set reasonable timeout (30s) and max concurrency to avoid rate limiting

### Phase 2: check.fish integration

- [x] Add `check_lychee_changed_files` function to `check_cargo.fish` (scoped to git-modified files)
- [x] Add lychee to `run_full_checks` in `check_orchestrators.fish` (aggregated into pass/fail)
- [x] Update `--full` description and notification messages in `check.fish`
- [x] Update help text in `check_cli.fish`

### Phase 3: Skill and docs updates

- [x] Add lychee step to `check-code-quality/SKILL.md` after step 4 (docs), before linting
- [x] Add lychee section to `check-code-quality/reference.md` explaining the tool, config, and categories
- [x] Update the `check.fish --full` description in `CLAUDE.md`

### Phase 4: Test and verify

- [x] Run `fish run.fish install-cargo-tools` to verify idempotent install of lychee via cargo binstall
- [x] Run `./check.fish --full` to verify lychee checks links in git modified files and `lychee.toml` exclusions work correctly
