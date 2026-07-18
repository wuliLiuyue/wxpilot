// SPDX-License-Identifier: AGPL-3.0-or-later
// wxpilot - 面向 AI Agent 的微信小程序自动化 CLI
// Copyright (C) 2026  wuliLiuyue
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    fs, io,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, Result};

#[derive(Clone, Debug)]
pub struct RequestLog {
    pub method: String,
    pub url: String,
    pub req_body: Option<String>,
    pub res_body: Option<String>,
    pub status: Option<u16>,
    pub ts: i64,
    pub mocked: bool,
}

impl RequestLog {
    pub fn new(
        method: &str,
        url: &str,
        req_body: Option<String>,
        res_body: Option<String>,
    ) -> Self {
        let truncate = |body: Option<String>| {
            body.map(|value| {
                if value.len() > 2000 {
                    let mut end = 1998.min(value.len());
                    while end > 0 && !value.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}…", &value[..end])
                } else {
                    value
                }
            })
        };

        Self {
            method: method.into(),
            url: url.into(),
            req_body: truncate(req_body),
            res_body: truncate(res_body),
            status: None,
            ts: current_timestamp_ms(),
            mocked: false,
        }
    }

    pub fn with_meta(
        method: &str,
        url: &str,
        req_body: Option<String>,
        res_body: Option<String>,
        status: Option<u16>,
        mocked: bool,
    ) -> Self {
        Self {
            status,
            mocked,
            ..Self::new(method, url, req_body, res_body)
        }
    }
}

#[derive(Clone)]
pub struct StoreWriter {
    conn: Arc<Mutex<Connection>>,
}

impl StoreWriter {
    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn insert_request(&self, log: RequestLog) -> Result<()> {
        let conn = self.conn.lock().expect("store connection lock poisoned");
        conn.execute(
            "INSERT INTO requests (method, url, req_body, res_body, status, ts, mocked)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                log.method,
                log.url,
                log.req_body,
                log.res_body,
                log.status.map(i64::from),
                log.ts,
                if log.mocked { 1_i64 } else { 0_i64 }
            ],
        )?;
        Ok(())
    }

    pub fn list_requests(&self, limit: usize, with_body: bool) -> Result<Vec<RequestLog>> {
        self.list_requests_filtered(limit, with_body, None)
    }

    pub fn list_requests_filtered(
        &self,
        limit: usize,
        with_body: bool,
        filter: Option<&str>,
    ) -> Result<Vec<RequestLog>> {
        let conn = self.conn.lock().expect("store connection lock poisoned");
        let limit = i64::try_from(limit).map_err(|_| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::new(
                io::ErrorKind::InvalidInput,
                "request limit exceeds i64::MAX",
            )))
        })?;
        let filter = filter.map(str::to_string);
        let mut stmt = conn.prepare(
            "SELECT method, url, req_body, res_body, status, ts, mocked
             FROM requests
             WHERE (?2 IS NULL OR instr(url, ?2) > 0)
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit, filter], |row| {
            let req_body = if with_body { row.get(2)? } else { None };
            let res_body = if with_body { row.get(3)? } else { None };
            let status = row
                .get::<_, Option<i64>>(4)?
                .map(u16::try_from)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Integer,
                        Box::new(error),
                    )
                })?;

            Ok(RequestLog {
                method: row.get(0)?,
                url: row.get(1)?,
                req_body,
                res_body,
                status,
                ts: row.get(5)?,
                mocked: row.get::<_, i64>(6)? != 0,
            })
        })?;

        rows.collect()
    }

    pub fn clear_requests(&self) -> Result<()> {
        let conn = self.conn.lock().expect("store connection lock poisoned");
        conn.execute("DELETE FROM requests", [])?;
        Ok(())
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS requests (
            id INTEGER PRIMARY KEY,
            method TEXT NOT NULL,
            url TEXT NOT NULL,
            req_body TEXT,
            res_body TEXT,
            status INTEGER,
            ts INTEGER NOT NULL DEFAULT 0,
            mocked INTEGER NOT NULL DEFAULT 0
        );",
    )?;
    ensure_column(conn, "status", "INTEGER")?;
    ensure_column(conn, "ts", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "mocked", "INTEGER NOT NULL DEFAULT 0")?;
    Ok(())
}

fn current_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as i64
}

fn ensure_column(conn: &Connection, column: &str, definition: &str) -> Result<()> {
    if has_column(conn, column)? {
        return Ok(());
    }

    conn.execute(
        &format!("ALTER TABLE requests ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}

fn has_column(conn: &Connection, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(requests)")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in rows {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}
