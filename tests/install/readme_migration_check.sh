#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
README_PATH="${ROOT_DIR}/README.md"

grep -q 'curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | sh' "${README_PATH}"
grep -q '~/.local/bin/wxpilot' "${README_PATH}"
grep -q '~/.wxpilot/bin/wxp-daemon' "${README_PATH}"
