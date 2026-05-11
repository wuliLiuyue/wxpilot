#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
  echo "Usage: gen-license-materials.sh <out-dir> [key-id] [server-url]" >&2
  exit 1
fi

out_dir="$1"
key_id="${2:-}"
server_url="${3:-http://127.0.0.1:8787}"

if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl is required" >&2
  exit 1
fi

if ! command -v xxd >/dev/null 2>&1; then
  echo "xxd is required" >&2
  exit 1
fi

if [[ -z "$key_id" ]]; then
  key_id="dev-$(date +%Y%m%d)"
fi

server_env="$out_dir/server.env"
client_env="$out_dir/client.env"
manifest_file="$out_dir/manifest.env"

for path in "$server_env" "$client_env" "$manifest_file"; do
  if [[ -e "$path" ]]; then
    echo "refusing to overwrite existing file: $path" >&2
    exit 1
  fi
done

mkdir -p "$out_dir"

private_der_hex="$(openssl genpkey -algorithm ED25519 -outform DER | xxd -p -c 256 | tr -d '\n')"
private_key_hex="${private_der_hex:32:64}"

if [[ ${#private_key_hex} -ne 64 ]]; then
  echo "failed to derive 32-byte private key from openssl output" >&2
  exit 1
fi

public_der_hex="$(
  openssl pkey \
    -inform DER \
    -in <(printf '%s' "$private_der_hex" | xxd -r -p) \
    -pubout \
    -outform DER \
    2>/dev/null \
    | xxd -p -c 256 \
    | tr -d '\n'
)"
public_key_hex="${public_der_hex:24:64}"

if [[ ${#public_key_hex} -ne 64 ]]; then
  echo "failed to derive 32-byte public key from openssl output" >&2
  exit 1
fi

created_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

cat > "$server_env" <<EOF
WXP_LICENSE_PRIVATE_KEY_HEX=$private_key_hex
WXP_LICENSE_KEY_ID=$key_id
EOF

cat > "$client_env" <<EOF
WXP_LICENSE_SERVER_URL=$server_url
WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX=$public_key_hex
EOF

cat > "$manifest_file" <<EOF
WXP_LICENSE_KEY_ID=$key_id
WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX=$public_key_hex
WXP_LICENSE_CREATED_AT=$created_at
EOF

echo "generated license materials"
echo "server env: $server_env"
echo "client env: $client_env"
echo "manifest: $manifest_file"
echo "key id: $key_id"
