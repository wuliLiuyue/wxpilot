# wxpilot Curl 安装链路 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 wxpilot 提供基于 GitHub Releases 的 macOS `curl` 安装方式，默认安装最新 release，支持 `--version vX.Y.Z`，并把 `wxpilot` 安装到 `~/.local/bin`、把 `wxp-daemon` 安装到 `~/.wxpilot/bin`。

**Architecture:** 保持现有 Rust CLI + Rust daemon + UDS JSON-RPC 架构不变。新增一个仓库根目录 `install.sh` 作为用户安装入口，一个 `scripts/release/package-macos.sh` 作为维护者手工打包入口，同时调整 `rust/crates/wxp-cli/src/runner.rs` 的 daemon 二进制发现逻辑，让安装后的 `wxpilot` 能从 `~/.wxpilot/bin/wxp-daemon` 拉起 daemon。

**Tech Stack:** Bash, GitHub Releases, tar, curl, mktemp, Rust integration tests, Makefile

---

## File Structure

### New files

- Create: `install.sh` - 用户侧安装入口，支持 latest 和 `--version`，只支持 macOS `arm64` / `x86_64`。
- Create: `scripts/release/package-macos.sh` - 维护者手工打包脚本，生成 `wxpilot-darwin-arm64.tar.gz` / `wxpilot-darwin-amd64.tar.gz`。
- Create: `tests/install/package_macos_smoke.sh` - 验证打包脚本生成的归档命名与内容布局。
- Create: `tests/install/install_smoke.sh` - 通过 stub `curl` / `uname` 做离线安装脚本 smoke test。

### Existing files to modify

- Modify: `rust/crates/wxp-cli/src/runner.rs` - `locate_daemon_binary()` 增加 `~/.wxpilot/bin/wxp-daemon` 回退路径，并补单元测试。
- Modify: `Makefile` - 增加安装链路 smoke test 入口。
- Modify: `README.md` - 把 `curl` 安装方式提升为主入口，源码构建降级为开发者说明，并明确平台/`PATH`/版本安装方法。
- Modify: `rust/crates/wxp-cli/tests/readme_migration.rs` - 固定 README 的安装入口文案，避免文档回退到源码构建优先。

## Task 1: 让安装后的 CLI 能找到私有目录里的 daemon

**Files:**
- Modify: `rust/crates/wxp-cli/src/runner.rs`
- Test: `rust/crates/wxp-cli/src/runner.rs`

- [ ] **Step 1: 先在 `runner.rs` 写失败中的 daemon 定位测试**

```rust
#[cfg(test)]
mod tests {
    use super::locate_daemon_binary;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn reset_env(home: &PathBuf) {
        env::remove_var("WXP_DAEMON_BIN");
        env::set_var("HOME", home);
    }

    #[test]
    fn locate_daemon_binary_prefers_explicit_env_override() {
        let _guard = ENV_LOCK.lock().expect("lock");
        let home = tempdir().expect("home");
        reset_env(&home.path().to_path_buf());

        let custom = home.path().join("custom/wxp-daemon");
        env::set_var("WXP_DAEMON_BIN", &custom);

        assert_eq!(locate_daemon_binary(), custom);
    }

    #[test]
    fn locate_daemon_binary_uses_private_install_dir_when_present() {
        let _guard = ENV_LOCK.lock().expect("lock");
        let home = tempdir().expect("home");
        reset_env(&home.path().to_path_buf());

        let daemon = home.path().join(".wxpilot/bin/wxp-daemon");
        fs::create_dir_all(daemon.parent().expect("parent")).expect("mkdir");
        fs::write(&daemon, "#!/bin/sh\n").expect("write");

        assert_eq!(locate_daemon_binary(), daemon);
    }

    #[test]
    fn locate_daemon_binary_falls_back_to_path_when_private_install_is_missing() {
        let _guard = ENV_LOCK.lock().expect("lock");
        let home = tempdir().expect("home");
        reset_env(&home.path().to_path_buf());

        assert_eq!(locate_daemon_binary(), PathBuf::from("wxp-daemon"));
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli locate_daemon_binary_ -- --nocapture`

Expected: FAIL，`locate_daemon_binary_uses_private_install_dir_when_present` 断言失败，当前返回值仍是 `wxp-daemon`。

- [ ] **Step 3: 最小实现私有 daemon 路径回退**

