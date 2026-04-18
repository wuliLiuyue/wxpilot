#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
  echo "Usage: backup-sqlite-license.sh <db-path> [env-file] [out-dir]" >&2
  exit 1
fi

db_path="$1"
env_file="${2:-./.env}"
out_dir="${3:-./backups}"

if [[ ! -f "$db_path" ]]; then
  echo "missing database file: $db_path" >&2
  exit 1
fi

if [[ ! -f "$env_file" ]]; then
  echo "missing env file: $env_file" >&2
  exit 1
fi

ts="$(date +%Y%m%d-%H%M%S)"
mkdir -p "$out_dir"

db_backup="$out_dir/license-$ts.db"
env_backup="$out_dir/license-$ts.env"

cp "$db_path" "$db_backup"
cp "$env_file" "$env_backup"

echo "backup complete"
echo "db: $db_backup"
echo "env: $env_backup"
