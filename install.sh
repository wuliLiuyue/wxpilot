#!/usr/bin/env bash
#
# wxpilot 一键安装脚本
#
# 用法:
#   curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash -s -- --version v0.1.0
#   ./install.sh --version v0.1.0
#
# 环境变量覆盖:
#   WXPILOT_REPO              GitHub 仓库 (默认 wuliLiuyue/wxpilot)
#   WXPILOT_VERSION           版本 tag (默认 latest)
#   WXPILOT_INSTALL_DIR       wxpilot 安装目录 (默认 ~/.local/bin)
#   WXPILOT_DAEMON_DIR        wxp-daemon 安装目录 (默认 ~/.wxpilot/bin)
#   WXPILOT_RELEASE_BASE_URL  覆盖下载源 (测试用, 指向本地 file:// 目录)
#
set -euo pipefail

repo="${WXPILOT_REPO:-wuliLiuyue/wxpilot}"
version="${WXPILOT_VERSION:-latest}"
install_dir="${WXPILOT_INSTALL_DIR:-$HOME/.local/bin}"
daemon_dir="${WXPILOT_DAEMON_DIR:-$HOME/.wxpilot/bin}"
uname_bin="${UNAME_BIN:-uname}"

usage() {
  cat <<'EOF'
Usage: install.sh [options]

Options:
  --version <tag>       Release tag to install (default: latest)
  --repo <owner/name>   GitHub repository (default: wuliLiuyue/wxpilot)
  -h, --help            Show this help message

Environment variables:
  WXPILOT_VERSION           Same as --version
  WXPILOT_REPO              Same as --repo
  WXPILOT_INSTALL_DIR       wxpilot install dir (default: ~/.local/bin)
  WXPILOT_DAEMON_DIR        wxp-daemon install dir (default: ~/.wxpilot/bin)
  WXPILOT_RELEASE_BASE_URL  Override download base (for testing)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      [[ $# -ge 2 ]] || { echo "missing value for --version" >&2; exit 1; }
      version="$2"
      shift 2
      ;;
    --repo)
      [[ $# -ge 2 ]] || { echo "missing value for --repo" >&2; exit 1; }
      repo="$2"
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

[[ -n "${HOME:-}" ]] || { echo "HOME is required" >&2; exit 1; }

os="$("$uname_bin" -s)"
arch="$("$uname_bin" -m)"

if [[ "$os" != "Darwin" ]]; then
  echo "wxpilot 目前仅提供 macOS 二进制 (当前系统: $os)" >&2
  exit 1
fi

case "$arch" in
  arm64) asset="wxpilot-darwin-arm64.tar.gz" ;;
  x86_64) asset="wxpilot-darwin-x64.tar.gz" ;;
  *)
    echo "不支持的架构: $arch (当前仅支持 arm64 / x86_64)" >&2
    exit 1
    ;;
esac

if [[ -n "${WXPILOT_RELEASE_BASE_URL:-}" ]]; then
  base_url="${WXPILOT_RELEASE_BASE_URL%/}"
else
  base_url="https://github.com/${repo}/releases/download/${version}"
fi

asset_url="${base_url}/${asset}"
checksums_url="${base_url}/wxpilot-checksums.txt"

download() {
  local url="$1"
  local dest="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$dest"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$dest" "$url"
  else
    echo "需要 curl 或 wget 来下载" >&2
    exit 1
  fi
}

checksum_of() {
  local file="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{print $1}'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{print $1}'
  else
    echo ""
  fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "下载 $asset ..."
download "$asset_url" "$tmp/$asset"

if download "$checksums_url" "$tmp/wxpilot-checksums.txt" 2>/dev/null; then
  expected="$(grep -F "$asset" "$tmp/wxpilot-checksums.txt" | awk '{print $1}')"
  actual="$(checksum_of "$tmp/$asset")"
  if [[ -n "$expected" && -n "$actual" && "$expected" != "$actual" ]]; then
    echo "校验和不匹配: 期望 $expected, 实际 $actual" >&2
    exit 1
  fi
fi

tar -xzf "$tmp/$asset" -C "$tmp"

install -d "$install_dir" "$daemon_dir"
install -m 755 "$tmp/wxpilot" "$install_dir/wxpilot"
install -m 755 "$tmp/wxp-daemon" "$daemon_dir/wxp-daemon"

echo "installed wxpilot from $asset"
echo "  wxpilot    -> $install_dir/wxpilot"
echo "  wxp-daemon -> $daemon_dir/wxp-daemon"

if [[ ":${PATH:-}:" != *":$install_dir:"* ]]; then
  echo "PATH 提示: 将以下行加入 shell 配置"
  echo "export PATH=\"$install_dir:\$PATH\""
fi
