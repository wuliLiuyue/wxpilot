# License Server Deploy Guide

## Overview

当前文档覆盖两条部署路径：

- 本地 / 预发：`docker compose + Dockerfile` 本地构建
- 生产：构建机基于 `Dockerfile` 构建镜像，推送镜像仓库后，由阿里云 ECS 宝塔面板直接拉镜像部署

共同约束：

- 单个 `wxp-license-server` 容器
- SQLite 文件通过 volume 持久化
- 通过 `GET /healthz` 提供基础健康检查

## Build-Side Files

- `Dockerfile`
- `compose.yaml`
- `compose.prod.yaml`
- `.env.example`
- `.env`

从模板复制：

```bash
cp .env.example .env
make license-gen-materials KEY_ID=staging-20260511
```

然后将 `.wxpilot-license/server.env` 中的值写入 `.env`。

各文件定位：

- `compose.yaml`：本地 / 预发使用，基于仓库源码和 `Dockerfile` 本地构建
- `compose.prod.yaml`：生产配置对照模板，便于在构建侧核对镜像、端口、环境变量和 volume；宝塔面板直拉镜像时默认不直接执行它
- `.env.example`：`compose.prod.yaml` 或本地 compose 的变量模板，不等于直接可上线的生产配置

## Required Environment Variables

- `WXP_LICENSE_PRIVATE_KEY_HEX`
- `WXP_LICENSE_KEY_ID`

推荐先做一致性校验：

```bash
make license-verify-materials
```

## Build-Side Steps

### 1. Generate Production License Materials

生产构建目录中生成正式材料：

```bash
make license-gen-materials KEY_ID=prod-main SERVER_URL=https://license.example.com
make license-verify-materials
```

确认：

```bash
cat .wxpilot-license/client.env
cat .wxpilot-license/server.env
cat .wxpilot-license/manifest.env
```

必须确认 `WXP_LICENSE_SERVER_URL` 为正式地址，不是 `127.0.0.1`。

### 2. Build Production Client Binaries

```bash
set -a
source .wxpilot-license/client.env
set +a

make release-env-test
make local-build
```

如需在构建机安装验证：

```bash
make local-install
```

### 3. Build And Push License Server Image

登录阿里云镜像仓库：

```bash
docker login --username=<阿里云账号> registry.cn-<region>.aliyuncs.com
```

构建镜像：

```bash
docker build -t registry.cn-<region>.aliyuncs.com/<namespace>/wxp-license-server:20260518 .
```

可选追加稳定标签：

```bash
docker tag registry.cn-<region>.aliyuncs.com/<namespace>/wxp-license-server:20260518 \
  registry.cn-<region>.aliyuncs.com/<namespace>/wxp-license-server:latest
```

推送镜像：

```bash
docker push registry.cn-<region>.aliyuncs.com/<namespace>/wxp-license-server:20260518
docker push registry.cn-<region>.aliyuncs.com/<namespace>/wxp-license-server:latest
```

生产部署建议优先使用明确版本 tag，不要只依赖 `latest`。

如果需要把生产容器配置交接给运维或做上线前复核，可额外执行：

```bash
cp .env.example .env
# 将 .wxpilot-license/server.env 中的值写入 .env
```

再结合 `compose.prod.yaml` 一起检查以下配置是否齐全：

- `WXP_LICENSE_IMAGE`
- `WXP_LICENSE_PRIVATE_KEY_HEX`
- `WXP_LICENSE_KEY_ID`
- `8787:8787`
- `/data` volume 挂载

## Local / Staging Start Service

```bash
make license-compose-up
make license-compose-logs
make license-compose-ps
```

或者：

```bash
docker compose up -d --build
docker compose logs -f wxp-license-server
```

## Production Deployment In Bt Panel

生产环境不再推荐在 ECS 上执行 `docker compose up -d --build`，而是通过宝塔面板直接拉镜像部署。

### Container Settings

- 镜像：`registry.cn-<region>.aliyuncs.com/<namespace>/wxp-license-server:<tag>`
- 容器名：`wxp-license-server`
- 重启策略：`unless-stopped` 或 `always`
- 端口映射：`8787:8787`

### Environment Variables

从 `.wxpilot-license/server.env` 取值，并补齐以下变量：

- `WXP_LICENSE_SERVER_ADDR=0.0.0.0:8787`
- `WXP_LICENSE_DB=/data/license.db`
- `WXP_LICENSE_PRIVATE_KEY_HEX=<server.env 中的值>`
- `WXP_LICENSE_KEY_ID=<server.env 中的值>`

