# Task: Self-Hosted Star History Workflow & Local CLI Integration

## Background

Set up a self-hosted Star History chart workflow in this repository to replace third party
SVG generators (`star-history.com`) with a locally generated SVG stored at
`.github/assets/star-history.svg`.

Initial design proposed a weekly GitHub Actions cron job, but we refined the strategy to
use **local CLI generation via `check.fish`** and manual GitHub Actions triggers
(`workflow_dispatch`), avoiding unnecessary automated background commits.

## Key Design & Architectural Choices

1. **TypeScript Implementation (`.github/workflows/generate-star-history.ts`)**:
    - Zero external npm package dependencies.
    - Executed cleanly via `npx -y tsx`.
    - Uses `gh api` with pagination (`Accept: application/vnd.github.v3.star+json`) to
      fetch all stargazer timestamps (`starred_at`).
    - Generates vector SVG with developer monospace typography
      (`Iosevka, 'JetBrains Mono', 'Fira Code', 'Cascadia Code', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace`).
    - Styled to GitHub dark theme aesthetics (`#0d1117` background, `#58a6ff` line/fill
      accent, `#8b949e` labels, `#21262d` grid).
    - Display title formatted cleanly as `${repo} (${count} ★)`.

2. **GitHub Actions Workflow (`.github/workflows/star-history.yml`)**:
    - Triggered manually via `workflow_dispatch` (weekly `cron` schedule was removed for a
      cleaner workflow).
    - Granted `permissions: contents: write` so GitHub Actions can push asset updates when
      manually triggered.
    - Commit message convention:
      `[github-actions] Update star history chart in README.md`.

3. **`check.fish` & `bootstrap.sh` Integration**:
    - `bootstrap.sh` already includes OS-level Node.js/npm installation (`install_nodejs`
      function).
    - Standalone CLI command: `./check.fish --star-history` validates `node`/`npx`
      availability in PATH and re-generates `.github/assets/star-history.svg` locally in
      ~1 second.
    - Integrated into `./check.fish --full` as part of the workspace verification
      pipeline.
    - Documented in `./check.fish --help` and CLI argument parser (`check_cli.fish`).

4. **`README.md` Update**:
    - Replaced third-party `star-history.com` `<picture>` HTML tags under
      `## Star History` with local relative path:
        ```markdown
        ![Star History](./.github/assets/star-history.svg)
        ```

## Goals & Status

- [x] Create TypeScript SVG generator at `.github/workflows/generate-star-history.ts`.
- [x] Create GitHub Actions workflow at `.github/workflows/star-history.yml` with
      `workflow_dispatch`.
- [x] Configure dark-mode styled SVG matching GitHub aesthetics (`#0d1117`) and
      Iosevka/JetBrains Mono monospace font stack.
- [x] Replace `star-history.com` HTML block in `README.md` with
      `![Star History](./.github/assets/star-history.svg)`.
- [x] Integrate `./check.fish --star-history` and `./check.fish --full` in `check.fish`
      modules (`check_cargo.fish`, `check_cli.fish`, `check_orchestrators.fish`) with
      `node`/`npx` validation.
- [x] Test and verify local SVG generation (`477 ★`).

## Phases & Mandatory Reviews

### Phase 1: Create TypeScript Generator & GitHub Actions Workflow

- [x] Create `.github/workflows/generate-star-history.ts`.
- [x] Create `.github/workflows/star-history.yml` with `workflow_dispatch` and
      `contents: write`.
- [x] Remove `cron` schedule in favor of local `check.fish` generation and manual
      triggers.
- [x] Mandatory manual review:
    - [x] `.github/workflows/star-history.yml`
    - [x] `.github/workflows/generate-star-history.ts`

### Phase 2: Update `README.md`

- [x] Replace `star-history.com` image block with
      `![Star History](./.github/assets/star-history.svg)`.
- [x] Mandatory manual review:
    - [x] `README.md`

### Phase 3: CLI Integration & Verification

- [x] Add `./check.fish --star-history` and include `star-history` in
      `./check.fish --full`.
- [x] Visually inspect generated SVG `.github/assets/star-history.svg` and `README.md`.
- [x] Mandatory manual review:
    - [x] `.github/assets/star-history.svg`
    - [x] `check.fish`
    - [x] `check_cli.fish`
    - [x] `check_cargo.fish`
    - [x] `check_orchestrators.fish`
