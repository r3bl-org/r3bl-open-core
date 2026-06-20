# Task: PR 456 Integration (VT100 Pending Wrap fix)

## Overview

PR 456 (by Cecile Tonglet) implements correct VT100 pending-wrap (wrapneeded) state and
respects DECSTBM scroll region bounds in `apply_pending_wrap()`. This is needed so that
printing at the right margin correctly defers wrapping until the next printable character,
resolving spurious blank line issues in `fish`.

## PR Handling & Attribution

We are implementing our own fixes based on Cecile's original intent and closing her PR
without merging it directly due to underlying architectural changes. We must absolutely
add a `Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>` trailer to all of
the commits we make for this task to ensure she gets proper attribution for the feature!

## Implementation plan

### Phase 1: Pending Wrap Logic

- [ ] Add `pending_wrap` state to the parser or state struct.
- [ ] Implement `apply_pending_wrap()` helper to perform deferred wrap on the next
      printable character.
- [ ] Update `print_char()` to set `pending_wrap=true` at the margin instead of wrapping
      immediately.
- [ ] Ensure cursor movement ops (BS, TAB, LF, CR, CUP, etc.) clear the `pending_wrap`
      state.
- [ ] Ensure `apply_pending_wrap()` respects DECSTBM scroll region bounds.
- [ ] Run `./check.fish --check` and `./check.fish --test` to ensure correctness.
- [ ] Update `task/prepare-v0.8.0-meta-task.md` to check off this PR.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [ ] Files modified during implementation (TBD)
