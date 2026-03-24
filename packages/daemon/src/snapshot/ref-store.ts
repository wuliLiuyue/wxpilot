export class RefStore {
  private map = new Map<number, string>()

  assign(refs: Map<number, string>): void {
    this.map = new Map(refs)
  }

  resolve(ref: number): string | null {
    return this.map.get(ref) ?? null
  }

  has(ref: number): boolean {
    return this.map.has(ref)
  }

  invalidateAll(): void {
    this.map.clear()
  }

  size(): number {
    return this.map.size
  }
}
