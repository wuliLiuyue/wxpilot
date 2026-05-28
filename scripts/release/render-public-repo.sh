#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="${WXP_REPO_ROOT:-$(cd "$script_dir/../.." && pwd)}"
template_dir="$repo_root/distribution/public-repo"
output_dir="${WXP_PUBLIC_REPO_OUTPUT_DIR:-$repo_root/dist/public-repo}"
public_repo=""
example_version=""

usage() {
  cat <<'EOF'
Usage: render-public-repo.sh --repo <owner/name> --version <tag> [--output-dir <dir>]

Options:
  --repo <owner/name>      Public GitHub repository name
  --version <tag>          Example version shown in README, e.g. v0.1.0
  --output-dir <dir>       Render output directory (default: dist/public-repo)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo)
      [[ $# -ge 2 ]] || { echo "missing value for --repo" >&2; exit 1; }
      public_repo="$2"
      shift 2
      ;;
    --version)
      [[ $# -ge 2 ]] || { echo "missing value for --version" >&2; exit 1; }
      example_version="$2"
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

[[ -n "$public_repo" ]] || { echo "--repo is required" >&2; usage >&2; exit 1; }
[[ "$public_repo" == */* ]] || { echo "--repo must look like owner/name" >&2; exit 1; }
[[ -n "$example_version" ]] || { echo "--version is required" >&2; usage >&2; exit 1; }

rm -rf "$output_dir"
mkdir -p "$output_dir/skill"

render_template() {
  local src="$1"
  local dest="$2"

  sed \
    -e "s|__PUBLIC_REPO__|$public_repo|g" \
    -e "s|__EXAMPLE_VERSION__|$example_version|g" \
    "$src" > "$dest"
}

render_template "$template_dir/README.md.template" "$output_dir/README.md"
render_template "$template_dir/install.sh.template" "$output_dir/install.sh"
render_template "$template_dir/skill/SKILL.md.template" "$output_dir/skill/SKILL.md"

chmod +x "$output_dir/install.sh"

echo "rendered public repo: $output_dir"
echo "repo: $public_repo"
echo "example version: $example_version"
