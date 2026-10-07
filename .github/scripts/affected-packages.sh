#!/usr/bin/env bash
# Select independent package roots using a PR merge base or direct push range.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
event=push
base=
head=HEAD
full=false
while [ "$#" -gt 0 ]; do
  case "$1" in
    --all) full=true; shift ;;
    --event|--base|--head)
      [ "$#" -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
      case "$1" in --event) event=$2 ;; --base) base=$2 ;; --head) head=$2 ;; esac
      shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$event" in push|pull_request|workflow_dispatch) ;; *) echo "invalid event: $event" >&2; exit 2 ;; esac
if [ "$event" = workflow_dispatch ] || [ "$base" = 0000000000000000000000000000000000000000 ]; then full=true; fi
if [ "$full" = false ] && [ -z "$base" ]; then echo '--base required unless --all or manual selection' >&2; exit 2; fi
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
: > "$scratch/packages"
for manifest in "$root"/*/Cargo.toml; do
  [ -f "$manifest" ] || continue
  basename "$(dirname "$manifest")" >> "$scratch/packages"
done
[ -s "$scratch/packages" ] || { echo 'no independent participant packages discovered' >&2; exit 2; }
LC_ALL=C sort -o "$scratch/packages" "$scratch/packages"
: > "$scratch/selected"
is_documentation() {
  case "/$1/" in */tests/*|*/fixtures/*|*/src/*|*/assets/*) return 1 ;; esac
  case "$1" in
    *.[mM][dD]|*.[rR][sS][tT]|*.[tT][xX][tT]|README.md|LICENSE|*/LICENSE|CHANGELOG.md|docs/*|*/docs/*) return 0 ;;
    *) return 1 ;;
  esac
}
if [ "$full" = true ]; then
  cp "$scratch/packages" "$scratch/selected"
else
  if [ "$event" = pull_request ]; then base=$(git -C "$root" merge-base "$base" "$head"); fi
  # Both deleted and added paths participate, including cross-package renames.
  git -C "$root" diff --no-renames --name-only -z "$base" "$head" > "$scratch/changes"
  while IFS= read -r -d '' path; do
    is_documentation "$path" && continue
    owner=${path%%/*}
    if grep -Fxq -- "$owner" "$scratch/packages"; then
      printf '%s\n' "$owner" >> "$scratch/selected"
    else
      # Shared automation/toolchain inputs and removed roots affect all survivors.
      cp "$scratch/packages" "$scratch/selected"
      break
    fi
  done < "$scratch/changes"
fi
LC_ALL=C sort -u "$scratch/selected" | jq -Rsc '
  split("\n") | map(select(length > 0)) as $packages |
  {packages: $packages, matrix: {include:
    ([$packages[] | {package: ., os: "ubuntu-latest"}] +
     [ $packages[] | select(. == "gamepad") | {package: ., os: "macos-latest"}])}}
'
