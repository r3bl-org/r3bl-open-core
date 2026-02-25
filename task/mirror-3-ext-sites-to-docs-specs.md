# Plan: Fix Broken URL + Mirror At-Risk Specs + Rewrite Rustdoc Links

## Prerequisites (completed)

GitHub Actions and GitHub Pages have been enabled on the `r3bl-open-core` repo:

1. **GitHub Actions** — Enabled at the **repo level** (`Settings → Actions → General`) with
   "Allow all actions and reusable workflows". The **org level** (`r3bl-org`) was already set
   to "Allow all actions and reusable workflows" for all repositories.

   Note: The more restrictive "Allow r3bl-org actions and reusable workflows" repo setting
   blocked the built-in `pages-build-deployment` action (a GitHub-owned action, not r3bl-org),
   which is why "Allow all" was needed at the repo level.

2. **GitHub Pages** — Configured via `Settings → Pages`:
   - **Source**: Deploy from a branch
   - **Branch**: `main`, `/docs` folder
   - **URL**: `https://r3bl-org.github.io/r3bl-open-core/`

   Pages will auto-deploy on every push to `main` that changes files in `/docs`. A `.nojekyll`
   file must be placed in `/docs` to prevent Jekyll from processing the HTML spec files.

---

## Context

The `tui/src/core/ansi/` module references ~79 external URLs for terminal spec documentation.
An audit found 1 broken URL and 3 at-risk domains (personal/small sites) hosting critical spec
documents. We will:

1. Fix the 1 broken URL
2. Mirror 7 HTML files from 3 at-risk domains into `docs/specs/`
3. Enable GitHub Pages to serve `docs/` so mirrored HTML renders with working anchor links
4. Rewrite all 46 rustdoc references to point to our GitHub Pages mirrors
5. Provide a refresh script (`docs/specs/refresh-mirrors.sh`)

---

## Part 1: Fix Broken URL

**File**: `tui/src/core/ansi/vt_100_terminal_input_parser/utf8.rs`, line 347

```
# Change std → core in the URL:
https://doc.rust-lang.org/std/str/fn.utf8_char_width.html
→ https://doc.rust-lang.org/core/str/fn.utf8_char_width.html
```

(The other 2 flagged URLs — `serial_test` and `GNU Readline` — were false positives.)

---

## Part 2: Mirror Specs + GitHub Pages Setup

### Directory structure

```
docs/
├── .nojekyll                          # Empty file — tells GitHub Pages to skip Jekyll
└── specs/
    ├── README.md
    ├── refresh-mirrors.sh
    ├── vt100.net/
    │   ├── vt100-ug/
    │   │   ├── index.html
    │   │   └── chapter3.html
    │   └── vt510-rm/
    │       ├── contents.html
    │       ├── DECSTBM.html
    │       └── chapter4.html
    ├── invisible-island.net/
    │   └── xterm-ctlseqs.html
    └── ditig.com/
        └── 256-colors-cheat-sheet.html
```

### GitHub Pages config

- **Source**: Deploy from a branch (already configured by user)
- **Branch**: `main`, `/docs` folder (already configured by user)
- **`.nojekyll`**: Empty file in `docs/` — prevents Jekyll from processing the HTML spec files
- **Result**: Files served at `https://r3bl-org.github.io/r3bl-open-core/`

### Download method

`curl -L` to preserve exact original HTML.

### docs/specs/README.md contents

- Purpose: why we mirror (link rot risk for personal/small sites hosting terminal specs)
- Source URL → local path mapping table with retrieval dates
- GitHub Pages URL examples showing working anchor links
- Licensing notes (DEC docs freely available 20+ years, XTerm/ditig freely available)
- Instructions to run `refresh-mirrors.sh`

### docs/specs/refresh-mirrors.sh

A bash script that re-downloads all 7 files:
- `curl -L` commands for all 7 URLs
- Verification (check file sizes, grep for expected anchors)
- Prints summary of what was refreshed

---

## Part 3: Rewrite Rustdoc Links to GitHub Pages

### GitHub Pages URL base

```
https://r3bl-org.github.io/r3bl-open-core/specs/
```

Anchors like `#S3.3.1` work because GitHub Pages serves raw HTML — the browser renders
it natively with full fragment navigation.

### URL mapping (10 unique URLs → 10 mirror URLs)

