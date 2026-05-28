#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p \
  "$tmp/input/aarch64-apple-darwin" \
  "$tmp/input/x86_64-apple-darwin" \
  "$tmp/output"

printf '#!/usr/bin/env bash\necho arm64-cli\n' > "$tmp/input/aarch64-apple-darwin/wxpilot"
printf '#!/usr/bin/env bash\necho arm64-daemon\n' > "$tmp/input/aarch64-apple-darwin/wxp-daemon"
printf '#!/usr/bin/env bash\necho x64-cli\n' > "$tmp/input/x86_64-apple-darwin/wxpilot"
printf '#!/usr/bin/env bash\necho x64-daemon\n' > "$tmp/input/x86_64-apple-darwin/wxp-daemon"
chmod +x \
  "$tmp/input/aarch64-apple-darwin/wxpilot" \
  "$tmp/input/aarch64-apple-darwin/wxp-daemon" \
  "$tmp/input/x86_64-apple-darwin/wxpilot" \
  "$tmp/input/x86_64-apple-darwin/wxp-daemon"

bash "$repo_root/scripts/release/package-public-release.sh" \
  --version v9.9.9 \
  --input-dir "$tmp/input" \
  --output-dir "$tmp/output"

test -f "$tmp/output/wxpilot-darwin-arm64.tar.gz"
test -f "$tmp/output/wxpilot-darwin-x64.tar.gz"
test -f "$tmp/output/wxpilot-checksums.txt"

tar -tzf "$tmp/output/wxpilot-darwin-arm64.tar.gz" | grep -Fx 'wxpilot'
tar -tzf "$tmp/output/wxpilot-darwin-arm64.tar.gz" | grep -Fx 'wxp-daemon'
tar -tzf "$tmp/output/wxpilot-darwin-x64.tar.gz" | grep -Fx 'wxpilot'
tar -tzf "$tmp/output/wxpilot-darwin-x64.tar.gz" | grep -Fx 'wxp-daemon'

grep -F 'wxpilot-darwin-arm64.tar.gz' "$tmp/output/wxpilot-checksums.txt"
grep -F 'wxpilot-darwin-x64.tar.gz' "$tmp/output/wxpilot-checksums.txt"
