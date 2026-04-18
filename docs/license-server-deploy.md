# License Server Deploy Guide

## Overview

当前推荐的最小部署形态：

- `docker compose`
- 单个 `wxp-license-server` 容器
- SQLite 文件通过 volume 持久化
- 通过 `GET /healthz` 提供基础健康检查

## Required Files

- `Dockerfile`
- `compose.yaml`
- `.env`

从模板复制：

```bash
cp .env.example .env
```

## Required Environment Variables

- `WXP_LICENSE_PRIVATE_KEY_HEX`
- `WXP_LICENSE_KEY_ID`

## Start Service

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

## Health Check

服务健康检查地址：

```bash
curl -fsS http://127.0.0.1:8787/healthz
```

预期返回：

```json
{"ok":true,"databasePath":"/data/license.db"}
```

## Seed Test License

```bash
make license-seed DB=./data/license.db KEY=lic-e2e-001
```

也可直接运行脚本：

```bash
scripts/license/seed-sqlite-license.sh ./data/license.db lic-e2e-001
```

## Backup

当前使用 SQLite，至少要备份以下内容：

- `./data/license.db`
- `.env`

建议最小备份方式：

```bash
make license-backup DB=./data/license.db
```

也可直接运行脚本：

```bash
scripts/license/backup-sqlite-license.sh ./data/license.db ./.env ./backups
```

## Upgrade

```bash
docker compose up -d --build
docker compose ps
```

升级前先执行一次备份。
