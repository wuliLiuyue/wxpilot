# wxpilot Rust Daemon 详细技术方案（CLI 保持 TS，双 Daemon 并存）

## Summary

本方案将 `wxpilot` 演进为 `TS CLI + Rust Daemon + TS Daemon fallback` 的双实现架构，目标是在不打断现有使用方式的前提下，把性能敏感、稳定性敏感、后续可闭源的核心能力全部下沉到 Rust。

这版方案的核心原则已经固定：

- 只重构 `daemon`，CLI 暂不整体重构
- Rust daemon 对现有 TS CLI 完全兼容
- 过渡期同时支持 TS/Rust daemon，CLI 默认优先 Rust，失败回退 TS
- automator 由 Rust 直接接管，基于 `miniprogram-automator` npm 包编译产物逆向实现
- 首阶段不做商业化授权控制，但在模块边界上预留挂点
- Rust 切主门槛是 `功能对齐 + 回归通过`，性能提升作为阶段收益，不作为唯一门槛

## 技术架构

### 总体分层

- `packages/cli` 保持 TS，职责只保留为命令解析、daemon 探测、socket 请求、结果输出
- 新增 Rust workspace，建议目录为 `rust/`
- Rust daemon 作为长期主实现，负责所有运行时、协议、代理、解析、落库能力
- TS daemon 在过渡期保留，只作为 fallback，不继续演进新能力

### Rust workspace 结构

- `crates/wxp-daemon`: daemon 入口、生命周期、UDS server、RPC 路由、健康检查
- `crates/wxp-rpc`: 行分隔 JSON-RPC 编解码、错误码映射、请求上下文
- `crates/wxp-automator-protocol`: 从逆向结果沉淀出的 method/params/result 类型定义
- `crates/wxp-automator-client`: WebSocket 连接、request/response 匹配、对象句柄封装
- `crates/wxp-snapshot`: WXML 解析、事件指纹、节点树、摘要渲染
- `crates/wxp-ref-store`: ref 分配、快照版本、失效校验
- `crates/wxp-proxy`: HTTP/HTTPS 代理、MITM、mock、请求日志采集
- `crates/wxp-store`: SQLite 读写、mock 规则存储、日志查询
- `crates/wxp-compat`: 与当前 TS daemon 行为兼容的返回结构、默认值、错误文本
- `crates/wxp-common`: 路径、配置、常量、共享类型

### 推荐技术栈

- 运行时：`tokio`
- 序列化：`serde` + `serde_json`
- UDS：`tokio::net::UnixListener`
- WebSocket：`tokio-tungstenite`
- 日志：`tracing` + `tracing-subscriber`
- SQLite：`rusqlite`
- HTML/WXML 解析：`tl`
- HTTPS 证书：`rcgen`
- TLS：`tokio-rustls`
- 错误模型：`thiserror`
- 单元测试：`cargo test`
- 基准测试：`criterion`

## 模块详细设计

### 1. CLI 兼容层

- CLI 继续保留当前命令面、帮助文本、默认参数和输出风格
- 仅增加 daemon 发现逻辑，不改命令语义
- 探测顺序固定为：
  - 先连接 Rust socket
  - Rust 不可用时连接 TS socket
  - 两者都不可用时先拉起 Rust，Rust 拉起失败再拉起 TS
- CLI 需要支持读取 Rust daemon 的 `health/version/capabilities`，但这些只作为内部探测接口
- `status` 返回中新增 `daemonKind: rust | ts`，其余字段保持兼容
- 输出截断、`ref_expired` 格式、`state/view/net log` 的默认简化逻辑保持现状

### 2. Daemon Runtime

- Rust daemon 使用独立目录，例如 `~/.wxpilot/rust/`
- 路径包含：
  - `daemon.sock`
  - `daemon.pid`
  - `daemon.db`
  - `ca/`
  - `logs/`
- 保留单实例约束，使用 pid file + 文件锁双保险
- 保留 30 分钟空闲退出语义
- 所有外部请求经过统一 `RequestContext`，记录：
  - request id
  - method
  - start_at
  - daemon_kind
  - trace id
- `shutdown` 顺序固定为：
  - 停止接收新请求
  - 停止 proxy
  - 断开 automator
  - flush store writer
  - 删除 socket/pid

### 3. RPC 协议层

- 继续使用当前 `line-delimited JSON-RPC over UDS`
- 不切换到 gRPC，不引入额外传输复杂度
- 请求模型固定为：
  - `jsonrpc: "2.0"`
  - `id: string | number`
  - `method: string`
  - `params: object`
- 响应模型固定为：
  - `result`
  - `error: { code, message, data? }`
