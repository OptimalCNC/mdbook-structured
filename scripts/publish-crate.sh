#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )) || [[ -z ${1:-} ]] || [[ -z ${2:-} ]]; then
  printf 'usage: %s <crate> <version>\n' "$0" >&2
  exit 2
fi

crate=$1
version=$2

if [[ ! $crate =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]] || (( ${#crate} > 64 )); then
  printf 'invalid crate name; expected an ASCII crates.io identifier\n' >&2
  exit 2
fi

if [[ ! $version =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
  printf 'invalid version; expected a semantic version\n' >&2
  exit 2
fi

endpoint="https://crates.io/api/v1/crates/${crate}/${version}"
user_agent='mdbook-structured-publish-helper/1.0'
export CARGO_HTTP_USER_AGENT="$user_agent"

query_version_status() {
  local status rc

  if status=$(curl --silent --show-error --location \
    --connect-timeout 5 --max-time 10 \
    --user-agent "$user_agent" \
    --output /dev/null --write-out '%{http_code}' \
    "$endpoint"); then
    if [[ ! $status =~ ^[0-9]{3}$ ]]; then
      printf 'crates.io returned an invalid HTTP status for %s %s: %s\n' \
        "$crate" "$version" "$status" >&2
      return 1
    fi
    printf '%s\n' "$status"
  else
    rc=$?
    printf 'crates.io request failed for %s %s\n' "$crate" "$version" >&2
    return "$rc"
  fi
}

wait_for_index() {
  local attempt search_output info_output line first second third registry_path
  local version_seen registry_seen

  for ((attempt = 1; attempt <= 12; attempt++)); do
    search_output=''
    if search_output=$(CARGO_TERM_COLOR=never cargo search "$crate" --limit 100 2>&1) \
      && version_visible_in_search "$search_output"; then
      printf '%s %s is published and indexed on crates.io.\n' "$crate" "$version"
      return 0
    fi

    # Search normally reports only the latest release. The registry-scoped
    # query also confirms historical versions without resolving a workspace path.
    info_output=''
    if info_output=$(CARGO_TERM_COLOR=never cargo info --registry crates-io "$crate@$version" 2>&1); then
      version_seen=false
      registry_seen=false
      while IFS= read -r line; do
        line=${line%$'\r'}
        read -r first second third _ <<<"$line"
        if [[ "$first" == 'version:' && "$second" == "$version" ]] \
          || [[ "$first" == "$crate" && "$second" == "v$version" ]] \
          || [[ "$first" == 'Downloaded' && "$second" == "$crate" && "$third" == "v$version" ]]; then
          version_seen=true
        fi
        if [[ "$first" == 'crates.io:' ]]; then
          registry_path=${second#https://crates.io/crates/}
          if [[ "$registry_path" == "$crate/$version" ]]; then
            registry_seen=true
          fi
        fi
      done <<<"$info_output"
      if [[ "$version_seen" == true && "$registry_seen" == true ]]; then
        printf '%s %s is published and indexed on crates.io (cargo info).\n' \
          "$crate" "$version"
        return 0
      fi
    fi

    if (( attempt < 12 )); then
      sleep 5
    fi
  done

  printf 'timed out waiting for %s %s in the crates.io index after 12 attempts; retry the release.\n' \
    "$crate" "$version" >&2
  return 1
}

version_visible_in_search() {
  local output=$1 line

  while IFS= read -r line; do
    if [[ $line =~ ^[[:space:]]*([^[:space:]]+)[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]] \
      && [[ ${BASH_REMATCH[1]} == "$crate" ]] \
      && [[ ${BASH_REMATCH[2]} == "$version" ]]; then
      return 0
    fi
  done <<<"$output"
  return 1
}

if initial_status=$(query_version_status); then
  :
else
  initial_rc=$?
  exit "$initial_rc"
fi

case $initial_status in
  200)
    printf '%s %s is already published on crates.io.\n' "$crate" "$version"
    exit 0
    ;;
  404)
    ;;
  *)
    printf 'cannot publish %s %s: crates.io returned HTTP %s\n' \
      "$crate" "$version" "$initial_status" >&2
    exit 1
    ;;
esac

if cargo publish --locked -p "$crate"; then
  :
else
  publish_rc=$?
  if recheck_status=$(query_version_status); then
    if [[ $recheck_status == 200 ]]; then
      printf '%s %s became visible on crates.io after publish interruption.\n' \
        "$crate" "$version"
      exit 0
    fi
  fi
  printf 'cargo publish failed for %s %s (exit %s); version was not confirmed on crates.io.\n' \
    "$crate" "$version" "$publish_rc" >&2
  exit "$publish_rc"
fi

visible=false
for ((attempt = 1; attempt <= 60; attempt++)); do
  if poll_status=$(query_version_status); then
    case $poll_status in
      200)
        visible=true
        break
        ;;
      404)
        ;;
      *)
        printf 'cannot confirm %s %s: crates.io returned HTTP %s\n' \
          "$crate" "$version" "$poll_status" >&2
        exit 1
        ;;
    esac
  else
    poll_rc=$?
    exit "$poll_rc"
  fi

  if (( attempt < 60 )); then
    sleep 5
  fi
done

if [[ $visible != true ]]; then
  printf 'timed out waiting for %s %s on crates.io after 60 attempts; retry the release.\n' \
    "$crate" "$version" >&2
  exit 1
fi

wait_for_index
