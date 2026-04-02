# wxpilot Rust Daemon 基础技术方案

创建日期：2026-03-30  
最近整理：2026-04-03

## 文档定位

这是一份基础技术方案，用来解释 `wxpilot` 为什么采用 `TS CLI + Rust Daemon + 可选 TS Daemon` 的演进路线，以及当前已经冻结的核心架构决策。

它不是执行清单，也不是最新落地计划。具体实施拆解以同目录下后续计划为准：

- `plans/2026-03-31-rust-daemon-refactor.md`
- `plans/2026-04-01-rust-daemon-native-realignment.md`
- `plans/2026-04-02-rust-daemon-gap-closure.md`

## 当前已冻结的决策

- `packages/cli` 继续保留 TypeScript，实现命令解析、daemon 选择、socket 请求与结果输出。
- Rust daemon 是长期主实现，承担运行时、协议、automator、snapshot、proxy、store 等核心能力。
- TS daemon 只保留为兼容实现；仅在用户显式指定 `--daemon-kind ts` 时连接，不再自动回退。
- Rust runtime 内部不得桥接到 TS daemon，两个 daemon 必须相互独立。
- 契约相关内容统一收口到 `packages/contracts`，包括：
  - `packages/contracts/docs`
  - `packages/contracts/src`
  - `packages/contracts/tests`
- 过程性计划统一存放在 `plans/`，不再混放在 `docs/`。
- 对外行为是否可切主，判断标准是“契约对齐 + 真实回归通过”，性能提升是收益，不是唯一门槛。

## 当前仓库映射

### CLI 与兼容实现

- `packages/cli`: 当前 CLI 主入口
- `packages/daemon`: TS daemon 兼容实现
- `packages/contracts`: 契约单一来源

### Rust workspace

- `rust/crates/wxp-daemon`: daemon 入口、server、handlers、runtime
- `rust/crates/wxp-rpc`: RPC 协议模型与编解码
- `rust/crates/wxp-automator-protocol`: automator 协议类型
- `rust/crates/wxp-automator-client`: automator WebSocket client
- `rust/crates/wxp-snapshot`: WXML 解析与摘要渲染
- `rust/crates/wxp-ref-store`: ref 与 snapshot version 管理
- `rust/crates/wxp-proxy`: 代理与抓包
- `rust/crates/wxp-store`: SQLite 存储
- `rust/crates/wxp-compat`: Rust 对外兼容层
- `rust/crates/wxp-common`: 共用路径、常量、共享类型

## 架构目标

### 为什么保留 TS CLI

- CLI 层已经稳定，改写收益低于风险。
- CLI 的主要职责是输入输出与调用编排，不是性能瓶颈。
- 保留 TS CLI 可以让 Rust daemon 在不改变用户命令面的前提下逐步切入。

### 为什么核心逻辑下沉到 Rust

- `view/find/state/net.*` 这类路径对稳定性、性能和资源占用更敏感。
- proxy、snapshot、store、automator client 这几块更适合长期沉淀为强约束的系统能力。
- Rust 方案更适合后续做封装边界、性能治理和潜在商业化控制。

## 系统分层

### 1. CLI 层

职责固定为：

- 命令解析
- daemon kind 选择
- UDS 请求发送
- JSON / 文本输出
- 最小运行时清理

CLI 不负责：

- 业务状态持久化
- 页面解析
- 代理逻辑
- automator 协议实现

### 2. Daemon 层

daemon 是唯一运行时核心，职责包括：

- 维持单实例
- 管理 automator 连接
- 维护页面快照与 ref
- 执行读路径与动作路径
- 管理代理和 SQLite store
- 暴露稳定 RPC 接口

### 3. Contracts 层

`packages/contracts` 是对外行为的冻结层，用来收口三类内容：

- 协议文档
- 可复用契约常量与输出辅助
- 跨实现 contract tests

所有“TS daemon、Rust daemon、CLI 输出必须一致”的约束，都应先在这里表达，再由实现去对齐。

## 对外协议约束

### 传输

- 继续使用 `line-delimited JSON-RPC over UDS`
- 不引入 gRPC 或额外协议层

### 公开 RPC

