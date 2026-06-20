# Task: PR 455 Integration (DA1 Responses timeout fix)

## Overview

PR 455 (by Cecile Tonglet) handles DA1 (Primary Device Attributes, `CSI c`) queries in the
PTY mux shim to prevent fish shell and other terminal applications from timing out while
waiting for a terminal capability response.

It responds with `CSI ? 62 ; 22 c` on DA1 (VT220 + ANSI color) and correctly forwards DA
responses alongside existing OSC and DSR channels.

## Implementation plan

### Phase 1: DA1 Support

- [ ] Extend `apply_ansi_bytes()` return type from 2-tuple to 3-tuple (or equivalent) to
      propagate DA responses.
- [ ] Add logic to respond to `CSI c` (DA1) with `CSI ? 62 ; 22 c`.
- [ ] Ignore DA2 (`CSI > c`), DA3 (`CSI = c`), and parameterized variants.
- [ ] Forward DA responses back to the PTY child via `PtyInputEvent::Write` (or
      equivalent).
- [ ] Add/update conformance tests for DA1/DA2 behavior.
- [ ] Run `./check.fish --check` and `./check.fish --test` to ensure stability.
- [ ] Update `task/prepare-v0.8.0-meta-task.md` to check off this PR.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [ ] Files modified during implementation (TBD)
