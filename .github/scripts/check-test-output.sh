#!/usr/bin/env bash
# A successful Cargo command must have executed passing ordinary tests.
set -euo pipefail
[ "$#" -eq 1 ] || { echo 'usage: check-test-output.sh TEST_LOG' >&2; exit 2; }
awk '
  /^test result: (ok|FAILED)\. [0-9]+ passed; [0-9]+ failed;/ {
    summaries++; passed += $4
    if ($3 != "ok." || $6 != 0) failed = 1
  }
  END {
    if (!summaries || failed || !passed) {
      print "ordinary tests failed or executed no passing tests" > "/dev/stderr"
      exit 1
    }
  }
' "$1"