如果宝塔面板通过域名或反向代理对外暴露服务，必须同时确认：

- `.wxpilot-license/client.env` 中的 `WXP_LICENSE_SERVER_URL` 已指向最终对外地址
- 反向代理、证书、ACL / 安全组和端口放行规则与该地址一致

### Volume Mapping

必须在宝塔面板中为容器配置宿主机目录到 `/data` 的挂载。

- 宿主机目录：由宝塔面板管理的持久化目录
- 容器目录：`/data`

不要省略 volume，不要使用匿名临时卷；否则 `license.db` 会随容器销毁而丢失。

## Health Check

服务健康检查地址：

```bash
curl -fsS http://127.0.0.1:8787/healthz
```

预期返回：

```json
{"ok":true,"databasePath":"/data/license.db"}
```

生产环境中，宝塔容器启动后先检查日志，再执行以下两类检查：

1. 宿主机本机检查：

```bash
curl -fsS http://127.0.0.1:8787/healthz
```

2. 与客户端配置一致的正式地址检查：

```bash
curl -fsS https://license.example.com/healthz
```

第二步不能省略。客户端实际激活走的是 `WXP_LICENSE_SERVER_URL`，如果域名、反向代理、证书或公网访问控制配置错误，本机 `127.0.0.1` 检查仍然可能通过。

## Seed Test License

本地 / 预发可直接使用：

```bash
make license-gen-key PREFIX=test COUNT=1 OUT_FILE=.wxpilot-license/license-keys.txt
make license-seed DB=./data/license.db KEY=$(head -n 1 .wxpilot-license/license-keys.txt)
```

也可直接运行脚本：

```bash
scripts/license/seed-sqlite-license.sh ./data/license.db lic-e2e-001
```

生产环境建议使用单独的 `prod-smoke` key 做冒烟：

```bash
make license-gen-key PREFIX=prod-smoke COUNT=1 OUT_FILE=.wxpilot-license/prod-smoke.txt
cat .wxpilot-license/prod-smoke.txt
```

如果生产机没有仓库脚本环境，推荐直接通过容器内 `sqlite3` 写入：

```bash
# 下面示例默认在 Linux 宿主机执行；若在 macOS 构建机操作，请改用 `shasum -a 256`
LICENSE_KEY="这里替换成 prod-smoke key"
LICENSE_HASH=$(printf '%s' "$LICENSE_KEY" | sha256sum | awk '{print $1}')
CREATED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)

docker exec wxp-license-server sqlite3 /data/license.db "
CREATE TABLE IF NOT EXISTS licenses (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  key_hash TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'active',
  created_at TEXT NOT NULL
);
INSERT OR IGNORE INTO licenses (key_hash, status, created_at)
VALUES ('$LICENSE_HASH', 'active', '$CREATED_AT');
"
```

## Backup

当前使用 SQLite，至少要备份以下内容：

- `./data/license.db`
- 环境变量记录

建议最小备份方式：

```bash
make license-backup DB=./data/license.db
```

上面的 `make license-backup` 只适用于“仓库目录 + `.env` 文件”管理容器的场景。

也可直接运行脚本：

```bash
scripts/license/backup-sqlite-license.sh ./data/license.db ./.env ./backups
```

如果生产环境通过宝塔面板部署，仍然需要定期备份：

- volume 对应宿主机目录中的 `license.db`
- 宝塔容器环境变量来源记录

建议至少保留以下其中一种：

- 宝塔容器环境变量导出文本
- 宝塔面板配置截图
- 与当前线上一致的受控 `.env` 文件副本

## Upgrade

本地 / 预发继续使用：

```bash
make license-gen-materials KEY_ID=prod-20260511
make license-verify-materials
docker compose up -d --build
docker compose ps
```

升级前先执行一次备份。

生产环境升级路径改为：

1. 在生产构建目录复用现有 `.wxpilot-license/`
2. 重新构建客户端
3. 重新构建并推送新的服务镜像 tag
4. 在宝塔面板中将容器镜像版本切换到新 tag
5. 保持原环境变量和 `/data` 挂载不变
6. 重建或重启容器
7. 再次执行 `healthz` 和一轮 `prod-smoke` 激活验证

## Rotation Notes

- 客户端当前只信任一个内嵌公钥
- 服务端私钥轮换后，必须同步更新客户端构建使用的 `.wxpilot-license/client.env`
- 完成轮换后重新执行 `make local-build`

如果生产公私钥不需要频繁更换，推荐长期复用固定的生产 `.wxpilot-license/`，后续版本只更新客户端产物和镜像 tag。
