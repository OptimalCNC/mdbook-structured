#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd -P)"
fake_path="$repo_root/scripts/test-fixtures/package-cli"

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
