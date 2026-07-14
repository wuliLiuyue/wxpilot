#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="${WXP_REPO_ROOT:-$(cd "$script_dir/../.." && pwd)}"
input_dir="${WXP_PUBLIC_INPUT_DIR:-$repo_root/dist/local}"
output_dir="${WXP_PUBLIC_OUTPUT_DIR:-}"
version=""

usage() {
  cat <<'EOF'
Usage: package-public-release.sh --version <tag> [--input-dir <dir>] [--output-dir <dir>]

Options:
  --version <tag>          Release tag, e.g. v0.1.0
  --input-dir <dir>        Input artifact root (default: dist/local)
  --output-dir <dir>       Output directory (default: dist/public-release/<tag>)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      [[ $# -ge 2 ]] || { echo "missing value for --version" >&2; exit 1; }
      version="$2"
      shift 2
      ;;
    --input-dir)
      [[ $# -ge 2 ]] || { echo "missing value for --input-dir" >&2; exit 1; }
      input_dir="$2"
      shift 2
      ;;
    --output-dir)
      [[ $# -ge 2 ]] || { echo "missing value for --output-dir" >&2; exit 1; }
      output_dir="$2"
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

[[ -n "$version" ]] || { echo "--version is required" >&2; usage >&2; exit 1; }

if [[ -z "$output_dir" ]]; then
  output_dir="$repo_root/dist/public-release/$version"
fi

sha256_file() {
  local file="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
    return
  fi
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
    return
  fi

  echo "missing checksum tool: need shasum or sha256sum" >&2
  exit 1
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$output_dir"

package_one() {
  local target="$1"
  local archive_name="$2"
  local src_dir="$input_dir/$target"
  local stage_dir="$tmp/$target"
  local cli_src="$src_dir/wxpilot"
  local daemon_src="$src_dir/wxp-daemon"

  [[ -f "$cli_src" ]] || { echo "missing artifact: $cli_src" >&2; exit 1; }
  [[ -f "$daemon_src" ]] || { echo "missing artifact: $daemon_src" >&2; exit 1; }

  mkdir -p "$stage_dir"
  install -m 755 "$cli_src" "$stage_dir/wxpilot"
  install -m 755 "$daemon_src" "$stage_dir/wxp-daemon"
  install -m 644 "$repo_root/LICENSE" "$stage_dir/LICENSE"

  tar -czf "$output_dir/$archive_name" -C "$stage_dir" wxpilot wxp-daemon LICENSE
}

package_one "aarch64-apple-darwin" "wxpilot-darwin-arm64.tar.gz"
package_one "x86_64-apple-darwin" "wxpilot-darwin-x64.tar.gz"

checksums_file="$output_dir/wxpilot-checksums.txt"
: > "$checksums_file"

for archive in wxpilot-darwin-arm64.tar.gz wxpilot-darwin-x64.tar.gz; do
  printf '%s  %s\n' "$(sha256_file "$output_dir/$archive")" "$archive" >> "$checksums_file"
done

echo "packaged release: $version"
echo "output dir: $output_dir"
echo "assets:"
echo "  - wxpilot-darwin-arm64.tar.gz"
echo "  - wxpilot-darwin-x64.tar.gz"
echo "  - wxpilot-checksums.txt"
