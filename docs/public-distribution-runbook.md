# wxpilot Public Distribution Runbook

本 runbook 用于把当前私有仓库里的发布产物整理成对外分发形态：公开仓库只保留安装、用法、skill 和 release 资产，不暴露实现细节。

## 目标形态

公开仓库建议只包含以下内容：

- `README.md`
- `install.sh`
- `skill/SKILL.md`
- GitHub Releases 里的二进制压缩包和校验文件

不要同步以下内容到公开仓库：

- `rust/`
- `packages/`
- `docs/` 中的实现、部署、授权或运维细节
- 内部脚本、测试和构建逻辑

## Release 资产约定

每个公开 tag 对应三份资产：

- `wxpilot-darwin-arm64.tar.gz`
- `wxpilot-darwin-x64.tar.gz`
- `wxpilot-checksums.txt`

推荐 tag 格式：

- `v0.1.0`
- `v0.1.1`
- `v0.2.0`

同一个版本号应同时用于：

- 私有仓库构建记录
- 公开仓库 release tag
- README 中的安装示例版本

## 发布步骤

1. 在私有仓库构建发布版二进制。

```bash
make local-build
```

2. 生成公开 release 资产。

```bash
bash scripts/release/package-public-release.sh --version v0.1.0
```

默认输出目录：

- `dist/public-release/v0.1.0/`

3. 渲染公开仓库素材。

```bash
bash scripts/release/render-public-repo.sh --repo your-org/wxpilot --version v0.1.0
```

默认输出目录：

- `dist/public-repo/`

4. 将 `dist/public-repo/` 中的文件同步到公开仓库根目录。

5. 在公开仓库创建同名 tag 和 GitHub Release，并上传：

- `dist/public-release/v0.1.0/wxpilot-darwin-arm64.tar.gz`
- `dist/public-release/v0.1.0/wxpilot-darwin-x64.tar.gz`
- `dist/public-release/v0.1.0/wxpilot-checksums.txt`

## 对外说明边界

公开 README 只建议包含：

- 它是什么
- 支持的平台
- `curl` 安装方式
- 激活命令
- 常用命令示例
- skill 安装方式
- releases 页面入口

公开 README 不建议包含：

- Rust / TypeScript / Daemon 等实现细节
- 授权服务部署方式
- 内部环境变量
- 构建命令
- 测试、迁移或兼容层说明

## 发布后验证

至少验证以下内容：

1. `curl -fsSL https://raw.githubusercontent.com/<repo>/main/install.sh | bash` 可以安装成功
2. `wxpilot --version` 可执行
3. `wxpilot activate <license-key>` 可进入激活流程
4. `https://raw.githubusercontent.com/<repo>/main/skill/SKILL.md` 可访问
5. `https://github.com/<repo>/releases` 中的资产名称与 README 一致

## 仓库内辅助脚本

- `scripts/release/package-public-release.sh`
- `scripts/release/render-public-repo.sh`
- `scripts/verify/public-release-package-smoke.sh`
- `scripts/verify/public-installer-smoke.sh`
