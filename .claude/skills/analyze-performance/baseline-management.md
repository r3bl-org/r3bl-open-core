# Performance Baseline Management

This document provides detailed guidance on when and how to update performance baselines.

---

## What is a Performance Baseline?

The baseline flamegraph (`tui/flamegraph-benchmark-baseline.perf-folded`) represents:

- A snapshot of the **"current best"** performance state
- A reference point for detecting regressions
- A historical record committed to git
- The performance characteristics you want to maintain or improve upon

**Think of it as:** The performance you achieved that you don't want to accidentally regress from.

---

## When to Update the Baseline

### ✅ Update the Baseline When:

#### 1. After Successful Optimization

You've made code changes that improve performance:

```bash
# Before optimization
render_loop: 3500 samples

# After optimization
render_loop: 2800 samples  (-20%!)
```

**Criteria:**
- Performance measurably improved
- All tests pass (`cargo test --all-targets`)
- Behavior is correct
- Change is committed to main branch

**Update baseline to lock in this improvement!**

#### 2. After Architectural Changes

You've refactored code in a way that changes performance characteristics:

```bash
# Old architecture
old_algorithm: 2000 samples

# New architecture
new_algorithm: 1500 samples  (different approach, better perf)
```

**Criteria:**
- New architecture is the "going forward" design
- Performance is acceptable or improved
- Tests validate correctness
- Team has approved the change

**Update baseline to reflect new architecture.**

#### 3. After Dependency Updates

External dependencies changed performance:

```bash
# Old dep version
external_lib::process: 500 samples

# New dep version
external_lib::process: 400 samples  (dep got faster!)
```

**Criteria:**
- Dependency update is permanent
- Performance change is verified
- No regressions in other areas

**Update baseline to match new dependency performance.**

#### 4. After Accepting Necessary Trade-offs

You've made a conscious trade-off for other benefits:

```bash
# Before adding feature X
total_time: 3000 samples

# After adding feature X (adds functionality, slight perf cost)
total_time: 3200 samples  (+7%, but feature is valuable)
```

**Criteria:**
- Team agrees feature is worth the performance cost
- Performance cost is acceptable (< 10% degradation)
- Feature is essential

**Update baseline to reflect new reality.**

---

### ❌ DO NOT Update the Baseline When:

#### 1. Performance Regressed Without Good Reason

```bash
# Accidentally introduced regression
render_loop: 3500 → 4200 samples  (+20% slower!)
```

**Don't update!** This hides the regression. Investigate and fix instead.

#### 2. Still Debugging/Optimizing

You're in the middle of performance work:

```bash
# Work in progress
Iteration 1: 3500 samples
Iteration 2: 3200 samples  (getting better)
Iteration 3: ??? (still optimizing)
```

**Don't update yet!** Wait until optimization work is complete.

#### 3. Temporary Experimental Code

Trying out a new approach:

```bash
# Experiment: rewrite in different style
experimental_approach: 2500 samples  (looks promising!)
```

**Don't update!** Verify the approach is solid first.

#### 4. Flaky or Inconsistent Results

Flamegraph results vary between runs:

```bash
Run 1: 3500 samples
Run 2: 3900 samples
Run 3: 3400 samples
```

**Don't update!** Figure out why results are inconsistent first.

---

## How to Update the Baseline

### Step 1: Verify Current Performance

Ensure the new performance is real and repeatable:

```bash
# Run multiple times
./run.fish run-examples-flamegraph-fold --benchmark
./run.fish run-examples-flamegraph-fold --benchmark
./run.fish run-examples-flamegraph-fold --benchmark

# Compare results - should be consistent
```

### Step 2: Compare with Current Baseline

```bash
# Analyze the difference
diff tui/flamegraph-benchmark-baseline.perf-folded tui/flamegraph-benchmark.perf-folded
```

**Ask yourself:**
- Is this an improvement or acceptable trade-off?
- Are regressions explained and necessary?
- Is this the performance we want going forward?

### Step 3: Replace Baseline

```bash
# Copy current to baseline
cp tui/flamegraph-benchmark.perf-folded tui/flamegraph-benchmark-baseline.perf-folded
```

### Step 4: Commit with Clear Message

