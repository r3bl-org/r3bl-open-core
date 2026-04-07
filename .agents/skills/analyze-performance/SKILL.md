---
name: analyze-performance
description: Establish performance baselines and detect regressions using flamegraph analysis. Use when optimizing performance-critical code, investigating performance issues, or before creating commits with performance-sensitive changes.
---

# Performance Regression Analysis with Flamegraphs

## When to Use

- Optimizing performance-critical code
- Detecting performance regressions after changes
- Establishing performance baselines for reference
- Investigating performance issues or slow code paths
- Before creating commits with performance-sensitive changes
- When user says "check performance", "analyze flamegraph", "detect regressions", etc.

## Instructions

Follow these steps to analyze performance and detect regressions:

### Step 1: Generate Current Flamegraph

Run the automated benchmark script to collect current performance data:

```bash
./run.fish run-examples-flamegraph-fold --benchmark
```

**What this does:**
- Runs an 8-second continuous workload stress test
- Samples at 999Hz for high precision
- Tests the rendering pipeline with realistic load
- Generates flamegraph data in: `tui/flamegraph-benchmark.perf-folded`

**Implementation details:**
- The benchmark script is in `script-lib.fish`
- Uses an automated testing script that stress tests the rendering pipeline
- Simulates real-world usage patterns

### Step 2: Compare with Baseline

Compare the newly generated flamegraph with the baseline:

**Baseline file:**
```
tui/flamegraph-benchmark-baseline.perf-folded
```

**Current file:**
```
tui/flamegraph-benchmark.perf-folded
```

**The baseline file contains:**
- Performance snapshot of the "current best" performance state
- Typically saved when performance is optimal
- Committed to git for historical reference

### Step 3: Analyze Differences

Compare the two flamegraph files to identify regressions or improvements:

**Key metrics to analyze:**

1. **Hot path changes**
   - Which functions appear more/less frequently?
   - New hot paths that weren't in baseline?

2. **Sample count changes**
   - Increased samples = function taking more time
   - Decreased samples = optimization working!

3. **Call stack depth changes**
   - Deeper stacks might indicate unnecessary abstraction
   - Shallower stacks might indicate inlining working

4. **New allocations or I/O**
   - Look for memory allocation hot paths
   - Unexpected I/O operations

### Step 4: Prepare Regression Report

Create a comprehensive report analyzing the performance changes:

**Report structure:**

```markdown
# Performance Regression Analysis

## Summary
[Overall performance verdict: regression, improvement, or neutral]

## Hot Path Changes
- Function X: 1500 → 2200 samples (+47%) ⚠️ REGRESSION
- Function Y: 800 → 600 samples (-25%) ✅ IMPROVEMENT
- Function Z: NEW in current (300 samples) 🔍 INVESTIGATE

## Top 5 Most Expensive Functions

### Baseline
1. render_loop: 3500 samples
2. paint_buffer: 2100 samples
3. diff_algorithm: 1800 samples
...

### Current
1. render_loop: 3600 samples (+3%)
2. paint_buffer: 2500 samples (+19%) ⚠️
3. diff_algorithm: 1700 samples (-6%) ✅
...

## Regressions Detected
[List of functions with significant increases]

## Improvements Detected
[List of functions with significant decreases]

## Recommendations
[What should be investigated or optimized]
```

### Step 5: Present to User

Present the regression report to the user with:

- ✅ Clear summary (regression, improvement, or neutral)
- 📊 Key metrics with percentage changes
- ⚠️ Highlighted regressions that need attention
- 🎯 Specific recommendations for optimization
- 📈 Overall performance trend

## Optional: Update Baseline

**When to update the baseline:**

Only update when you've achieved a new "best" performance state:

1. After successful optimization work
2. All tests pass
3. Behavior is correct
4. Ready to lock in this performance as the new reference

**How to update:**

```bash
# Replace baseline with current
cp tui/flamegraph-benchmark.perf-folded tui/flamegraph-benchmark-baseline.perf-folded

# Commit the new baseline
git add tui/flamegraph-benchmark-baseline.perf-folded
git commit -m "perf: Update performance baseline after optimization"
```

