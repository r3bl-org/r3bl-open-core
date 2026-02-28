# Better check.fish Integration

## Goal

Enhance `check.fish` to be the single source of truth for code quality checks, with:
1. New commands (`--check`, `--build`, `--clippy`, `--full`)
2. Simplified ICE escalation that calls `rust-toolchain-update.fish`
3. Integration with `check-code-quality` skill

## Status

- [x] Plan created
- [x] Phase 1: Add new one-off commands to check.fish
- [x] Phase 2: Implement simplified ICE escalation
- [x] Phase 3: Update check-code-quality skill
- [x] Phase 4: Test and verify
- [x] Phase 5: Update documentation

**✅ ALL PHASES COMPLETE**

---

## Test Results

### Phase 4: Testing

| Test | Result |
|:-----|:-------|
| `./check.fish --check` | ✅ Passed (1s) |
| `./check.fish --build` | ✅ Passed (2s) |
| `./check.fish --clippy` | ✅ Passed (10s) |
| `./check.fish --full` | ✅ Passed (all 6 checks) |
| `./check.fish --watch-doc` | ✅ Working (detected file change, rebuilt) |
| `fish --no-execute check.fish` | ✅ Syntax OK |
| `./check.fish --help` | ✅ Shows all new commands |

### Phase 5: Documentation Updated

| Doc | Changes |
|:----|:--------|
| `check.fish` header comments | Updated Workflow section (6 steps), Usage section (all commands) |
| `README.md` | Added ICE escalation feature, all new commands in Usage section, fixed typo |
| `SKILL.md` | Added Quick Approach, command table, ICE escalation section |
| `task/better-check-integration.md` | Complete task tracking |

---

## Summary of All Changes

### check.fish

**New check functions (Level 1):**
```fish
check_cargo_check    # cargo check
check_cargo_build    # cargo build
check_clippy         # cargo clippy --all-targets
```

**New orchestrator functions (Level 3b, 4b):**
```fish
run_all_full_checks_with_recovery     # Runs all 6 checks
run_full_checks_with_ice_recovery     # With ICE escalation to toolchain update
```

**New CLI commands:**
- `--check` - Fast typecheck
- `--build` - Compile production
- `--clippy` - Lint warnings
- `--full` - Comprehensive check with ICE escalation

**Updated documentation:**
- Header comments: Workflow section, Usage section
- `show_help` function: All new commands documented

### check-code-quality Skill

- Added "Quick Approach (Recommended)" section with `./check.fish --full`
- Added command table for all check.fish options
- Added "ICE Recovery and Toolchain Escalation" section with flow diagram
- Kept step-by-step approach as "Alternative" for granular control

### README.md

- Added ICE escalation feature to "What it does" list
- Added all new commands to Usage section
- Fixed typo: `--watch-tests` → `--watch-test`

---

## Implementation Notes

### ICE Escalation Flow

```
ICE detected → cleanup target/ → retry
                                   ↓
                            still ICE?
                                   ↓
              escalate to rust-toolchain-update.fish
              (searches 46 nightly candidates, validates each)
                                   ↓
                         new stable nightly installed
                                   ↓
                               retry checks
```

### Files Modified

| File | Type of Change |
|:-----|:---------------|
| `check.fish` | Added functions, modes, help, header docs |
| `.claude/skills/check-code-quality/SKILL.md` | Major rewrite |
| `README.md` | Added features, commands, fixed typo |
| `task/better-check-integration.md` | Created for tracking |

---

## Log

### Session Start
- Created plan file

### Phase 1 Complete
- Added 3 new check functions
- Added 4 new parse_arguments cases
- Added 4 new mode handlers in main
- Updated show_help with new commands

### Phase 2 Complete
- Added `run_all_full_checks_with_recovery` function
- Added `run_full_checks_with_ice_recovery` with escalation to toolchain update
- Updated help to mention ICE escalation feature

### Phase 3 Complete
- Updated SKILL.md with Quick Approach section
- Added command table and ICE escalation documentation
- Kept step-by-step approach as alternative

### Phase 4 Complete
- All one-off commands tested: `--check`, `--build`, `--clippy`, `--full`
- `--watch-doc` tested: detected file changes, rebuilt docs correctly
- Syntax verification passed

### Phase 5 Complete
- Updated check.fish header comments (Workflow, Usage sections)
- Updated README.md (features, commands, typo fix)

### ✅ DONE
All implementation and testing complete.
