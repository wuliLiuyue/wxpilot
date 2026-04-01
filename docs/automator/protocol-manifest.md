# Automator Protocol Manifest

| owner | method | params | result | used_by_wxpilot | covered |
| --- | --- | --- | --- | --- | --- |
| Tool | `Tool.getInfo` | `{}` | `tool info` | `status` | yes |
| App | `App.getCurrentPage` | `{}` | `page` | `status/state/view/find/reload/tap/type/scroll` | yes |
| App | `App.getPageStack` | `{}` | `page[]` | `state` | partial |
| App | `App.callWxMethod` | `{method,args}` | `{}` | `goto/back/reload` | yes |
| Page | `Page.getData` | `{pageId}` | `{data}` | `state` | yes |
| Page | `Page.getElement` | `{pageId,selector}` | `{elementId}` | `view/find/tap/type/scroll` | yes |
| Element | `Element.getWXML` | `{elementId}` | `{outerWxml}` | `view/find/wait` | yes |
| Element | `Element.tap` | `{elementId}` | `{}` | `tap` | yes |
| Element | `Element.callFunction` | `{elementId,name,args}` | `{}` | `type/scroll` | yes |
