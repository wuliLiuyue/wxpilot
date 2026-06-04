#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/bin"

cat > "$tmp/bin/uname" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "-s" ]]; then
  echo Darwin
elif [[ "${1:-}" == "-m" ]]; then
  echo arm64
else
  /usr/bin/uname "$@"
fi
EOF

cat > "$tmp/bin/rustup" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "target" && "${2:-}" == "list" && "${3:-}" == "--installed" ]]; then
  printf '%s\n' aarch64-apple-darwin x86_64-apple-darwin
  exit 0
fi
echo "unexpected rustup invocation: $*" >&2
exit 1
EOF

cat > "$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
echo "cargo should not run when required env is missing" >&2
exit 1
EOF

chmod +x "$tmp/bin/uname" "$tmp/bin/rustup" "$tmp/bin/cargo"

run_missing_case() {
  local label="$1"
  local expected="$2"
  shift 2

  set +e
  output="$(
    PATH="$tmp/bin:$PATH" \
    CARGO_BIN="$tmp/bin/cargo" \
    RUSTUP_BIN="$tmp/bin/rustup" \
    UNAME_BIN="$tmp/bin/uname" \
    WXP_RUST_TARGET_DIR="$tmp/target-$label" \
    WXP_DIST_DIR="$tmp/dist-$label" \
    "$@" \
    bash "$repo_root/scripts/local/build-macos-binaries.sh" --all 2>&1
  )"
  status=$?
  set -e

  test "$status" -ne 0
  printf '%s' "$output" | grep -F "$expected" >/dev/null
}

run_missing_case missing-public-key "WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX" \
  env WXP_LICENSE_SERVER_URL="https://feai.work/wxpilot-api"

run_missing_case missing-server-url "WXP_LICENSE_SERVER_URL" \
  env WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX="79b5562e8fe654f94078b112e8a98ba7901f853ae695bed7e0e3910bad049664"