```rust
fn locate_daemon_binary() -> PathBuf {
    if let Some(path) = env::var_os("WXP_DAEMON_BIN") {
        return PathBuf::from(path);
    }

    if let Ok(current_exe) = env::current_exe() {
        if let Some(directory) = current_exe.parent() {
            let sibling = directory.join("wxp-daemon");
            if sibling.exists() {
                return sibling;
            }
        }
    }

    let private_install = runtime_dir().join("bin").join("wxp-daemon");
    if private_install.exists() {
        return private_install;
    }

    PathBuf::from("wxp-daemon")
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli locate_daemon_binary_ -- --nocapture`

Expected: PASS，三个 `locate_daemon_binary_*` 测试通过。

- [ ] **Step 5: 提交**

```bash
git add rust/crates/wxp-cli/src/runner.rs
git commit -m "feat: resolve installed daemon from private bin"
```

## Task 2: 增加手工 macOS release 打包脚本

**Files:**
- Create: `scripts/release/package-macos.sh`
- Create: `tests/install/package_macos_smoke.sh`

- [ ] **Step 1: 先写失败中的打包 smoke test**

```bash
#!/usr/bin/env bash
set -euo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/bin" "$tmp/out"
printf '#!/usr/bin/env bash\necho wxpilot\n' > "$tmp/bin/wxpilot"
printf '#!/usr/bin/env bash\necho wxp-daemon\n' > "$tmp/bin/wxp-daemon"
chmod +x "$tmp/bin/wxpilot" "$tmp/bin/wxp-daemon"

bash scripts/release/package-macos.sh \
  --arch arm64 \
  --wxpilot "$tmp/bin/wxpilot" \
  --wxp-daemon "$tmp/bin/wxp-daemon" \
  --output "$tmp/out"

archive="$tmp/out/wxpilot-darwin-arm64.tar.gz"
test -f "$archive"

entries="$(tar -tzf "$archive")"
printf '%s\n' "$entries" | grep -Fx 'wxpilot'
printf '%s\n' "$entries" | grep -Fx 'wxp-daemon'
printf '%s\n' "$entries" | grep -Fx 'README-install.txt'
```

- [ ] **Step 2: 运行测试确认失败**

Run: `bash tests/install/package_macos_smoke.sh`

Expected: FAIL，报错 `scripts/release/package-macos.sh: No such file or directory`。

- [ ] **Step 3: 实现最小打包脚本，固定归档命名和内容布局**

```bash
#!/usr/bin/env bash
set -euo pipefail

arch=""
wxpilot_bin=""
wxp_daemon_bin=""
output_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch)
      arch="${2:-}"
      shift 2
      ;;
    --wxpilot)
      wxpilot_bin="${2:-}"
      shift 2
      ;;
    --wxp-daemon)
      wxp_daemon_bin="${2:-}"
      shift 2
      ;;
    --output)
      output_dir="${2:-}"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

case "$arch" in
  arm64|amd64) ;;
  *)
    echo "arch must be arm64 or amd64" >&2
    exit 1
    ;;
esac

test -x "$wxpilot_bin" || { echo "wxpilot binary not executable: $wxpilot_bin" >&2; exit 1; }
test -x "$wxp_daemon_bin" || { echo "wxp-daemon binary not executable: $wxp_daemon_bin" >&2; exit 1; }
mkdir -p "$output_dir"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

cp "$wxpilot_bin" "$stage/wxpilot"
cp "$wxp_daemon_bin" "$stage/wxp-daemon"
chmod +x "$stage/wxpilot" "$stage/wxp-daemon"

cat > "$stage/README-install.txt" <<'EOF'
Install with:
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash
EOF

archive="$output_dir/wxpilot-darwin-${arch}.tar.gz"
tar -C "$stage" -czf "$archive" wxpilot wxp-daemon README-install.txt
echo "created $archive"
```

- [ ] **Step 4: 运行测试确认通过**

Run: `bash tests/install/package_macos_smoke.sh`

Expected: PASS，`wxpilot-darwin-arm64.tar.gz` 存在，且只包含 `wxpilot`、`wxp-daemon`、`README-install.txt`。

- [ ] **Step 5: 提交**

```bash
git add scripts/release/package-macos.sh tests/install/package_macos_smoke.sh
git commit -m "feat: add macos release packager"
```

## Task 3: 增加用户侧 `curl` 安装脚本

**Files:**
- Create: `install.sh`
- Create: `tests/install/install_smoke.sh`
- Modify: `Makefile`