- 兼容错误码沿用 TS：
  - `REF_EXPIRED = -32000`
  - `NOT_CONNECTED = -32001`
  - `TIMEOUT = -32002`
  - `NOT_FOUND = -32003`
  - `INTERNAL = -32099`
- 内部新增但对 CLI 透明的方法：
  - `health`
  - `version`
  - `capabilities`

### 4. Automator 协议逆向与 Rust Client

- `miniprogram-automator` 的编译产物已经足够提供对象模型：
  - `Connection`
  - `MiniProgram`
  - `Page`
  - `Element`
- Rust client 严格按这四层建模，不直接裸调 method string
- 连接层设计：
  - `AutomatorConnection` 负责 websocket、request id、pending map、event dispatch
  - request id 使用 UUID 字符串，和 npm 包行为一致
  - 支持 request timeout、connection close reject、事件通知监听
- 对象层设计：
  - `MiniProgramClient`
  - `PageHandle { page_id, path, query }`
  - `ElementHandle { element_id, page_id, node_id?, video_id?, tag_name }`
- 第一阶段只实现 wxpilot 已用到的方法：
  - `Tool.getInfo`
  - `App.getCurrentPage`
  - `App.getPageStack`
  - `App.callWxMethod`
  - `App.callFunction`
  - `App.captureScreenshot`
  - `Page.getElement`
  - `Page.getElements`
  - `Page.getData`
  - `Page.getWindowProperties`
  - `Element.getWXML`
  - `Element.tap`
  - `Element.callFunction`
  - `Element.getProperties`
  - `Element.getDOMProperties`
- 路由封装保持与现有 TS 行为一致：
  - `goto` -> `navigateTo`
  - `back` -> `navigateBack`
  - `reload` -> `reLaunch(currentPage)`
  - `type` -> `Element.callFunction("input.input" | "textarea.input")`
  - `scroll` -> `scroll-view.scrollTo`
- 逆向产物需要沉淀为一份内部协议清单文件，字段至少包括：
  - rpc method
  - params shape
  - result shape
  - object owner
  - current wxpilot usage
  - test coverage status

### 5. Snapshot 模块

- 输入为 `outerWxml` 字符串 + 可选 `eventFingerprints`
- 输出为 `Vec<ElementNode>`
- `ElementNode` 结构固定为：
  - `tag`
  - `classes`
  - `text`
  - `placeholder`
  - `attrs`
  - `interactive`
  - `selector`
  - `ref`
  - `children`
- 交互判断逻辑完全复刻 TS：
  - 原生交互标签
  - 事件属性
  - `hover-start-time` / `hover-stay-time`
  - 非框架内部的 `data-*`
  - 源 WXML 指纹补全
- 框架内部 data 属性排除规则保留：
  - `data-sid`
  - `data-v-*`
  - `data-__*`
- `renderTree` 行为保持一致：
  - `compact`
  - `depth`
  - `limit`
  - `textLimit`
  - `shownCount`
  - `totalCount`
- `view` 的 header 格式与当前保持一致，避免 CLI 输出漂移

### 6. Ref Store

- Ref 不再只是 `ref -> selector`，而是：
  - `ref`
  - `selector`
  - `snapshot_version`
  - `page_path`
  - `created_at`
- 每次 `view` 生成新的 `snapshot_version`
- `goto/back/reload/connect/disconnect` 后统一失效全部 ref
- `tap/type/read/assert/scroll` 必须在执行前验证：
  - ref 存在
  - snapshot_version 是当前版本
- Rust 内部保留版本校验，外部错误仍返回 `ref_expired`

### 7. Proxy 模块

- HTTP 模式延续当前行为：
  - HTTP 请求走代理并记录
  - HTTPS CONNECT 透明透传
- HTTPS 模式实现 MITM：
  - 自建 root CA
  - 为目标域动态签发 leaf cert
  - leaf cert 做内存缓存
- `mock` 匹配规则第一阶段保持最简单：
  - 精确 URL 匹配
  - `GET` 优先保持与当前实现一致
- 日志采集模型：
  - `request meta` 先进入内存队列
  - body 在阈值内缓存
  - 超大 body 直接截断并标记
- 代理模块不直接写库，而是通过 channel 发给 store writer，避免请求线程阻塞
- `net log` 默认只返回摘要字段，不返回 body
- `--with-body` 时最多截断到 2000 字符，和现行为保持一致

### 8. Store 模块

- SQLite 路径与 Rust daemon 独立
- 开库后执行：
  - `journal_mode=WAL`
  - `synchronous=NORMAL`
  - `busy_timeout`
- 写入模型采用单 writer actor：
  - proxy 和 mock 更新都发消息给 writer
  - writer 统一顺序写库
