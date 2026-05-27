#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
compose_file="$repo_root/compose.yaml"
env_file="$repo_root/.env"

usage() {
  cat <<'EOF'
Usage: deploy-compose.sh <up|down|logs|ps>

Commands:
  up      Build and start wxp-license-server
  down    Stop and remove wxp-license-server
  logs    Tail wxp-license-server logs
  ps      Show compose service status
EOF
}

if [[ $# -ne 1 ]]; then
  usage >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

if [[ ! -f "$compose_file" ]]; then
  echo "missing compose file: $compose_file" >&2
  exit 1
fi

command="$1"

case "$command" in
  up)
    if [[ ! -f "$env_file" ]]; then
      echo "missing .env file. start from .env.example:" >&2
      echo "cp .env.example .env" >&2
      exit 1
    fi
    (cd "$repo_root" && docker compose up -d --build)
    ;;
  down)
    (cd "$repo_root" && docker compose down)
    ;;
  logs)
    (cd "$repo_root" && docker compose logs -f wxp-license-server)
    ;;
  ps)
    (cd "$repo_root" && docker compose ps)
    ;;
  *)
    usage >&2
    exit 1
    ;;
esac