- [ ] **Step 1: 先写失败中的安装脚本 smoke test**

```bash
#!/usr/bin/env bash
set -euo pipefail

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

home="$tmp/home"
release_dir="$tmp/release"
stub_dir="$tmp/stubs"
mkdir -p "$home" "$release_dir" "$stub_dir"

stage="$tmp/stage"
mkdir -p "$stage"
printf '#!/usr/bin/env bash\necho cli\n' > "$stage/wxpilot"
printf '#!/usr/bin/env bash\necho daemon\n' > "$stage/wxp-daemon"
printf 'artifact readme\n' > "$stage/README-install.txt"
chmod +x "$stage/wxpilot" "$stage/wxp-daemon"
tar -C "$stage" -czf "$release_dir/wxpilot-darwin-arm64.tar.gz" wxpilot wxp-daemon README-install.txt
cp "$release_dir/wxpilot-darwin-arm64.tar.gz" "$release_dir/wxpilot-darwin-amd64.tar.gz"

cat > "$stub_dir/uname" <<'EOF'
#!/usr/bin/env bash
if [[ "${1:-}" == "-s" ]]; then
  echo "${WXP_TEST_UNAME_S:-Darwin}"
elif [[ "${1:-}" == "-m" ]]; then
  echo "${WXP_TEST_UNAME_M:-arm64}"
else
  /usr/bin/uname "$@"
fi
EOF
chmod +x "$stub_dir/uname"

cat > "$stub_dir/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

output=""
url=""
want_effective_url=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    -o)
      output="$2"
      shift 2
      ;;
    -w)
      if [[ "${2:-}" == *url_effective* ]]; then
        want_effective_url=1
      fi
      shift 2
      ;;
    http*)
      url="$1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

if [[ "$url" == *"/releases/latest" ]]; then
  if [[ $want_effective_url -eq 1 ]]; then
    printf 'https://github.com/wuliLiuyue/wxpilot/releases/tag/v0.1.0'
    exit 0
  fi
fi

if [[ "$url" == *"wxpilot-darwin-arm64.tar.gz" ]]; then
  cp "${WXP_TEST_RELEASE_DIR}/wxpilot-darwin-arm64.tar.gz" "$output"
  exit 0
fi

if [[ "$url" == *"wxpilot-darwin-amd64.tar.gz" ]]; then
  cp "${WXP_TEST_RELEASE_DIR}/wxpilot-darwin-amd64.tar.gz" "$output"
  exit 0
fi

echo "unexpected curl url: $url" >&2
exit 1
EOF
chmod +x "$stub_dir/curl"

output_latest="$(
  env \
    HOME="$home" \
    PATH="$stub_dir:/usr/bin:/bin" \
    WXP_TEST_RELEASE_DIR="$release_dir" \
    bash ./install.sh 2>&1
)"

test -x "$home/.local/bin/wxpilot"
test -x "$home/.wxpilot/bin/wxp-daemon"
grep -Fx 'v0.1.0' "$home/.wxpilot/version"
grep -F '"arch": "arm64"' "$home/.wxpilot/install-meta.json"
printf '%s' "$output_latest" | grep -F 'export PATH="$HOME/.local/bin:$PATH"'

rm -rf "$home/.local" "$home/.wxpilot"

output_versioned="$(
  env \
    HOME="$home" \
    PATH="$stub_dir:/usr/bin:/bin" \
    WXP_TEST_RELEASE_DIR="$release_dir" \
    WXP_TEST_UNAME_M="x86_64" \
    bash ./install.sh --version v0.1.0 2>&1
)"

grep -Fx 'v0.1.0' "$home/.wxpilot/version"
grep -F '"arch": "amd64"' "$home/.wxpilot/install-meta.json"
printf '%s' "$output_versioned" | grep -F 'wxpilot installed to'

if env \
  HOME="$home" \
  PATH="$stub_dir:/usr/bin:/bin" \
  WXP_TEST_RELEASE_DIR="$release_dir" \
  WXP_TEST_UNAME_S="Linux" \
  bash ./install.sh >"$tmp/linux.out" 2>&1; then
  echo "install.sh should reject Linux" >&2
  exit 1
fi

grep -F '当前安装脚本仅支持 macOS arm64/x86_64' "$tmp/linux.out"
```

- [ ] **Step 2: 运行测试确认失败**

Run: `bash tests/install/install_smoke.sh`

Expected: FAIL，报错 `./install.sh: No such file or directory`。

