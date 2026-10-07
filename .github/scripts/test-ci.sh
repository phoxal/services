#!/usr/bin/env bash
# Run real Git-range and result-guard regressions with Bash 3.2 or newer.
set -euo pipefail
scripts=$(cd "$(dirname "$0")" && pwd)
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
root=$scratch/repo
mkdir -p "$root/.github/scripts" "$root/a/src" "$root/b/src" "$root/gamepad"
cp "$scripts/affected-packages.sh" "$root/.github/scripts/"
git -C "$root" init -q --template=
git -C "$root" config user.name 'CI fixture'
git -C "$root" config user.email fixture@example.invalid
git -C "$root" config commit.gpgsign false
for package in a b gamepad; do printf '[package]\nname="%s"\n' "$package" > "$root/$package/Cargo.toml"; done
printf 'unchanged\n' > "$root/a/src/code.rs"
commit() { git -C "$root" add -A; git -C "$root" commit -qm "$1"; }
head() { git -C "$root" rev-parse HEAD; }
select_packages() { "$BASH" "$root/.github/scripts/affected-packages.sh" "$@"; }
expect_packages() {
  expected=$1; shift
  actual=$(select_packages "$@" | jq -c .packages)
  [ "$actual" = "$expected" ] || { echo "expected $expected, got $actual" >&2; exit 1; }
}
reject() { if "$@" > "$scratch/rejected.log" 2>&1; then echo "unexpected success: $*" >&2; exit 1; fi; }
commit base
base=$(head)
expect_packages '["a","b","gamepad"]' --all
expect_packages '["a","b","gamepad"]' --event workflow_dispatch
expect_packages '["a","b","gamepad"]' --base 0000000000000000000000000000000000000000
select_packages --all | jq -e '[.matrix.include[] | select(.package == "gamepad") | .os] == ["ubuntu-latest","macos-latest"]' > /dev/null
reject select_packages
reject select_packages --base missing-ref
reject select_packages --base "$base" --head missing-ref
reject select_packages --event invalid --all
reject select_packages --base
printf 'license\n' > "$root/a/LICENSE"
commit package-license
expect_packages '[]' --base "$base"
base=$(head)
printf 'docs\n' > "$root/README.md"
mkdir "$root/a/docs"
printf 'docs\n' > "$root/a/docs/guide.md"
commit documentation
expect_packages '[]' --base "$base"
printf 'changed\n' > "$root/a/src/code.rs"
commit upstream
upstream=$(head)
git -C "$root" checkout -qb pr "$base"
printf 'PR docs\n' > "$root/README.md"
commit pr-documentation
pr=$(head)
expect_packages '[]' --event pull_request --base "$upstream" --head "$pr"
expect_packages '["a"]' --event push --base "$upstream" --head "$pr"
git -C "$root" checkout -q --detach "$base"
mv "$root/a/src/code.rs" "$root/b/src/code.rs"
commit cross-package-rename
expect_packages '["a","b"]' --base "$base"
last=$(head)
mkdir "$root/new"
printf ' [package] # participant\nname="new"\n' > "$root/new/Cargo.toml"
commit new-package
expect_packages '["new"]' --base "$last"
last=$(head)
rm "$root/b/Cargo.toml"
commit deleted-package
expect_packages '["a","gamepad","new"]' --base "$last"
last=$(head)
mkdir -p "$root/a/tests/fixtures" "$root/new/assets"
printf 'fixture license\n' > "$root/a/tests/fixtures/LICENSE"
printf 'reply\n' > "$root/a/tests/fixtures/reply.txt"
printf 'expected\n' > "$root/a/tests/fixtures/expected.md"
printf 'model\n' > "$root/new/model.xml"
printf 'asset\n' > "$root/new/assets/mesh.obj"
commit fixtures-model-assets
expect_packages '["a","new"]' --base "$last"
for shared in .github/workflows/ci.yml .github/scripts/helper.sh rust-toolchain.toml .cargo/config.toml; do
  last=$(head)
  mkdir -p "$root/$(dirname "$shared")"
  printf 'shared input\n' > "$root/$shared"
  commit shared-input
  expect_packages '["a","gamepad","new"]' --base "$last"
done
empty=$scratch/empty
mkdir -p "$empty/.github/scripts"
cp "$scripts/affected-packages.sh" "$empty/.github/scripts/"
reject "$BASH" "$empty/.github/scripts/affected-packages.sh" --all
"$BASH" "$scripts/check-selection.sh" success '[]' skipped
"$BASH" "$scripts/check-selection.sh" success '["a"]' success
for result in skipped failure cancelled; do reject "$BASH" "$scripts/check-selection.sh" success '["a"]' "$result"; done
for result in success failure cancelled; do reject "$BASH" "$scripts/check-selection.sh" success '[]' "$result"; done
for result in failure cancelled skipped; do reject "$BASH" "$scripts/check-selection.sh" "$result" '[]' skipped; done
reject "$BASH" "$scripts/check-selection.sh" success invalid skipped
reject "$BASH" "$scripts/check-selection.sh" success '{}' skipped
printf 'test result: ok. 1 passed; 0 failed; 0 ignored;\n' > "$scratch/tests.log"
"$BASH" "$scripts/check-test-output.sh" "$scratch/tests.log"
for output in \
  'test result: ok. 0 passed; 0 failed; 0 ignored;' \
  'test result: ok. 0 passed; 0 failed; 1 ignored;' \
  'test result: FAILED. 1 passed; 1 failed; 0 ignored;' \
  'compiler failed'; do
  printf '%s\n' "$output" > "$scratch/tests.log"
  reject "$BASH" "$scripts/check-test-output.sh" "$scratch/tests.log"
done
printf 'test result: ok. 1 passed; 0 failed;\ntest result: FAILED. 0 passed; 1 failed;\n' > "$scratch/tests.log"
reject "$BASH" "$scripts/check-test-output.sh" "$scratch/tests.log"
reject "$BASH" "$scripts/check-test-output.sh" "$scratch/missing.log"
printf 'PASS: Git selection, platform matrix, nonzero tests and truthful aggregate (%s)\n' "$BASH_VERSION"
