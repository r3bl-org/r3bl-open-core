#!/usr/bin/env fish

# Make sure 1 argument are passed, and if not then display helpful usage instructions.
if test (count $argv) -lt 1
  echo "Usage: ./run-one-test.fish <test_name>"
  return 1
end

set -l prefix "cargo watch -x check -x 'test -- --test-threads=1 --nocapture"
set -l middle "$argv'"
set -l postfix "-c -q -d 5"
set -l cmd "$prefix $middle $postfix"
echo $cmd
sh -c $cmd