- `start`
- `connect`
- `disconnect`
- `status`
- `goto`
- `back`
- `reload`
- `view`
- `find`
- `wait`
- `tap`
- `type`
- `scroll`
- `read`
- `assert`
- `state`
- `shot`
- `run`
- `wx`
- `net.start`
- `net.stop`
- `net.log`
- `net.mock`
- `net.unmock`
- `net.clear`
- `net.install-ca`
- `daemon.stop`

### 内部探测 RPC

- `health`
- `version`
- `capabilities`

这些接口只用于 CLI 探测与运行时自检，不是用户命令面的一部分。

### 错误码

错误码继续与 TS 契约对齐：

- `REF_EXPIRED = -32000`
- `NOT_CONNECTED = -32001`
- `TIMEOUT = -32002`
- `NOT_FOUND = -32003`
- `INTERNAL = -32099`

## 模块设计要点

### CLI 兼容层

- 默认连接 Rust daemon。
- Rust 不可用时，仅拉起并继续探测 Rust daemon。
- 仅当用户显式指定 `--daemon-kind ts` 时，才连接或拉起 TS daemon。
- `status` 返回需要显式包含 `daemonKind`。
- `state/view/find/net log` 的默认简化输出保持与当前 CLI 契约一致。

### Runtime 与进程文件

Rust daemon 使用 `~/.wxpilot/` 下的独立文件前缀：

- `rust-daemon.sock`
- `rust-daemon.pid`
- `rust-daemon-state.json`
- `rust-daemon.db`
- `rust-ca/`

保留：

- 单实例约束
- 空闲退出语义
- 优雅关闭顺序

### Automator client

- Rust 侧直接对接 automator WebSocket。
- 协议清单沉淀在 `packages/contracts/docs/automator-protocol-manifest.md`。
- 第一阶段只实现 `wxpilot` 已用到的方法，不追求一次性覆盖全部上游能力。

### Snapshot / Ref

- `view` 和 `find` 共用同一套 snapshot 构建链路。
- `find` 默认只查交互节点，`all=true` 时允许命中非交互节点。
- 每次 `view` / `find` 生成新的 `snapshot_version`。
- `goto/back/reload/connect/disconnect` 后统一失效旧 ref。
- Rust 内部可以保留更强约束，但对外错误文本仍要兼容 `ref_expired`。

### Proxy / Store

- HTTP 模式下记录 HTTP，请求 HTTPS CONNECT 透传。
- HTTPS 模式支持 MITM、CA 生成与安装。
- `net log` 默认只返回摘要，不返回 body。
- store 使用 SQLite；读写模型要避免阻塞主请求路径。

### Compat

`wxp-compat` 负责统一：

- 默认值
- 输出字段命名
- 错误文本
- 截断提示
- human-readable 输出细节

目标是让 Rust 内部实现可以继续演进，但对 CLI 的外部结果保持稳定。

## 验证策略

### 契约验证

- TS daemon、Rust daemon、CLI 必须围绕同一份 contracts 收敛。
- `status/view/find/state/shot/net log` 是优先对齐项。
- `packages/contracts/tests/*` 是最小跨实现护栏，不能继续散落到根目录。

### 真实回归

- 需要保留真实小程序项目回归，而不只依赖单元测试。
- 真实回归应覆盖至少：
  - `start`
  - `connect`
  - `view`
  - `find`
  - `state`

### 切主门槛

Rust daemon 切主必须同时满足：

- 公开 RPC 与输出契约对齐
- 关键自动化链路可回归
- proxy/store 不引入阻塞性回归

## 已淘汰的旧判断

以下说法已经不再适用，本文件明确废弃：

- “CLI 默认 Rust，失败后自动回退 TS”
- “TS daemon 保留观察期 fallback”
- “契约文档放在 `docs/`，测试放在根 `tests/`，辅助绑定放在 `rust-bindings/`”

当前仓库的正确组织方式是：

- 契约：`packages/contracts`
- 计划：`plans`
- CLI 只显式连接用户指定的 daemon kind

## 后续使用方式

阅读顺序建议如下：

1. 先看本文件，理解为什么会有 Rust daemon 演进路线。
2. 再看 `README.md` 中的当前行为说明。
3. 最后看 `plans/2026-03-31`、`2026-04-01`、`2026-04-02` 这三份执行计划，理解具体落地顺序。

如果后续架构决策再次变化，应优先修改本文件的“当前已冻结的决策”与“已淘汰的旧判断”，而不是继续在旧段落上追加补丁。