**See `baseline-management.md` for detailed guidance on when and how to update baselines.**

## Understanding Flamegraph Format

The `.perf-folded` files contain stack traces with sample counts:

```
main;render_loop;paint_buffer;draw_cell 45
main;render_loop;diff_algorithm;compare 30
```

**Format:**
- Semicolon-separated call stack (deepest function last)
- Space + sample count at end
- More samples = more time spent in that stack

## Performance Optimization Workflow

```
1. Make code change
   ↓
2. Run: ./run.fish run-examples-flamegraph-fold --benchmark
   ↓
3. Analyze flamegraph vs baseline
   ↓
4. ┌─ Performance improved?
  │  ├─ YES → Update baseline, commit
  │  └─ NO  → Investigate regressions, optimize
  └→ Repeat
```

## Additional Performance Tools

For more granular performance analysis, consider:

### cargo bench

Run benchmarks for specific functions:

```bash
cargo bench
```

**When to use:**
- Micro-benchmarks for specific functions
- Tests marked with `#[bench]`
- Precise timing measurements

### cargo flamegraph

Generate visual flamegraph SVG:

```bash
cargo flamegraph
```

**When to use:**
- Visual analysis of call stacks
- Identifying hot paths visually
- Sharing performance analysis

**Requirements:**
- `flamegraph` crate installed
- Profiling symbols enabled

### Manual Profiling

For deep investigation:

```bash
# Profile with perf
perf record -F 999 --call-graph dwarf ./target/release/app

# Generate flamegraph
perf script | stackcollapse-perf.pl | flamegraph.pl > flame.svg
```

## Common Performance Issues to Look For

When analyzing flamegraphs, watch for:

### 1. Allocations in Hot Paths

```
render_loop;Vec::push;alloc::grow 500 samples  ⚠️
```

**Problem:** Allocating in tight loops
**Fix:** Pre-allocate or use capacity hints

### 2. Excessive Cloning

```
process_data;String::clone 300 samples  ⚠️
```

**Problem:** Unnecessary data copies
**Fix:** Use references or `Cow<str>`

### 3. Deep Call Stacks

```
a;b;c;d;e;f;g;h;i;j;k;l;m 50 samples  ⚠️
```

**Problem:** Too much abstraction or recursion
**Fix:** Flatten, inline, or optimize

### 4. I/O in Critical Paths

```
render_loop;write;syscall 200 samples  ⚠️
```

**Problem:** Blocking I/O in rendering
**Fix:** Buffer or defer I/O

## Reporting Results

After performance analysis:

- ✅ No regressions → "Performance analysis complete: no regressions detected!"
- ⚠️ Regressions found → Provide detailed report with function names and percentages
- 🎯 Improvements found → Celebrate and document what worked!
- 📊 Mixed results → Explain trade-offs and recommendations

## Supporting Files in This Skill

This skill includes additional reference material:

- **`baseline-management.md`** - Comprehensive guide on when and how to update performance baselines: when to update (after optimization, architectural changes, dependency updates, accepting trade-offs), when NOT to update (regressions, still debugging, experimental code, flaky results), step-by-step update process, baseline update checklist, reading flamegraph differences, example workflows, and common mistakes. **Read this when:**
  - Deciding whether to update the baseline → "When to Update" section
  - Performance improved and want to lock it in → Update workflow
  - Unsure if baseline update is appropriate → Checklist
  - Need to understand flamegraph diff signals → "Reading Flamegraph Differences"
  - Avoiding common mistakes → "Common Mistakes" section

## Related Skills

- `check-code-quality` - Run before performance analysis to ensure correctness
- `write-documentation` - Document performance characteristics

## Related Commands

- `/check-regression` - Explicitly invokes this skill

## Related Agents

- `perf-checker` - Agent that delegates to this skill

## Additional Resources

- Flamegraph format: `tui/*.perf-folded` files
- Benchmark script: `script-lib.fish`
- Visual flamegraphs: Use `flamegraph.pl` to generate SVGs
