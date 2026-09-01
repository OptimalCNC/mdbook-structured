#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd -P)"
fake_path="$repo_root/scripts/test-fixtures/package-cli"

assert_mdbook_precedes_tests() {
  local workflow=$1 install_line install_command_line test_line

  install_line="$(grep -nF '      - name: Install mdBook' "$workflow" | cut -d: -f1 || true)"
  install_command_line="$(
    grep -nF '        run: cargo install mdbook --version 0.5.3 --locked' "$workflow" \
      | cut -d: -f1 || true
  )"
  test_line="$(
    grep -nF '      - name: Run locked workspace tests' "$workflow" \
      | cut -d: -f1 || true
  )"
  if [[ ! $install_line =~ ^[0-9]+$ \
    || ! $install_command_line =~ ^[0-9]+$ \
    || ! $test_line =~ ^[0-9]+$ ]] \
    || (( install_line >= install_command_line || install_command_line >= test_line )); then
    printf '%s must install pinned mdBook before locked workspace tests\n' \
      "$workflow" >&2
    return 1
  fi
}

assert_mdbook_precedes_tests "$repo_root/.github/workflows/bootstrap-release.yml"
assert_mdbook_precedes_tests "$repo_root/.github/workflows/release.yml"

if ! output="$(
  PATH="$fake_path:$PATH" \
    FAKE_CARGO_MODE=version-mismatch \
    GITHUB_WORKSPACE="$repo_root" \
    "$repo_root/scripts/release-check.sh" package-cli 2>&1
)"; then
  printf 'package-cli rejected the expected unpublished-version diagnostic:\n%s\n' \
    "$output" >&2
  exit 1
fi

if output="$(
  PATH="$fake_path:$PATH" \
    FAKE_CARGO_MODE=unexpected \
    GITHUB_WORKSPACE="$repo_root" \
    "$repo_root/scripts/release-check.sh" package-cli 2>&1
)"; then
  printf 'package-cli accepted an unexpected Cargo failure\n' >&2
  exit 1
fi

if [[ $output != *'cargo package failed for an unexpected reason'* ]]; then
  printf 'package-cli did not retain its fail-closed diagnostic:\n%s\n' \
    "$output" >&2
  exit 1
fi

printf 'release-check package-cli diagnostics passed\n'
