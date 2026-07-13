#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# 构造假的 release 产物 (arm64 + x86_64)
mkdir -p "$tmp/release" "$tmp/pkg-arm64" "$tmp/pkg-x64"

printf '#!/usr/bin/env bash\necho wxpilot 9.9.9 arm64\n' > "$tmp/pkg-arm64/wxpilot"
printf '#!/usr/bin/env bash\necho daemon 9.9.9 arm64\n' > "$tmp/pkg-arm64/wxp-daemon"
printf '#!/usr/bin/env bash\necho wxpilot 9.9.9 x64\n' > "$tmp/pkg-x64/wxpilot"
printf '#!/usr/bin/env bash\necho daemon 9.9.9 x64\n' > "$tmp/pkg-x64/wxp-daemon"
chmod +x \
  "$tmp/pkg-arm64/wxpilot" "$tmp/pkg-arm64/wxp-daemon" \
  "$tmp/pkg-x64/wxpilot" "$tmp/pkg-x64/wxp-daemon"

tar -czf "$tmp/release/wxpilot-darwin-arm64.tar.gz" -C "$tmp/pkg-arm64" wxpilot wxp-daemon
tar -czf "$tmp/release/wxpilot-darwin-x64.tar.gz" -C "$tmp/pkg-x64" wxpilot wxp-daemon

checksum_cmd() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

{
  printf '%s  %s\n' \
    "$(checksum_cmd "$tmp/release/wxpilot-darwin-arm64.tar.gz")" \
    "wxpilot-darwin-arm64.tar.gz"
  printf '%s  %s\n' \
    "$(checksum_cmd "$tmp/release/wxpilot-darwin-x64.tar.gz")" \
    "wxpilot-darwin-x64.tar.gz"
} > "$tmp/release/wxpilot-checksums.txt"

# 模拟 arm64 宿主安装
home_dir="$tmp/home"
mkdir -p "$home_dir"

output="$(
  HOME="$home_dir" \
  PATH="/usr/bin:/bin" \
  WXPILOT_RELEASE_BASE_URL="file://$tmp/release" \
  bash "$repo_root/install.sh"
)"

test -x "$home_dir/.local/bin/wxpilot"
test -x "$home_dir/.wxpilot/bin/wxp-daemon"

if [[ "$(uname -m)" == "arm64" ]]; then
  printf '%s' "$output" | grep -F "installed wxpilot from wxpilot-darwin-arm64.tar.gz"
else
  printf '%s' "$output" | grep -F "installed wxpilot from wxpilot-darwin-x64.tar.gz"
fi
"$home_dir/.local/bin/wxpilot" --version | grep -F 'wxpilot 9.9.9'

# 非 macOS 应被拒绝
cat > "$tmp/bin-uname-linux" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "-s" ]]; then
  echo Linux
elif [[ "${1:-}" == "-m" ]]; then
  echo x86_64
else
  /usr/bin/uname "$@"
fi
EOF
chmod +x "$tmp/bin-uname-linux"

set +e
unsupported_output="$(
  HOME="$tmp/home-linux" \
  PATH="/usr/bin:/bin" \
  UNAME_BIN="$tmp/bin-uname-linux" \
  WXPILOT_RELEASE_BASE_URL="file://$tmp/release" \
  bash "$repo_root/install.sh" 2>&1
)"
unsupported_status=$?
set -e

test "$unsupported_status" -ne 0
printf '%s' "$unsupported_output" | grep -F 'macOS'
