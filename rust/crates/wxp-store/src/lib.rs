use std::sync::{Arc, Mutex};

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

#[derive(Clone, Default)]
pub struct StoreWriter {
    logs: Arc<Mutex<Vec<RequestLog>>>,
}

impl StoreWriter {
    pub fn memory() -> Result<Self, &'static str> {
        Ok(Self::default())
    }

    pub fn insert_request(&self, log: RequestLog) -> Result<(), &'static str> {
        self.logs.lock().map_err(|_| "lock_error")?.push(log);
        Ok(())
    }

    pub fn list_requests(
        &self,
        limit: usize,
        _with_body: bool,
    ) -> Result<Vec<RequestLog>, &'static str> {
        let logs = self.logs.lock().map_err(|_| "lock_error")?;
        Ok(logs.iter().rev().take(limit).cloned().collect())
    }
}
