#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
. "${ROOT_DIR}/tests/install/common.sh"

workspace="$(mktemp -d "${TMPDIR:-/tmp}/wxpilot-e2e-install-test.XXXXXX")"
cleanup() {
  rm -rf "${workspace}"
}
trap cleanup EXIT INT TERM

version="v0.1.0"
build_dir="${workspace}/build"
output_dir="${workspace}/output"
release_dir="${workspace}/releases/download/${version}"
home_dir="${workspace}/home"
mkdir -p "${release_dir}" "${home_dir}"

make_fake_release "${build_dir}" "${version}"

archive_path="$(
  env \
    BUILD_DIR="${build_dir}" \
    OUTPUT_DIR="${output_dir}" \
    VERSION="${version}" \
    TARGET_ARCH="arm64" \
    sh "${ROOT_DIR}/scripts/release/package-macos.sh"
)"

archive_name="$(basename "${archive_path}")"
cp "${archive_path}" "${release_dir}/${archive_name}"

cat > "${workspace}/latest.json" <<EOF
{"tag_name":"${version}"}
EOF

env \
  HOME="${home_dir}" \
  WXP_INSTALL_ROOT="${home_dir}" \
  WXP_INSTALL_OS="Darwin" \
  WXP_INSTALL_ARCH="arm64" \
  WXP_RELEASE_API_URL="file://${workspace}/latest.json" \
  WXP_RELEASE_DOWNLOAD_BASE_URL="file://${workspace}/releases/download" \
  sh "${ROOT_DIR}/install.sh"

test -x "${home_dir}/.local/bin/wxpilot"
test -x "${home_dir}/.wxpilot/bin/wxp-daemon"
grep -q "wxpilot ${version}" "${home_dir}/.local/bin/wxpilot"
