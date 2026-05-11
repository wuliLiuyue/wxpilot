# License Release Runbook

## Purpose

这份 Runbook 用于把授权发布链路从“构建发布包”推进到“服务器部署”和“真机回归”。

## 1. Prepare Release Variables

先生成授权材料：

```bash
make license-gen-materials KEY_ID=prod-20260511
make license-verify-materials
```

默认会输出到 `.wxpilot-license/`：

- `.wxpilot-license/server.env`
- `.wxpilot-license/client.env`
- `.wxpilot-license/manifest.env`

客户端 release 构建前必须准备：

- `WXP_LICENSE_SERVER_URL`
- `WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX`

服务端部署前必须准备：

- `WXP_LICENSE_PRIVATE_KEY_HEX`
- `WXP_LICENSE_KEY_ID`

## 2. Validate Release Build Preconditions

```bash
set -a
source .wxpilot-license/client.env
set +a
make release-env-test
```

## 3. Build And Install Client Binaries

```bash
make local-build
make local-install
```

## 4. Deploy License Server

按 [license-server-deploy.md](license-server-deploy.md) 完成：

1. `.env` 初始化
2. 将 `.wxpilot-license/server.env` 的内容写入 `.env`
3. `make license-compose-up`
4. `make license-compose-ps`
5. `curl -fsS http://127.0.0.1:8787/healthz`

## 5. Initialize Test Environment

```bash
make license-gen-key PREFIX=test COUNT=1 OUT_FILE=.wxpilot-license/license-keys.txt
make license-init-test-env KEY=$(head -n 1 .wxpilot-license/license-keys.txt)
```

这一步会：

1. 启动 `compose`
2. 检查 `/healthz`
3. 写入测试 license

## 6. Run Real-Device Regression

使用这份模板逐项留痕：

- [docs/license-real-device-regression-checklist.md](license-real-device-regression-checklist.md)

## 7. Backup Before Upgrade

按 [license-server-deploy.md](license-server-deploy.md) 的备份步骤执行：

```bash
make license-backup DB=./data/license.db
```

## 8. Upgrade

```bash
make license-compose-up
make license-compose-ps
```

## 9. Go-Live Checklist

上线前至少确认：

- [ ] 已执行 `make release-env-test`
- [ ] `WXP_LICENSE_SERVER_URL` 为正式服务地址
- [ ] `WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX` 与服务端私钥匹配
- [ ] 已执行 `make license-gen-materials KEY_ID=<prod-date>`
- [ ] 已执行 `make license-verify-materials`
- [ ] 已执行 `make local-build`
- [ ] 已执行 `make local-install`
- [ ] 已复制 `.env.example` 为 `.env`
- [ ] `.env` 中 `WXP_LICENSE_PRIVATE_KEY_HEX` 已填写
- [ ] `.env` 中 `WXP_LICENSE_KEY_ID` 已填写
- [ ] 已执行 `make license-compose-up`
- [ ] 已执行 `make license-compose-ps`
- [ ] `curl -fsS http://127.0.0.1:8787/healthz` 成功
- [ ] 已执行 `make license-backup DB=./data/license.db`
- [ ] 已使用 [docs/license-real-device-regression-checklist.md](license-real-device-regression-checklist.md) 跑完真机回归

## 10. Rollback

最小回滚路径：

```bash
make license-compose-down
# 恢复备份的 data/license.db 与 .env
# 如需回滚客户端，同时恢复旧版 client.env 后重新构建发布包
make license-compose-up
make license-compose-ps
```
