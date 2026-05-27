# License Real-Device Regression Checklist

用于测试机 / 预发环境的手工回归记录模板。

## Metadata

- Date:
- Tester:
- Machine:
- wxpilot build:
- License server URL:
- Test project path:
- Test license key:

## Preflight

- [x] 已执行 `make license-gen-materials KEY_ID=<env-date>`
- [x] 已执行 `make license-verify-materials`
- [x] 已设置 `WXP_LICENSE_SERVER_URL`
- [x] 已设置 `WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX`
- [x] 已执行 `make local-build`
- [x] 已执行 `make local-install`
- [x] 已执行 `make license-compose-up`
- [x] `make license-compose-ps` 显示服务正常
- [x] `curl -fsS http://127.0.0.1:8787/healthz` 成功
- [x] 已执行 `make license-seed DB=./data/license.db KEY=<key>`

## Gate Before Activation

- [x] `wxpilot --help` 成功
- [x] `wxpilot --version` 成功
- [x] `wxpilot status` 被拦截
- [x] `wxpilot start <project>` 被拦截
- [x] `wxpilot connect` 被拦截
- [x] `wxpilot view` 被拦截

## Activation

- [x] `wxpilot activate <key>` 成功
- [x] `~/.wxpilot/license.json` 已生成
- [x] `wxpilot status` 不再被授权门禁拦截

## Command Regression After Activation

- [x] `wxpilot start <project>`
- [x] `wxpilot connect`
- [x] `wxpilot disconnect`
- [x] `wxpilot status`
- [x] `wxpilot goto <route>`
- [x] `wxpilot back`
- [x] `wxpilot reload`
- [x] `wxpilot view`
- [x] `wxpilot find <query>`
- [x] `wxpilot wait %N`
- [x] `wxpilot tap %N`
- [x] `wxpilot type %N <text>`
- [x] `wxpilot scroll %N <up|down>`
- [x] `wxpilot read %N`
- [x] `wxpilot assert %N <expected>`
- [x] `wxpilot state`
- [x] `wxpilot shot`
- [x] `wxpilot run <js>`
- [x] `wxpilot wx <method>`
- [x] `wxpilot net start`
- [x] `wxpilot net stop`
- [x] `wxpilot net log`
- [x] `wxpilot net mock`
- [x] `wxpilot net unmock`
- [x] `wxpilot net clear`
- [x] `wxpilot net install-ca`
- [x] `wxpilot daemon stop`
- [x] `wxpilot daemon restart`

## License Failure Regression

- [x] 篡改 `~/.wxpilot/license.json` 后，受限命令报“本地激活凭证无效”
- [x] 删除 `~/.wxpilot/license.json` 后，受限命令报“当前发布版需要先激活”
- [x] 第二台机器使用同一 key 激活时被拒绝

## Notes

- Issue:
- Command:
- Output:
- Screenshot / Log Path:
