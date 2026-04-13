#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
BUILD_DIR="${BUILD_DIR:-${ROOT_DIR}/rust/target/release}"
OUTPUT_DIR="${OUTPUT_DIR:-${ROOT_DIR}/dist/release}"
VERSION="${VERSION:-}"
TARGET_OS="${TARGET_OS:-darwin}"
TARGET_ARCH="${TARGET_ARCH:-$(uname -m)}"

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

resolve_version() {
  if [ -n "${VERSION}" ]; then
    echo "${VERSION}"
    return
  fi

  sed -n 's/^version = "\(.*\)"/v\1/p' "${ROOT_DIR}/rust/crates/wxp-cli/Cargo.toml" | head -n 1
}

resolved_version="$(resolve_version)"
resolved_arch="$(normalize_arch "${TARGET_ARCH}")"
archive_name="wxpilot-${resolved_version}-${TARGET_OS}-${resolved_arch}.tar.gz"
stage_dir="$(mktemp -d "${TMPDIR:-/tmp}/wxpilot-package.XXXXXX")"
archive_root="${stage_dir}/archive"

cleanup() {
  rm -rf "${stage_dir}"
}
trap cleanup EXIT INT TERM

mkdir -p "${archive_root}" "${OUTPUT_DIR}"

for binary in wxpilot wxp-daemon; do
  if [ ! -f "${BUILD_DIR}/${binary}" ]; then
    echo "missing build artifact: ${BUILD_DIR}/${binary}" >&2
    exit 1
  fi
  install -m 0755 "${BUILD_DIR}/${binary}" "${archive_root}/${binary}"
done

cat > "${archive_root}/README-install.txt" <<EOF
wxpilot ${resolved_version}

Install:
  curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | sh

Installed paths:
  wxpilot    -> ~/.local/bin/wxpilot
  wxp-daemon -> ~/.wxpilot/bin/wxp-daemon
EOF

tar -czf "${OUTPUT_DIR}/${archive_name}" -C "${archive_root}" .
echo "${OUTPUT_DIR}/${archive_name}"
