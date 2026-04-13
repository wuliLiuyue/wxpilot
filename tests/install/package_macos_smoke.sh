#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
. "${ROOT_DIR}/tests/install/common.sh"

workspace="$(mktemp -d "${TMPDIR:-/tmp}/wxpilot-package-test.XXXXXX")"
cleanup() {
  rm -rf "${workspace}"
}
trap cleanup EXIT INT TERM

build_dir="${workspace}/build"
output_dir="${workspace}/output"
extract_dir="${workspace}/extract"
make_fake_release "${build_dir}" "v0.1.0"
mkdir -p "${extract_dir}"

archive_path="$(
  env \
    BUILD_DIR="${build_dir}" \
    OUTPUT_DIR="${output_dir}" \
    VERSION="v0.1.0" \
    TARGET_ARCH="arm64" \
    sh "${ROOT_DIR}/scripts/release/package-macos.sh"
)"

test -f "${archive_path}"
tar -xzf "${archive_path}" -C "${extract_dir}"

for required in wxpilot wxp-daemon README-install.txt; do
  test -f "${extract_dir}/${required}"
done

if find "${extract_dir}" -mindepth 2 -type f | grep -q .; then
  echo "archive should not contain nested top-level directories" >&2
  exit 1
fi