| Original | GitHub Pages URL |
|----------|-----------------|
| `https://vt100.net/docs/vt510-rm/contents.html` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt510-rm/contents.html` |
| `https://vt100.net/docs/vt510-rm/DECSTBM.html` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt510-rm/DECSTBM.html` |
| `https://vt100.net/docs/vt510-rm/chapter4.html#S4.3.4` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt510-rm/chapter4.html#S4.3.4` |
| `https://vt100.net/docs/vt100-ug/` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt100-ug/index.html` |
| `https://vt100.net/docs/vt100-ug/chapter3.html` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt100-ug/chapter3.html` |
| `https://vt100.net/docs/vt100-ug/chapter3.html#S3.3.1` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt100-ug/chapter3.html#S3.3.1` |
| `https://vt100.net/docs/vt100-ug/chapter3.html#S3.3.2` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt100-ug/chapter3.html#S3.3.2` |
| `https://vt100.net/docs/vt100-ug/chapter3.html#S3.3.4` | `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt100-ug/chapter3.html#S3.3.4` |
| `https://invisible-island.net/xterm/ctlseqs/ctlseqs.html` | `https://r3bl-org.github.io/r3bl-open-core/specs/invisible-island.net/xterm-ctlseqs.html` |
| `https://www.ditig.com/256-colors-cheat-sheet` | `https://r3bl-org.github.io/r3bl-open-core/specs/ditig.com/256-colors-cheat-sheet.html` |

### Files to edit (46 occurrences across 10 files)

All paths relative to `tui/src/core/ansi/`:

**vt100.net rewrites (28 occurrences):**
- `constants/generic.rs` — lines 49, 71, 81, 91, 101, 111, 121, 131 (8x `vt510-rm/contents.html`)
- `constants/csi.rs` — line 131 (1x `vt510-rm/DECSTBM.html`)
- `vt_100_pty_output_parser/performer.rs` — lines 193, 197, 413, 893, 910, 922
- `vt_100_pty_output_parser/protocols/params_ext.rs` — lines 218, 250, 373
- `vt_100_pty_output_parser/protocols/csi_codes/erase_mode.rs` — lines 27, 54, 103
- `vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/mod.rs` — line 340
- `vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/conformance_data/mod.rs` — line 215
- `vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/conformance_data/basic_sequences.rs` — lines 25, 27, 29, 30
- `vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/conformance_data/styling_sequences.rs` — line 15

**invisible-island.net rewrites (7 occurrences):**
- `constants/generic.rs` — lines 50, 144, 160, 175, 192, 208, 224

**ditig.com rewrites (11 occurrences):**
- `color/ansi_value.rs` — line 103
- `color/rgb_value.rs` — line 150
- `generator/cli_text.rs` — lines 433, 444, 455, 466, 477, 488, 499, 510, 521

### Edit strategy

Use `replace_all` where possible to batch-replace identical URLs within a single file:

- `generic.rs`: `replace_all` for `vt510-rm/contents.html` (8x) and `ctlseqs.html` (7x)
- `cli_text.rs`: `replace_all` for `ditig.com` (9x)
- `ansi_value.rs`, `rgb_value.rs`: individual edits for `ditig.com` (1x each)
- `performer.rs`: individual edits (mixed URL patterns)
- Other files: individual edits

---

## Part 4: Update `docs/README.md`

Add a "Specification Mirrors" section under "Technical Documentation and Design Docs":

```markdown
### Specification Mirrors

[`specs/`](specs/) — Locally mirrored copies of external terminal specification documents
(VT-100/VT-510 manuals, XTerm control sequences, ANSI color references) for preservation.
Served via GitHub Pages with working anchor links. See [`specs/README.md`](specs/README.md)
for details and refresh instructions.
```

---

## Execution order

1. Create `docs/.nojekyll` (empty file for GitHub Pages)
2. Create `docs/specs/` directory structure
3. Download 7 HTML files with `curl -L`
4. Verify downloads (file sizes, anchor spot-checks)
5. Write `docs/specs/README.md`
6. Write `docs/specs/refresh-mirrors.sh` (make executable)
7. Fix broken `utf8_char_width` URL (Part 1 — 1 edit)
8. Rewrite all 46 rustdoc references to GitHub Pages URLs (Part 3)
9. Update `docs/README.md` (Part 4)
10. Run `./check.fish --doc` to verify rustdoc builds cleanly

---

## Verification

1. `curl -I https://doc.rust-lang.org/core/str/fn.utf8_char_width.html` — confirm 200
2. Verify all 7 HTML files downloaded correctly (sizes, anchors)
3. `./check.fish --doc` — rustdoc builds without broken link warnings
4. `bash docs/specs/refresh-mirrors.sh` — script runs successfully
5. After push: visit `https://r3bl-org.github.io/r3bl-open-core/specs/vt100.net/vt100-ug/chapter3.html#S3.3.1` to confirm GitHub Pages serves HTML with working anchors
