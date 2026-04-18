#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="${WXP_REPO_ROOT:-$(cd "$script_dir/../.." && pwd)}"
dist_root="${WXP_DIST_DIR:-$repo_root/dist/local}"
target_root="${WXP_RUST_TARGET_DIR:-$repo_root/rust/target}"
cargo_bin="${CARGO_BIN:-cargo}"
rustup_bin="${RUSTUP_BIN:-rustup}"
uname_bin="${UNAME_BIN:-uname}"

targets=()

usage() {
  cat <<'EOF'
Usage: build-macos-binaries.sh [--all] [--target <triple>]

Options:
  --all                     Build aarch64-apple-darwin and x86_64-apple-darwin
  --target <triple>         Build a single macOS target

Required environment variables for release CLI:
  WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX
  WXP_LICENSE_SERVER_URL
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --all)
      targets=("aarch64-apple-darwin" "x86_64-apple-darwin")
      shift
      ;;
    --target)
      [[ $# -ge 2 ]] || { echo "missing value for --target" >&2; exit 1; }
      targets+=("$2")
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ ${#targets[@]} -eq 0 ]]; then
  targets=("aarch64-apple-darwin" "x86_64-apple-darwin")
fi

if [[ "$("$uname_bin" -s)" != "Darwin" ]]; then
  echo "local macOS build only supports Darwin hosts" >&2
  exit 1
fi

require_release_env() {
  local missing=()

  [[ -n "${WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX:-}" ]] || missing+=("WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX")
  [[ -n "${WXP_LICENSE_SERVER_URL:-}" ]] || missing+=("WXP_LICENSE_SERVER_URL")

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo "missing required release env: ${missing[*]}" >&2
    echo "set the embedded public key and activation server URL before building wxpilot release binaries" >&2
    exit 1
  fi
}

require_release_env

check_target_installed() {
  local target="$1"
  if ! "$rustup_bin" target list --installed | grep -Fx "$target" >/dev/null; then
    echo "missing rust target: $target" >&2
    echo "run: rustup target add $target" >&2
    exit 1
  fi
}

build_one() {
  local target="$1"
  local out_dir="$dist_root/$target"
  check_target_installed "$target"

  CARGO_TARGET_DIR="$target_root" "$cargo_bin" build \
    --manifest-path "$repo_root/rust/Cargo.toml" \
    --release \
    -p wxp-cli \
    --bin wxpilot \
    --target "$target"

  CARGO_TARGET_DIR="$target_root" "$cargo_bin" build \
    --manifest-path "$repo_root/rust/Cargo.toml" \
    --release \
    -p wxp-daemon \
    --bin wxp-daemon \
    --target "$target"

  install -d "$out_dir"
  install -m 755 "$target_root/$target/release/wxpilot" "$out_dir/wxpilot"
  install -m 755 "$target_root/$target/release/wxp-daemon" "$out_dir/wxp-daemon"
  echo "built $target -> $out_dir"
}

for target in "${targets[@]}"; do
  build_one "$target"
done
