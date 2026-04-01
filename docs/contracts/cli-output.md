# CLI Output Contract

- `status` 保留 `ok/connected/projectPath/proxy`，新增 `daemonKind`
- `view` header 固定为 `页面: {pagePath}`
- `find` header 固定为 `页面: {pagePath}` + `查找: "{query}"`
- `find` 空结果固定为 `未找到匹配项`
- `state` 默认返回摘要，完整数据由 `--full` 控制
- `net log` 默认不返回 body
