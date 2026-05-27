#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: gen-license-key.sh <count> <prefix> [out-file]" >&2
  exit 1
fi

count="$1"
prefix="$2"
out_file="${3:-}"

if ! [[ "$count" =~ ^[1-9][0-9]*$ ]]; then
  echo "count must be a positive integer" >&2
  exit 1
fi

if ! [[ "$prefix" =~ ^[a-z0-9-]+$ ]]; then
  echo "prefix must match ^[a-z0-9-]+$" >&2
  exit 1
fi

if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl is required" >&2
  exit 1
fi

date_tag="$(date +%Y%m%d)"
tmp_file=""

if [[ -n "$out_file" ]]; then
  if [[ -e "$out_file" ]]; then
    echo "refusing to overwrite existing file: $out_file" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$out_file")"
  tmp_file="$out_file"
else
  tmp_file="/dev/stdout"
fi

generate_one() {
  local random
  random="$(openssl rand -hex 5 | tr '[:lower:]' '[:upper:]')"
  printf 'wxp-%s-%s-%s\n' "$prefix" "$date_tag" "$random"
}

for ((i = 0; i < count; i++)); do
  generate_one
done > "$tmp_file"

if [[ -n "$out_file" ]]; then
  echo "generated license keys: $out_file"
fi
