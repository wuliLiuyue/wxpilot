#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/bin" "$tmp/dist/aarch64-apple-darwin" "$tmp/dist/x86_64-apple-darwin"

cat > "$tmp/bin/uname-arm64" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "-s" ]]; then
  echo Darwin
elif [[ "${1:-}" == "-m" ]]; then
  echo arm64
else
  /usr/bin/uname "$@"
fi
EOF

cat > "$tmp/bin/uname-x86" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "-s" ]]; then
  echo Darwin
elif [[ "${1:-}" == "-m" ]]; then
  echo x86_64
else
  /usr/bin/uname "$@"
fi
EOF

cat > "$tmp/bin/uname-linux" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "-s" ]]; then
  echo Linux
elif [[ "${1:-}" == "-m" ]]; then
  echo x86_64
else
  /usr/bin/uname "$@"
fi
EOF

chmod +x "$tmp/bin/uname-arm64" "$tmp/bin/uname-x86" "$tmp/bin/uname-linux"

printf '#!/usr/bin/env bash\necho wxpilot 0.1.0\n' > "$tmp/dist/aarch64-apple-darwin/wxpilot"
printf '#!/usr/bin/env bash\necho daemon-arm64\n' > "$tmp/dist/aarch64-apple-darwin/wxp-daemon"
printf '#!/usr/bin/env bash\necho wxpilot 0.1.0\n' > "$tmp/dist/x86_64-apple-darwin/wxpilot"
printf '#!/usr/bin/env bash\necho daemon-x86\n' > "$tmp/dist/x86_64-apple-darwin/wxp-daemon"
chmod +x \
  "$tmp/dist/aarch64-apple-darwin/wxpilot" \
  "$tmp/dist/aarch64-apple-darwin/wxp-daemon" \
  "$tmp/dist/x86_64-apple-darwin/wxpilot" \
  "$tmp/dist/x86_64-apple-darwin/wxp-daemon"

home_arm="$tmp/home-arm"
mkdir -p "$home_arm"
output_arm="$(
  HOME="$home_arm" \
  PATH="/usr/bin:/bin" \
  UNAME_BIN="$tmp/bin/uname-arm64" \
  WXP_DIST_DIR="$tmp/dist" \
  bash "$repo_root/scripts/local/install-macos-binaries.sh"
)"

test -x "$home_arm/.local/bin/wxpilot"
test -x "$home_arm/.wxpilot/bin/wxp-daemon"
printf '%s' "$output_arm" | grep -F 'PATH hint:'
"$home_arm/.local/bin/wxpilot" --version | grep -F 'wxpilot 0.1.0'

home_x86="$tmp/home-x86"
mkdir -p "$home_x86"
HOME="$home_x86" \
PATH="/usr/bin:/bin" \
UNAME_BIN="$tmp/bin/uname-arm64" \
WXP_DIST_DIR="$tmp/dist" \
bash "$repo_root/scripts/local/install-macos-binaries.sh" --target x86_64-apple-darwin >/dev/null
test -x "$home_x86/.local/bin/wxpilot"
test -x "$home_x86/.wxpilot/bin/wxp-daemon"

set +e
missing_output="$(
  HOME="$tmp/home-missing" \
  PATH="/usr/bin:/bin" \
  UNAME_BIN="$tmp/bin/uname-x86" \
  WXP_DIST_DIR="$tmp/dist-missing" \
  bash "$repo_root/scripts/local/install-macos-binaries.sh" 2>&1
)"
missing_status=$?
set -e
test "$missing_status" -ne 0
printf '%s' "$missing_output" | grep -F 'missing artifact:'

set +e
unsupported_output="$(
  HOME="$tmp/home-linux" \
  PATH="/usr/bin:/bin" \
  UNAME_BIN="$tmp/bin/uname-linux" \
  WXP_DIST_DIR="$tmp/dist" \
  bash "$repo_root/scripts/local/install-macos-binaries.sh" 2>&1
)"
unsupported_status=$?
set -e
test "$unsupported_status" -ne 0
printf '%s' "$unsupported_output" | grep -F 'local install only supports macOS'
