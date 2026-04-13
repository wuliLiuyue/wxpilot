#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readme="$repo_root/README.md"

grep -F '## 开发者本地打包与安装' "$readme" >/dev/null
grep -F 'make local-build' "$readme" >/dev/null
grep -F 'make local-install' "$readme" >/dev/null
if grep -F 'curl -fsSL' "$readme" >/dev/null; then
  echo "README should not introduce curl install flow for local packaging" >&2
  exit 1
fi