- [ ] **Step 3: 实现最小安装脚本**

```bash
#!/usr/bin/env bash
set -euo pipefail

repo_owner="wuliLiuyue"
repo_name="wxpilot"
version=""

fail() {
  echo "Error: $*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      version="${2:-}"
      shift 2
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

[[ "$(uname -s)" == "Darwin" ]] || fail "当前安装脚本仅支持 macOS arm64/x86_64"

need_cmd curl
need_cmd tar
need_cmd mktemp

arch_raw="$(uname -m)"
case "$arch_raw" in
  arm64|aarch64)
    arch="arm64"
    ;;
  x86_64|amd64)
    arch="amd64"
    ;;
  *)
    fail "当前安装脚本仅支持 macOS arm64/x86_64"
    ;;
esac

if [[ -z "$version" ]]; then
  latest_url="$(
    curl -fsSL -o /dev/null -w '%{url_effective}' \
      "https://github.com/${repo_owner}/${repo_name}/releases/latest"
  )"
  version="${latest_url##*/}"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

archive="$tmp/wxpilot.tar.gz"
extract_dir="$tmp/extract"
download_url="https://github.com/${repo_owner}/${repo_name}/releases/download/${version}/wxpilot-darwin-${arch}.tar.gz"

curl -fsSL "$download_url" -o "$archive" || fail "failed to download ${download_url}"
mkdir -p "$extract_dir"
tar -xzf "$archive" -C "$extract_dir"

test -f "$extract_dir/wxpilot" || fail "release archive missing wxpilot"
test -f "$extract_dir/wxp-daemon" || fail "release archive missing wxp-daemon"

mkdir -p "$HOME/.local/bin" "$HOME/.wxpilot/bin"
cp "$extract_dir/wxpilot" "$HOME/.local/bin/wxpilot"
cp "$extract_dir/wxp-daemon" "$HOME/.wxpilot/bin/wxp-daemon"
chmod +x "$HOME/.local/bin/wxpilot" "$HOME/.wxpilot/bin/wxp-daemon"

printf '%s\n' "$version" > "$HOME/.wxpilot/version"
cat > "$HOME/.wxpilot/install-meta.json" <<EOF
{
  "version": "$version",
  "platform": "darwin",
  "arch": "$arch",
  "downloadUrl": "$download_url",
  "installedAt": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "cliPath": "$HOME/.local/bin/wxpilot",
  "daemonPath": "$HOME/.wxpilot/bin/wxp-daemon"
}
EOF

echo "wxpilot installed to $HOME/.local/bin/wxpilot"
echo "wxp-daemon installed to $HOME/.wxpilot/bin/wxp-daemon"
echo "Add wxpilot to PATH if needed:"
echo '  export PATH="$HOME/.local/bin:$PATH"'
```

- [ ] **Step 4: 把安装 smoke test 接入 Makefile**

```make
.PHONY: build test lint rust-cli-build rust-cli-help rust-test install-test

install-test:
	bash tests/install/package_macos_smoke.sh
	bash tests/install/install_smoke.sh
```

- [ ] **Step 5: 运行测试确认通过**

Run: `make install-test`

Expected: PASS，两个 shell smoke test 都通过。

- [ ] **Step 6: 提交**

```bash
git add install.sh tests/install/install_smoke.sh tests/install/package_macos_smoke.sh Makefile
git commit -m "feat: add curl installer and smoke tests"
```

## Task 4: 更新 README，改成对外安装入口

**Files:**
- Modify: `README.md`
- Modify: `rust/crates/wxp-cli/tests/readme_migration.rs`

- [ ] **Step 1: 先写失败中的 README 迁移测试**

```rust
use std::fs;

#[test]
fn readme_uses_curl_install_as_primary_guide() {
    let readme = fs::read_to_string("../../../README.md").expect("read README");
    assert!(readme.contains(
        "curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash"
    ));
    assert!(readme.contains(
        "curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash -s -- --version v0.1.0"
    ));
    assert!(readme.contains("当前 curl 安装仅支持 macOS arm64/x86_64"));
    assert!(readme.contains("源码构建（开发者）"));
    assert!(readme.contains("wxpilot start <projectPath> --proxy"));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd rust && cargo test -p wxp-cli --test readme_migration`

Expected: FAIL，README 仍然以 `cargo build -p wxp-cli --bin wxpilot` 开头。

- [ ] **Step 3: 改写 README 的安装与维护说明**

