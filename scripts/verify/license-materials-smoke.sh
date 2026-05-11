#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

out_dir="$tmp/materials"

bash "$repo_root/scripts/license/gen-license-materials.sh" "$out_dir" "test-20260511" >/dev/null

test -f "$out_dir/server.env"
test -f "$out_dir/client.env"
test -f "$out_dir/manifest.env"
grep -F 'WXP_LICENSE_PRIVATE_KEY_HEX=' "$out_dir/server.env" >/dev/null
grep -F 'WXP_LICENSE_KEY_ID=test-20260511' "$out_dir/server.env" >/dev/null
grep -F 'WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX=' "$out_dir/client.env" >/dev/null

bash "$repo_root/scripts/license/verify-license-materials.sh" "$out_dir/server.env" "$out_dir/client.env" >/dev/null

set +e
overwrite_output="$(
  bash "$repo_root/scripts/license/gen-license-materials.sh" "$out_dir" "test-20260511" 2>&1
)"
overwrite_status=$?
set -e
test "$overwrite_status" -ne 0
printf '%s' "$overwrite_output" | grep -F 'refusing to overwrite existing file:' >/dev/null
