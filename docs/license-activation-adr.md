# License Activation ADR

## Summary

- 发布版仅放行 `--help`、`--version`、`activate`
- `wxpilot activate <key>` 联网调用激活服务
- 服务端首绑机器后返回签名证书
- CLI 将证书保存到 `~/.wxpilot/license.json`
- 后续所有受限命令仅做本地验签和机器指纹比对

## Certificate Shape

```json
{
  "certificate": {
    "payload": {
      "version": 1,
      "licenseId": "lic_1",
      "keyId": "dev",
      "machineFingerprint": "<sha256>",
      "issuedAt": "2026-04-13T00:00:00Z",
      "clientVersion": "0.1.0"
    },
    "signature": "<hex>"
  }
}
```

## Backend Contract

- `POST /v1/activate`
- request:
  - `licenseKey`
  - `machineFingerprint`
  - `clientVersion`
- success:
  - `certificate`
- errors:
  - `invalid_license_key`
  - `inactive_license`
  - `already_bound`
  - `invalid_request`
  - `internal_error`