- 读写分离：
  - writer connection 常驻
  - query connection 独立
- 表结构保持兼容：
  - `requests`
  - `mocks`
- `RequestLog` 模型与当前字段一致，避免 RPC 层映射复杂化
- `clearRequests`、`upsertMock`、`deleteMock` 保持同步语义

### 9. 兼容层

- 所有默认值统一在 `wxp-compat` 中定义，不分散在 handler 内
- 兼容层负责：
  - 返回字段顺序和命名
  - 默认 limit/depth/textLimit
  - 错误消息文本
  - 截断提示文本
  - human-readable 友好字段
- 目标是 Rust 业务实现可以内部重构，但对外结果稳定

### 10. 为未来商业化预留的挂点

- daemon 启动前预留 `BootstrapGuard` 接口
- 当前实现返回 `allow`
- 后续可插入：
  - license 校验
  - 设备指纹
  - 远程配置
  - 功能开关
- 这部分第一阶段不落实现逻辑，只保留模块边界和调用点

## Public APIs / Interfaces

### CLI 到 Daemon 的公开 RPC

- `start`
- `connect`
- `disconnect`
- `status`
- `goto`
- `back`
- `reload`
- `view`
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

### 核心内部类型

- `RpcRequest`
- `RpcResponse`
- `AutomatorRequest`
- `AutomatorResponse`
- `MiniProgramClient`
- `PageHandle`
- `ElementHandle`
- `ElementNode`
- `RefEntry`
- `RequestLog`
- `MockRule`
- `DaemonStatus`

## 阶段拆解

### Phase 0：协议冻结

- 固化现有 TS daemon 的 method/params/result/error 契约
- 固化 CLI 输出行为
- 生成 automator 逆向协议清单
- 记录 TS 性能基线

### Phase 1：基础设施

- 建 Rust workspace
- 建 runtime、rpc、compat 骨架
- CLI 增加双 daemon 探测与回退

### Phase 2：automator 读路径

- 完成 websocket client
- 跑通 `connect/status/state/view`
- 建立 Page/Element 句柄模型

### Phase 3：snapshot/ref

- 完成 `view/wait/read/assert`
- 与现有 parser tests 行为对齐
- 引入 `snapshot_version`

### Phase 4：action path

- 完成 `goto/back/reload/tap/type/scroll/shot/run/wx`
- 跑通自动化主链路

### Phase 5：proxy/store

- 完成 `net.*`
- 取代 `mockttp + better-sqlite3`

### Phase 6：灰度切主

- CLI 默认主连 Rust
- TS daemon 保留观察期 fallback
- 观察期结束后再立项 Rust CLI

## Test Plan

### 契约测试

- TS daemon 与 Rust daemon 跑同一组 CLI 用例
- 比较成功结构、失败结构、错误码、超时、默认输出
- `status/view/state/shot/net log` 必须逐项对齐

### Automator 测试

- websocket 建连与断连
- `Tool.getInfo`
- 当前页面获取
- 页面数据读取
- outerWxml 读取
- 元素选择、点击、输入、滚动
- 截图、脚本执行、wx 方法调用

### Snapshot/Ref 测试

- 现有 parser tests 全量迁移到 Rust
- 保留已知局限测试，如 `tag:class` 指纹碰撞误判
- 校验 `goto/back/reload` 后 ref 失效
- 校验 `snapshot_version` 校验逻辑

### Proxy/Store 测试

- HTTP 透传
- HTTPS CONNECT
- HTTPS MITM
- mock 命中与取消
- body 截断
- SQLite 高并发写入
- daemon 重启恢复
- query 过滤行为

### 灰度与稳定性

- 一个真实生产项目灰度
- 长时间 daemon 常驻稳定性
- 高频 `view` 循环稳定性
- 高频代理日志写入稳定性
- Rust 不可用时 CLI 回退 TS 成功

### 性能观察指标

- 大 WXML 解析耗时低于 TS 基线
- 高频 `view` CPU 占用显著低于 TS
- 代理日志写入不阻塞主请求
- 内存曲线稳定，无明显泄漏
- 这些指标作为阶段收益记录，不作为首个切主唯一门槛

## Assumptions

- npm 包编译产物足以支撑第一阶段 automator 协议逆向
- 不依赖拿到 `miniprogram-automator` 的上游 TS 源码仓库才开工
- CLI 允许做最小必要改动以支持双 daemon 探测与回退，这不视为 CLI 重构
- 首阶段发布目标平台是 macOS
- 商业化控制后置，但模块边界必须现在预留
- TS daemon 进入维护态，只修阻塞性问题，不再承接新架构能力
