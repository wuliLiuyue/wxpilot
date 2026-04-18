#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: seed-sqlite-license.sh <db-path> <license-key> [status]" >&2
  exit 1
fi

db_path="$1"
license_key="$2"
status="${3:-active}"

case "$status" in
  active|inactive) ;;
  *)
    echo "status must be active or inactive" >&2
    exit 1
    ;;
esac

hash_license_key() {
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s' "$license_key" | sha256sum | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    printf '%s' "$license_key" | shasum -a 256 | awk '{print $1}'
    return
  fi

  echo "missing sha256sum or shasum" >&2
  exit 1
}

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required" >&2
  exit 1
fi

mkdir -p "$(dirname "$db_path")"
license_hash="$(hash_license_key)"
created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

sqlite3 "$db_path" <<SQL
CREATE TABLE IF NOT EXISTS licenses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key_hash TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL
);

INSERT OR IGNORE INTO licenses (key_hash, status, created_at)
VALUES ('$license_hash', '$status', '$created_at');
SQL

echo "seeded license key into $db_path"
echo "key: $license_key"
echo "status: $status"
