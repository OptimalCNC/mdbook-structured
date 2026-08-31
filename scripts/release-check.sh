#!/usr/bin/env bash
set -euo pipefail

readonly RELEASE_USER_AGENT='mdbook-structured-release/1.0'
readonly INDEX_ATTEMPTS=12
readonly INDEX_RETRY_SECONDS=5
readonly RELEASE_VERSION_PATTERN='^[0-9]+\.[0-9]+\.[0-9]+$'
readonly SEMVER_PATTERN='^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'
export CARGO_HTTP_USER_AGENT="$RELEASE_USER_AGENT"

usage() {
  printf 'usage: %s {guard-bootstrap|validate|package-cli|wait-index|verify-registry} [args...]\n' \
    "$0" >&2
}

validate_crate_name() {
  local crate=$1

  if [[ ! $crate =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]] || (( ${#crate} > 64 )); then
    printf 'invalid crate name; expected an ASCII crates.io identifier\n' >&2
    return 1
  fi
}

validate_semver() {
  local version=$1

  if [[ ! $version =~ $SEMVER_PATTERN ]]; then
    printf 'invalid version; expected a semantic version: %s\n' "$version" >&2
    return 1
  fi
}

guard_bootstrap() {
  if (( $# != 0 )); then
    printf 'guard-bootstrap accepts no arguments\n' >&2
    return 2
  fi

  local release_tag=${RELEASE_TAG:-}

  if [[ "$release_tag" != 'v0.1.0' ]]; then
    printf 'bootstrap accepts only v0.1.0\n' >&2
    return 1
  fi
}

validate_release() {
  local mode=${1:-}
  local release_tag release_version tag_commit checked_out_commit metadata package_text
  local core_version='' cli_version='' row package_name package_version
  local dependency_info='' dependency_count='' dependency_req='' expected_dependency_req
  local -a package_rows

  case "$mode" in
    bootstrap)
      release_tag=${RELEASE_TAG:-}
      if [[ "$release_tag" != 'v0.1.0' ]]; then
        printf 'bootstrap accepts only v0.1.0\n' >&2
        return 1
      fi
      ;;
    normal)
      release_tag=${GITHUB_REF_NAME:-}
      if [[ ! $release_tag =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        printf 'unsupported release tag: %s\n' "$release_tag" >&2
        return 1
      fi
      ;;
    *)
      printf 'validate requires bootstrap or normal mode\n' >&2
      return 2
      ;;
  esac

  if [[ "$mode" == normal ]]; then
    release_version="${GITHUB_REF_NAME#v}"
  else
    release_version="${RELEASE_TAG#v}"
  fi
  if [[ ! $release_version =~ $RELEASE_VERSION_PATTERN ]]; then
    printf 'unsupported release version: %s\n' "$release_version" >&2
    return 1
  fi

  git fetch --no-tags origin main
  if ! tag_commit="$(git rev-parse --verify "refs/tags/${release_tag}^{commit}")"; then
    printf 'could not resolve release tag %s\n' "$release_tag" >&2
    return 1
  fi
  checked_out_commit="$(git rev-parse --verify HEAD)"
  if [[ "$checked_out_commit" != "$tag_commit" ]]; then
    printf 'checked out %s, expected tag commit %s\n' \
      "$checked_out_commit" "$tag_commit" >&2
    return 1
  fi

  if [[ "$mode" == normal ]]; then
    if [[ -z ${RELEASE_SHA:-} || "$RELEASE_SHA" != "$tag_commit" ]]; then
      printf 'event SHA %s differs from tag commit %s\n' \
        "${RELEASE_SHA:-<unset>}" "$tag_commit" >&2
      return 1
    fi
  fi

  if ! git merge-base --is-ancestor "$tag_commit" origin/main; then
    printf 'tag %s is not reachable from origin/main\n' "$release_tag" >&2
    return 1
  fi

  metadata="$(cargo metadata --no-deps --format-version 1)"
  if ! package_text="$(jq -r '
    .packages[]
    | select(.name == "mdbook-structured-core" or .name == "mdbook-structured")
    | [.name, .version]
    | @tsv
  ' <<<"$metadata")"; then
    printf 'cargo metadata did not produce valid package data\n' >&2
    return 1
  fi
  mapfile -t package_rows <<<"$package_text"
  if (( ${#package_rows[@]} != 2 )); then
    printf 'expected exactly two release packages in cargo metadata\n' >&2
    return 1
  fi

  for row in "${package_rows[@]}"; do
    IFS=$'\t' read -r package_name package_version <<<"$row"
    case "$package_name" in
      mdbook-structured-core) core_version="$package_version" ;;
      mdbook-structured) cli_version="$package_version" ;;
    esac
  done
  if [[ "$core_version" != "$release_version" || "$cli_version" != "$release_version" ]]; then
    printf 'tag %s expects %s; package versions are core=%s cli=%s\n' \
      "$release_tag" "$release_version" "$core_version" "$cli_version" >&2
    return 1
  fi

  # Cargo normalizes the workspace's plain `version = "x.y.z"` declaration to
  # a caret requirement in metadata. Keep that registry requirement in lockstep
  # with the release while preserving the manifest/packaging contract.
  if ! dependency_info="$(jq -r '
    [
      .packages[]
      | select(.name == "mdbook-structured")
      | .dependencies[]?
      | select(.name == "mdbook-structured-core")
      | .req
    ]
    | [length, (if length == 1 then .[0] else "" end)]
    | @tsv
  ' <<<"$metadata")"; then
    printf 'cargo metadata did not produce a readable CLI core dependency\n' >&2
    return 1
  fi
  IFS=$'\t' read -r dependency_count dependency_req <<<"$dependency_info"
  if [[ "$dependency_count" != 1 ]]; then
    printf 'expected exactly one mdbook-structured-core dependency; found %s\n' \
      "${dependency_count:-0}" >&2
    return 1
  fi
  expected_dependency_req="^${release_version}"
  if [[ "$dependency_req" != "$expected_dependency_req" ]]; then
    printf 'tag %s expects CLI mdbook-structured-core requirement %s; found %s\n' \
      "$release_tag" "$expected_dependency_req" "${dependency_req:-<missing>}" >&2
    return 1
  fi

  if [[ -z ${GITHUB_OUTPUT:-} ]]; then
    printf 'GITHUB_OUTPUT is required for release validation\n' >&2
    return 1
  fi
  printf 'version=%s\n' "$release_version" >>"$GITHUB_OUTPUT"
  printf 'commit=%s\n' "$tag_commit" >>"$GITHUB_OUTPUT"
  printf 'validated %s at %s\n' "$release_tag" "$tag_commit"
}

registry_core_version_unavailable() {
  local package_log=$1

  grep -Fq 'no matching package named `mdbook-structured-core` found' "$package_log" \
    || {
      grep -Fq 'failed to select a version for the requirement `mdbook-structured-core = "' \
        "$package_log" \
        && grep -Fq 'candidate versions found which did' "$package_log" \
        && grep -Fq 'location searched: crates.io index' "$package_log"
    }
}

package_cli() (
  set -euo pipefail

  local package_log package_config='' workspace
  package_log="$(mktemp)"
  cleanup() {
    rm -f -- "$package_log"
    if [[ -n "$package_config" ]]; then
      rm -f -- "$package_config"
    fi
  }
  trap cleanup EXIT

  # The unmodified command is the authoritative first attempt.
  if cargo package --locked -p mdbook-structured 2>&1 | tee "$package_log"; then
    exit 0
  fi

  # If the exact registry version is not available yet, permit a local patch
  # only for Cargo's missing-package or missing-version diagnostics.
  if ! registry_core_version_unavailable "$package_log"; then
    printf 'cargo package failed for an unexpected reason\n' >&2
    exit 1
  fi

  workspace=${GITHUB_WORKSPACE:-$PWD}
  package_config="$(mktemp)"
  printf '%s\n' \
    '[patch.crates-io]' \
    "mdbook-structured-core = { path = \"${workspace}/crates/mdbook-structured-core\" }" \
    >"$package_config"
  cargo --config "$package_config" package --locked -p mdbook-structured
)

version_visible_in_search() {
  local crate=$1 version=$2 output=$3 line

  while IFS= read -r line; do
    if [[ $line =~ ^[[:space:]]*([^[:space:]]+)[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]] \
      && [[ ${BASH_REMATCH[1]} == "$crate" ]] \
      && [[ ${BASH_REMATCH[2]} == "$version" ]]; then
      return 0
    fi
  done <<<"$output"
  return 1
}

version_visible_in_info() {
  local crate=$1 version=$2 output=$3 line first second third
  local version_seen=false registry_seen=false registry_path

  while IFS= read -r line; do
    line=${line%$'\r'}
    read -r first second third _ <<<"$line"
    if [[ "$first" == 'version:' && "$second" == "$version" ]]; then
      version_seen=true
    fi
    if [[ "$first" == "$crate" && "$second" == "v$version" ]]; then
      version_seen=true
    fi
    if [[ "$first" == 'Downloaded' && "$second" == "$crate" && "$third" == "v$version" ]]; then
      version_seen=true
    fi
    if [[ "$first" == 'crates.io:' ]]; then
      registry_path=${second#https://crates.io/crates/}
      if [[ "$registry_path" == "$crate/$version" ]]; then
        registry_seen=true
      fi
    fi
  done <<<"$output"

  [[ "$version_seen" == true && "$registry_seen" == true ]]
}

wait_index() {
  if (( $# != 2 )); then
    printf 'wait-index requires <crate> <version>\n' >&2
    return 2
  fi

  local crate=$1 version=$2 attempt search_output info_output
  validate_crate_name "$crate"
  validate_semver "$version"

  for ((attempt = 1; attempt <= INDEX_ATTEMPTS; attempt++)); do
    search_output=''
    if search_output="$(CARGO_TERM_COLOR=never cargo search "$crate" --limit 100 2>&1)" \
      && version_visible_in_search "$crate" "$version" "$search_output"; then
      printf '%s %s is published and indexed on crates.io.\n' "$crate" "$version"
      return 0
    fi

    # Search usually reports only the latest release. Exact cargo info keeps
    # historical-version reruns safe while still proving the requested entry.
    info_output=''
    if info_output="$(CARGO_TERM_COLOR=never cargo info --registry crates-io "$crate@$version" 2>&1)" \
      && version_visible_in_info "$crate" "$version" "$info_output"; then
      printf '%s %s is published and indexed on crates.io (cargo info).\n' "$crate" "$version"
      return 0
    fi

    if (( attempt < INDEX_ATTEMPTS )); then
      sleep "$INDEX_RETRY_SECONDS"
    fi
  done

  printf 'timed out waiting for %s %s in the crates.io index after %d attempts; retry the release.\n' \
    "$crate" "$version" "$INDEX_ATTEMPTS" >&2
  return 1
}

verify_registry() {
  if (( $# > 1 )); then
    printf 'verify-registry accepts at most one version argument\n' >&2
    return 2
  fi

  local release_version=${1:-${RELEASE_VERSION:-}}
  local crate_name endpoint body
  if [[ -z "$release_version" ]]; then
    printf 'RELEASE_VERSION is required for registry verification\n' >&2
    return 2
  fi
  validate_semver "$release_version"

  for crate_name in mdbook-structured-core mdbook-structured; do
    endpoint="https://crates.io/api/v1/crates/${crate_name}/${release_version}"
    if ! body="$(curl --silent --show-error --location --fail \
      --connect-timeout 5 --max-time 10 \
      --user-agent "$RELEASE_USER_AGENT" "$endpoint")"; then
      printf 'could not fetch exact crates.io record for %s %s\n' \
        "$crate_name" "$release_version" >&2
      return 1
    fi
    if ! jq -e --arg expected "$release_version" \
      '(.version.num == $expected) and (.version.yanked == false)' \
      <<<"$body" >/dev/null; then
      printf 'crates.io record for %s %s is absent or yanked\n' \
        "$crate_name" "$release_version" >&2
      return 1
    fi
    printf '%s %s is visible with yanked=false\n' "$crate_name" "$release_version"
  done
}

main() {
  if (( $# == 0 )); then
    usage
    return 2
  fi

  local command=$1
  shift
  case "$command" in
    guard-bootstrap) guard_bootstrap "$@" ;;
    validate) validate_release "$@" ;;
    package-cli) package_cli "$@" ;;
    wait-index) wait_index "$@" ;;
    verify-registry) verify_registry "$@" ;;
    *)
      usage
      return 2
      ;;
  esac
}

main "$@"
