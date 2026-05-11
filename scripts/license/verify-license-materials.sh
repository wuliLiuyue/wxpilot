#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "Usage: verify-license-materials.sh <server-env> <client-env> [key-file]" >&2
  exit 1
fi

server_env="$1"
client_env="$2"
key_file="${3:-}"

for path in "$server_env" "$client_env"; do
  if [[ ! -f "$path" ]]; then
    echo "missing env file: $path" >&2
    exit 1
  fi
done

if ! command -v openssl >/dev/null 2>&1; then
  echo "openssl is required" >&2
  exit 1
fi

if ! command -v xxd >/dev/null 2>&1; then
  echo "xxd is required" >&2
  exit 1
fi

load_env_var() {
  local file="$1"
  local key="$2"
  local value
  value="$(awk -F= -v key="$key" '$1 == key { print substr($0, index($0, "=") + 1) }' "$file" | tail -n 1)"
  if [[ -z "$value" ]]; then
    echo "missing $key in $file" >&2
    exit 1
  fi
  printf '%s' "$value"
}

private_key_hex="$(load_env_var "$server_env" WXP_LICENSE_PRIVATE_KEY_HEX)"
key_id="$(load_env_var "$server_env" WXP_LICENSE_KEY_ID)"
public_key_hex="$(load_env_var "$client_env" WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX)"

if ! [[ "$private_key_hex" =~ ^[0-9a-f]{64}$ ]]; then
  echo "invalid WXP_LICENSE_PRIVATE_KEY_HEX: expected 32-byte lowercase hex" >&2
  exit 1
fi

if ! [[ "$public_key_hex" =~ ^[0-9a-f]{64}$ ]]; then
  echo "invalid WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX: expected 32-byte lowercase hex" >&2
  exit 1
fi

if ! [[ "$key_id" =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "invalid WXP_LICENSE_KEY_ID: expected [A-Za-z0-9._-]+" >&2
  exit 1
fi

private_der_hex="302e020100300506032b657004220420$private_key_hex"
derived_public_hex="$(
  openssl pkey \
    -inform DER \
    -in <(printf '%s' "$private_der_hex" | xxd -r -p) \
    -pubout \
    -outform DER \
    2>/dev/null \
    | xxd -p -c 256 \
    | tr -d '\n'
)"
derived_public_hex="${derived_public_hex:24:64}"

if [[ "$derived_public_hex" != "$public_key_hex" ]]; then
  echo "public key does not match private key" >&2
  exit 1
fi

if [[ -n "$key_file" ]]; then
  if [[ ! -f "$key_file" ]]; then
    echo "missing key file: $key_file" >&2
    exit 1
  fi
  while IFS= read -r line || [[ -n "$line" ]]; do
    [[ -z "$line" ]] && continue
    if ! [[ "$line" =~ ^wxp-[a-z0-9-]+-[0-9]{8}-[A-F0-9]{10}$ ]]; then
      echo "invalid license key format: $line" >&2
      exit 1
    fi
  done < "$key_file"
fi

echo "license materials verified"
echo "key id: $key_id"
echo "public key: $public_key_hex"
