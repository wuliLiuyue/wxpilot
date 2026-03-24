import Database from 'better-sqlite3'
import fs from 'fs'
import path from 'path'
import { DB_FILE } from '../constants.js'

export interface RequestLog {
  id: number
  ts: number
  method: string
  url: string
  req_headers: string | null
  req_body: Buffer | null
  status: number | null
  res_headers: string | null
  res_body: Buffer | null
  mocked: number
  duration: number | null
}

export interface MockRule {
  id: number
  url: string
  response: string
}

let db: Database.Database | null = null

function getDb(): Database.Database {
  if (!db) {
    fs.mkdirSync(path.dirname(DB_FILE), { recursive: true })
    db = new Database(DB_FILE)
    db.pragma('journal_mode = WAL')
    migrate(db)
  }
  return db
}

function migrate(db: Database.Database) {
  db.exec(`
    CREATE TABLE IF NOT EXISTS requests (
      id          INTEGER PRIMARY KEY AUTOINCREMENT,
      ts          INTEGER NOT NULL,
      method      TEXT NOT NULL,
      url         TEXT NOT NULL,
      req_headers TEXT,
      req_body    BLOB,
      status      INTEGER,
      res_headers TEXT,
      res_body    BLOB,
      mocked      INTEGER NOT NULL DEFAULT 0,
      duration    INTEGER
    );

    CREATE TABLE IF NOT EXISTS mocks (
      id       INTEGER PRIMARY KEY AUTOINCREMENT,
      url      TEXT NOT NULL UNIQUE,
      response TEXT NOT NULL
    );
  `)
}

export function insertRequest(log: Omit<RequestLog, 'id'>): number {
  const stmt = getDb().prepare(`
    INSERT INTO requests (ts, method, url, req_headers, req_body, status, res_headers, res_body, mocked, duration)
    VALUES (@ts, @method, @url, @req_headers, @req_body, @status, @res_headers, @res_body, @mocked, @duration)
  `)
  const result = stmt.run(log)
  return result.lastInsertRowid as number
}

export function queryRequests(filter?: string): RequestLog[] {
  const db = getDb()
  if (filter) {
    return db.prepare('SELECT * FROM requests WHERE url LIKE ? ORDER BY ts DESC LIMIT 200').all(`%${filter}%`) as RequestLog[]
  }
  return db.prepare('SELECT * FROM requests ORDER BY ts DESC LIMIT 200').all() as RequestLog[]
}

export function clearRequests(): void {
  getDb().prepare('DELETE FROM requests').run()
}

export function upsertMock(url: string, response: string): void {
  getDb().prepare('INSERT OR REPLACE INTO mocks (url, response) VALUES (?, ?)').run(url, response)
}

export function deleteMock(url: string): void {
  getDb().prepare('DELETE FROM mocks WHERE url = ?').run(url)
}

export function listMocks(): MockRule[] {
  return getDb().prepare('SELECT * FROM mocks').all() as MockRule[]
}

export function getMock(url: string): MockRule | undefined {
  return getDb().prepare('SELECT * FROM mocks WHERE url = ?').get(url) as MockRule | undefined
}

export function closeDb(): void {
  if (db) {
    db.close()
    db = null
  }
}
