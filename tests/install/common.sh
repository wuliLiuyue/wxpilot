#!/bin/sh
set -eu

ROOT_DIR="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
FIXTURES_DIR="${ROOT_DIR}/tests/install/fixtures"

make_fake_release() {
  destination="$1"
  version="$2"
  mkdir -p "${destination}"

  cat > "${destination}/wxpilot" <<EOF
#!/bin/sh
echo "wxpilot ${version}"
EOF
  cat > "${destination}/wxp-daemon" <<EOF
#!/bin/sh
echo "wxp-daemon ${version}"
EOF
  cat > "${destination}/README-install.txt" <<EOF
wxpilot ${version}
EOF
  chmod +x "${destination}/wxpilot" "${destination}/wxp-daemon"
}
