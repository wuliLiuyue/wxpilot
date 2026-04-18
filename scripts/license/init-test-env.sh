#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: init-test-env.sh <license-key> [db-path]" >&2
  exit 1
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
license_key="$1"
db_path="${2:-./data/license.db}"

echo "[1/3] starting license server via compose"
bash "$script_dir/deploy-compose.sh" up

echo "[2/3] checking health"
if ! curl -fsS http://127.0.0.1:8787/healthz >/dev/null; then
  echo "health check failed: http://127.0.0.1:8787/healthz" >&2
  exit 1
fi

echo "[3/3] seeding license key"
bash "$script_dir/seed-sqlite-license.sh" "$db_path" "$license_key"

echo "test environment ready"
echo "key: $license_key"
echo "db: $db_path"