```bash
git add tui/flamegraph-benchmark-baseline.perf-folded
git commit -m "perf: Update baseline after [reason]

- Function X: -20% improvement (optimized algorithm)
- Function Y: +5% trade-off (added caching)
- Overall: 15% faster for common workload

Baseline updated to lock in these improvements."
```

**Commit message should explain:**
- Why the baseline changed
- What specific changes affected performance
- Overall performance impact

---

## Baseline Update Checklist

Before updating the baseline, verify:

- [ ] New flamegraph generated multiple times (consistent results)
- [ ] All tests pass: `cargo test --all-targets`
- [ ] Performance change is intentional and understood
- [ ] Either improvement OR acceptable trade-off
- [ ] Code is committed to main branch (or will be)
- [ ] Team agrees with the change (if significant)
- [ ] Commit message documents the rationale

---

## Reading Flamegraph Differences

When comparing baseline to current:

### Improvement Signals

```diff
- function_a;heavy_work 2000
+ function_a;heavy_work 1500
```

✅ Function taking less time (fewer samples)

```diff
- function_b;allocate;grow 500
  (removed entirely)
```

✅ Eliminated expensive operation

### Regression Signals

```diff
- function_c;process 1000
+ function_c;process 1500
```

⚠️ Function taking more time (more samples)

```diff
  (not in baseline)
+ function_d;new_expensive_operation 800
```

⚠️ New expensive operation added

### Neutral Changes

```diff
- function_e;old_algorithm 2000
+ function_f;new_algorithm 2000
```

↔️ Different implementation, same performance

---

## Example Workflow

### Scenario: Optimizing Render Loop

**Initial State:**
```bash
$ ./run.fish run-examples-flamegraph-fold --benchmark
# Results: render_loop 3500 samples
```

**Baseline exists at:**
```
tui/flamegraph-benchmark-baseline.perf-folded
render_loop: 3500 samples
```

**Make optimization:**
```rust
// Replace Vec with SmallVec to avoid allocations
```

**Test new performance:**
```bash
$ ./run.fish run-examples-flamegraph-fold --benchmark
# Results: render_loop 2800 samples  (-20%!)

$ ./run.fish run-examples-flamegraph-fold --benchmark
# Results: render_loop 2850 samples  (consistent!)
```

**Verify correctness:**
```bash
$ cargo test --all-targets
# All tests pass ✅
```

**Update baseline:**
```bash
$ cp tui/flamegraph-benchmark.perf-folded tui/flamegraph-benchmark-baseline.perf-folded
$ git add tui/flamegraph-benchmark-baseline.perf-folded
$ git commit -m "perf: Update baseline after render loop optimization

Replaced Vec with SmallVec to avoid allocations in hot path.

- render_loop: 3500 → 2800 samples (-20%)
- Eliminated 300+ allocation samples

Baseline updated to lock in this improvement."
```

**Done!** Future changes will be compared against the new, faster baseline.

---

## Common Mistakes

### ❌ Mistake 1: Updating Too Frequently

```bash
# After every tiny change
git commit "perf: baseline update"
git commit "perf: baseline update again"
git commit "perf: baseline update v3"
```

**Problem:** Baseline becomes meaningless if updated constantly.

**Fix:** Update only after significant, verified improvements or architectural changes.

### ❌ Mistake 2: Hiding Regressions

```bash
# Introduced regression
render_loop: 3500 → 4200 samples  (+20% slower!)

# Update baseline to hide it
cp current baseline
git commit "perf: update baseline"  # NO!!!
```

**Problem:** This hides the regression instead of fixing it.

**Fix:** Investigate the regression, optimize, THEN update if performance is acceptable.

### ❌ Mistake 3: Updating Without Verification

```bash
# One run, seems faster
$ ./run.fish run-examples-flamegraph-fold --benchmark
# Update baseline immediately  # NO!!!
```

**Problem:** Results might be fluky or inconsistent.

**Fix:** Run multiple times, verify consistency, check tests.

---

## Summary

**Baseline Philosophy:**

> The baseline represents the best performance you've achieved that you want to maintain.
> Update it to lock in improvements, not to hide regressions.

**Golden Rule:**

> If you're hesitant about updating the baseline, don't update it yet.
> Wait until you're confident the performance change is correct and permanent.

**When in doubt:**
1. Run more tests
2. Verify correctness
3. Discuss with team
4. Document rationale clearly

This ensures the baseline remains a meaningful reference point for performance regression detection!
