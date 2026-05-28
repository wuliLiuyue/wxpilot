#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/release" "$tmp/public" "$tmp/pkg-arm64" "$tmp/pkg-x64"

printf '#!/usr/bin/env bash\necho wxpilot 9.9.9 arm64\n' > "$tmp/pkg-arm64/wxpilot"
printf '#!/usr/bin/env bash\necho daemon 9.9.9 arm64\n' > "$tmp/pkg-arm64/wxp-daemon"
printf '#!/usr/bin/env bash\necho wxpilot 9.9.9 x64\n' > "$tmp/pkg-x64/wxpilot"
printf '#!/usr/bin/env bash\necho daemon 9.9.9 x64\n' > "$tmp/pkg-x64/wxp-daemon"
chmod +x \
  "$tmp/pkg-arm64/wxpilot" \
  "$tmp/pkg-arm64/wxp-daemon" \
  "$tmp/pkg-x64/wxpilot" \
  "$tmp/pkg-x64/wxp-daemon"

tar -czf "$tmp/release/wxpilot-darwin-arm64.tar.gz" -C "$tmp/pkg-arm64" wxpilot wxp-daemon
tar -czf "$tmp/release/wxpilot-darwin-x64.tar.gz" -C "$tmp/pkg-x64" wxpilot wxp-daemon

checksum_cmd() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
    return
  fi
  sha256sum "$1" | awk '{print $1}'
}

{
  printf '%s  %s\n' \
    "$(checksum_cmd "$tmp/release/wxpilot-darwin-arm64.tar.gz")" \
    "wxpilot-darwin-arm64.tar.gz"
  printf '%s  %s\n' \
    "$(checksum_cmd "$tmp/release/wxpilot-darwin-x64.tar.gz")" \
    "wxpilot-darwin-x64.tar.gz"
} > "$tmp/release/wxpilot-checksums.txt"

bash "$repo_root/scripts/release/render-public-repo.sh" \
  --repo acme/wxpilot \
  --version v9.9.9 \
  --output-dir "$tmp/public"

grep -F 'acme/wxpilot' "$tmp/public/README.md" >/dev/null
grep -F 'v9.9.9' "$tmp/public/README.md" >/dev/null
test -x "$tmp/public/install.sh"

home_dir="$tmp/home"
mkdir -p "$home_dir"

output="$(
  HOME="$home_dir" \
  PATH="/usr/bin:/bin" \
  WXP_RELEASE_BASE_URL="file://$tmp/release" \
  bash "$tmp/public/install.sh"
)"

test -x "$home_dir/.local/bin/wxpilot"
test -x "$home_dir/.wxpilot/bin/wxp-daemon"
if [[ "$(uname -m)" == "arm64" ]]; then
  printf '%s' "$output" | grep -F "installed wxpilot from wxpilot-darwin-arm64.tar.gz"
else
  printf '%s' "$output" | grep -F "installed wxpilot from wxpilot-darwin-x64.tar.gz"
fi
printf '%s' "$output" | grep -F "next step: run 'wxpilot activate <license-key>'"
"$home_dir/.local/bin/wxpilot" --version | grep -F 'wxpilot 9.9.9'
