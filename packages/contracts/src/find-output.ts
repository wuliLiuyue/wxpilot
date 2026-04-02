export function formatFindOutput(input: {
  pagePath: string
  query: string
  matches: unknown[]
  shownCount: number
  totalCount: number
}): string {
  const rendered = input.matches.length > 0 ? String(input.matches.join('\n')) : '未找到匹配项'
  const suffix = input.totalCount > input.shownCount
    ? ` [显示 ${input.shownCount}/${input.totalCount} 条]`
    : ''
  return `页面: ${input.pagePath}\n查找: ${JSON.stringify(input.query)}${suffix}\n──────────────────────────────────\n${rendered}`
}
