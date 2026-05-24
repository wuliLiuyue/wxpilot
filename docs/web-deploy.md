# 官网部署指南

## 目标

wxpilot 官网是纯静态站点，生产部署推荐采用：

- Docker 镜像构建静态资源
- Docker Compose 负责容器编排
- 宝塔面板只负责域名、HTTPS 和反向代理

## 文件

- `packages/web/Dockerfile`
- `packages/web/nginx.conf`
- `compose.web.yaml`
- `compose.web.prod.yaml`
- `.env.web.example`

## 本地构建

```bash
docker compose -f compose.web.yaml up -d --build
curl -fsS http://127.0.0.1:8080/
```

## 生产构建

```bash
docker build -t registry.cn-<region>.aliyuncs.com/<namespace>/wxpilot-web:<tag> -f packages/web/Dockerfile .
docker push registry.cn-<region>.aliyuncs.com/<namespace>/wxpilot-web:<tag>
```

## ECS + 宝塔

1. 在 ECS 上安装 Docker 和 Docker Compose。
2. 如果镜像仓库是私有的，先执行 `docker login`。
3. 复制 `.env.web.example` 为 `.env.web`，并填写 `WXP_WEB_IMAGE=registry.cn-<region>.aliyuncs.com/<namespace>/wxpilot-web:<tag>`。
4. 执行 `docker compose --env-file .env.web -f compose.web.prod.yaml pull && docker compose --env-file .env.web -f compose.web.prod.yaml up -d`。
5. 在宝塔面板里新增网站，反向代理到 `127.0.0.1:8080`。
6. 在宝塔里申请并绑定 HTTPS 证书。
7. 安全组只放行 80/443，`8080` 保持本机监听即可。

## 健康检查

```bash
curl -fsS http://127.0.0.1:8080/
```

## 回滚

把 `WXP_WEB_IMAGE` 切回旧 tag，然后重启容器即可。
