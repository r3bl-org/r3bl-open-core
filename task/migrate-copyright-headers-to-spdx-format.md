# Task: Migrate Copyright Headers to SPDX Format and Update VS Code Extension

## Overview

Migrate all source file copyright headers across the repository from the date-based
one-line format (`// Copyright (c) <range> R3BL LLC...`) to the modern, standardized,
machine-readable **SPDX** (Software Package Data Exchange) format:

```rust
// SPDX-FileCopyrightText: R3BL LLC <https://r3bl.com>
// SPDX-License-Identifier: Apache-2.0
```

Because SPDX headers omit calendar years by design (relying on git history and automatic
protection under the Berne Convention), this eliminates the perpetual need for annual
repository-wide copyright date-range sweeps.

This task also includes updating the `R3BL.r3bl-auto-insert-copyright` extension in
`~/github/r3bl-vscode-extensions` so that future files automatically generate the new SPDX
format and correctly identify existing SPDX headers.

---

## Scope & Components

1. **VS Code Extension (`~/github/r3bl-vscode-extensions`)**:
    - `packages/r3bl-auto-insert-copyright/`
        - Add `SPDXApache2` license template.
        - Update `configuration.ts` and `package.json` schemas.
        - Update detection logic in `copyrightService.ts` (`hasCopyright`) to recognize
          `SPDX-FileCopyrightText` and `SPDX-License-Identifier`.
        - Build, package, and release new version of `R3BL.r3bl-auto-insert-copyright` and
          `R3BL.r3bl-extension-pack`.

2. **Workspace Configuration (`r3bl-open-core`)**:
    - Update `.vscode/settings.json` to use the new
      `"copyrighter.license": "SPDX-Apache2"`.
    - Remove obsolete `nazmulidris.copyrighter` instructions from
      `.vscode/extensions.json`.

3. **Repository Source Migration (`r3bl-open-core`)**:
    - Perform a clean, surgical replacement across all `.rs` files in workspace crates
      (`tui`, `build-infra`, `rust-analyzer-mcp-server`, etc.).
    - Ensure custom folding (`baincd.custom-auto-fold` or similar) continues to handle the
      2-line header smoothly.

---

## Detailed Implementation Plan

### Phase 1: Update `r3bl-auto-insert-copyright` Extension

**Repository**: `~/github/r3bl-vscode-extensions`

1. **Add SPDX License Template**: Create
   `packages/r3bl-auto-insert-copyright/src/copyright/licenses/spdx-apache2.ts`:

    ```typescript
    // Copyright (c) 2024-2026 R3BL LLC. Licensed under MIT License.

    "use strict"

    import { Copyright } from "../copyright"

    export class SPDXApache2 extends Copyright {
        constructor() {
            super()
        }

        public header(): string {
            return `// SPDX-FileCopyrightText: ${this.author} <https://r3bl.com>
    // SPDX-License-Identifier: Apache-2.0
    
    `
        }
    }
    ```

2. **Update Configuration and Registration**: In
   `packages/r3bl-auto-insert-copyright/src/configuration.ts`:
    - Import `SPDXApache2`.
    - Add branch for `"SPDX-Apache2"` (or make it the default).
    - In `packages/r3bl-auto-insert-copyright/package.json`:
        - Add `"SPDX-Apache2"` to `copyrighter.license` enum and enum descriptions.

3. **Update Header Detection (`hasCopyright`)**: In
   `packages/r3bl-auto-insert-copyright/src/copyright/copyrightService.ts`:
    - Update `hasCopyright()` to recognize SPDX tags in the first 2 lines:
        ```typescript
        if (
            !firstLine.isEmptyOrWhitespace &&
            firstLine.text.trim().startsWith("//") &&
            (firstLine.text.includes("Copyright") ||
                firstLine.text.includes("SPDX-FileCopyrightText") ||
                firstLine.text.includes("SPDX-License-Identifier"))
        ) {
            return true
        }
        ```

4. **Compile and Package**:
    - Run `npm run build` in `packages/r3bl-auto-insert-copyright`.
    - Increment version in `package.json`.
    - Package via `npm run package`.
    - Update `R3BL.r3bl-extension-pack` version and build.

---

### Phase 2: Update Workspace Configuration in `r3bl-open-core`

1. **Update `.vscode/settings.json`**:

    ```json
    "copyrighter.author": "R3BL LLC",
    "copyrighter.license": "SPDX-Apache2",
    ```

2. **Clean up `.vscode/extensions.json`**:
    - Remove legacy lines 5-13 referencing `nazmulidris.copyrighter` and
      `deploy_locally.fish`.

---

### Phase 3: Bulk Migration in `r3bl-open-core`

1. **Audit Existing Headers**: Locate all variations of legacy copyright headers across
   the workspace:
    - `// Copyright (c) 2023-2026 R3BL LLC. Licensed under Apache License, Version 2.0.`
    - `// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.`
    - `// Copyright (c) 2025-2026 R3BL LLC. Licensed under Apache License, Version 2.0.`
    - `// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.`
    - Any other variations across `tui/`, `build-infra/`, and `rust-analyzer-mcp-server/`.

2. **Perform File-by-File Replacements**: Replace each header with:

    ```rust
    // SPDX-FileCopyrightText: R3BL LLC <https://r3bl.com>
    // SPDX-License-Identifier: Apache-2.0
    ```

3. **Audit Line Spacing**: Ensure a clean single blank line follows the 2-line SPDX block
   before module docs or imports.

---

### Phase 4: Quality Checks & Verification

1. Run `./check.fish --check` to ensure no syntax issues.
2. Run `./check.fish --fmt` (`cargo fmt` + `cargo rustdoc-fmt`).
3. Run `./check.fish --clippy`.
4. Run `./check.fish --quick-doc` to ensure rustdoc links and rendering remain intact.
5. Run `./check.fish --test`.
6. Run `git diff` audit to ensure all changes are strictly surgical.

---

### Phase 5: Mandatory Manual Review

- [ ] `.vscode/settings.json`
- [ ] `.vscode/extensions.json`
- [ ] `~/github/r3bl-vscode-extensions/packages/r3bl-auto-insert-copyright/`
- [ ] `tui/` crates and modules
- [ ] `build-infra/` crates and modules
- [ ] `rust-analyzer-mcp-server/` crates and modules
