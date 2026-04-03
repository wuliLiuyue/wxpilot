# CLI Output Contract

- `status` 保留 `ok/connected/projectPath/proxy`，新增 `daemonKind`
- `view` header 固定为 `页面: {pagePath}`
- `find` header 固定为 `页面: {pagePath}` + `查找: "{query}"`
- `find` 空结果固定为 `未找到匹配项`
- `state` 默认返回 `summary`，`state [path]` 返回子路径值，`state --full` 返回完整 `data`
- `shot --base64` 返回 `path + base64`
- `run` / `wx` 返回 `result`
- `net log` 支持 `filter/limit/withBody`
- `net install-ca` 返回 `certFile`
