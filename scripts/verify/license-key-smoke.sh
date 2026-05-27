#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

out_file="$tmp/license-keys.txt"

bash "$repo_root/scripts/license/gen-license-key.sh" 3 test "$out_file" >/dev/null

test -f "$out_file"
test "$(wc -l < "$out_file" | tr -d ' ')" = "3"
grep -E '^wxp-test-[0-9]{8}-[A-F0-9]{10}$' "$out_file" >/dev/null

set +e
bad_prefix_output="$(
  bash "$repo_root/scripts/license/gen-license-key.sh" 1 BAD "$tmp/bad.txt" 2>&1
)"
bad_prefix_status=$?
set -e
test "$bad_prefix_status" -ne 0
printf '%s' "$bad_prefix_output" | grep -F 'prefix must match' >/dev/null
