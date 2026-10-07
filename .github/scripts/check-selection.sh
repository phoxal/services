#!/usr/bin/env bash
# Validate the aggregate even when documentation intentionally skips Rust jobs.
set -euo pipefail
[ "$#" -eq 3 ] || { echo 'usage: check-selection.sh SELECTION PACKAGES_JSON PACKAGE_RESULT' >&2; exit 2; }
[ "$1" = success ] || { echo 'package selection did not succeed' >&2; exit 1; }
count=$(printf '%s\n' "$2" | jq -er 'if type == "array" and all(.[]; type == "string") then length else error("invalid package selection") end')
expected=success
[ "$count" -ne 0 ] || expected=skipped
[ "$3" = "$expected" ] || { echo "expected package result $expected, observed $3" >&2; exit 1; }
