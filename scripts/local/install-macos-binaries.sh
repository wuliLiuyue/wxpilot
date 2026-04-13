#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="${WXP_REPO_ROOT:-$(cd "$script_dir/../.." && pwd)}"
dist_root="${WXP_DIST_DIR:-$repo_root/dist/local}"
uname_bin="${UNAME_BIN:-uname}"

target=""

usage() {
  cat <<'EOF'
Usage: install-macos-binaries.sh [--target <triple>]

Options:
  --target <triple>         Install artifacts for a specific macOS target
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      [[ $# -ge 2 ]] || { echo "missing value for --target" >&2; exit 1; }
      target="$2"
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

detect_target() {
  local os arch
  os="$("$uname_bin" -s)"
  arch="$("$uname_bin" -m)"

  [[ "$os" == "Darwin" ]] || {
    echo "local install only supports macOS (Darwin)" >&2
    exit 1
  }

  case "$arch" in
    arm64) echo "aarch64-apple-darwin" ;;
    x86_64) echo "x86_64-apple-darwin" ;;
    *)
      echo "unsupported macOS architecture: $arch" >&2
      exit 1
      ;;
  esac
}

if [[ -z "$target" ]]; then
  target="$(detect_target)"
fi

artifact_dir="$dist_root/$target"
cli_src="$artifact_dir/wxpilot"
daemon_src="$artifact_dir/wxp-daemon"

[[ -n "${HOME:-}" ]] || { echo "HOME is required" >&2; exit 1; }
[[ -f "$cli_src" ]] || { echo "missing artifact: $cli_src" >&2; exit 1; }
[[ -f "$daemon_src" ]] || { echo "missing artifact: $daemon_src" >&2; exit 1; }

cli_dir="$HOME/.local/bin"
daemon_dir="$HOME/.wxpilot/bin"

install -d "$cli_dir" "$daemon_dir"
install -m 755 "$cli_src" "$cli_dir/wxpilot"
install -m 755 "$daemon_src" "$daemon_dir/wxp-daemon"

echo "installed target: $target"
echo "wxpilot -> $cli_dir/wxpilot"
echo "wxp-daemon -> $daemon_dir/wxp-daemon"
echo "next step: run 'wxpilot activate <key>' before using release commands"

if [[ ":${PATH:-}:" != *":$cli_dir:"* ]]; then
  echo "PATH hint: add the following line to your shell profile"
  echo "export PATH=\"$cli_dir:\$PATH\""
fi
