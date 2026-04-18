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

- [ ] 已设置 `WXP_LICENSE_SERVER_URL`
- [ ] 已设置 `WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX`
- [ ] 已执行 `make local-build`
- [ ] 已执行 `make local-install`
- [ ] 已执行 `make license-compose-up`
- [ ] `make license-compose-ps` 显示服务正常
- [ ] `curl -fsS http://127.0.0.1:8787/healthz` 成功
- [ ] 已执行 `make license-seed DB=./data/license.db KEY=<key>`

## Gate Before Activation

- [ ] `wxpilot --help` 成功
- [ ] `wxpilot --version` 成功
- [ ] `wxpilot status` 被拦截
- [ ] `wxpilot start <project>` 被拦截
- [ ] `wxpilot connect` 被拦截
- [ ] `wxpilot view` 被拦截

## Activation

- [ ] `wxpilot activate <key>` 成功
- [ ] `~/.wxpilot/license.json` 已生成
- [ ] `wxpilot status` 不再被授权门禁拦截

## Command Regression After Activation

- [ ] `wxpilot start <project>`
- [ ] `wxpilot connect`
- [ ] `wxpilot disconnect`
- [ ] `wxpilot status`
- [ ] `wxpilot goto <route>`
- [ ] `wxpilot back`
- [ ] `wxpilot reload`
- [ ] `wxpilot view`
- [ ] `wxpilot find <query>`
- [ ] `wxpilot wait %N`
- [ ] `wxpilot tap %N`
- [ ] `wxpilot type %N <text>`
- [ ] `wxpilot scroll %N <up|down>`
- [ ] `wxpilot read %N`
- [ ] `wxpilot assert %N <expected>`
- [ ] `wxpilot state`
- [ ] `wxpilot shot`
- [ ] `wxpilot run <js>`
- [ ] `wxpilot wx <method>`
- [ ] `wxpilot net start`
- [ ] `wxpilot net stop`
- [ ] `wxpilot net log`
- [ ] `wxpilot net mock`
- [ ] `wxpilot net unmock`
- [ ] `wxpilot net clear`
- [ ] `wxpilot net install-ca`
- [ ] `wxpilot daemon stop`
- [ ] `wxpilot daemon restart`

## License Failure Regression

- [ ] 篡改 `~/.wxpilot/license.json` 后，受限命令报“本地激活凭证无效”
- [ ] 删除 `~/.wxpilot/license.json` 后，受限命令报“当前发布版需要先激活”
- [ ] 第二台机器使用同一 key 激活时被拒绝

## Notes

- Issue:
- Command:
- Output:
- Screenshot / Log Path:
