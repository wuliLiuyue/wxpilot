# License Release Runbook

## Purpose

这份 Runbook 用于把授权发布链路从“构建发布包”推进到“服务器部署”和“真机回归”。

## 1. Prepare Release Variables

先生成授权材料：

```bash
make license-gen-materials KEY_ID=prod-main SERVER_URL=https://feai.work/wxpilot-api
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

如果当前步骤只是在生产构建机上产出正式二进制，可只执行 `make local-build`；`make local-install` 仅用于本机验证。

## 4. Deploy License Server

按 [license-server-deploy.md](license-server-deploy.md) 完成：

1. 在生产构建目录基于 `Dockerfile` 构建 `wxp-license-server` 镜像
2. 推送镜像到阿里云镜像仓库
3. 在阿里云 ECS 宝塔面板中新建或更新容器
4. 将 `.wxpilot-license/server.env` 中的值写入宝塔容器环境变量，并补齐 `WXP_LICENSE_SERVER_ADDR=0.0.0.0:8787`、`WXP_LICENSE_DB=/data/license.db`
5. 配置 `8787:8787` 端口映射
6. 配置宿主机目录到容器 `/data` 的 volume 挂载
7. 启动容器
8. `curl -fsS http://127.0.0.1:8787/healthz`
9. `curl -fsS <prod-healthz-url>`

## 5. Initialize Test Environment

```bash
make license-gen-key PREFIX=prod-smoke COUNT=1 OUT_FILE=.wxpilot-license/prod-smoke.txt
```

然后将 `prod-smoke` key 写入生产数据库，并用正式构建的客户端执行一轮激活冒烟：

1. 激活前：`wxpilot status` 被拦截
2. `wxpilot activate <prod-smoke-key>` 成功
3. 激活后：`wxpilot status` 放行

## 6. Run Real-Device Regression

生产冒烟通过后，再决定是否需要使用这份模板做完整留痕：

- [docs/license-real-device-regression-checklist.md](license-real-device-regression-checklist.md)

## 7. Backup Before Upgrade

按 [license-server-deploy.md](license-server-deploy.md) 的备份步骤执行：

至少备份：

- 生产 volume 对应宿主机目录中的 `license.db`
- 当前宝塔容器环境变量记录

## 8. Upgrade

1. 在生产构建目录复用现有 `.wxpilot-license/`
2. 重新构建正式客户端
3. 构建并推送新的服务镜像 tag
4. 在宝塔面板中将容器切换到新 tag
5. 保持原环境变量和 `/data` 挂载不变
6. 启动新容器并执行 `healthz`
7. 再跑一轮 `prod-smoke` 激活冒烟

## 9. Go-Live Checklist

上线前至少确认：

- [ ] 已执行 `make release-env-test`
- [ ] `WXP_LICENSE_SERVER_URL` 为正式服务地址
- [ ] `WXP_LICENSE_EMBEDDED_PUBLIC_KEY_HEX` 与服务端私钥匹配
- [ ] 已执行 `make license-gen-materials KEY_ID=prod-main SERVER_URL=<prod-url>`
- [ ] 已执行 `make license-verify-materials`
- [ ] 已执行 `make local-build`
- [ ] 已构建并推送生产 `wxp-license-server` 镜像
- [ ] 宝塔容器环境变量已填写 `WXP_LICENSE_PRIVATE_KEY_HEX`
- [ ] 宝塔容器环境变量已填写 `WXP_LICENSE_KEY_ID`
- [ ] 宝塔容器环境变量已填写 `WXP_LICENSE_SERVER_ADDR=0.0.0.0:8787`
- [ ] 宝塔容器环境变量已填写 `WXP_LICENSE_DB=/data/license.db`
- [ ] 宝塔容器已配置 `/data` volume 挂载
- [ ] 宝塔容器已启动成功
- [ ] `curl -fsS http://127.0.0.1:8787/healthz` 成功
- [ ] `curl -fsS <prod-healthz-url>` 成功
- [ ] 已完成一轮 `prod-smoke` 激活验证
- [ ] 已备份生产数据库与环境变量记录
- [ ] 如需完整回归，已使用 [docs/license-real-device-regression-checklist.md](license-real-device-regression-checklist.md) 跑完真机回归

## 10. Rollback

最小回滚路径：

1. 在宝塔面板中将容器镜像 tag 切回旧版本
2. 保持原有 `/data` volume 挂载
3. 如需回滚环境变量，同时恢复旧配置
4. 重启容器
5. 如需回滚客户端，同时恢复旧版 `client.env` 后重新构建发布包
