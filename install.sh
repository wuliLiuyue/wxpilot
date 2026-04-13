#!/bin/sh
set -eu

REPO_SLUG="${WXP_INSTALL_REPO:-wuliLiuyue/wxpilot}"
RELEASE_API_URL="${WXP_RELEASE_API_URL:-https://api.github.com/repos/${REPO_SLUG}/releases/latest}"
DOWNLOAD_BASE_URL="${WXP_RELEASE_DOWNLOAD_BASE_URL:-https://github.com/${REPO_SLUG}/releases/download}"
INSTALL_ROOT="${WXP_INSTALL_ROOT:-${HOME}}"
CLI_INSTALL_DIR="${WXP_CLI_INSTALL_DIR:-${INSTALL_ROOT}/.local/bin}"
DAEMON_INSTALL_DIR="${WXP_DAEMON_INSTALL_DIR:-${INSTALL_ROOT}/.wxpilot/bin}"
CURL_BIN="${CURL_BIN:-curl}"
TMP_DIR="${TMPDIR:-/tmp}"

version=""

usage() {
  cat <<'EOF'
Usage: install.sh [--version <tag>]

Installs:
  wxpilot     -> ~/.local/bin/wxpilot
  wxp-daemon  -> ~/.wxpilot/bin/wxp-daemon
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      shift
      if [ "$#" -eq 0 ]; then
        echo "missing value for --version" >&2
        exit 1
      fi
      version="$1"
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
  shift
done

detect_os() {
  if [ -n "${WXP_INSTALL_OS:-}" ]; then
    echo "${WXP_INSTALL_OS}"
    return
  fi
  uname -s
}

detect_arch() {
  if [ -n "${WXP_INSTALL_ARCH:-}" ]; then
    echo "${WXP_INSTALL_ARCH}"
    return
  fi
  uname -m
}

normalize_arch() {
  case "$1" in
    arm64|aarch64)
      echo "arm64"
      ;;
    x86_64)
      echo "x86_64"
      ;;
    *)
      echo "unsupported arch: $1" >&2
      exit 1
      ;;
  esac
}

resolve_latest_version() {
  "${CURL_BIN}" -fsSL "${RELEASE_API_URL}" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1
}

resolve_version() {
  if [ -n "${version}" ]; then
    echo "${version}"
    return
  fi

  latest_version="$(resolve_latest_version)"
  if [ -z "${latest_version}" ]; then
    echo "failed to resolve latest version from ${RELEASE_API_URL}" >&2
    exit 1
  fi
  echo "${latest_version}"
}

build_archive_name() {
  resolved_version="$1"
  normalized_arch="$2"
  echo "wxpilot-${resolved_version}-darwin-${normalized_arch}.tar.gz"
}

build_download_url() {
  resolved_version="$1"
  archive_name="$2"
  echo "${DOWNLOAD_BASE_URL}/${resolved_version}/${archive_name}"
}

require_macos() {
  os_name="$(detect_os)"
  if [ "${os_name}" != "Darwin" ]; then
    echo "wxpilot install.sh currently supports macOS only (detected ${os_name})" >&2
    exit 1
  fi
}

install_archive() {
  archive_path="$1"

  mkdir -p "${CLI_INSTALL_DIR}" "${DAEMON_INSTALL_DIR}"

  extract_dir="$(mktemp -d "${TMP_DIR%/}/wxpilot-install.XXXXXX")"
  trap 'rm -rf "${extract_dir}" "${archive_path}"' EXIT INT TERM

  tar -xzf "${archive_path}" -C "${extract_dir}"

  for required in wxpilot wxp-daemon README-install.txt; do
    if [ ! -f "${extract_dir}/${required}" ]; then
      echo "archive missing required file: ${required}" >&2
      exit 1
    fi
  done

  install -m 0755 "${extract_dir}/wxpilot" "${CLI_INSTALL_DIR}/wxpilot"
  install -m 0755 "${extract_dir}/wxp-daemon" "${DAEMON_INSTALL_DIR}/wxp-daemon"

  echo "Installed wxpilot to ${CLI_INSTALL_DIR}/wxpilot"
  echo "Installed wxp-daemon to ${DAEMON_INSTALL_DIR}/wxp-daemon"
}

require_macos
resolved_arch="$(normalize_arch "$(detect_arch)")"
resolved_version="$(resolve_version)"
archive_name="$(build_archive_name "${resolved_version}" "${resolved_arch}")"
download_url="$(build_download_url "${resolved_version}" "${archive_name}")"
archive_path="$(mktemp "${TMP_DIR%/}/wxpilot-archive.XXXXXX.tar.gz")"

"${CURL_BIN}" -fsSL "${download_url}" -o "${archive_path}"
install_archive "${archive_path}"
