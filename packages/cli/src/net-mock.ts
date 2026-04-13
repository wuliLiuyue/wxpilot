export type NetMockOptions = {
  method?: string
}

export function buildNetMockParams(url: string, jsonFile: string, opts: NetMockOptions = {}) {
  return {
    url,
    jsonFile,
    method: normalizeHttpMethod(opts.method),
  }
}

function normalizeHttpMethod(method?: string) {
  const value = method?.trim()
  return value ? value.toUpperCase() : 'GET'
}
