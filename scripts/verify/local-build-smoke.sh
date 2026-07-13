#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/bin" "$tmp/target" "$tmp/dist"

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
set -euo pipefail
target=""
bin_name=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      target="$2"
      shift 2
      ;;
    --bin)
      bin_name="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
[[ -n "$target" && -n "$bin_name" ]] || exit 1
out_dir="$CARGO_TARGET_DIR/$target/release"
mkdir -p "$out_dir"
printf '#!/usr/bin/env bash\necho %s\n' "$bin_name" > "$out_dir/$bin_name"
chmod +x "$out_dir/$bin_name"
EOF

chmod +x "$tmp/bin/uname" "$tmp/bin/rustup" "$tmp/bin/cargo"

PATH="$tmp/bin:$PATH" \
CARGO_BIN="$tmp/bin/cargo" \
RUSTUP_BIN="$tmp/bin/rustup" \
UNAME_BIN="$tmp/bin/uname" \
WXP_RUST_TARGET_DIR="$tmp/target" \
WXP_DIST_DIR="$tmp/dist" \
bash "$repo_root/scripts/local/build-macos-binaries.sh" --all

test -x "$tmp/dist/aarch64-apple-darwin/wxpilot"
test -x "$tmp/dist/aarch64-apple-darwin/wxp-daemon"
test -x "$tmp/dist/x86_64-apple-darwin/wxpilot"
test -x "$tmp/dist/x86_64-apple-darwin/wxp-daemon"
