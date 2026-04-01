use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, Result};

#[derive(Clone, Debug)]
pub struct RequestLog {
    pub method: String,
    pub url: String,
    pub req_body: Option<String>,
    pub res_body: Option<String>,
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
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                req_body TEXT,
                res_body TEXT
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_request(&self, log: RequestLog) -> Result<()> {
        let conn = self.conn.lock().expect("store connection lock poisoned");
        conn.execute(
            "INSERT INTO requests (method, url, req_body, res_body) VALUES (?1, ?2, ?3, ?4)",
            params![log.method, log.url, log.req_body, log.res_body],
        )?;
        Ok(())
    }

    pub fn list_requests(&self, limit: usize, with_body: bool) -> Result<Vec<RequestLog>> {
        let conn = self.conn.lock().expect("store connection lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT method, url, req_body, res_body
             FROM requests
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let req_body = if with_body { row.get(2)? } else { None };
            let res_body = if with_body { row.get(3)? } else { None };

            Ok(RequestLog {
                method: row.get(0)?,
                url: row.get(1)?,
                req_body,
                res_body,
            })
        })?;

        rows.collect()
    }

    pub fn clear_requests(&self) -> Result<()> {
        let conn = self.conn.lock().expect("store connection lock poisoned");
        conn.execute("DELETE FROM requests", [])?;
        Ok(())
    }
}