````md
## 安装

```bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash
curl -fsSL https://raw.githubusercontent.com/wuliLiuyue/wxpilot/main/install.sh | bash -s -- --version v0.1.0
```

当前 curl 安装仅支持 macOS arm64/x86_64。

安装完成后，如果当前 shell 还找不到 `wxpilot`，把下面这行加入 `~/.zshrc` 或 `~/.bashrc`：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### 源码构建（开发者）

```bash
cd rust
cargo build -p wxp-cli --bin wxpilot
cargo build -p wxp-daemon --bin wxp-daemon
./target/debug/wxpilot --help
```

### 手工发布产物

```bash
bash scripts/release/package-macos.sh \
  --arch arm64 \
  --wxpilot rust/target/aarch64-apple-darwin/release/wxpilot \
  --wxp-daemon rust/target/aarch64-apple-darwin/release/wxp-daemon \
  --output ./dist/release

bash scripts/release/package-macos.sh \
  --arch amd64 \
  --wxpilot rust/target/x86_64-apple-darwin/release/wxpilot \
  --wxp-daemon rust/target/x86_64-apple-darwin/release/wxp-daemon \
  --output ./dist/release
```

Release 产物命名必须固定为：

- `wxpilot-darwin-arm64.tar.gz`
- `wxpilot-darwin-amd64.tar.gz`
````

- [ ] **Step 4: 运行测试确认通过**

Run: `cd rust && cargo test -p wxp-cli --test readme_migration`

Expected: PASS，README 现在以 `curl` 安装为主入口，并保留开发者源码构建说明。

- [ ] **Step 5: 提交**

```bash
git add README.md rust/crates/wxp-cli/tests/readme_migration.rs
git commit -m "docs: document curl install and release packaging"
```

## Task 5: 做一次最小闭环验证

**Files:**
- Modify: `rust/crates/wxp-cli/src/runner.rs`
- Modify: `README.md`
- Modify: `Makefile`
- Create: `install.sh`
- Create: `scripts/release/package-macos.sh`
- Create: `tests/install/package_macos_smoke.sh`
- Create: `tests/install/install_smoke.sh`

- [ ] **Step 1: 跑 Rust 与 shell 的组合验证**

Run: `cd rust && cargo test -p wxp-cli locate_daemon_binary_ -- --nocapture && cargo test -p wxp-cli --test readme_migration && cd .. && make install-test`

Expected: PASS，daemon 路径单测、README 迁移测试、打包/安装 smoke test 全部通过。

- [ ] **Step 2: 做一次人工命令检查**

Run:

```bash
tmp="$(mktemp -d)"
printf '#!/usr/bin/env bash\necho cli\n' > "$tmp/wxpilot"
printf '#!/usr/bin/env bash\necho daemon\n' > "$tmp/wxp-daemon"
chmod +x "$tmp/wxpilot" "$tmp/wxp-daemon"

bash scripts/release/package-macos.sh \
  --arch arm64 \
  --wxpilot "$tmp/wxpilot" \
  --wxp-daemon "$tmp/wxp-daemon" \
  --output "$tmp/out"
tar -tzf "$tmp/out/wxpilot-darwin-arm64.tar.gz"
```

Expected: 输出依次包含 `wxpilot`、`wxp-daemon`、`README-install.txt`。

- [ ] **Step 3: 确认工作区只剩预期结果**

Run: `git status --short`

Expected: 空输出；如果还有未提交改动，只能是本计划明确涉及的 `install.sh`、`scripts/release/package-macos.sh`、`tests/install/*`、`README.md`、`Makefile`、`rust/crates/wxp-cli/src/runner.rs`、`rust/crates/wxp-cli/tests/readme_migration.rs`。

## Self-Review

- Spec coverage: 已覆盖安装入口、版本参数、macOS 双架构限制、`~/.local/bin`/`~/.wxpilot/bin` 落盘、release 归档命名、`PATH` 手动提示、README 迁移和最小验证闭环。
- Placeholder scan: 计划中没有 `TODO` / `TBD` / “自行实现” 这类占位语句；每个代码步骤都给了可直接落地的脚本或代码片段。
- Type consistency: 归档命名统一为 `wxpilot-darwin-arm64.tar.gz` / `wxpilot-darwin-amd64.tar.gz`；daemon 私有安装路径统一为 `~/.wxpilot/bin/wxp-daemon`；README、安装脚本和 smoke test 都使用同一套命名。
