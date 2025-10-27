# Regression Analysis with Flamegraphs

1. Run `./run.fish run-examples-flamegraph-fold --benchmark` to collect flamegraph data for
   benchmarks with an automated testing script that stress tests the rendering pipeline. The details
   for this are in `script-lib.fish`. The generated flamegraph file can be found at
   `tui/flamegraph-benchmark.perf-folded`

2. The `tui/flamegraph-benchmark-baseline.perf-folded` file contains the baseline performance data
   for comparison. This file is typically saved when we are ready to snapshot the "current best"
   performance state and committed to git.

3. Compare the two flamegraph files and prepare a regression report analyzing any performance
   changes, and present it to the user.
