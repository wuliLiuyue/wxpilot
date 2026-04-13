#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
. "${ROOT_DIR}/tests/install/common.sh"

workspace="$(mktemp -d "${TMPDIR:-/tmp}/wxpilot-install-test.XXXXXX")"
unsupported_log="${workspace}/unsupported.log"
cleanup() {
  rm -rf "${workspace}"
}
trap cleanup EXIT INT TERM

version="v0.1.0"
archive_dir="${workspace}/archive"
release_dir="${workspace}/releases/download/${version}"
mkdir -p "${archive_dir}" "${release_dir}"
make_fake_release "${archive_dir}" "${version}"
tar -czf "${release_dir}/wxpilot-${version}-darwin-arm64.tar.gz" -C "${archive_dir}" .
cat > "${workspace}/latest.json" <<EOF
{"tag_name":"${version}"}
EOF

home_latest="${workspace}/home-latest"
mkdir -p "${home_latest}"
env \
  HOME="${home_latest}" \
  WXP_INSTALL_ROOT="${home_latest}" \
  WXP_INSTALL_OS="Darwin" \
  WXP_INSTALL_ARCH="arm64" \
  WXP_RELEASE_API_URL="file://${workspace}/latest.json" \
  WXP_RELEASE_DOWNLOAD_BASE_URL="file://${workspace}/releases/download" \
  sh "${ROOT_DIR}/install.sh"

test -x "${home_latest}/.local/bin/wxpilot"
test -x "${home_latest}/.wxpilot/bin/wxp-daemon"
grep -q "wxpilot ${version}" "${home_latest}/.local/bin/wxpilot"

home_version="${workspace}/home-version"
mkdir -p "${home_version}"
env \
  HOME="${home_version}" \
  WXP_INSTALL_ROOT="${home_version}" \
  WXP_INSTALL_OS="Darwin" \
  WXP_INSTALL_ARCH="arm64" \
  WXP_RELEASE_DOWNLOAD_BASE_URL="file://${workspace}/releases/download" \
  sh "${ROOT_DIR}/install.sh" --version "${version}"

test -x "${home_version}/.local/bin/wxpilot"
test -x "${home_version}/.wxpilot/bin/wxp-daemon"

if env \
  HOME="${workspace}/home-unsupported" \
  WXP_INSTALL_ROOT="${workspace}/home-unsupported" \
  WXP_INSTALL_OS="Linux" \
  WXP_INSTALL_ARCH="arm64" \
  WXP_RELEASE_API_URL="file://${workspace}/latest.json" \
  WXP_RELEASE_DOWNLOAD_BASE_URL="file://${workspace}/releases/download" \
  sh "${ROOT_DIR}/install.sh" >"${unsupported_log}" 2>&1; then
  echo "expected install.sh to reject unsupported platform" >&2
  exit 1
fi

grep -q "supports macOS only" "${unsupported_log}"

invalid_latest_log="${workspace}/invalid-latest.log"
cat > "${workspace}/latest-invalid.json" <<'EOF'
{}
EOF

if env \
  HOME="${workspace}/home-invalid-latest" \
  WXP_INSTALL_ROOT="${workspace}/home-invalid-latest" \
  WXP_INSTALL_OS="Darwin" \
  WXP_INSTALL_ARCH="arm64" \
  WXP_RELEASE_API_URL="file://${workspace}/latest-invalid.json" \
  WXP_RELEASE_DOWNLOAD_BASE_URL="file://${workspace}/releases/download" \
  sh "${ROOT_DIR}/install.sh" >"${invalid_latest_log}" 2>&1; then
  echo "expected install.sh to fail when latest metadata is invalid" >&2
  exit 1
fi

grep -q "failed to resolve latest version" "${invalid_latest_log}"
